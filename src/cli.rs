//! `prunr check [--json | --github] [--since <ref>]`, `prunr baseline`, `prunr init` and
//! `prunr schema`.

use std::collections::BTreeSet;
use std::io::Write;
use std::path::Path;

use crate::baseline;
use crate::checks::{Finding, check};
use crate::config::Config;
use crate::project::{FileId, Project};
use crate::report;

pub const USAGE: &str = "usage: prunr check [--json | --github | --sarif | --codeclimate \
     | --markdown] [--production] [--since <ref>] | prunr fix [--dry-run] \
     | prunr why <file | file:export | package> | prunr baseline | prunr init | prunr schema \
     | prunr --version";

/// How `prunr check` prints its findings.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Format {
    Human,
    Json,
    Github,
    Sarif,
    CodeClimate,
    Markdown,
}

struct CheckOptions<'a> {
    format: Format,
    /// Only report what goes to production (`--production`).
    production: bool,
    /// Only report findings that this git commit does not have.
    since: Option<&'a str>,
}

/// The options of `prunr check`, or `None` when they are not valid.
fn check_options<'a>(args: &[&'a str]) -> Option<CheckOptions<'a>> {
    let mut options = CheckOptions {
        format: Format::Human,
        production: false,
        since: None,
    };
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let format = match *arg {
            "--json" => Format::Json,
            "--github" => Format::Github,
            "--sarif" => Format::Sarif,
            "--codeclimate" => Format::CodeClimate,
            "--markdown" => Format::Markdown,
            "--production" => {
                options.production = true;
                continue;
            }
            "--since" => {
                options.since = Some(args.next()?);
                continue;
            }
            _ => return None,
        };
        if options.format != Format::Human {
            return None;
        }
        options.format = format;
    }
    Some(options)
}

/// Runs the command in `root` and returns the exit code: 0 without errors, 1 with findings
/// that are errors, 2 when the check could not run.
pub fn run(args: &[String], root: &Path, out: &mut impl Write, err: &mut impl Write) -> u8 {
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match args.as_slice() {
        ["check", rest @ ..] if let Some(options) = check_options(rest) => {
            check_command(root, &options, out)
        }
        ["fix"] => fix_command(root, false, out),
        ["fix", "--dry-run"] => fix_command(root, true, out),
        ["baseline"] => baseline_command(root, out),
        ["why", target] => why_command(root, target, out),
        ["init"] => crate::init::init(root).map(|created| {
            let from = match &created.from {
                Some(from) => format!(" from {from}"),
                None => String::new(),
            };
            let _ = writeln!(out, "created {}{from}", crate::config::FILE_NAME);
            if !created.left_out.is_empty() {
                let left_out = created.left_out.join(", ");
                let _ = writeln!(out, "not translated (no equivalent in prunr): {left_out}");
            }
            0
        }),
        ["--version"] => {
            let _ = writeln!(out, "prunr {}", env!("CARGO_PKG_VERSION"));
            Ok(0)
        }
        ["schema"] => {
            let _ = out.write_all(crate::config::json_schema().as_bytes());
            Ok(0)
        }
        _ => {
            let _ = writeln!(err, "{USAGE}");
            return 2;
        }
    };
    result.unwrap_or_else(|message| {
        let _ = writeln!(err, "error: {message}");
        2
    })
}

fn check_command(root: &Path, options: &CheckOptions, out: &mut impl Write) -> Result<u8, String> {
    let config = Config::load(root).map_err(|e| e.to_string())?;
    let (project, production, tests) = graph(root, &config)?;
    let mut findings = check(&project, &production, &tests);
    let mut known = match &config.baseline {
        Some(path) => baseline::read(&root.join(path))?,
        None => Vec::new(),
    };
    if options.production {
        let keep = crate::checks::in_production(&project, &production);
        findings.retain(&keep);
        known.retain(&keep);
    }
    let mut diff = baseline::diff(findings, &known);
    if let Some(base) = options.since {
        let before = findings_at(root, base, &config)?;
        diff.fresh.retain(|finding| !before.contains(finding));
    }
    let text = match options.format {
        Format::Human => report::human(&diff.fresh, &diff.stale),
        Format::Json => report::json(&diff.fresh, &diff.stale),
        Format::Github => report::github(&diff.fresh, &diff.stale, &project),
        Format::Sarif => report::sarif(&diff.fresh, &diff.stale, &project),
        Format::CodeClimate => report::codeclimate(&diff.fresh, &diff.stale, &project),
        Format::Markdown => report::markdown(&diff.fresh, &diff.stale, &project),
    };
    let _ = out.write_all(text.as_bytes());
    let failed = !diff.stale.is_empty() || diff.fresh.iter().any(|f| !f.is_warning());
    Ok(u8::from(failed))
}

/// How many rounds `prunr fix` makes at most: removing code can leave more code unused.
const FIX_ROUNDS: usize = 10;

fn fix_command(root: &Path, dry_run: bool, out: &mut impl Write) -> Result<u8, String> {
    let config = Config::load(root).map_err(|e| e.to_string())?;
    let known = match &config.baseline {
        Some(path) => baseline::read(&root.join(path))?,
        None => Vec::new(),
    };
    let verb = if dry_run { "would fix" } else { "fixed" };
    let mut fixed_any = false;
    for _ in 0..FIX_ROUNDS {
        let (project, findings) = analyze(root, &config)?;
        let fresh = baseline::diff(findings, &known).fresh;
        let plan = crate::fix::plan(&project, &fresh);
        if plan.fixed.is_empty() {
            break;
        }
        for finding in &plan.fixed {
            let _ = writeln!(out, "{verb} {}", report::line(finding));
        }
        if dry_run {
            return Ok(0);
        }
        for file in &plan.delete {
            let path = root.join(file);
            changed(&path, std::fs::remove_file(&path))?;
        }
        for (file, text) in &plan.files {
            let path = root.join(file);
            changed(&path, std::fs::write(&path, text))?;
        }
        for (manifest, text) in &plan.manifests {
            let path = root.join(manifest);
            changed(&path, std::fs::write(&path, text))?;
        }
        fixed_any = true;
    }
    if fixed_any {
        let _ = writeln!(out);
    }
    let (_, findings) = analyze(root, &config)?;
    let diff = baseline::diff(findings, &known);
    let _ = out.write_all(report::human(&diff.fresh, &diff.stale).as_bytes());
    let failed = !diff.stale.is_empty() || diff.fresh.iter().any(|f| !f.is_warning());
    Ok(u8::from(failed))
}

/// The result of changing the file at `path`, with the path in the error.
fn changed(path: &Path, result: std::io::Result<()>) -> Result<(), String> {
    result.map_err(|e| format!("{}: {e}", path.display()))
}

fn baseline_command(root: &Path, out: &mut impl Write) -> Result<u8, String> {
    let config = Config::load(root).map_err(|e| e.to_string())?;
    let (_, findings) = analyze(root, &config)?;
    let Some(path) = &config.baseline else {
        return Err(format!(
            "set baseline in {} to use prunr baseline",
            crate::config::FILE_NAME
        ));
    };
    let count = baseline::write(&root.join(path), &findings)?;
    let _ = writeln!(out, "wrote {path}: {count} findings");
    Ok(0)
}

fn why_command(root: &Path, target: &str, out: &mut impl Write) -> Result<u8, String> {
    let config = Config::load(root).map_err(|e| e.to_string())?;
    let (project, production, tests) = graph(root, &config)?;
    let target = target.trim_start_matches("./");
    let answer = crate::why::why(&project, &production, &tests, target)?;
    let _ = out.write_all(answer.as_bytes());
    Ok(0)
}

/// The project at `root`, with its production and test entries under `config`.
fn graph(
    root: &Path,
    config: &Config,
) -> Result<(Project, BTreeSet<FileId>, BTreeSet<FileId>), String> {
    let mut project = Project::load_with(root, &config.plugins).map_err(|e| e.to_string())?;
    project.ignore_packages(&config.dependencies.ignore);
    let mut production = project.entries(&config.entries.production)?;
    production.extend(project.production_files.iter().cloned());
    let mut tests = project.entries(&config.entries.test)?;
    tests.extend(project.test_files.iter().cloned());
    Ok((project, production, tests))
}

/// The project at `root` and its findings under `config`.
fn analyze(root: &Path, config: &Config) -> Result<(Project, Vec<Finding>), String> {
    let (project, production, tests) = graph(root, config)?;
    let findings = check(&project, &production, &tests);
    Ok((project, findings))
}

/// The findings the project at `root` had at the git commit `base`, under the same `config`.
/// The commit is checked out in a temporary worktree, which shares the project's
/// `node_modules`. A project that did not exist then had no findings.
fn findings_at(root: &Path, base: &str, config: &Config) -> Result<Vec<Finding>, String> {
    let toplevel = git(root, &["rev-parse", "--show-toplevel"])
        .map_err(|e| format!("--since needs a git repository: {e}"))?;
    let toplevel = crate::resolve::canonical(Path::new(toplevel.trim()));
    let relative = crate::resolve::canonical(root)
        .strip_prefix(&toplevel)
        .expect("the project is inside its repository")
        .to_path_buf();
    let worktree = std::env::temp_dir().join(format!("prunr-since-{}", std::process::id()));
    let checkout = worktree.to_string_lossy().to_string();
    git(
        &toplevel,
        &["worktree", "add", "--detach", "--quiet", &checkout, base],
    )
    .map_err(|e| format!("cannot check out {base}: {e}"))?;
    let project = worktree.join(relative);
    let modules = root.join("node_modules");
    #[cfg(unix)]
    if modules.is_dir() && project.is_dir() {
        let _ = std::os::unix::fs::symlink(&modules, project.join("node_modules"));
    }
    #[cfg(not(unix))]
    let _ = modules;
    let findings = if project.is_dir() {
        analyze(&project, config).map(|(_, findings)| findings)
    } else {
        Ok(Vec::new())
    };
    let _ = git(&toplevel, &["worktree", "remove", "--force", &checkout]);
    findings
}

/// Runs git in `dir`: its output, or its error message.
fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|e| e.to_string())?;
    let text = |bytes: &[u8]| String::from_utf8_lossy(bytes).trim().to_string();
    if output.status.success() {
        Ok(text(&output.stdout))
    } else {
        Err(text(&output.stderr))
    }
}

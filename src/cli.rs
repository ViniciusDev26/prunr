//! `prunr check [--json | --github] [--since <ref>]`, `prunr baseline`, `prunr init` and
//! `prunr schema`.

use std::io::Write;
use std::path::Path;

use crate::baseline;
use crate::checks::{Finding, check};
use crate::config::Config;
use crate::project::Project;
use crate::report;

pub const USAGE: &str = "usage: prunr check [--json | --github] [--since <ref>] | prunr baseline \
     | prunr init | prunr schema | prunr --version";

/// How `prunr check` prints its findings.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Format {
    Human,
    Json,
    Github,
}

struct CheckOptions<'a> {
    format: Format,
    /// Only report findings that this git commit does not have.
    since: Option<&'a str>,
}

/// The options of `prunr check`, or `None` when they are not valid.
fn check_options<'a>(args: &[&'a str]) -> Option<CheckOptions<'a>> {
    let mut options = CheckOptions {
        format: Format::Human,
        since: None,
    };
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let format = match *arg {
            "--json" => Format::Json,
            "--github" => Format::Github,
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
        ["baseline"] => baseline_command(root, out),
        ["init"] => crate::init::init(root).map(|_| {
            let _ = writeln!(out, "created {}", crate::config::FILE_NAME);
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
    let (project, findings) = analyze(root, &config)?;
    let known = match &config.baseline {
        Some(path) => baseline::read(&root.join(path))?,
        None => Vec::new(),
    };
    let mut diff = baseline::diff(findings, &known);
    if let Some(base) = options.since {
        let before = findings_at(root, base, &config)?;
        diff.fresh.retain(|finding| !before.contains(finding));
    }
    let text = match options.format {
        Format::Human => report::human(&diff.fresh, &diff.stale),
        Format::Json => report::json(&diff.fresh, &diff.stale),
        Format::Github => {
            let package_json =
                std::fs::read_to_string(root.join("package.json")).unwrap_or_default();
            report::github(&diff.fresh, &diff.stale, &project, &package_json)
        }
    };
    let _ = out.write_all(text.as_bytes());
    let failed = !diff.stale.is_empty() || diff.fresh.iter().any(|f| !f.is_warning());
    Ok(u8::from(failed))
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

/// The project at `root` and its findings under `config`.
fn analyze(root: &Path, config: &Config) -> Result<(Project, Vec<Finding>), String> {
    let mut project = Project::load(root).map_err(|e| e.to_string())?;
    project.ignore_packages(&config.dependencies.ignore);
    let production = project.entries(&config.entries.production)?;
    let mut tests = project.entries(&config.entries.test)?;
    tests.extend(project.test_files.iter().cloned());
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

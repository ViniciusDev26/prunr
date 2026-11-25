//! `prunr check [--json]`, `prunr baseline`, `prunr init` and `prunr schema`.

use std::io::Write;
use std::path::Path;

use crate::baseline;
use crate::checks::check;
use crate::config::Config;
use crate::project::Project;
use crate::report;

pub const USAGE: &str = "usage: prunr check [--json] | prunr baseline | prunr init | prunr schema";

/// Runs the command in `root` and returns the exit code: 0 without errors, 1 with findings
/// that are errors, 2 when the check could not run.
pub fn run(args: &[String], root: &Path, out: &mut impl Write, err: &mut impl Write) -> u8 {
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match args.as_slice() {
        ["check"] => check_command(root, false, out),
        ["check", "--json"] => check_command(root, true, out),
        ["baseline"] => baseline_command(root, out),
        ["init"] => crate::init::init(root).map(|_| {
            let _ = writeln!(out, "created {}", crate::config::FILE_NAME);
            0
        }),
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

fn check_command(root: &Path, json: bool, out: &mut impl Write) -> Result<u8, String> {
    let (config, findings) = findings(root)?;
    let known = match &config.baseline {
        Some(path) => baseline::read(&root.join(path))?,
        None => Vec::new(),
    };
    let diff = baseline::diff(findings, &known);
    let text = if json {
        report::json(&diff.fresh, &diff.stale)
    } else {
        report::human(&diff.fresh, &diff.stale)
    };
    let _ = out.write_all(text.as_bytes());
    let failed = !diff.stale.is_empty() || diff.fresh.iter().any(|f| !f.is_warning());
    Ok(u8::from(failed))
}

fn baseline_command(root: &Path, out: &mut impl Write) -> Result<u8, String> {
    let (config, findings) = findings(root)?;
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

fn findings(root: &Path) -> Result<(Config, Vec<crate::checks::Finding>), String> {
    let config = Config::load(root).map_err(|e| e.to_string())?;
    let mut project = Project::load(root).map_err(|e| e.to_string())?;
    project.ignore_packages(&config.dependencies.ignore);
    let production = project.entries(&config.entries.production)?;
    let tests = project.entries(&config.entries.test)?;
    let findings = check(&project, &production, &tests);
    Ok((config, findings))
}

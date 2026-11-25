//! `prunr check [--json]`, `prunr init` and `prunr schema`.

use std::io::Write;
use std::path::Path;

use crate::checks::check;
use crate::config::Config;
use crate::project::Project;
use crate::report;

pub const USAGE: &str = "usage: prunr check [--json] | prunr init | prunr schema";

/// Runs the command in `root` and returns the exit code: 0 without errors, 1 with findings
/// that are errors, 2 when the check could not run.
pub fn run(args: &[String], root: &Path, out: &mut impl Write, err: &mut impl Write) -> u8 {
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let json = match args.as_slice() {
        ["check"] => false,
        ["check", "--json"] => true,
        ["init"] => {
            return match crate::init::init(root) {
                Ok(_) => {
                    let _ = writeln!(out, "created {}", crate::config::FILE_NAME);
                    0
                }
                Err(message) => {
                    let _ = writeln!(err, "error: {message}");
                    2
                }
            };
        }
        ["schema"] => {
            let _ = out.write_all(crate::config::json_schema().as_bytes());
            return 0;
        }
        _ => {
            let _ = writeln!(err, "{USAGE}");
            return 2;
        }
    };
    match findings(root) {
        Ok(findings) => {
            let text = if json {
                report::json(&findings)
            } else {
                report::human(&findings)
            };
            let _ = out.write_all(text.as_bytes());
            u8::from(findings.iter().any(|f| !f.is_warning()))
        }
        Err(message) => {
            let _ = writeln!(err, "error: {message}");
            2
        }
    }
}

fn findings(root: &Path) -> Result<Vec<crate::checks::Finding>, String> {
    let config = Config::load(root).map_err(|e| e.to_string())?;
    let mut project = Project::load(root).map_err(|e| e.to_string())?;
    project.ignore_packages(&config.dependencies.ignore);
    let production = project.entries(&config.entries.production)?;
    let tests = project.entries(&config.entries.test)?;
    Ok(check(&project, &production, &tests))
}

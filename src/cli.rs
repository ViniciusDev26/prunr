//! `prunr check [--json]`.

use std::io::Write;
use std::path::Path;

use crate::checks::check;
use crate::config::Config;
use crate::project::Project;
use crate::report;

pub const USAGE: &str = "usage: prunr check [--json]";

/// Runs the command in `root` and returns the exit code: 0 without errors, 1 with findings
/// that are errors, 2 when the check could not run.
pub fn run(args: &[String], root: &Path, out: &mut impl Write, err: &mut impl Write) -> u8 {
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let json = match args.as_slice() {
        ["check"] => false,
        ["check", "--json"] => true,
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
    let project = Project::load(root).map_err(|e| e.to_string())?;
    let mut entries = Vec::new();
    for entry in &config.entries {
        let Some(id) = project.id_of(entry) else {
            return Err(format!("entry is not a .js or .cjs file: {entry}"));
        };
        entries.push(id);
    }
    Ok(check(&project, &entries))
}

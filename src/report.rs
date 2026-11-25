//! How findings are printed.

use serde::Serialize;

use crate::checks::Finding;

/// One line per finding and a summary, for people.
pub fn human(findings: &[Finding]) -> String {
    if findings.is_empty() {
        return "No findings.\n".into();
    }
    let mut out = String::new();
    for finding in findings {
        let (kind, detail) = describe(finding);
        let warning = if finding.is_warning() {
            " (warning)"
        } else {
            ""
        };
        out.push_str(&format!("{kind:<20}{detail}{warning}\n"));
    }
    let warnings = findings.iter().filter(|f| f.is_warning()).count();
    let errors = findings.len() - warnings;
    out.push_str(&format!("\nerrors: {errors}, warnings: {warnings}\n"));
    out
}

fn describe(finding: &Finding) -> (&'static str, String) {
    match finding {
        Finding::UnusedFile { file, .. } => ("unused-file", file.clone()),
        Finding::UnusedExport { file, symbol, .. } => {
            ("unused-export", format!("{file}: {symbol}"))
        }
        Finding::UnusedDependency { package } => ("unused-dependency", package.clone()),
        Finding::MisplacedDependency { .. } => todo!(),
        Finding::MissingDependency { file, package } => {
            ("missing-dependency", format!("{file}: {package}"))
        }
        Finding::CircularImport { cycle } => ("circular-import", cycle.join(" -> ")),
        Finding::UnresolvedRequire { file, specifier } => {
            ("unresolved-require", format!("{file}: {specifier}"))
        }
    }
}

/// `{ "findings": [...] }`, for machines.
pub fn json(findings: &[Finding]) -> String {
    #[derive(Serialize)]
    struct Report<'a> {
        findings: &'a [Finding],
    }
    let text = serde_json::to_string_pretty(&Report { findings }).expect("findings serialize");
    text + "\n"
}

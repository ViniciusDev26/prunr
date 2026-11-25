//! How findings are printed.

use serde::Serialize;

use crate::checks::Finding;

/// One line per finding and a summary, for people.
/// `stale` are baseline entries no longer found.
pub fn human(findings: &[Finding], stale: &[Finding]) -> String {
    if findings.is_empty() && stale.is_empty() {
        return "No findings.\n".into();
    }
    let mut out = String::new();
    for finding in findings {
        let (kind, detail) = describe(finding);
        let note = match finding {
            Finding::UnresolvedRequire { .. } => " (warning)",
            Finding::UnusedFile {
                test_only: true, ..
            }
            | Finding::UnusedExport {
                test_only: true, ..
            } => " (only tests use it)",
            _ => "",
        };
        out.push_str(&format!("{kind:<22}{detail}{note}\n"));
    }
    for finding in stale {
        let (kind, detail) = describe(finding);
        out.push_str(&format!(
            "{:<22}{kind} {detail} (fixed: run `prunr baseline`)\n",
            "stale-baseline"
        ));
    }
    let warnings = findings.iter().filter(|f| f.is_warning()).count();
    let errors = findings.len() - warnings + stale.len();
    out.push_str(&format!("\nerrors: {errors}, warnings: {warnings}\n"));
    out
}

fn describe(finding: &Finding) -> (&'static str, String) {
    match finding {
        Finding::UnusedFile { file, .. } => ("unused-file", file.clone()),
        Finding::UnusedExport { file, symbol, .. } => {
            ("unused-export", format!("{file}: {symbol}"))
        }
        Finding::UnusedImport {
            file,
            name,
            specifier,
        } => ("unused-import", format!("{file}: {name} from {specifier}")),
        Finding::UnusedIgnore { file, line } => {
            ("unused-ignore", format!("{file}:{line}: ignores nothing"))
        }
        Finding::UnusedDependency { package } => ("unused-dependency", package.clone()),
        Finding::MisplacedDependency { package } => (
            "misplaced-dependency",
            format!("{package}: only tests use it, move it to devDependencies"),
        ),
        Finding::MissingDependency { file, package } => {
            ("missing-dependency", format!("{file}: {package}"))
        }
        Finding::CircularImport { cycle } => ("circular-import", cycle.join(" -> ")),
        Finding::UnresolvedRequire { file, specifier } => {
            ("unresolved-require", format!("{file}: {specifier}"))
        }
    }
}

/// `{ "findings": [...], "stale": [...] }`, for machines. `stale` is left out when empty.
pub fn json(findings: &[Finding], stale: &[Finding]) -> String {
    #[derive(Serialize)]
    struct Report<'a> {
        findings: &'a [Finding],
        #[serde(skip_serializing_if = "<[Finding]>::is_empty")]
        stale: &'a [Finding],
    }
    let report = Report { findings, stale };
    let text = serde_json::to_string_pretty(&report).expect("findings serialize");
    text + "\n"
}

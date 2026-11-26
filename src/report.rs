//! How findings are printed.

use serde::Serialize;

use crate::checks::Finding;
use crate::project::Project;

/// One line per finding and a summary, for people.
/// `stale` are baseline entries no longer found.
pub fn human(findings: &[Finding], stale: &[Finding]) -> String {
    if findings.is_empty() && stale.is_empty() {
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
        out.push_str(&format!("{kind:<22}{detail}{}{warning}\n", note(finding)));
    }
    for finding in stale {
        out.push_str(&format!(
            "{:<22}{}\n",
            "stale-baseline",
            stale_entry(finding)
        ));
    }
    out + &summary(findings, stale)
}

/// A finding on one line, as the human report prints it, without the warning mark.
pub fn line(finding: &Finding) -> String {
    let (kind, detail) = describe(finding);
    format!("{kind:<22}{detail}{}", note(finding))
}

/// GitHub Actions annotations: one per finding, at its file and line, and the summary.
pub fn github(findings: &[Finding], stale: &[Finding], project: &Project) -> String {
    if findings.is_empty() && stale.is_empty() {
        return "No findings.\n".into();
    }
    let mut out = String::new();
    for finding in findings {
        let (kind, detail) = describe(finding);
        let level = if finding.is_warning() {
            "warning"
        } else {
            "error"
        };
        let (file, line) = position(finding, project);
        let mut properties = format!("file={}", property(&file));
        if let Some(line) = line {
            properties.push_str(&format!(",line={line}"));
        }
        out.push_str(&format!(
            "::{level} {properties},title={kind}::{}\n",
            data(&format!("{detail}{}", note(finding)))
        ));
    }
    for finding in stale {
        out.push_str(&format!(
            "::error title=stale-baseline::{}\n",
            data(&stale_entry(finding))
        ));
    }
    out + &summary(findings, stale)
}

/// Where a finding is: its file and, when it has one, its line.
fn position(finding: &Finding, project: &Project) -> (String, Option<u32>) {
    let facts = |file: &String| &project.files[file].facts;
    let manifest = |package: &str, workspace: &Option<String>| {
        let file = manifest_path(workspace);
        let key = format!("\"{package}\"");
        let line = std::fs::read_to_string(project.root.join(&file))
            .unwrap_or_default()
            .lines()
            .position(|line| line.contains(&key))
            .map(|index| index as u32 + 1);
        (file, line)
    };
    match finding {
        Finding::UnusedFile { file, .. } | Finding::UnparsedFile { file, .. } => {
            (file.clone(), None)
        }
        Finding::UnusedExport { file, symbol, .. } => {
            let path: Vec<String> = symbol.split('.').map(str::to_string).collect();
            (file.clone(), facts(file).export_lines.get(&path).copied())
        }
        Finding::UnusedImport {
            file, specifier, ..
        }
        | Finding::UnresolvedRequire { file, specifier } => (
            file.clone(),
            facts(file).require_lines.get(specifier).copied(),
        ),
        Finding::UnusedIgnore { file, line } => (file.clone(), Some(*line)),
        Finding::UnusedDependency { package, workspace }
        | Finding::MisplacedDependency { package, workspace } => manifest(package, workspace),
        Finding::MissingDependency { file, package } => {
            let line = facts(file)
                .require_lines
                .iter()
                .filter(|(specifier, _)| crate::resolve::package_name(specifier) == package)
                .map(|(_, line)| *line)
                .min();
            (file.clone(), line)
        }
        Finding::CircularImport { cycle } => {
            let lines = &project.files[&cycle[0]].edge_lines;
            (cycle[0].clone(), lines.get(&cycle[1]).copied())
        }
    }
}

/// The `package.json` of a package of the repository, relative to the root.
pub fn manifest_path(workspace: &Option<String>) -> String {
    match workspace {
        Some(dir) => format!("{dir}/package.json"),
        None => "package.json".to_string(),
    }
}

/// A package name, with the package of the repository that declares it, in a monorepo.
fn in_workspace(package: &str, workspace: &Option<String>) -> String {
    match workspace {
        Some(dir) => format!("{package} ({dir})"),
        None => package.to_string(),
    }
}

/// An annotation's message, escaped as GitHub Actions expects.
fn data(text: &str) -> String {
    text.replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

/// An annotation's property value, escaped as GitHub Actions expects.
fn property(text: &str) -> String {
    data(text).replace(':', "%3A").replace(',', "%2C")
}

/// What a finding adds after its detail.
fn note(finding: &Finding) -> &'static str {
    match finding {
        Finding::UnusedFile {
            test_only: true, ..
        }
        | Finding::UnusedExport {
            test_only: true, ..
        } => " (only tests use it)",
        _ => "",
    }
}

fn stale_entry(finding: &Finding) -> String {
    let (kind, detail) = describe(finding);
    format!("{kind} {detail} (fixed: run `prunr baseline`)")
}

fn summary(findings: &[Finding], stale: &[Finding]) -> String {
    let warnings = findings.iter().filter(|f| f.is_warning()).count();
    let errors = findings.len() - warnings + stale.len();
    format!("\nerrors: {errors}, warnings: {warnings}\n")
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
        Finding::UnusedDependency { package, workspace } => {
            ("unused-dependency", in_workspace(package, workspace))
        }
        Finding::MisplacedDependency { package, workspace } => (
            "misplaced-dependency",
            format!(
                "{}: only tests use it, move it to devDependencies",
                in_workspace(package, workspace)
            ),
        ),
        Finding::MissingDependency { file, package } => {
            ("missing-dependency", format!("{file}: {package}"))
        }
        Finding::CircularImport { cycle } => ("circular-import", cycle.join(" -> ")),
        Finding::UnresolvedRequire { file, specifier } => {
            ("unresolved-require", format!("{file}: {specifier}"))
        }
        Finding::UnparsedFile { file, message } => ("unparsed-file", format!("{file}: {message}")),
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

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

/// What each kind of finding means, for the reports that describe their rules.
const RULES: &[(&str, &str)] = &[
    ("unused-file", "No production entry reaches the file."),
    ("unused-export", "No production code uses the export."),
    (
        "unused-enum-member",
        "No production code reads the member of the exported enum.",
    ),
    (
        "unused-class-member",
        "No production code calls the method of the exported class.",
    ),
    (
        "duplicate-export",
        "The same binding is exported under more than one name.",
    ),
    (
        "unused-import",
        "A require or import brings in a name the file never uses.",
    ),
    (
        "unused-ignore",
        "A prunr-ignore comment that no longer ignores anything.",
    ),
    (
        "unused-dependency",
        "Nothing imports the package and no script runs it.",
    ),
    (
        "misplaced-dependency",
        "The package is in dependencies, but only the tests use it.",
    ),
    (
        "missing-dependency",
        "The code imports a package package.json does not declare.",
    ),
    (
        "missing-binary",
        "A script runs a binary of a package package.json does not declare.",
    ),
    ("circular-import", "There is an import cycle."),
    ("unresolved-require", "A require prunr cannot follow."),
    ("unparsed-file", "A file the graph reaches does not parse."),
    (
        "stale-baseline",
        "A baseline entry that is no longer found.",
    ),
];

/// Each finding (and stale baseline entry) with its kind, description, file, line and whether
/// it is a warning: what the structured reports are built from.
fn located<'f>(
    findings: &'f [Finding],
    stale: &'f [Finding],
    project: &Project,
) -> Vec<(&'static str, String, String, Option<u32>, bool)> {
    let mut items: Vec<_> = findings
        .iter()
        .map(|finding| {
            let (kind, detail) = describe(finding);
            let (file, line) = position(finding, project);
            let text = format!("{detail}{}", note(finding));
            (kind, text, file, line, finding.is_warning())
        })
        .collect();
    items.extend(stale.iter().map(|finding| {
        let entry = stale_entry(finding);
        (
            "stale-baseline",
            entry,
            crate::config::FILE_NAME.to_string(),
            None,
            false,
        )
    }));
    items
}

/// SARIF 2.1.0, for code scanning tools: one rule per kind of finding, one result per finding.
pub fn sarif(findings: &[Finding], stale: &[Finding], project: &Project) -> String {
    let rules: Vec<serde_json::Value> = RULES
        .iter()
        .map(|(id, text)| serde_json::json!({ "id": id, "shortDescription": { "text": text } }))
        .collect();
    let results: Vec<serde_json::Value> = located(findings, stale, project)
        .into_iter()
        .map(|(kind, text, file, line, warning)| {
            let mut location = serde_json::json!({ "artifactLocation": { "uri": file } });
            if let Some(line) = line {
                location["region"] = serde_json::json!({ "startLine": line });
            }
            serde_json::json!({
                "ruleId": kind,
                "level": if warning { "warning" } else { "error" },
                "message": { "text": text },
                "locations": [{ "physicalLocation": location }],
            })
        })
        .collect();
    let report = serde_json::json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": { "driver": {
                "name": "prunr",
                "version": env!("CARGO_PKG_VERSION"),
                "informationUri": "https://github.com/sugomi-dev/prunr",
                "rules": rules,
            } },
            "results": results,
        }],
    });
    serde_json::to_string_pretty(&report).expect("the report serializes") + "\n"
}

/// GitLab's Code Quality report (the Code Climate format): one issue per finding.
pub fn codeclimate(findings: &[Finding], stale: &[Finding], project: &Project) -> String {
    let issues: Vec<serde_json::Value> = located(findings, stale, project)
        .into_iter()
        .map(|(kind, text, file, line, warning)| {
            serde_json::json!({
                "type": "issue",
                "check_name": kind,
                "description": text,
                "categories": ["Clarity"],
                "severity": if warning { "minor" } else { "major" },
                "fingerprint": fingerprint(&format!("{kind}:{text}")),
                "location": { "path": file, "lines": { "begin": line.unwrap_or(1) } },
            })
        })
        .collect();
    serde_json::to_string_pretty(&issues).expect("the report serializes") + "\n"
}

/// A stable identifier for an issue: the FNV-1a hash of its kind and description, in hex.
fn fingerprint(text: &str) -> String {
    let hash = text.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
    });
    format!("{hash:016x}")
}

/// A Markdown table, for a pull request comment.
pub fn markdown(findings: &[Finding], stale: &[Finding], project: &Project) -> String {
    let items = located(findings, stale, project);
    if items.is_empty() {
        return "### prunr: no findings\n".into();
    }
    let warnings = items.iter().filter(|item| item.4).count();
    let errors = items.len() - warnings;
    let plural = |count: usize, word: &str| match count {
        1 => format!("{count} {word}"),
        _ => format!("{count} {word}s"),
    };
    let mut out = format!(
        "### prunr: {}, {}\n\n| Finding | Where | What |\n|---|---|---|\n",
        plural(errors, "error"),
        plural(warnings, "warning")
    );
    for (kind, text, file, line, warning) in items {
        let mark = if warning { " (warning)" } else { "" };
        let at = line.map_or(file.clone(), |line| format!("{file}:{line}"));
        let text = text.replace('|', "\\|");
        out.push_str(&format!("| `{kind}`{mark} | `{at}` | {text} |\n"));
    }
    out
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
        Finding::UnusedExport { file, symbol, .. }
        | Finding::UnusedEnumMember { file, symbol, .. }
        | Finding::UnusedClassMember { file, symbol, .. } => {
            let path: Vec<String> = symbol.split('.').map(str::to_string).collect();
            (file.clone(), facts(file).export_lines.get(&path).copied())
        }
        Finding::DuplicateExport { file, exports } => {
            let lines = &facts(file).export_lines;
            let line = exports
                .iter()
                .filter_map(|name| lines.get(&vec![name.clone()]))
                .max();
            (file.clone(), line.copied())
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
        Finding::MissingBinary {
            script, workspace, ..
        } => manifest(script, workspace),
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
        }
        | Finding::UnusedEnumMember {
            test_only: true, ..
        }
        | Finding::UnusedClassMember {
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
        Finding::UnusedEnumMember { file, symbol, .. } => {
            ("unused-enum-member", format!("{file}: {symbol}"))
        }
        Finding::UnusedClassMember { file, symbol, .. } => {
            ("unused-class-member", format!("{file}: {symbol}"))
        }
        Finding::DuplicateExport { file, exports } => (
            "duplicate-export",
            format!("{file}: {}", exports.join(", ")),
        ),
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
        Finding::MissingBinary {
            script,
            binary,
            package,
            workspace,
        } => (
            "missing-binary",
            format!(
                "{script}: {binary}, from {package}, which {} does not declare",
                manifest_path(workspace)
            ),
        ),
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

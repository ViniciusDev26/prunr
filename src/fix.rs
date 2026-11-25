//! `prunr fix`: what removing each finding takes, as file deletions and text edits.

use std::collections::{BTreeMap, BTreeSet};

use oxc_span::Span;

use crate::checks::Finding;
use crate::edit::{Edit, delete_lines, remove_items};
use crate::facts::{ImportParts, Removal, Specifier};
use crate::project::{FileId, Project};

/// The changes one round of `prunr fix` makes.
#[derive(Debug, Default)]
pub struct Plan {
    /// Files to delete.
    pub delete: BTreeSet<FileId>,
    /// The new text of each file to edit.
    pub files: BTreeMap<FileId, String>,
    /// The new text of `package.json`, when it changes.
    pub package_json: Option<String>,
    /// The findings the changes fix, in the order they were reported.
    pub fixed: Vec<Finding>,
}

/// What fixing `findings` takes. Findings it cannot fix by itself (a missing dependency, a
/// cycle, what only tests use) are left out.
pub fn plan(project: &Project, findings: &[Finding], package_json: &str) -> Plan {
    let mut plan = Plan::default();
    let mut removals: BTreeMap<&FileId, Vec<(&Finding, Removal)>> = BTreeMap::new();
    let mut manifest = package_json.to_string();
    for finding in findings {
        match finding {
            Finding::UnusedFile {
                file,
                test_only: false,
            } => {
                plan.delete.insert(file.clone());
                plan.fixed.push(finding.clone());
            }
            Finding::UnusedExport {
                file,
                symbol,
                test_only: false,
            } => {
                let path: Vec<String> = symbol.split('.').map(str::to_string).collect();
                if let Some(removal) = project.files[file].facts.export_removals.get(&path) {
                    removals
                        .entry(file)
                        .or_default()
                        .push((finding, removal.clone()));
                }
            }
            Finding::UnusedImport {
                file,
                name,
                specifier,
            } => {
                let key = (specifier.clone(), name.clone());
                if let Some(removal) = project.files[file].facts.import_removals.get(&key) {
                    removals
                        .entry(file)
                        .or_default()
                        .push((finding, removal.clone()));
                }
            }
            Finding::UnusedIgnore { file, line } => {
                let span = project.files[file].facts.ignore_comments[line];
                removals
                    .entry(file)
                    .or_default()
                    .push((finding, Removal::Statement(span)));
            }
            Finding::UnusedDependency { package } => {
                let removed = ["dependencies", "devDependencies"].iter().fold(
                    None,
                    |changed: Option<String>, field| {
                        let text = changed.as_deref().unwrap_or(&manifest);
                        crate::package_json::remove(text, field, package).or(changed)
                    },
                );
                plan.fixed
                    .extend(removed.is_some().then(|| finding.clone()));
                manifest = removed.unwrap_or(manifest);
            }
            Finding::MisplacedDependency { package } => {
                let moved = crate::package_json::move_to_dev(&manifest, package);
                plan.fixed.extend(moved.is_some().then(|| finding.clone()));
                manifest = moved.unwrap_or(manifest);
            }
            _ => {}
        }
    }
    if manifest != package_json {
        plan.package_json = Some(manifest);
    }
    // A file being deleted is not edited.
    let removals = removals
        .into_iter()
        .filter(|(file, _)| !plan.delete.contains(*file));
    for (file, removals) in removals {
        let source = std::fs::read_to_string(project.root.join(file)).unwrap_or_default();
        let (edits, fixed) = edits(
            &source,
            &project.files[file].facts.export_removals,
            &removals,
        );
        plan.fixed.extend(fixed);
        plan.files
            .insert(file.clone(), crate::edit::apply(&source, &edits));
    }
    plan.fixed
        .sort_by_key(|finding| findings.iter().position(|f| f == finding));
    plan
}

/// The edits that make `removals` in `source`, and the findings they fix. An `export` keyword
/// shared by several names (`export const a = 1, b = 2`) only goes when all of them do.
fn edits(
    source: &str,
    exports: &BTreeMap<Vec<String>, Removal>,
    removals: &[(&Finding, Removal)],
) -> (Vec<Edit>, Vec<Finding>) {
    let mut edits = Vec::new();
    let mut fixed = Vec::new();
    let mut lists: BTreeMap<Vec<Span>, (Option<Span>, BTreeSet<usize>)> = BTreeMap::new();
    let mut imports: BTreeMap<Span, (ImportParts, BTreeSet<usize>)> = BTreeMap::new();
    for (finding, removal) in removals {
        match removal {
            Removal::Prefix(span) => {
                let sharing = exports.values().filter(|r| *r == removal).count();
                let removing = removals.iter().filter(|(_, r)| r == removal).count();
                if sharing != removing {
                    continue;
                }
                edits.push(Edit::delete(*span));
            }
            Removal::Statement(span) => edits.push(delete_lines(source, *span)),
            Removal::Item { items, index, all } => {
                let entry = lists
                    .entry(items.clone())
                    .or_insert((*all, BTreeSet::new()));
                entry.1.insert(*index);
            }
            Removal::Import {
                statement,
                index,
                parts,
            } => {
                let entry = imports
                    .entry(*statement)
                    .or_insert((parts.clone(), BTreeSet::new()));
                entry.1.insert(*index);
            }
        }
        fixed.push((*finding).clone());
    }
    for (items, (all, removed)) in lists {
        match all {
            Some(all) if removed.len() == items.len() => edits.push(delete_lines(source, all)),
            _ => {
                let removed: Vec<usize> = removed.into_iter().collect();
                edits.extend(remove_items(source, &items, &removed));
            }
        }
    }
    for (statement, (parts, removed)) in imports {
        if removed.len() == parts.specifiers.len() {
            edits.push(delete_lines(source, statement));
        } else {
            edits.push(Edit {
                start: statement.start,
                end: statement.end,
                text: rebuild(&parts, &removed),
            });
        }
    }
    edits.sort();
    edits.dedup();
    (edits, fixed)
}

/// An ESM import without the specifiers at `removed`.
fn rebuild(parts: &ImportParts, removed: &BTreeSet<usize>) -> String {
    let kept: Vec<&(Specifier, String)> = parts
        .specifiers
        .iter()
        .enumerate()
        .filter(|(index, _)| !removed.contains(index))
        .map(|(_, specifier)| specifier)
        .collect();
    let mut bindings: Vec<String> = kept
        .iter()
        .filter(|(kind, _)| *kind != Specifier::Named)
        .map(|(_, text)| text.clone())
        .collect();
    let named: Vec<&str> = kept
        .iter()
        .filter(|(kind, _)| *kind == Specifier::Named)
        .map(|(_, text)| text.as_str())
        .collect();
    if !named.is_empty() {
        bindings.push(format!("{{ {} }}", named.join(", ")));
    }
    let semicolon = if parts.semicolon { ";" } else { "" };
    format!(
        "{} {} from {}{semicolon}",
        parts.keyword,
        bindings.join(", "),
        parts.source
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(files: &[(&str, &str)]) -> (tempfile::TempDir, Project) {
        let dir = tempfile::tempdir().unwrap();
        for (path, content) in files {
            std::fs::write(dir.path().join(path), content).unwrap();
        }
        let project = Project::load(dir.path()).unwrap();
        (dir, project)
    }

    #[test]
    fn a_rebuilt_import_keeps_its_style() {
        let parts = ImportParts {
            keyword: "import type".into(),
            specifiers: vec![
                (Specifier::Default, "D".into()),
                (Specifier::Named, "A".into()),
                (Specifier::Named, "B as C".into()),
            ],
            source: "'./types'".into(),
            semicolon: false,
        };
        assert_eq!(
            rebuild(&parts, &BTreeSet::from([1])),
            "import type D, { B as C } from './types'"
        );
    }

    #[test]
    fn a_dependency_package_json_does_not_list_is_left_alone() {
        let (_dir, project) = project(&[]);
        let findings = [
            Finding::UnusedDependency {
                package: "x".into(),
            },
            Finding::MisplacedDependency {
                package: "y".into(),
            },
        ];
        let plan = plan(&project, &findings, "{}");
        assert!(plan.fixed.is_empty());
        assert_eq!(plan.package_json, None);
    }

    #[test]
    fn a_file_to_delete_is_not_edited() {
        let (_dir, project) = project(&[("a.js", "exports.a = 1;")]);
        let findings = [
            Finding::UnusedFile {
                file: "a.js".into(),
                test_only: false,
            },
            Finding::UnusedExport {
                file: "a.js".into(),
                symbol: "a".into(),
                test_only: false,
            },
        ];
        let plan = plan(&project, &findings, "{}");
        assert_eq!(plan.delete, BTreeSet::from(["a.js".to_string()]));
        assert!(plan.files.is_empty());
        assert_eq!(plan.fixed, findings[..1]);
    }

    #[test]
    fn what_only_tests_use_is_not_fixed() {
        let (_dir, project) = project(&[("a.js", "exports.a = 1;")]);
        let findings = [
            Finding::UnusedFile {
                file: "a.js".into(),
                test_only: true,
            },
            Finding::UnusedExport {
                file: "a.js".into(),
                symbol: "a".into(),
                test_only: true,
            },
            Finding::CircularImport {
                cycle: vec!["a.js".into(), "a.js".into()],
            },
        ];
        let plan = plan(&project, &findings, "{}");
        assert!(plan.fixed.is_empty());
        assert!(plan.delete.is_empty() && plan.files.is_empty());
    }
}

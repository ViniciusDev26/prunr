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
    /// The new text of each `package.json` that changes, by its path relative to the root.
    pub manifests: BTreeMap<String, String>,
    /// The findings the changes fix, in the order they were reported.
    pub fixed: Vec<Finding>,
}

/// What fixing `findings` takes. Findings it cannot fix by itself (a missing dependency, a
/// cycle, what only tests use) are left out.
pub fn plan(project: &Project, findings: &[Finding]) -> Plan {
    let mut plan = Plan::default();
    let mut removals: BTreeMap<&FileId, Vec<(&Finding, Removal)>> = BTreeMap::new();
    // Each `package.json` as it was, and as the fixes leave it.
    let mut manifests: BTreeMap<String, (String, String)> = BTreeMap::new();
    let mut removed: BTreeMap<&FileId, BTreeSet<Vec<String>>> = BTreeMap::new();
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
                    removed.entry(file).or_default().insert(path);
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
            Finding::UnusedDependency { package, workspace } => {
                let text = manifest(&mut manifests, project, workspace);
                let removed = ["dependencies", "devDependencies"].iter().fold(
                    None,
                    |changed: Option<String>, field| {
                        let current = changed.as_deref().unwrap_or(text);
                        crate::package_json::remove(current, field, package).or(changed)
                    },
                );
                plan.fixed
                    .extend(removed.is_some().then(|| finding.clone()));
                *text = removed.unwrap_or_else(|| std::mem::take(text));
            }
            Finding::MisplacedDependency { package, workspace } => {
                let text = manifest(&mut manifests, project, workspace);
                let moved = crate::package_json::move_to_dev(text, package);
                plan.fixed.extend(moved.is_some().then(|| finding.clone()));
                *text = moved.unwrap_or_else(|| std::mem::take(text));
            }
            _ => {}
        }
    }
    plan.manifests = manifests
        .into_iter()
        .filter(|(_, (before, after))| before != after)
        .map(|(path, (_, after))| (path, after))
        .collect();
    // A file being deleted is not edited.
    let removals = removals
        .into_iter()
        .filter(|(file, _)| !plan.delete.contains(*file));
    for (file, removals) in removals {
        let source = std::fs::read_to_string(project.root.join(file)).unwrap_or_default();
        let facts = &project.files[file].facts;
        let (mut edits, fixed) = edits(&source, &facts.export_removals, &removals);
        let removed = removed.remove(file).unwrap_or_default();
        edits.extend(declarations(&source, &facts.export_declarations, &removed));
        plan.fixed.extend(fixed);
        plan.files.insert(
            file.clone(),
            tidy(&source, crate::edit::apply(&source, &edits)),
        );
    }
    plan.fixed
        .sort_by_key(|finding| findings.iter().position(|f| f == finding));
    plan
}

/// The text of the `package.json` of `workspace` as the fixes so far leave it, read the first
/// time it is needed.
fn manifest<'m>(
    manifests: &'m mut BTreeMap<String, (String, String)>,
    project: &Project,
    workspace: &Option<String>,
) -> &'m mut String {
    let path = crate::report::manifest_path(workspace);
    let entry = manifests.entry(path).or_insert_with_key(|path| {
        let text = std::fs::read_to_string(project.root.join(path)).unwrap_or_default();
        (text.clone(), text)
    });
    &mut entry.1
}

/// The declarations left unused once the exports at `removed` go: those all of whose exports
/// go.
fn declarations(
    source: &str,
    declarations: &BTreeMap<Vec<String>, Span>,
    removed: &BTreeSet<Vec<String>>,
) -> Vec<Edit> {
    let spans: BTreeSet<Span> = removed
        .iter()
        .filter_map(|path| declarations.get(path).copied())
        .collect();
    spans
        .into_iter()
        .filter(|span| {
            declarations
                .iter()
                .filter(|(_, declaration)| *declaration == span)
                .all(|(path, _)| removed.contains(path))
        })
        .map(|span| delete_lines(source, span))
        .collect()
}

/// `edited` without the blank lines the deletions left behind: no run of blank lines, nor a
/// blank line at the start or the end, that `original` did not have.
fn tidy(original: &str, edited: String) -> String {
    let mut text = edited;
    let runs = [("\r\n\r\n\r\n", "\r\n\r\n"), ("\n\n\n", "\n\n")];
    for (run, shorter) in runs.into_iter().filter(|(run, _)| !original.contains(run)) {
        while text.contains(run) {
            text = text.replace(run, shorter);
        }
    }
    // Where the text would end with one line break, and where it starts after the blank ones.
    let end = (text.trim_end_matches('\n').len() + 1).min(text.len());
    let end = if original.ends_with("\n\n") {
        text.len()
    } else {
        end
    };
    let start = text.len() - text.trim_start_matches(['\r', '\n']).len();
    let start = if original.starts_with('\n') {
        0
    } else {
        start.min(end)
    };
    text[start..end].to_string()
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
    // A declaration of several names goes only when all of them do.
    let partly_removed = |removal: &Removal| {
        let sharing = exports.values().filter(|r| *r == removal).count();
        let removing = removals.iter().filter(|(_, r)| r == removal).count();
        sharing > 0 && sharing != removing
    };
    for (finding, removal) in removals {
        match removal {
            _ if partly_removed(removal) => continue,
            Removal::Prefix(span) => edits.push(Edit::delete(*span)),
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
    fn tidying_removes_only_the_blank_lines_deletions_left() {
        assert_eq!(tidy("a\n\nb\n", "\n\na\n\n\n\nb\n\n".into()), "a\n\nb\n");
        assert_eq!(
            tidy("\na\n\n\n\nb\n\n", "\na\n\n\n\nb\n\n".into()),
            "\na\n\n\n\nb\n\n",
            "blank lines the file already had stay"
        );
        assert_eq!(
            tidy("a\r\n\r\nb\r\n", "a\r\n\r\n\r\nb\r\n".into()),
            "a\r\n\r\nb\r\n"
        );
    }

    #[test]
    fn a_dependency_package_json_does_not_list_is_left_alone() {
        let (_dir, project) = project(&[]);
        let findings = [
            Finding::UnusedDependency {
                package: "x".into(),
                workspace: None,
            },
            Finding::MisplacedDependency {
                package: "y".into(),
                workspace: None,
            },
        ];
        let plan = plan(&project, &findings);
        assert!(plan.fixed.is_empty());
        assert!(plan.manifests.is_empty());
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
        let plan = plan(&project, &findings);
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
        let plan = plan(&project, &findings);
        assert!(plan.fixed.is_empty());
        assert!(plan.delete.is_empty() && plan.files.is_empty());
    }
}

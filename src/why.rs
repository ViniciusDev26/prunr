//! `prunr why <target>`: why a file, an export or a package is alive (or not), from the same
//! graph the check uses.
//!
//! - A file: the shortest chain of imports from an entry to it, or who imports it when nothing
//!   reaches it.
//! - `file:export`: the files that use the export.
//! - A package: the files that import it, and the scripts, tools and packages that need it.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::checks::reachable;
use crate::project::{FileId, Project};

/// Where a file stands: reached by production, only by tests, or by nothing.
fn kind(id: &FileId, live: &BTreeSet<FileId>, tested: &BTreeSet<FileId>) -> &'static str {
    if live.contains(id) {
        "production"
    } else if tested.contains(id) {
        "test"
    } else {
        "unreached"
    }
}

/// The answer to `prunr why <target>`, or why the target is not something prunr knows.
pub fn why(
    project: &Project,
    production: &BTreeSet<FileId>,
    tests: &BTreeSet<FileId>,
    target: &str,
) -> Result<String, String> {
    let live = reachable(project, production);
    let tested = reachable(project, tests);
    if project.files.contains_key(target) {
        return Ok(file(project, production, tests, &live, &tested, target));
    }
    if let Some((file, symbol)) = target.rsplit_once(':')
        && project.files.contains_key(file)
    {
        return Ok(export(project, &live, &tested, file, symbol));
    }
    let known = project
        .workspaces
        .iter()
        .any(|workspace| workspace.declared_packages.contains(target))
        || project
            .files
            .values()
            .any(|file| file.packages.contains(target));
    if known {
        return Ok(package(project, &live, &tested, target));
    }
    Err(format!(
        "{target} is not a file of the project nor a package it declares or imports"
    ))
}

fn file(
    project: &Project,
    production: &BTreeSet<FileId>,
    tests: &BTreeSet<FileId>,
    live: &BTreeSet<FileId>,
    tested: &BTreeSet<FileId>,
    target: &str,
) -> String {
    for (entries, how, label) in [
        (production, "is used in production", "entry"),
        (tests, "is used only by tests", "test entry"),
    ] {
        if let Some(chain) = chain(project, entries, target) {
            let mut text = format!("{target} {how}:\n  {} ({label})\n", chain[0]);
            for pair in chain.windows(2) {
                let line = project.files[&pair[0]].edge_lines[&pair[1]];
                text.push_str(&format!("  -> {} ({}:{line})\n", pair[1], pair[0]));
            }
            return text;
        }
    }
    let importers: Vec<String> = project
        .files
        .iter()
        .filter_map(|(id, file)| {
            let line = file.edge_lines.get(target)?;
            Some(format!("  {id}:{line} ({})\n", kind(id, live, tested)))
        })
        .collect();
    match importers.is_empty() {
        true => format!("{target} is not reached by any entry, and no file imports it.\n"),
        false => format!(
            "{target} is not reached by any entry. Imported by:\n{}",
            importers.concat()
        ),
    }
}

/// The shortest chain of imports from one of `entries` to `target`.
fn chain(project: &Project, entries: &BTreeSet<FileId>, target: &str) -> Option<Vec<FileId>> {
    let mut parent: BTreeMap<&FileId, Option<&FileId>> =
        entries.iter().map(|entry| (entry, None)).collect();
    let mut queue: VecDeque<&FileId> = entries.iter().collect();
    while let Some(id) = queue.pop_front() {
        if id == target {
            let mut chain = vec![id.clone()];
            let mut at = id;
            while let Some(Some(previous)) = parent.get(at) {
                chain.push((*previous).clone());
                at = previous;
            }
            chain.reverse();
            return Some(chain);
        }
        for edge in &project.files[id].edges {
            if !parent.contains_key(&edge.target) {
                parent.insert(&edge.target, Some(id));
                queue.push_back(&edge.target);
            }
        }
    }
    None
}

fn export(
    project: &Project,
    live: &BTreeSet<FileId>,
    tested: &BTreeSet<FileId>,
    target: &str,
    symbol: &str,
) -> String {
    let path: Vec<&str> = symbol.split('.').collect();
    let users: Vec<String> = project
        .files
        .iter()
        .filter_map(|(id, file)| {
            let uses = file.edges.iter().filter(|edge| edge.target == target);
            let used = uses.flat_map(|edge| &edge.uses).any(|used| {
                let shared = used.path.iter().zip(&path).all(|(a, b)| a == b);
                shared && (used.path.len() >= path.len() || used.whole)
            });
            used.then(|| {
                let line = file.edge_lines[target];
                format!("  {id}:{line} ({})\n", kind(id, live, tested))
            })
        })
        .collect();
    match users.is_empty() {
        true => format!("{target}: {symbol} is used by no file.\n"),
        false => format!("{target}: {symbol} is used by:\n{}", users.concat()),
    }
}

fn package(
    project: &Project,
    live: &BTreeSet<FileId>,
    tested: &BTreeSet<FileId>,
    target: &str,
) -> String {
    let mut lines: Vec<String> = project
        .files
        .iter()
        .filter(|(_, file)| file.packages.contains(target))
        .map(|(id, file)| {
            let line = file
                .facts
                .require_lines
                .iter()
                .filter(|(specifier, _)| crate::resolve::package_name(specifier) == target)
                .map(|(_, line)| *line)
                .min()
                .unwrap_or(1);
            format!("  {id}:{line} ({})\n", kind(id, live, tested))
        })
        .collect();
    for workspace in &project.workspaces {
        let at = match workspace.dir.as_str() {
            "" => "package.json".to_string(),
            dir => format!("{dir}/package.json"),
        };
        if workspace.script_packages.contains(target) {
            lines.push(format!("  a script of {at}\n"));
        }
        if workspace.test_packages.contains(target) {
            lines.push(format!(
                "  a tool configured in {} (test)\n",
                workspace_dir(&at)
            ));
        }
        if workspace.production_packages.contains(target) {
            lines.push(format!(
                "  a framework of {} (production)\n",
                workspace_dir(&at)
            ));
        }
        for (host, peers) in &workspace.peers {
            if peers.contains(target) {
                lines.push(format!("  a peer dependency of {host}\n"));
            }
        }
    }
    match lines.is_empty() {
        true => format!("{target} is used by no file, script or tool.\n"),
        false => format!("{target} is used by:\n{}", lines.concat()),
    }
}

/// `the project` for the root `package.json`, else the package's folder.
fn workspace_dir(manifest: &str) -> String {
    match manifest.strip_suffix("/package.json") {
        Some(dir) => dir.to_string(),
        None => "the project".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(files: &[(&str, &str)]) -> (tempfile::TempDir, Project) {
        let dir = tempfile::tempdir().unwrap();
        for (path, content) in files {
            let path = dir.path().join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, content).unwrap();
        }
        let project = Project::load(dir.path()).unwrap();
        (dir, project)
    }

    fn ids(items: &[&str]) -> BTreeSet<FileId> {
        items.iter().map(|item| item.to_string()).collect()
    }

    #[test]
    fn a_file_only_tests_reach_and_one_only_dead_code_imports() {
        let (_dir, project) = project(&[
            ("index.js", ""),
            ("a.test.js", "require('./helper').x();"),
            ("helper.js", "exports.x = () => {};"),
            ("dead.js", "require('./orphan');"),
            ("orphan.js", ""),
        ]);
        let (production, tests) = (ids(&["index.js"]), ids(&["a.test.js"]));
        assert_eq!(
            why(&project, &production, &tests, "helper.js").unwrap(),
            "helper.js is used only by tests:\n  a.test.js (test entry)\n  -> helper.js (a.test.js:1)\n"
        );
        assert_eq!(
            why(&project, &production, &tests, "orphan.js").unwrap(),
            "orphan.js is not reached by any entry. Imported by:\n  dead.js:1 (unreached)\n"
        );
        assert_eq!(
            why(&project, &production, &tests, "helper.js:x").unwrap(),
            "helper.js: x is used by:\n  a.test.js:1 (test)\n"
        );
    }

    #[test]
    fn a_package_scripts_tools_frameworks_and_peers_need() {
        let (_dir, project) = project(&[
            (
                "package.json",
                r#"{
                    "workspaces": ["web"],
                    "scripts": { "lint": "eslint ." },
                    "devDependencies": { "eslint": "9", "jest": "29", "ts-jest": "29" },
                    "jest": { "preset": "ts-jest" }
                }"#,
            ),
            (
                "web/package.json",
                r#"{ "dependencies": { "next": "15", "react": "19", "react-dom": "19" } }"#,
            ),
            (
                "node_modules/next/package.json",
                r#"{ "peerDependencies": { "react": "19" } }"#,
            ),
            ("web/index.js", "require('react');"),
            ("index.js", "require('undeclared');"),
        ]);
        let none = BTreeSet::new();
        assert_eq!(
            why(&project, &none, &none, "undeclared").unwrap(),
            "undeclared is used by:\n  index.js:1 (unreached)\n"
        );
        assert!(why(&project, &none, &none, "nope").is_err());
        assert_eq!(
            why(&project, &none, &none, "eslint").unwrap(),
            "eslint is used by:\n  a script of package.json\n"
        );
        assert_eq!(
            why(&project, &none, &none, "ts-jest").unwrap(),
            "ts-jest is used by:\n  a tool configured in the project (test)\n"
        );
        assert_eq!(
            why(&project, &none, &none, "react").unwrap(),
            "react is used by:\n  web/index.js:1 (unreached)\n  a framework of web (production)\n  a peer dependency of next\n"
        );
    }
}

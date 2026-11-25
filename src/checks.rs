//! The checks, all run over the same module graph.
//!
//! Only the files the entries reach take part: a file nothing reaches is reported as unused,
//! and what it requires or exports does not count.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::Serialize;

use crate::facts::Use;
use crate::project::{FileId, Project};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Finding {
    /// No production entry reaches the file. With `test_only`, tests do: delete it with them.
    UnusedFile {
        file: FileId,
        #[serde(rename = "testOnly", skip_serializing_if = "is_false")]
        test_only: bool,
    },
    /// No production file uses the export, and neither does its own file. With `test_only`,
    /// tests do.
    UnusedExport {
        file: FileId,
        symbol: String,
        #[serde(rename = "testOnly", skip_serializing_if = "is_false")]
        test_only: bool,
    },
    /// Declared in `dependencies` or `devDependencies`, required by no file.
    UnusedDependency { package: String },
    /// Declared in `dependencies`, but only tests require it: it belongs in `devDependencies`.
    MisplacedDependency { package: String },
    /// Required, but declared in no dependency field of `package.json`.
    MissingDependency { file: FileId, package: String },
    /// A cycle, from its first file back to it.
    CircularImport { cycle: Vec<FileId> },
    /// A require the graph cannot follow. A warning: it does not fail the check.
    UnresolvedRequire { file: FileId, specifier: String },
}

impl Finding {
    pub fn is_warning(&self) -> bool {
        matches!(self, Self::UnresolvedRequire { .. })
    }
}

fn is_false(value: &bool) -> bool {
    !value
}

/// Every finding, sorted. `production` are the production entries; `tests` are the test files,
/// which are entries of the test graph and are never reported.
pub fn check(
    project: &Project,
    production: &BTreeSet<FileId>,
    tests: &BTreeSet<FileId>,
) -> Vec<Finding> {
    todo!()
}

fn old_check(project: &Project, entries: &[FileId]) -> Vec<Finding> {
    let reachable = reachable(project, entries);
    let mut findings = Vec::new();
    for file in project.files.keys().filter(|id| !reachable.contains(*id)) {
        findings.push(Finding::UnusedFile {
            file: file.clone(),
            test_only: false,
        });
    }
    unused_exports(project, entries, &reachable, &mut findings);
    dependencies(project, &reachable, &mut findings);
    for cycle in cycles(project, &reachable) {
        findings.push(Finding::CircularImport { cycle });
    }
    for id in &reachable {
        for specifier in &project.files[id].unresolved {
            findings.push(Finding::UnresolvedRequire {
                file: id.clone(),
                specifier: specifier.clone(),
            });
        }
    }
    findings.sort();
    findings
}

/// The files the entries reach, the entries included.
fn reachable(project: &Project, entries: &[FileId]) -> BTreeSet<FileId> {
    let mut seen: BTreeSet<FileId> = entries.iter().cloned().collect();
    let mut queue: VecDeque<&FileId> = entries.iter().collect();
    while let Some(id) = queue.pop_front() {
        for edge in &project.files[id].edges {
            if seen.insert(edge.target.clone()) {
                queue.push_back(&edge.target);
            }
        }
    }
    seen
}

/// Exports of reachable files that no reachable file uses. An entry's exports are its
/// interface to whoever runs it, so they are not checked.
fn unused_exports(
    project: &Project,
    entries: &[FileId],
    reachable: &BTreeSet<FileId>,
    findings: &mut Vec<Finding>,
) {
    let mut uses: BTreeMap<&FileId, Vec<&Use>> = BTreeMap::new();
    for id in reachable {
        for edge in &project.files[id].edges {
            uses.entry(&edge.target).or_default().extend(&edge.uses);
        }
    }
    for id in reachable.iter().filter(|id| !entries.contains(id)) {
        let file = &project.files[id];
        let incoming = uses.get(id).map(Vec::as_slice).unwrap_or_default();
        for export in &file.facts.exports {
            let used = export.used_internally
                || incoming
                    .iter()
                    .any(|u| covers(u, &export.path, file.facts.default_export));
            if !used {
                findings.push(Finding::UnusedExport {
                    test_only: false,
                    file: id.clone(),
                    symbol: export.path.join("."),
                });
            }
        }
    }
}

/// Whether `used` keeps `export` alive. Reading a member uses its parent too, and a whole use
/// covers every member below it. A default export's uses are relative to `default`.
fn covers(used: &Use, export: &[String], default_export: bool) -> bool {
    let mut path = Vec::new();
    if default_export {
        path.push("default".to_string());
    }
    path.extend(used.path.iter().cloned());
    path.starts_with(export) || (used.whole && export.starts_with(&path))
}

fn dependencies(project: &Project, reachable: &BTreeSet<FileId>, findings: &mut Vec<Finding>) {
    let mut required = BTreeSet::new();
    for id in reachable {
        for package in &project.files[id].packages {
            required.insert(package);
            if !project.declared_packages.contains(package) {
                findings.push(Finding::MissingDependency {
                    file: id.clone(),
                    package: package.clone(),
                });
            }
        }
    }
    for package in project.dependencies.union(&project.dev_dependencies) {
        if !required.contains(package) {
            findings.push(Finding::UnusedDependency {
                package: package.clone(),
            });
        }
    }
}

/// One cycle per strongly connected component: the shortest one through its first file.
fn cycles(project: &Project, reachable: &BTreeSet<FileId>) -> Vec<Vec<FileId>> {
    let mut tarjan = Tarjan {
        project,
        index: BTreeMap::new(),
        low: BTreeMap::new(),
        stack: Vec::new(),
        on_stack: BTreeSet::new(),
        components: Vec::new(),
    };
    for id in reachable {
        if !tarjan.index.contains_key(id) {
            tarjan.visit(id);
        }
    }
    tarjan
        .components
        .into_iter()
        .filter(|component| {
            component.len() > 1
                || project.files[component[0]]
                    .edges
                    .iter()
                    .any(|edge| &edge.target == component[0])
        })
        .map(|component| {
            let members: BTreeSet<&FileId> = component.into_iter().collect();
            shortest_cycle(project, &members)
        })
        .collect()
}

/// The shortest cycle from the first member of a strongly connected component back to it.
fn shortest_cycle(project: &Project, members: &BTreeSet<&FileId>) -> Vec<FileId> {
    let start = *members.first().expect("a component has members");
    let mut parent: BTreeMap<&FileId, &FileId> = BTreeMap::new();
    let mut queue = VecDeque::from([start]);
    loop {
        let node = queue.pop_front().expect("a component holds a cycle");
        for edge in &project.files[node].edges {
            let next = &edge.target;
            if next == start {
                let mut cycle = vec![node.clone()];
                let mut current = node;
                while let Some(previous) = parent.get(current) {
                    cycle.push((*previous).clone());
                    current = previous;
                }
                cycle.reverse();
                cycle.push(start.clone());
                return cycle;
            }
            if members.contains(next) && !parent.contains_key(next) {
                parent.insert(next, node);
                queue.push_back(next);
            }
        }
    }
}

/// Tarjan's strongly connected components.
struct Tarjan<'p> {
    project: &'p Project,
    index: BTreeMap<&'p FileId, usize>,
    low: BTreeMap<&'p FileId, usize>,
    stack: Vec<&'p FileId>,
    on_stack: BTreeSet<&'p FileId>,
    components: Vec<Vec<&'p FileId>>,
}

impl<'p> Tarjan<'p> {
    fn visit(&mut self, node: &'p FileId) {
        let index = self.index.len();
        self.index.insert(node, index);
        self.low.insert(node, index);
        self.stack.push(node);
        self.on_stack.insert(node);
        for edge in &self.project.files[node].edges {
            let next = &edge.target;
            if !self.index.contains_key(next) {
                self.visit(next);
                let low = self.low[node].min(self.low[next]);
                self.low.insert(node, low);
            } else if self.on_stack.contains(next) {
                let low = self.low[node].min(self.index[next]);
                self.low.insert(node, low);
            }
        }
        if self.low[node] == self.index[node] {
            let mut component = Vec::new();
            loop {
                let member = self.stack.pop().expect("the node is on the stack");
                self.on_stack.remove(member);
                component.push(member);
                if member == node {
                    break;
                }
            }
            self.components.push(component);
        }
    }
}

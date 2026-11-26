//! The checks, all run over the same module graph.
//!
//! Two graphs share the files: production, from the production entries, and test, from the test
//! files. What only tests reach is dead in production. A file neither reaches is reported as
//! unused, and what it requires or exports does not count.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::facts::{Ignore, ReexportKind, Use};
use crate::project::{FileId, Project, as_commonjs};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Finding {
    /// No production entry reaches the file. With `test_only`, tests do: delete it with them.
    UnusedFile {
        file: FileId,
        #[serde(rename = "testOnly", default, skip_serializing_if = "is_false")]
        test_only: bool,
    },
    /// No production file uses the export, and neither does its own file. With `test_only`,
    /// tests do.
    UnusedExport {
        file: FileId,
        symbol: String,
        #[serde(rename = "testOnly", default, skip_serializing_if = "is_false")]
        test_only: bool,
    },
    /// A binding a require declares that its file never references. The require still runs the
    /// module, so the edge stays until the import is deleted.
    UnusedImport {
        file: FileId,
        name: String,
        specifier: String,
    },
    /// A `prunr-ignore-next-line` or `prunr-ignore-file` comment that keeps nothing from being
    /// reported: what it ignored is used now, or it is before no export.
    UnusedIgnore { file: FileId, line: u32 },
    /// Declared in `dependencies` or `devDependencies`, required by no file and run by no script.
    UnusedDependency {
        package: String,
        /// The directory of the package of the repository that declares it, in a monorepo; none
        /// for the root `package.json`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        workspace: Option<String>,
    },
    /// Declared in `dependencies`, but only tests require it (and no script runs it): it belongs
    /// in `devDependencies`.
    MisplacedDependency {
        package: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        workspace: Option<String>,
    },
    /// Required, but declared in no dependency field of `package.json`.
    MissingDependency { file: FileId, package: String },
    /// A cycle, from its first file back to it.
    CircularImport { cycle: Vec<FileId> },
    /// A require the graph cannot follow. A warning: it does not fail the check.
    UnresolvedRequire { file: FileId, specifier: String },
    /// A file the graph reaches but that does not parse: what it uses is unknown. A warning.
    UnparsedFile { file: FileId, message: String },
}

impl Finding {
    pub fn is_warning(&self) -> bool {
        matches!(
            self,
            Self::UnresolvedRequire { .. } | Self::UnparsedFile { .. }
        )
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
    let live = reachable(project, production);
    let tested = reachable(project, tests);
    let all: BTreeSet<FileId> = live.union(&tested).cloned().collect();
    let mut findings = Vec::new();
    for (file, facts) in project.files.iter().map(|(id, file)| (id, &file.facts)) {
        if !live.contains(file) && !tests.contains(file) && facts.ignore_file.is_none() {
            findings.push(Finding::UnusedFile {
                file: file.clone(),
                test_only: tested.contains(file),
            });
        }
    }
    unused_exports(project, production, &live, &tested, &mut findings);
    dependencies(project, &live, &tested, &mut findings);
    for cycle in cycles(project, &all) {
        findings.push(Finding::CircularImport { cycle });
    }
    for id in &all {
        for require in &project.files[id].facts.requires {
            for name in &require.unused {
                findings.push(Finding::UnusedImport {
                    file: id.clone(),
                    name: name.clone(),
                    specifier: require.specifier.clone(),
                });
            }
        }
        if let Some(message) = &project.files[id].parse_error {
            findings.push(Finding::UnparsedFile {
                file: id.clone(),
                message: message.clone(),
            });
        }
        for specifier in &project.files[id].unresolved {
            findings.push(Finding::UnresolvedRequire {
                file: id.clone(),
                specifier: specifier.clone(),
            });
        }
    }
    // Test fixtures are data the tests read: what is in them is not a finding.
    findings.retain(|finding| match finding {
        Finding::CircularImport { cycle } => !cycle.iter().all(|file| in_test_fixture(file)),
        Finding::UnusedFile { file, .. }
        | Finding::UnusedExport { file, .. }
        | Finding::UnusedImport { file, .. }
        | Finding::UnusedIgnore { file, .. }
        | Finding::MissingDependency { file, .. }
        | Finding::UnresolvedRequire { file, .. }
        | Finding::UnparsedFile { file, .. } => !in_test_fixture(file),
        Finding::UnusedDependency { .. } | Finding::MisplacedDependency { .. } => true,
    });
    findings.sort();
    findings
}

/// The folders that hold tests, and the ones inside them that hold the data tests read.
const TEST_FOLDERS: &[&str] = &["test", "tests", "__tests__", "spec", "e2e"];
const FIXTURE_FOLDERS: &[&str] = &["fixtures", "__fixtures__", "fixture", "samples"];

/// Whether `file` is in a fixtures folder of a test folder (`test/fixtures/a.js`).
fn in_test_fixture(file: &str) -> bool {
    let folders: Vec<&str> = file.split('/').collect();
    let folders = &folders[..folders.len() - 1];
    folders
        .iter()
        .position(|folder| TEST_FOLDERS.contains(folder))
        .is_some_and(|at| {
            folders[at + 1..]
                .iter()
                .any(|f| FIXTURE_FOLDERS.contains(f))
        })
}

/// Whether a finding is about what goes to production (`prunr check --production`): unused
/// files and exports, misplaced dependencies, the rest only in files production reaches, and
/// unused dependencies only among `dependencies`.
pub fn in_production(
    project: &Project,
    production: &BTreeSet<FileId>,
) -> impl Fn(&Finding) -> bool {
    let live = reachable(project, production);
    move |finding| match finding {
        Finding::UnusedFile { .. }
        | Finding::UnusedExport { .. }
        | Finding::MisplacedDependency { .. } => true,
        Finding::UnusedImport { file, .. }
        | Finding::UnusedIgnore { file, .. }
        | Finding::MissingDependency { file, .. }
        | Finding::UnresolvedRequire { file, .. }
        | Finding::UnparsedFile { file, .. } => live.contains(file),
        Finding::CircularImport { cycle } => cycle.iter().all(|file| live.contains(file)),
        Finding::UnusedDependency { package, workspace } => {
            let dir = workspace.as_deref().unwrap_or_default();
            project
                .workspaces
                .iter()
                .any(|w| w.dir == dir && w.dependencies.contains(package))
        }
    }
}

/// The files the entries reach, the entries included.
pub(crate) fn reachable(project: &Project, entries: &BTreeSet<FileId>) -> BTreeSet<FileId> {
    let mut seen = entries.clone();
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

type Uses<'p> = BTreeMap<&'p FileId, BTreeSet<Use>>;

/// What `files` use of each file they require. An entry is used whole: its exports are its
/// interface. Uses of a file that reexports others are forwarded to them.
fn uses<'p>(
    project: &'p Project,
    files: &BTreeSet<FileId>,
    entries: &'p BTreeSet<FileId>,
) -> Uses<'p> {
    let mut uses: Uses = BTreeMap::new();
    for id in files {
        for edge in &project.files[id].edges {
            uses.entry(&edge.target)
                .or_default()
                .extend(edge.uses.iter().cloned());
        }
    }
    for id in entries {
        uses.entry(id).or_default().insert(Use {
            path: vec![],
            whole: true,
        });
    }
    let mut queue: Vec<&FileId> = uses.keys().copied().collect();
    while let Some(id) = queue.pop() {
        let file = &project.files[id];
        let mut shadowed: BTreeSet<&str> = file
            .facts
            .exports
            .iter()
            .map(|export| export.path[0].as_str())
            .collect();
        shadowed.insert("default");
        for (_, kind) in &file.reexports {
            if let ReexportKind::Named { exported, .. } = kind {
                shadowed.insert(exported);
            }
        }
        let incoming = uses[id].clone();
        for (target, kind) in &file.reexports {
            let commonjs = !project.files[target].facts.esm;
            let forwarded: Vec<Use> = incoming
                .iter()
                .filter_map(|used| forward(used, kind, &shadowed))
                .map(|used| if commonjs { as_commonjs(&used) } else { used })
                .collect();
            let known = uses.entry(target).or_default();
            let before = known.len();
            known.extend(forwarded);
            if known.len() > before {
                queue.push(target);
            }
        }
    }
    uses
}

/// The use of a reexported module that a use of the reexporting one makes. `shadowed` are the
/// names `export *` does not cover: the module's own exports, its named reexports and `default`.
fn forward(used: &Use, kind: &ReexportKind, shadowed: &BTreeSet<&str>) -> Option<Use> {
    let whole = |path: Vec<String>| Some(Use { path, whole: true });
    let Some((first, rest)) = used.path.split_first() else {
        return match kind {
            _ if !used.whole => None,
            ReexportKind::Named { imported, .. } => whole(vec![imported.clone()]),
            ReexportKind::All | ReexportKind::Namespace(_) => whole(vec![]),
        };
    };
    let path = match kind {
        ReexportKind::Named { imported, exported } if first == exported => {
            [vec![imported.clone()], rest.to_vec()].concat()
        }
        ReexportKind::Namespace(name) if first == name => rest.to_vec(),
        ReexportKind::All if !shadowed.contains(first.as_str()) => used.path.clone(),
        _ => return None,
    };
    Some(Use {
        path,
        whole: used.whole,
    })
}

/// Exports of live files that no live file uses. A production entry's exports are its interface
/// to whoever runs it, so they are not checked.
fn unused_exports(
    project: &Project,
    production: &BTreeSet<FileId>,
    live: &BTreeSet<FileId>,
    tested: &BTreeSet<FileId>,
    findings: &mut Vec<Finding>,
) {
    let live_uses = uses(project, live, production);
    let none = BTreeSet::new();
    let test_uses = uses(project, &tested.difference(live).cloned().collect(), &none);
    for id in live {
        let file = &project.files[id];
        let facts = &file.facts;
        let used_by = |uses: &Uses, path: &[String]| {
            uses.get(id).is_some_and(|incoming| {
                incoming
                    .iter()
                    .any(|u| covers(u, path, facts.default_export))
            })
        };
        let unused_ignore = |line: u32| Finding::UnusedIgnore {
            file: id.clone(),
            line,
        };
        // An entry's exports are its interface: they are never reported, so nothing is ignored.
        let entry = production.contains(id);
        let mut file_ignored = false;
        for export in &facts.exports {
            let alive = entry || export.used_internally || used_by(&live_uses, &export.path);
            let ignore = facts.ignores.get(&export.path);
            if alive {
                if let Some(Ignore::NextLine(line)) = ignore {
                    findings.push(unused_ignore(*line));
                }
            } else if facts.ignore_file.is_some() {
                file_ignored = true;
            } else if ignore.is_none() {
                findings.push(Finding::UnusedExport {
                    file: id.clone(),
                    symbol: export.path.join("."),
                    test_only: used_by(&test_uses, &export.path),
                });
            }
        }
        if let Some(line) = facts.ignore_file
            && !file_ignored
        {
            findings.push(unused_ignore(line));
        }
        findings.extend(facts.stray_ignores.iter().map(|line| unused_ignore(*line)));
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

/// The dependency checks, per package of the repository. The root package is the whole
/// repository's: what it declares is available to every package, and every file, script and
/// tool counts as using it, unless the package that uses it declares it too. Any other package
/// only counts its own.
fn dependencies(
    project: &Project,
    live: &BTreeSet<FileId>,
    tested: &BTreeSet<FileId>,
    findings: &mut Vec<Finding>,
) {
    let typescript = project
        .files
        .keys()
        .any(|id| id.ends_with("ts") || id.ends_with("tsx"));
    let builtins = live.union(tested).any(|id| project.files[id].builtins);
    let types_of = |package: &String| {
        format!(
            "@types/{}",
            package.trim_start_matches('@').replace('/', "__")
        )
    };
    let root = &project.workspaces[0];
    for id in live.union(tested) {
        let workspace = &project.workspaces[project.workspace_of(id)];
        // The root of a monorepo is not published, and package managers link the packages of
        // the repository into it: its files can import them without declaring them.
        let at_root = workspace.dir.is_empty();
        let declared = |package: &String| {
            workspace.name.as_ref() == Some(package)
                || (at_root
                    && project.workspaces[1..]
                        .iter()
                        .any(|other| other.name.as_ref() == Some(package)))
                || workspace.declared_packages.contains(package)
                || root.declared_packages.contains(package)
                || project.nested_declared.iter().any(|(dir, declared)| {
                    id.strip_prefix(dir.as_str())
                        .is_some_and(|rest| rest.starts_with('/'))
                        && declared.contains(package)
                })
        };
        for package in &project.files[id].packages {
            if !declared(package)
                && !declared(&types_of(package))
                && !project.vendored.contains(package)
            {
                findings.push(Finding::MissingDependency {
                    file: id.clone(),
                    package: package.clone(),
                });
            }
        }
    }
    for (index, workspace) in project.workspaces.iter().enumerate() {
        let scope: Vec<&crate::project::Workspace> = if index == 0 {
            project.workspaces.iter().collect()
        } else {
            vec![workspace]
        };
        // The root's dependencies serve the packages of the repository too, but not one a
        // package declares itself: its own declaration is the one that installs it.
        let serves = |member: usize, package: &String| {
            let declares = |at: usize| {
                let other = &project.workspaces[at];
                other.dependencies.contains(package) || other.dev_dependencies.contains(package)
            };
            member == index || !declares(member)
        };
        // What production, tests and scripts use; `served` leaves out what a package uses but
        // declares itself, for the root.
        let collect = |served: bool| {
            let keep = |member: usize, package: &String| !served || serves(member, package);
            let required = |files: &BTreeSet<FileId>| -> BTreeSet<&String> {
                files
                    .iter()
                    .filter(|id| index == 0 || project.workspace_of(id) == index)
                    .flat_map(|id| {
                        let member = project.workspace_of(id);
                        project.files[id]
                            .packages
                            .iter()
                            .filter(move |package| keep(member, package))
                    })
                    .collect()
            };
            let mut used = [required(live), required(tested), BTreeSet::new()];
            // The scope of the root is every package, in order; another package's is itself.
            for (at, member) in scope.iter().enumerate() {
                let at = if index == 0 { at } else { index };
                let packages = [
                    &member.production_packages,
                    &member.test_packages,
                    &member.script_packages,
                ];
                for (set, packages) in used.iter_mut().zip(packages) {
                    set.extend(packages.iter().filter(|p| keep(at, p)));
                }
            }
            // A package needs its peers: whoever uses it uses them, the same way.
            for set in &mut used {
                with_peers(set, &scope);
            }
            used
        };
        let [in_production, in_tests, in_scripts] = collect(true);
        let used = |package: &String| {
            in_production.contains(package)
                || in_tests.contains(package)
                || in_scripts.contains(package)
        };
        // `@types/x` at the root gives types to an `x` a package declares itself.
        let anywhere = collect(false);
        let used_anywhere = |package: &String| anywhere.iter().any(|set| set.contains(package));
        let types_used = |package: &String| match package.strip_prefix("@types/") {
            Some("node") => typescript || builtins,
            Some(typed) => used_anywhere(&typed_package(typed)),
            None => false,
        };
        let at = (index > 0).then(|| workspace.dir.clone());
        for package in workspace.dependencies.union(&workspace.dev_dependencies) {
            if !used(package) && !types_used(package) {
                findings.push(Finding::UnusedDependency {
                    package: package.clone(),
                    workspace: at.clone(),
                });
            }
        }
        for package in &workspace.dependencies {
            if !in_production.contains(package)
                && in_tests.contains(package)
                && !in_scripts.contains(package)
            {
                findings.push(Finding::MisplacedDependency {
                    package: package.clone(),
                    workspace: at.clone(),
                });
            }
        }
    }
}

/// `used` with the peers of each package in it, and theirs, as the packages of `scope` see them.
fn with_peers<'p>(used: &mut BTreeSet<&'p String>, scope: &[&'p crate::project::Workspace]) {
    let mut pending: Vec<&String> = used.iter().copied().collect();
    while let Some(package) = pending.pop() {
        for member in scope {
            for peer in member.peers.get(package).into_iter().flatten() {
                if used.insert(peer) {
                    pending.push(peer);
                }
            }
        }
    }
}

/// The files `id` requires at runtime: imports of types only are gone once TypeScript compiles,
/// so they cannot form a cycle.
fn runtime_edges<'p>(project: &'p Project, id: &FileId) -> impl Iterator<Item = &'p FileId> {
    project.files[id]
        .edges
        .iter()
        .filter(|edge| !edge.type_only)
        .map(|edge| &edge.target)
}

/// The package `@types/<typed>` gives types to: `scope__name` → `@scope/name`.
fn typed_package(typed: &str) -> String {
    match typed.split_once("__") {
        Some((scope, name)) => format!("@{scope}/{name}"),
        None => typed.to_string(),
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
                || runtime_edges(project, component[0]).any(|target| target == component[0])
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
        for next in runtime_edges(project, node) {
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
        for next in runtime_edges(self.project, node) {
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

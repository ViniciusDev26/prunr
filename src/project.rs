//! The project as one module graph: every file is read and parsed once, and its requires are
//! resolved to the files and packages they point to.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::facts::{Facts, ReexportKind, Use, analyze};
use crate::resolve::{Resolver, Target};

/// A path relative to the project root, with `/` separators.
pub type FileId = String;

#[derive(Debug)]
pub struct Project {
    /// The canonical project root.
    pub root: PathBuf,
    pub files: BTreeMap<FileId, File>,
    /// The packages of the repository: the root one first, then those a monorepo tool declares.
    pub workspaces: Vec<Workspace>,
    /// Test code the plugins name: test files, the tools' configuration and setup files.
    pub test_files: BTreeSet<FileId>,
    /// Production entries the frameworks start from by convention (Next.js routes, NestJS main),
    /// and the entry of each package of the repository that is published.
    pub production_files: BTreeSet<FileId>,
    /// How many files were parsed.
    pub parsed: usize,
}

/// One package of the repository and what it declares and uses outside its files.
#[derive(Debug, Default)]
pub struct Workspace {
    /// Its directory, relative to the root, with `/` separators: empty for the root package.
    pub dir: String,
    /// Packages in `dependencies`: they must be used, and by production code.
    pub dependencies: BTreeSet<String>,
    /// Packages in `devDependencies`: they must be used.
    pub dev_dependencies: BTreeSet<String>,
    /// Every package its `package.json` declares, in any of its dependency fields.
    pub declared_packages: BTreeSet<String>,
    /// Packages its `scripts` run or preload.
    pub script_packages: BTreeSet<String>,
    /// Packages the plugins say the tools load by name (presets, environments, reporters).
    pub test_packages: BTreeSet<String>,
    /// Packages the frameworks load in production without an import.
    pub production_packages: BTreeSet<String>,
}

#[derive(Debug, Default)]
pub struct File {
    pub facts: Facts,
    /// Requires that point to project files.
    pub edges: Vec<Edge>,
    /// Packages the file requires.
    pub packages: BTreeSet<String>,
    /// Requires that point nowhere: dynamic ones by their source, missing files by specifier.
    pub unresolved: Vec<String>,
    /// `export ... from` other project files: uses of this file that they forward there.
    pub reexports: Vec<(FileId, ReexportKind)>,
    /// The file requires a Node.js builtin module.
    pub builtins: bool,
    /// The first line that requires each project file this one depends on.
    pub edge_lines: BTreeMap<FileId, u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    pub target: FileId,
    /// What the file uses of the target, in the target's terms: an ESM import of a CommonJS
    /// module has `module.exports` as its default, so the `default` is dropped.
    pub uses: Vec<Use>,
    /// Only types flow through it: TypeScript erases it.
    pub type_only: bool,
}

impl File {
    /// Records where a specifier points; the project file it names, if any.
    fn link(
        &mut self,
        target: Target,
        specifier: &str,
        files: &BTreeMap<PathBuf, FileId>,
    ) -> Option<FileId> {
        match target {
            Target::File(path) => return files.get(&path).cloned(),
            Target::Package(package) => {
                self.packages.insert(package);
            }
            Target::Builtin => self.builtins = true,
            Target::Unresolved => self.unresolved.push(specifier.to_string()),
        }
        None
    }
}

/// A use of a module through an ESM import, made relative to `module.exports` for a CommonJS
/// target: its `default` is `module.exports` itself.
pub fn as_commonjs(used: &Use) -> Use {
    let path = match used.path.split_first() {
        Some((first, rest)) if first == "default" => rest.to_vec(),
        _ => used.path.clone(),
    };
    Use {
        path,
        whole: used.whole,
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ProjectError {
    Parse { file: FileId, message: String },
    PackageJson(String),
}

impl std::fmt::Display for ProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse { file, message } => write!(f, "{file}: {message}"),
            Self::PackageJson(reason) => write!(f, "invalid package.json: {reason}"),
        }
    }
}

/// The dependency fields of `package.json`.
#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    name: Option<String>,
    #[serde(default)]
    private: bool,
    main: Option<String>,
    module: Option<String>,
    exports: Option<serde_json::Value>,
    #[serde(default)]
    dependencies: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    dev_dependencies: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    peer_dependencies: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    optional_dependencies: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    scripts: BTreeMap<String, String>,
    /// Jest's configuration, when it lives in `package.json`.
    jest: Option<serde_json::Value>,
}

impl Manifest {
    /// The `package.json` in `dir` (relative to `root`), or an empty one.
    fn load(root: &Path, dir: &str) -> Result<Self, ProjectError> {
        let path = root.join(dir).join("package.json");
        if !path.is_file() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path).unwrap_or_default();
        serde_json::from_str(&text).map_err(|e| {
            let at = if dir.is_empty() {
                String::new()
            } else {
                format!("{dir}/package.json: ")
            };
            ProjectError::PackageJson(format!("{at}{e}"))
        })
    }

    /// Every package it declares, in any dependency field.
    fn declared(&self) -> BTreeSet<String> {
        [
            &self.dependencies,
            &self.dev_dependencies,
            &self.peer_dependencies,
            &self.optional_dependencies,
        ]
        .into_iter()
        .flat_map(|field| field.keys().cloned())
        .collect()
    }

    /// Where its entry may be, relative to its directory: what `exports` and `main` say, then
    /// the source a build would compile it from.
    fn entry_candidates(&self) -> Vec<String> {
        let mut candidates = Vec::new();
        let mut exports = self.exports.as_ref();
        if let Some(serde_json::Value::Object(map)) = exports
            && let Some(dot) = map.get(".")
        {
            exports = Some(dot);
        }
        match exports {
            Some(serde_json::Value::String(entry)) => candidates.push(entry.clone()),
            Some(serde_json::Value::Object(conditions)) => {
                for condition in ["require", "import", "node", "default"] {
                    if let Some(serde_json::Value::String(entry)) = conditions.get(condition) {
                        candidates.push(entry.clone());
                    }
                }
            }
            _ => {}
        }
        candidates.extend(self.main.iter().chain(&self.module).cloned());
        candidates.extend(["src/index".to_string(), "index".to_string()]);
        candidates
    }
}

/// The file `candidates` (relative to the package at `dir`) point to: the first that exists.
fn package_file(
    resolver: &Resolver,
    root: &Path,
    dir: &str,
    candidates: &[String],
    files: &BTreeMap<PathBuf, FileId>,
) -> Option<FileId> {
    let manifest = root.join(dir).join("package.json");
    candidates.iter().find_map(|candidate| {
        let relative = format!("./{}", candidate.trim_start_matches("./"));
        match resolver.resolve(&manifest, &relative) {
            Target::File(path) => files.get(&path).cloned(),
            _ => None,
        }
    })
}

/// The binaries each package declares, mapped to the package. An installed package declares
/// those in the `bin` of its `package.json`; one that is not installed (or unreadable) is guessed
/// to declare a binary named after it, without the scope.
fn binaries(dirs: &[PathBuf], packages: &BTreeSet<String>) -> BTreeMap<String, String> {
    let mut binaries = BTreeMap::new();
    for package in packages {
        let installed: Option<serde_json::Value> = dirs.iter().find_map(|dir| {
            let manifest = dir.join("node_modules").join(package).join("package.json");
            std::fs::read_to_string(manifest)
                .ok()
                .and_then(|text| serde_json::from_str(&text).ok())
        });
        let unscoped = || package.rsplit('/').next().unwrap_or(package).to_string();
        let names = match installed.as_ref().map(|manifest| manifest.get("bin")) {
            Some(Some(serde_json::Value::Object(bins))) => bins.keys().cloned().collect(),
            Some(Some(_)) | None => vec![unscoped()],
            Some(None) => Vec::new(),
        };
        for name in names {
            binaries.insert(name, package.clone());
        }
    }
    binaries
}

/// `path` relative to `root`, with `/` separators.
fn file_id(root: &Path, path: &Path) -> Option<FileId> {
    let relative = path.strip_prefix(root).ok()?;
    Some(relative.to_string_lossy().replace('\\', "/"))
}

/// The extensions of the source files prunr reads.
const EXTENSIONS: &[&str] = &["js", "cjs", "mjs", "jsx", "ts", "cts", "mts", "tsx"];

/// A TypeScript declaration file: it only declares types, so it is not part of the graph.
fn is_declaration(path: &Path) -> bool {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    [".d.ts", ".d.cts", ".d.mts"]
        .iter()
        .any(|suffix| name.ends_with(suffix))
}

/// Every source file under `root`, without `node_modules`, declarations and ignored files.
fn source_files(root: &Path) -> BTreeMap<PathBuf, FileId> {
    let walker = ignore::WalkBuilder::new(root)
        .require_git(false)
        .filter_entry(|entry| entry.file_name() != "node_modules")
        .build();
    let mut files = BTreeMap::new();
    for entry in walker.flatten() {
        let path = entry.into_path();
        let source = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|extension| EXTENSIONS.contains(&extension));
        if source && !is_declaration(&path) && path.is_file() {
            let id = file_id(root, &path).expect("walked files are under the root");
            files.insert(path, id);
        }
    }
    files
}

/// The files other than `from` that a require pattern in `from` can load: those the glob
/// matches with or without their extension, and directories through their index file.
fn matching<'f>(from: &FileId, glob: &str, files: impl Iterator<Item = &'f FileId>) -> Vec<FileId> {
    let mut segments: Vec<&str> = from.split('/').collect();
    segments.pop();
    for segment in glob.split('/') {
        match segment {
            "." => {}
            ".." if segments.pop().is_none() => return Vec::new(),
            ".." => {}
            segment => segments.push(segment),
        }
    }
    let glob = globset::GlobBuilder::new(&segments.join("/"))
        .literal_separator(true)
        .build()
        .expect("a require pattern escapes its literal text")
        .compile_matcher();
    let mut targets: Vec<FileId> = files
        .filter(|id| *id != from)
        .filter(|id| {
            let stem = id.rsplit_once('.').map_or(id.as_str(), |(stem, _)| stem);
            let directory = stem.strip_suffix("/index");
            glob.is_match(id) || glob.is_match(stem) || directory.is_some_and(|d| glob.is_match(d))
        })
        .cloned()
        .collect();
    targets.sort();
    targets
}

/// The files the globs a plugin gives match. An invalid glob matches nothing.
fn plugin_files(patterns: &[String], files: &BTreeMap<FileId, File>) -> BTreeSet<FileId> {
    patterns
        .iter()
        .filter_map(|pattern| glob(pattern).ok())
        .flat_map(|glob| files.keys().filter(move |id| glob.is_match(id)))
        .cloned()
        .collect()
}

/// A glob over file ids, where `*` does not cross `/` and `**` does.
fn glob(pattern: &str) -> Result<globset::GlobMatcher, globset::Error> {
    Ok(globset::GlobBuilder::new(pattern)
        .literal_separator(true)
        .build()?
        .compile_matcher())
}

impl Project {
    /// Reads every `.js` and `.cjs` file under `root`, skipping `node_modules` and what
    /// `.gitignore` ignores.
    pub fn load(root: &Path) -> Result<Self, ProjectError> {
        let root = crate::resolve::canonical(root);
        let dirs: Vec<String> = std::iter::once(String::new())
            .chain(crate::workspaces::discover(&root))
            .collect();
        let manifests = dirs
            .iter()
            .map(|dir| Manifest::load(&root, dir))
            .collect::<Result<Vec<_>, _>>()?;
        // A package of the repository imported by its name: `@app/shared` or `@app/shared/x`.
        let names: BTreeMap<&str, usize> = manifests
            .iter()
            .enumerate()
            .skip(1)
            .filter_map(|(index, manifest)| Some((manifest.name.as_deref()?, index)))
            .collect();
        let paths = source_files(&root);
        let resolver = Resolver::default();
        let mut analyzed = BTreeMap::new();
        for (path, id) in &paths {
            let source = std::fs::read_to_string(path).unwrap_or_default();
            let facts = analyze(&source, path).map_err(|message| ProjectError::Parse {
                file: id.clone(),
                message,
            })?;
            analyzed.insert(id.clone(), facts);
        }
        let esm: BTreeSet<FileId> = analyzed
            .iter()
            .filter(|(_, facts)| facts.esm)
            .map(|(id, _)| id.clone())
            .collect();
        let workspace_file = |specifier: &str| -> Option<(String, FileId)> {
            let name = crate::resolve::package_name(specifier);
            let &index = names.get(name)?;
            let subpath = &specifier[name.len()..];
            let candidates = if subpath.is_empty() {
                manifests[index].entry_candidates()
            } else {
                vec![format!(".{subpath}"), format!("src{subpath}")]
            };
            let file = package_file(&resolver, &root, &dirs[index], &candidates, &paths)?;
            Some((name.to_string(), file))
        };
        // Where a specifier points: a project file, else what `link` records.
        let target_of =
            |file: &mut File, from: &Path, specifier: &str| match workspace_file(specifier) {
                Some((package, target)) => {
                    file.packages.insert(package);
                    Some(target)
                }
                None => file.link(resolver.resolve(from, specifier), specifier, &paths),
            };
        let mut files = BTreeMap::new();
        for (path, id) in &paths {
            let facts = analyzed.remove(id).expect("every file was analyzed");
            let mut file = File::default();
            for require in &facts.requires {
                if let Some(target) = target_of(&mut file, path, &require.specifier) {
                    file.edge_lines
                        .entry(target.clone())
                        .or_insert(facts.require_lines[&require.specifier]);
                    let uses = if require.esm && !esm.contains(&target) {
                        require.uses.iter().map(as_commonjs).collect()
                    } else {
                        require.uses.clone()
                    };
                    file.edges.push(Edge {
                        target,
                        uses,
                        type_only: require.type_only,
                    });
                }
            }
            for reexport in &facts.reexports {
                if let Some(target) = target_of(&mut file, path, &reexport.specifier) {
                    file.edge_lines
                        .entry(target.clone())
                        .or_insert(facts.require_lines[&reexport.specifier]);
                    file.edges.push(Edge {
                        target: target.clone(),
                        uses: Vec::new(),
                        type_only: reexport.type_only,
                    });
                    file.reexports.push((target, reexport.kind.clone()));
                }
            }
            for pattern in &facts.patterns {
                let targets = matching(id, &pattern.glob, paths.values());
                if targets.is_empty() {
                    file.unresolved.push(pattern.source.clone());
                }
                for target in targets {
                    file.edge_lines
                        .entry(target.clone())
                        .or_insert(facts.require_lines[&pattern.source]);
                    file.edges.push(Edge {
                        target,
                        uses: vec![Use {
                            path: vec![],
                            whole: true,
                        }],
                        type_only: false,
                    });
                }
            }
            file.unresolved
                .extend(facts.dynamic_requires.iter().cloned());
            file.facts = facts;
            files.insert(id.clone(), file);
        }
        let root_declared = manifests[0].declared();
        let mut workspaces = Vec::new();
        let mut production_files = BTreeSet::new();
        let mut test_files = BTreeSet::new();
        for (index, (dir, manifest)) in dirs.iter().zip(&manifests).enumerate() {
            let package = root.join(dir);
            let declared_packages = manifest.declared();
            // What the root declares is available to every package (hoisting, shared tools).
            let visible: BTreeSet<String> =
                declared_packages.union(&root_declared).cloned().collect();
            let binaries = binaries(&[package.clone(), root.clone()], &visible);
            let script_packages = manifest
                .scripts
                .values()
                .flat_map(|script| crate::scripts::packages(script, &binaries))
                .collect();
            let plugins = crate::plugins::contribution(&crate::plugins::Context {
                root: &package,
                declared: &visible,
                manifest: &serde_json::json!({ "jest": manifest.jest, "scripts": manifest.scripts }),
            });
            let in_package = |globs: &[String]| -> Vec<String> {
                globs
                    .iter()
                    .map(|glob| match dir.as_str() {
                        "" => glob.clone(),
                        dir => format!("{dir}/{glob}"),
                    })
                    .collect()
            };
            production_files.extend(plugin_files(&in_package(&plugins.production), &files));
            test_files.extend(plugin_files(&in_package(&plugins.tests), &files));
            for file in &plugins.files {
                if let Target::File(path) = resolver.resolve(&package.join("package.json"), file)
                    && let Some(id) = paths.get(&path)
                {
                    test_files.insert(id.clone());
                }
            }
            // A published package's entry is its interface to whoever installs it.
            if index > 0 && !manifest.private {
                let candidates = manifest.entry_candidates();
                production_files.extend(package_file(&resolver, &root, dir, &candidates, &paths));
            }
            workspaces.push(Workspace {
                dir: dir.clone(),
                dependencies: manifest.dependencies.keys().cloned().collect(),
                dev_dependencies: manifest.dev_dependencies.keys().cloned().collect(),
                declared_packages,
                script_packages,
                test_packages: plugins.packages,
                production_packages: plugins.production_packages,
            });
        }
        Ok(Self {
            root,
            parsed: files.len(),
            files,
            workspaces,
            test_files,
            production_files,
        })
    }

    /// The index in `workspaces` of the package `file` belongs to: the one whose directory is
    /// the longest prefix of its path.
    pub fn workspace_of(&self, file: &str) -> usize {
        self.workspaces
            .iter()
            .enumerate()
            .filter(|(_, workspace)| {
                workspace.dir.is_empty()
                    || file
                        .strip_prefix(&workspace.dir)
                        .is_some_and(|rest| rest.starts_with('/'))
            })
            .max_by_key(|(_, workspace)| workspace.dir.len())
            .map_or(0, |(index, _)| index)
    }

    /// Leaves `packages` out of every dependency check.
    pub fn ignore_packages(&mut self, packages: &[String]) {
        for package in packages {
            for workspace in &mut self.workspaces {
                workspace.dependencies.remove(package);
                workspace.dev_dependencies.remove(package);
                workspace.declared_packages.remove(package);
            }
            for file in self.files.values_mut() {
                file.packages.remove(package);
            }
        }
    }

    /// The files that entry paths or globs (relative to the root) name. Each must name at least
    /// one project file.
    pub fn entries(&self, patterns: &[String]) -> Result<BTreeSet<FileId>, String> {
        let mut entries = BTreeSet::new();
        for pattern in patterns {
            let matched = self.entry(pattern)?;
            if matched.is_empty() {
                return Err(format!("entry matches no source file: {pattern}"));
            }
            entries.extend(matched);
        }
        Ok(entries)
    }

    fn entry(&self, pattern: &str) -> Result<Vec<FileId>, String> {
        if !pattern.contains(['*', '?', '[', '{']) {
            let id = dunce::canonicalize(self.root.join(pattern))
                .ok()
                .and_then(|path| file_id(&self.root, &path));
            return Ok(id
                .into_iter()
                .filter(|id| self.files.contains_key(id))
                .collect());
        }
        let glob = glob(pattern).map_err(|e| format!("invalid entry glob {pattern}: {e}"))?;
        Ok(self
            .files
            .keys()
            .filter(|id| glob.is_match(id))
            .cloned()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (path, content) in files {
            let path = dir.path().join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, content).unwrap();
        }
        dir
    }

    fn ids(project: &Project) -> Vec<&str> {
        project.files.keys().map(String::as_str).collect()
    }

    #[test]
    fn it_reads_javascript_and_typescript_files_but_not_declarations() {
        let dir = project(&[
            ("index.js", ""),
            ("lib/a.cjs", ""),
            ("lib/b.mjs", ""),
            ("lib/c.jsx", ""),
            ("lib/d.ts", ""),
            ("lib/e.cts", ""),
            ("lib/f.mts", ""),
            ("lib/g.tsx", ""),
            ("lib/types.d.ts", ""),
            ("lib/types.d.mts", ""),
            ("lib/types.d.cts", ""),
            ("data.json", "{}"),
        ]);
        let project = Project::load(dir.path()).unwrap();
        assert_eq!(
            ids(&project),
            [
                "index.js",
                "lib/a.cjs",
                "lib/b.mjs",
                "lib/c.jsx",
                "lib/d.ts",
                "lib/e.cts",
                "lib/f.mts",
                "lib/g.tsx"
            ]
        );
    }

    fn edge(target: &str, uses: Vec<Use>) -> Edge {
        Edge {
            target: target.into(),
            uses,
            type_only: false,
        }
    }

    fn used(path: &[&str], whole: bool) -> Use {
        Use {
            path: path.iter().map(|s| s.to_string()).collect(),
            whole,
        }
    }

    #[test]
    fn importing_commonjs_uses_module_exports_as_the_default() {
        let dir = project(&[
            (
                "index.mjs",
                r#"import a from "./a.cjs"; import { b } from "./a.cjs"; import * as ns from "./a.cjs";
                   import e from "./e.mjs";
                   a.x(); b(); ns.default.y; ns.z; e.w;"#,
            ),
            ("a.cjs", "exports.x = 1;"),
            ("e.mjs", "export default {};"),
        ]);
        let project = Project::load(dir.path()).unwrap();
        assert_eq!(
            project.files["index.mjs"].edges,
            [
                edge("a.cjs", vec![used(&[], false), used(&["x"], true)]),
                edge("a.cjs", vec![used(&["b"], false)]),
                edge("a.cjs", vec![used(&[], true), used(&["z"], true)]),
                edge(
                    "e.mjs",
                    vec![used(&["default"], false), used(&["default", "w"], true)]
                ),
            ]
        );
    }

    #[test]
    fn an_import_of_types_only_is_a_type_only_edge() {
        let dir = project(&[
            ("index.ts", r#"import type { T } from "./t"; let x: T;"#),
            ("t.ts", "export type T = 1;"),
        ]);
        let project = Project::load(dir.path()).unwrap();
        assert!(project.files["index.ts"].edges[0].type_only);
    }

    #[test]
    fn reexports_are_edges_that_forward_uses() {
        let dir = project(&[
            (
                "index.ts",
                r#"export { a as b } from "./a"; export type * from "./t"; export * from "pkg";
                   export * from "./missing";"#,
            ),
            ("a.ts", "export const a = 1;"),
            ("t.ts", "export type T = 1;"),
        ]);
        let project = Project::load(dir.path()).unwrap();
        let index = &project.files["index.ts"];
        assert_eq!(
            index.reexports,
            [
                (
                    "a.ts".to_string(),
                    ReexportKind::Named {
                        imported: "a".into(),
                        exported: "b".into()
                    }
                ),
                ("t.ts".to_string(), ReexportKind::All),
            ]
        );
        let edges: Vec<(&str, bool)> = index
            .edges
            .iter()
            .map(|edge| (edge.target.as_str(), edge.type_only))
            .collect();
        assert_eq!(edges, [("a.ts", false), ("t.ts", true)]);
        assert!(index.edges.iter().all(|edge| edge.uses.is_empty()));
        assert_eq!(index.packages, BTreeSet::from(["pkg".to_string()]));
        assert_eq!(index.unresolved, ["./missing"]);
    }

    #[test]
    fn a_file_that_requires_a_builtin_says_so() {
        let dir = project(&[("a.js", r#"require("fs");"#), ("b.js", "")]);
        let project = Project::load(dir.path()).unwrap();
        assert!(project.files["a.js"].builtins);
        assert!(!project.files["b.js"].builtins);
    }

    #[test]
    fn it_skips_node_modules_and_gitignored_files() {
        let dir = project(&[
            ("index.js", ""),
            ("node_modules/pkg/index.js", ""),
            (".gitignore", "dist/\n"),
            ("dist/bundle.js", ""),
        ]);
        let project = Project::load(dir.path()).unwrap();
        assert_eq!(ids(&project), ["index.js"]);
    }

    #[test]
    fn each_file_is_parsed_once_however_often_it_is_required() {
        let dir = project(&[
            (
                "index.js",
                r#"require("./shared"); require("./a"); require("./b");"#,
            ),
            ("a.js", r#"require("./shared");"#),
            ("b.js", r#"require("./shared"); require("./shared");"#),
            ("shared.js", ""),
        ]);
        let project = Project::load(dir.path()).unwrap();
        assert_eq!(project.parsed, 4);
        assert_eq!(project.files.len(), 4);
    }

    #[test]
    fn requires_become_edges_packages_or_unresolved() {
        let dir = project(&[
            (
                "index.js",
                r#"const { a } = require("./lib/a");
                   a();
                   require("./data.json");
                   require("lodash/fp");
                   require("fs");
                   require("./missing");
                   require(name);"#,
            ),
            ("lib/a.js", "exports.a = 1;"),
            ("data.json", "{}"),
        ]);
        let project = Project::load(dir.path()).unwrap();
        let index = &project.files["index.js"];
        assert_eq!(index.edges, [edge("lib/a.js", vec![used(&["a"], false)])]);
        assert_eq!(index.packages, BTreeSet::from(["lodash".to_string()]));
        assert_eq!(index.unresolved, ["./missing", "require(name)"]);
    }

    fn whole() -> Vec<Use> {
        vec![Use {
            path: vec![],
            whole: true,
        }]
    }

    fn targets(file: &File) -> Vec<&str> {
        file.edges.iter().map(|edge| edge.target.as_str()).collect()
    }

    #[test]
    fn a_pattern_uses_every_file_it_matches_whole() {
        let dir = project(&[
            (
                "lib/index.js",
                "require(`./${a}`); require(`../jobs/${a}.job.js`); require(`./sub/${a}`);",
            ),
            ("lib/a.js", ""),
            ("lib/sub/index.js", ""),
            ("lib/sub/deep/b.js", ""),
            ("jobs/daily.job.js", ""),
            ("jobs/daily.js", ""),
        ]);
        let project = Project::load(dir.path()).unwrap();
        let index = &project.files["lib/index.js"];
        assert_eq!(
            targets(index),
            [
                "lib/a.js",
                "lib/sub/index.js",
                "jobs/daily.job.js",
                "lib/sub/index.js"
            ],
            "a file never matches its own pattern, and a directory matches its index"
        );
        assert!(index.edges.iter().all(|edge| edge.uses == whole()));
        assert!(index.unresolved.is_empty(), "{:?}", index.unresolved);
    }

    #[test]
    fn a_pattern_that_matches_nothing_is_unresolved() {
        let dir = project(&[(
            "index.js",
            "require(`./nope/${a}`); require(`../outside/${a}`); require(`./${a}`);",
        )]);
        let project = Project::load(dir.path()).unwrap();
        assert_eq!(
            project.files["index.js"].unresolved,
            [
                "require(`./nope/${a}`)",
                "require(`../outside/${a}`)",
                "require(`./${a}`)"
            ]
        );
    }

    #[test]
    fn package_json_declares_the_packages() {
        let dir = project(&[(
            "package.json",
            r#"{
                "dependencies": { "a": "1" },
                "devDependencies": { "b": "1" },
                "peerDependencies": { "c": "1" },
                "optionalDependencies": { "d": "1" }
            }"#,
        )]);
        let project = Project::load(dir.path()).unwrap();
        assert_eq!(
            project.workspaces[0].dependencies,
            BTreeSet::from(["a".into()])
        );
        assert_eq!(
            project.workspaces[0].dev_dependencies,
            BTreeSet::from(["b".into()])
        );
        assert_eq!(
            project.workspaces[0].declared_packages,
            BTreeSet::from(["a".into(), "b".into(), "c".into(), "d".into()])
        );
    }

    #[test]
    fn the_scripts_use_the_packages_whose_binaries_they_run() {
        let dir = project(&[
            (
                "package.json",
                r#"{
                    "scripts": {
                        "a": "tsc && tool && plain && solo && broken && missing",
                        "b": "node -r undeclared/register x.js"
                    },
                    "devDependencies": {
                        "typescript": "1", "@scope/tool": "1", "plain": "1",
                        "solo-pkg": "1", "broken": "1", "missing": "1", "unused": "1"
                    }
                }"#,
            ),
            (
                "node_modules/typescript/package.json",
                r#"{ "bin": { "tsc": "bin/tsc" } }"#,
            ),
            (
                "node_modules/@scope/tool/package.json",
                r#"{ "bin": "cli.js" }"#,
            ),
            ("node_modules/plain/package.json", "{}"),
            (
                "node_modules/solo-pkg/package.json",
                r#"{ "bin": { "solo": "cli.js" } }"#,
            ),
            ("node_modules/broken/package.json", "{ nope"),
        ]);
        let project = Project::load(dir.path()).unwrap();
        assert_eq!(
            project.workspaces[0].script_packages,
            BTreeSet::from([
                "@scope/tool".into(),
                "broken".into(),
                "missing".into(),
                "solo-pkg".into(),
                "typescript".into(),
                "undeclared".into(),
            ]),
            "a package without a bin field runs nothing; one not installed is guessed by name"
        );
    }

    #[test]
    fn the_test_runners_name_test_files_and_packages() {
        let dir = project(&[
            (
                "package.json",
                r#"{
                    "devDependencies": { "jest": "29" },
                    "jest": {
                        "testMatch": ["[", "*.test.js"],
                        "setupFiles": ["./missing.js", "./setup", "jest-plugin"]
                    }
                }"#,
            ),
            ("a.test.js", ""),
            ("setup.js", ""),
            ("index.js", ""),
        ]);
        let project = Project::load(dir.path()).unwrap();
        assert_eq!(
            project.test_files,
            BTreeSet::from(["a.test.js".into(), "setup.js".into()]),
            "an invalid glob or a missing file is left out"
        );
        assert_eq!(
            project.workspaces[0].test_packages,
            BTreeSet::from(["jest-plugin".into()])
        );
    }

    #[test]
    fn packages_of_the_repository_resolve_by_name_and_run_their_own_plugins() {
        let dir = project(&[
            (
                "package.json",
                r#"{ "workspaces": ["apps/*", "packages/*"] }"#,
            ),
            (
                "apps/web/package.json",
                r#"{ "name": "web", "private": true, "dependencies": { "next": "15", "@x/ui": "*" } }"#,
            ),
            (
                "apps/web/app/page.tsx",
                r#"import { Button } from "@x/ui"; import { icon } from "@x/ui/icon"; export default () => [Button, icon];"#,
            ),
            (
                "packages/ui/package.json",
                r#"{ "name": "@x/ui", "private": true, "exports": { ".": { "types": "./x.d.ts", "import": "./src/index.mjs" } } }"#,
            ),
            ("packages/ui/src/index.mjs", "export const Button = 1;"),
            ("packages/ui/src/icon.mjs", "export const icon = 1;"),
            (
                "packages/pub/package.json",
                r#"{ "name": "pub", "exports": "./lib.js" }"#,
            ),
            ("packages/pub/lib.js", "exports.api = 1;"),
            (
                "packages/cond/package.json",
                r#"{ "name": "cond", "exports": { "require": "./main.cjs" } }"#,
            ),
            ("packages/cond/main.cjs", ""),
            (
                "packages/plain/package.json",
                r#"{ "name": "plain", "main": "lib/main.js" }"#,
            ),
            ("packages/plain/lib/main.js", ""),
        ]);
        let project = Project::load(dir.path()).unwrap();
        let dirs: Vec<&str> = project
            .workspaces
            .iter()
            .map(|workspace| workspace.dir.as_str())
            .collect();
        assert_eq!(
            dirs,
            [
                "",
                "apps/web",
                "packages/cond",
                "packages/plain",
                "packages/pub",
                "packages/ui"
            ]
        );
        assert_eq!(
            project.production_files,
            BTreeSet::from([
                "apps/web/app/page.tsx".into(),
                "packages/cond/main.cjs".into(),
                "packages/plain/lib/main.js".into(),
                "packages/pub/lib.js".into()
            ]),
            "the plugins of apps/web and the published packages' entries"
        );
        let page = &project.files["apps/web/app/page.tsx"];
        let targets: Vec<&str> = page.edges.iter().map(|edge| edge.target.as_str()).collect();
        assert_eq!(
            targets,
            ["packages/ui/src/index.mjs", "packages/ui/src/icon.mjs"]
        );
        assert_eq!(page.packages, BTreeSet::from(["@x/ui".to_string()]));
        assert_eq!(project.workspace_of("apps/web/app/page.tsx"), 1);
        assert_eq!(project.workspace_of("apps/website/x.js"), 0);
        assert_eq!(
            project.workspaces[1].production_packages,
            BTreeSet::from(["react".into(), "react-dom".into()])
        );
    }

    #[test]
    fn an_invalid_package_json_of_a_package_names_it() {
        let dir = project(&[
            ("package.json", r#"{ "workspaces": ["packages/*"] }"#),
            ("packages/a/package.json", "{ nope"),
        ]);
        let error = Project::load(dir.path()).unwrap_err();
        assert!(
            error
                .to_string()
                .starts_with("invalid package.json: packages/a/package.json: "),
            "{error}"
        );
    }

    #[test]
    fn without_package_json_nothing_is_declared() {
        let dir = project(&[("index.js", "")]);
        let project = Project::load(dir.path()).unwrap();
        assert!(project.workspaces[0].declared_packages.is_empty());
    }

    #[test]
    fn an_invalid_package_json_is_an_error() {
        let dir = project(&[("package.json", "{ nope")]);
        let error = Project::load(dir.path()).unwrap_err();
        assert!(matches!(error, ProjectError::PackageJson(_)), "{error:?}");
        assert!(
            error.to_string().starts_with("invalid package.json: "),
            "{error}"
        );
    }

    #[test]
    fn a_file_that_does_not_parse_is_an_error() {
        let dir = project(&[("lib/broken.js", "const = ;")]);
        let error = Project::load(dir.path()).unwrap_err();
        assert!(
            matches!(&error, ProjectError::Parse { file, .. } if file == "lib/broken.js"),
            "{error:?}"
        );
        assert!(error.to_string().starts_with("lib/broken.js: "), "{error}");
    }

    fn set(ids: &[&str]) -> BTreeSet<FileId> {
        ids.iter().map(|id| id.to_string()).collect()
    }

    fn entries(project: &Project, pattern: &str) -> Result<BTreeSet<FileId>, String> {
        project.entries(&[pattern.to_string()])
    }

    #[test]
    fn entries_are_paths_or_globs() {
        let dir = project(&[
            ("index.js", ""),
            ("lib/a.js", ""),
            ("lib/c.cjs", ""),
            ("lib/deep/b.js", ""),
        ]);
        let project = Project::load(dir.path()).unwrap();
        assert_eq!(entries(&project, "./index.js"), Ok(set(&["index.js"])));
        assert_eq!(entries(&project, "lib/*.js"), Ok(set(&["lib/a.js"])));
        assert_eq!(
            entries(&project, "lib/**"),
            Ok(set(&["lib/a.js", "lib/c.cjs", "lib/deep/b.js"]))
        );
        assert_eq!(
            project.entries(&["index.js".into(), "lib/*.cjs".into()]),
            Ok(set(&["index.js", "lib/c.cjs"]))
        );
    }

    #[test]
    fn an_entry_that_names_no_javascript_file_is_an_error() {
        let dir = project(&[("index.js", ""), ("data.json", "{}")]);
        let project = Project::load(dir.path()).unwrap();
        for pattern in ["nope.js", "data.json", "src/**/*.js"] {
            assert_eq!(
                entries(&project, pattern),
                Err(format!("entry matches no source file: {pattern}"))
            );
        }
    }

    #[test]
    fn an_invalid_glob_is_an_error() {
        let dir = project(&[("index.js", "")]);
        let project = Project::load(dir.path()).unwrap();
        let error = entries(&project, "lib/[a.js").unwrap_err();
        assert!(
            error.starts_with("invalid entry glob lib/[a.js: "),
            "{error}"
        );
    }
}

//! The project as one module graph: every file is read and parsed once, and its requires are
//! resolved to the files and packages they point to.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::facts::{Facts, Use, analyze};
use crate::resolve::{Resolver, Target};

/// A path relative to the project root, with `/` separators.
pub type FileId = String;

#[derive(Debug)]
pub struct Project {
    /// The canonical project root.
    pub root: PathBuf,
    pub files: BTreeMap<FileId, File>,
    /// Packages in `dependencies`: they must be used, and by production code.
    pub dependencies: BTreeSet<String>,
    /// Packages in `devDependencies`: they must be used.
    pub dev_dependencies: BTreeSet<String>,
    /// Every package `package.json` declares, in any of its dependency fields.
    pub declared_packages: BTreeSet<String>,
    /// Packages the `scripts` of `package.json` run or preload.
    pub script_packages: BTreeSet<String>,
    /// How many files were parsed.
    pub parsed: usize,
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    pub target: FileId,
    pub uses: Vec<Use>,
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
}

impl Manifest {
    fn load(root: &Path) -> Result<Self, ProjectError> {
        let path = root.join("package.json");
        if !path.is_file() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path).unwrap_or_default();
        serde_json::from_str(&text).map_err(|e| ProjectError::PackageJson(e.to_string()))
    }
}

/// The binaries each package declares, mapped to the package. An installed package declares
/// those in the `bin` of its `package.json`; one that is not installed (or unreadable) is guessed
/// to declare a binary named after it, without the scope.
fn binaries(root: &Path, packages: &BTreeSet<String>) -> BTreeMap<String, String> {
    let mut binaries = BTreeMap::new();
    for package in packages {
        let manifest = root.join("node_modules").join(package).join("package.json");
        let installed: Option<serde_json::Value> = std::fs::read_to_string(manifest)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok());
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

/// Every `.js` and `.cjs` file under `root`, without `node_modules` and ignored files.
fn source_files(root: &Path) -> BTreeMap<PathBuf, FileId> {
    let walker = ignore::WalkBuilder::new(root)
        .require_git(false)
        .filter_entry(|entry| entry.file_name() != "node_modules")
        .build();
    let mut files = BTreeMap::new();
    for entry in walker.flatten() {
        let path = entry.into_path();
        let javascript = matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("js" | "cjs")
        );
        if javascript && path.is_file() {
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

impl Project {
    /// Reads every `.js` and `.cjs` file under `root`, skipping `node_modules` and what
    /// `.gitignore` ignores.
    pub fn load(root: &Path) -> Result<Self, ProjectError> {
        let root = crate::resolve::canonical(root);
        let manifest = Manifest::load(&root)?;
        let paths = source_files(&root);
        let resolver = Resolver::default();
        let mut files = BTreeMap::new();
        for (path, id) in &paths {
            let source = std::fs::read_to_string(path).unwrap_or_default();
            let facts = analyze(&source).map_err(|message| ProjectError::Parse {
                file: id.clone(),
                message,
            })?;
            let mut file = File::default();
            for require in &facts.requires {
                match resolver.resolve(path, &require.specifier) {
                    Target::File(target) => {
                        if let Some(target) = paths.get(&target) {
                            file.edges.push(Edge {
                                target: target.clone(),
                                uses: require.uses.clone(),
                            });
                        }
                    }
                    Target::Package(package) => {
                        file.packages.insert(package);
                    }
                    Target::Builtin => {}
                    Target::Unresolved => file.unresolved.push(require.specifier.clone()),
                }
            }
            for pattern in &facts.patterns {
                let targets = matching(id, &pattern.glob, paths.values());
                if targets.is_empty() {
                    file.unresolved.push(pattern.source.clone());
                }
                for target in targets {
                    file.edges.push(Edge {
                        target,
                        uses: vec![Use {
                            path: vec![],
                            whole: true,
                        }],
                    });
                }
            }
            file.unresolved
                .extend(facts.dynamic_requires.iter().cloned());
            file.facts = facts;
            files.insert(id.clone(), file);
        }
        let dependencies: BTreeSet<String> = manifest.dependencies.into_keys().collect();
        let dev_dependencies: BTreeSet<String> = manifest.dev_dependencies.into_keys().collect();
        let mut declared_packages: BTreeSet<String> =
            dependencies.union(&dev_dependencies).cloned().collect();
        declared_packages.extend(manifest.peer_dependencies.into_keys());
        declared_packages.extend(manifest.optional_dependencies.into_keys());
        let binaries = binaries(&root, &declared_packages);
        let script_packages = manifest
            .scripts
            .values()
            .flat_map(|script| crate::scripts::packages(script, &binaries))
            .collect();
        Ok(Self {
            root,
            parsed: files.len(),
            files,
            dependencies,
            dev_dependencies,
            declared_packages,
            script_packages,
        })
    }

    /// Leaves `packages` out of every dependency check.
    pub fn ignore_packages(&mut self, packages: &[String]) {
        for package in packages {
            self.dependencies.remove(package);
            self.dev_dependencies.remove(package);
            self.declared_packages.remove(package);
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
                return Err(format!("entry matches no .js or .cjs file: {pattern}"));
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
        let glob = globset::GlobBuilder::new(pattern)
            .literal_separator(true)
            .build()
            .map_err(|e| format!("invalid entry glob {pattern}: {e}"))?
            .compile_matcher();
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
    fn it_reads_js_and_cjs_files_only() {
        let dir = project(&[
            ("index.js", ""),
            ("lib/a.cjs", ""),
            ("lib/b.mjs", ""),
            ("lib/c.ts", ""),
            ("data.json", "{}"),
        ]);
        let project = Project::load(dir.path()).unwrap();
        assert_eq!(ids(&project), ["index.js", "lib/a.cjs"]);
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
        assert_eq!(
            index.edges,
            [Edge {
                target: "lib/a.js".into(),
                uses: vec![Use {
                    path: vec!["a".into()],
                    whole: false
                }],
            }]
        );
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
        assert_eq!(project.dependencies, BTreeSet::from(["a".into()]));
        assert_eq!(project.dev_dependencies, BTreeSet::from(["b".into()]));
        assert_eq!(
            project.declared_packages,
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
            project.script_packages,
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
    fn without_package_json_nothing_is_declared() {
        let dir = project(&[("index.js", "")]);
        let project = Project::load(dir.path()).unwrap();
        assert!(project.declared_packages.is_empty());
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
                Err(format!("entry matches no .js or .cjs file: {pattern}"))
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

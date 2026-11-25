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
    /// Packages in `dependencies` and `devDependencies`: they must be used.
    pub required_packages: BTreeSet<String>,
    /// Every package `package.json` declares, in any of its dependency fields.
    pub declared_packages: BTreeSet<String>,
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
            file.unresolved
                .extend(facts.dynamic_requires.iter().cloned());
            file.facts = facts;
            files.insert(id.clone(), file);
        }
        let required_packages: BTreeSet<String> = manifest
            .dependencies
            .keys()
            .chain(manifest.dev_dependencies.keys())
            .cloned()
            .collect();
        let mut declared_packages = required_packages.clone();
        declared_packages.extend(manifest.peer_dependencies.into_keys());
        declared_packages.extend(manifest.optional_dependencies.into_keys());
        Ok(Self {
            root,
            parsed: files.len(),
            files,
            required_packages,
            declared_packages,
        })
    }

    /// The id of `path`, relative to the root, if it is a project file.
    pub fn id_of(&self, path: &str) -> Option<FileId> {
        let path = dunce::canonicalize(self.root.join(path)).ok()?;
        file_id(&self.root, &path).filter(|id| self.files.contains_key(id))
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
            project.required_packages,
            BTreeSet::from(["a".into(), "b".into()])
        );
        assert_eq!(
            project.declared_packages,
            BTreeSet::from(["a".into(), "b".into(), "c".into(), "d".into()])
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

    #[test]
    fn an_entry_maps_to_its_file_id() {
        let dir = project(&[("index.js", ""), ("lib/a.js", "")]);
        let project = Project::load(dir.path()).unwrap();
        assert_eq!(project.id_of("./lib/a.js"), Some("lib/a.js".into()));
        assert_eq!(project.id_of("index.js"), Some("index.js".into()));
        assert_eq!(project.id_of("nope.js"), None);
    }
}

//! The project as one module graph: every file is read and parsed once, and its requires are
//! resolved to the files and packages they point to.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::facts::{Facts, Use};

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
        todo!()
    }
}

impl Project {
    /// Reads every `.js` and `.cjs` file under `root`, skipping `node_modules` and what
    /// `.gitignore` ignores.
    pub fn load(root: &Path) -> Result<Self, ProjectError> {
        todo!()
    }

    /// The id of `path`, relative to the root, if it is a project file.
    pub fn id_of(&self, path: &str) -> Option<FileId> {
        todo!()
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
        let ProjectError::Parse { file, .. } = &error else {
            panic!("{error:?}");
        };
        assert_eq!(file, "lib/broken.js");
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

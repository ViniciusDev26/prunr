//! The checks, all run over the same module graph.

use serde::Serialize;

use crate::project::{FileId, Project};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Finding {
    /// No entry reaches the file.
    UnusedFile { file: FileId },
    /// No other file uses the export, and neither does its own file.
    UnusedExport { file: FileId, symbol: String },
    /// Declared in `dependencies` or `devDependencies`, required by no file.
    UnusedDependency { package: String },
    /// Required, but declared in no dependency field of `package.json`.
    MissingDependency { file: FileId, package: String },
    /// A cycle, from its first file back to it.
    CircularImport { cycle: Vec<FileId> },
    /// A require the graph cannot follow. A warning: it does not fail the check.
    UnresolvedRequire { file: FileId, specifier: String },
}

impl Finding {
    pub fn is_warning(&self) -> bool {
        todo!()
    }
}

/// Every finding in the part of the project the `entries` reach, sorted.
pub fn check(project: &Project, entries: &[FileId]) -> Vec<Finding> {
    todo!()
}

//! Where a `require()` specifier points: a file, a package, or a Node.js builtin.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    File(PathBuf),
    /// A bare specifier, reduced to its package name (`lodash/fp` → `lodash`).
    Package(String),
    Builtin,
    /// A relative or absolute specifier that names no file.
    Unresolved,
}

pub struct Resolver {
    inner: oxc_resolver::Resolver,
}

impl Default for Resolver {
    fn default() -> Self {
        todo!()
    }
}

impl Resolver {
    /// Resolves `specifier` as Node.js would from the file `from`.
    pub fn resolve(&self, from: &Path, specifier: &str) -> Target {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("lib/dir")).unwrap();
        for file in ["index.js", "lib/a.js", "lib/data.json", "lib/dir/index.js"] {
            std::fs::write(root.join(file), "").unwrap();
        }
        dir
    }

    fn resolve(root: &Path, specifier: &str) -> Target {
        Resolver::default().resolve(&root.join("index.js"), specifier)
    }

    fn file(root: &Path, path: &str) -> Target {
        Target::File(root.canonicalize().unwrap().join(path))
    }

    #[test]
    fn a_relative_specifier_resolves_like_node() {
        let dir = project();
        let root = dir.path();
        assert_eq!(resolve(root, "./lib/a"), file(root, "lib/a.js"));
        assert_eq!(resolve(root, "./lib/a.js"), file(root, "lib/a.js"));
        assert_eq!(resolve(root, "./lib/data"), file(root, "lib/data.json"));
        assert_eq!(resolve(root, "./lib/dir"), file(root, "lib/dir/index.js"));
    }

    #[test]
    fn it_resolves_from_the_requiring_file_directory() {
        let dir = project();
        let root = dir.path();
        let from = root.join("lib/dir/index.js");
        assert_eq!(Resolver::default().resolve(&from, "../a"), file(root, "lib/a.js"));
    }

    #[test]
    fn an_absolute_specifier_resolves() {
        let dir = project();
        let root = dir.path();
        let absolute = root.join("lib/a.js");
        assert_eq!(resolve(root, absolute.to_str().unwrap()), file(root, "lib/a.js"));
    }

    #[test]
    fn a_missing_file_is_unresolved() {
        let dir = project();
        assert_eq!(resolve(dir.path(), "./nope"), Target::Unresolved);
    }

    #[test]
    fn builtins_are_recognized_with_and_without_the_prefix() {
        let dir = project();
        for specifier in ["fs", "node:fs", "fs/promises", "node:test"] {
            assert_eq!(resolve(dir.path(), specifier), Target::Builtin, "{specifier}");
        }
    }

    #[test]
    fn a_bare_specifier_is_reduced_to_its_package_name() {
        let dir = project();
        let root = dir.path();
        assert_eq!(resolve(root, "lodash"), Target::Package("lodash".into()));
        assert_eq!(resolve(root, "lodash/fp"), Target::Package("lodash".into()));
        assert_eq!(resolve(root, "@scope/pkg"), Target::Package("@scope/pkg".into()));
        assert_eq!(resolve(root, "@scope/pkg/sub"), Target::Package("@scope/pkg".into()));
    }
}

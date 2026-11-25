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
        let strings = |items: &[&str]| items.iter().map(|item| item.to_string()).collect();
        let options = oxc_resolver::ResolveOptions {
            extensions: strings(&[
                ".js", ".mjs", ".cjs", ".jsx", ".ts", ".mts", ".cts", ".tsx", ".json", ".node",
            ]),
            extension_alias: vec![
                (".js".into(), strings(&[".js", ".ts", ".tsx", ".jsx"])),
                (".mjs".into(), strings(&[".mjs", ".mts"])),
                (".cjs".into(), strings(&[".cjs", ".cts"])),
            ],
            tsconfig: Some(oxc_resolver::TsconfigDiscovery::Auto),
            condition_names: strings(&["node", "import", "require", "default"]),
            ..oxc_resolver::ResolveOptions::default()
        };
        Self {
            inner: oxc_resolver::Resolver::new(options),
        }
    }
}

impl Resolver {
    /// Resolves `specifier` as Node.js (and TypeScript, for `paths`) would from the file `from`.
    /// A bare specifier is a package unless it resolves to a file outside `node_modules`.
    pub fn resolve(&self, from: &Path, specifier: &str) -> Target {
        let local = specifier.starts_with(['.', '#']) || Path::new(specifier).is_absolute();
        if !local && nodejs_built_in_modules::is_nodejs_builtin_module(specifier) {
            return Target::Builtin;
        }
        match self.inner.resolve_file(from, specifier) {
            Ok(resolution) if local || !in_node_modules(resolution.path()) => {
                Target::File(canonical(resolution.path()))
            }
            _ if local => Target::Unresolved,
            _ => Target::Package(package_name(specifier).to_string()),
        }
    }
}

fn in_node_modules(path: &Path) -> bool {
    path.components()
        .any(|component| component.as_os_str() == "node_modules")
}

/// The canonical form of a path that exists, without the `\\?\` prefix Windows adds.
pub fn canonical(path: &Path) -> PathBuf {
    dunce::canonicalize(path).expect("the path exists")
}

/// `@scope/name/sub` → `@scope/name`, `name/sub` → `name`.
pub fn package_name(specifier: &str) -> &str {
    let segments = if specifier.starts_with('@') { 2 } else { 1 };
    match specifier.match_indices('/').nth(segments - 1) {
        Some((end, _)) => &specifier[..end],
        None => specifier,
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
        Target::File(dunce::canonicalize(root).unwrap().join(path))
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
        assert_eq!(
            Resolver::default().resolve(&from, "../a"),
            file(root, "lib/a.js")
        );
    }

    #[test]
    fn an_absolute_specifier_resolves() {
        let dir = project();
        let root = dir.path();
        let absolute = root.join("lib/a.js");
        assert_eq!(
            resolve(root, absolute.to_str().unwrap()),
            file(root, "lib/a.js")
        );
    }

    fn typescript_project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        for (path, content) in [
            ("index.ts", ""),
            ("src/math.ts", ""),
            ("src/view.tsx", ""),
            ("src/lib/format.ts", ""),
            ("src/internal.mts", ""),
            ("node_modules/pkg/package.json", r#"{ "main": "index.js" }"#),
            ("node_modules/pkg/index.js", ""),
            (
                "tsconfig.json",
                r#"{ "compilerOptions": { "baseUrl": ".", "paths": { "@/*": ["src/*"] } } }"#,
            ),
            (
                "package.json",
                r##"{ "imports": { "#internal": "./src/internal.mts" } }"##,
            ),
        ] {
            let path = root.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, content).unwrap();
        }
        dir
    }

    #[test]
    fn typescript_files_resolve_with_or_without_a_javascript_extension() {
        let dir = typescript_project();
        let root = dir.path();
        let resolve = |specifier| Resolver::default().resolve(&root.join("index.ts"), specifier);
        assert_eq!(resolve("./src/math"), file(root, "src/math.ts"));
        assert_eq!(resolve("./src/math.js"), file(root, "src/math.ts"));
        assert_eq!(resolve("./src/view.js"), file(root, "src/view.tsx"));
        assert_eq!(
            resolve("./src/internal.mjs"),
            file(root, "src/internal.mts")
        );
    }

    #[test]
    fn tsconfig_paths_and_package_imports_resolve_to_files() {
        let dir = typescript_project();
        let root = dir.path();
        let resolve = |specifier| Resolver::default().resolve(&root.join("index.ts"), specifier);
        assert_eq!(resolve("@/lib/format"), file(root, "src/lib/format.ts"));
        assert_eq!(resolve("#internal"), file(root, "src/internal.mts"));
        assert_eq!(resolve("#missing"), Target::Unresolved);
        assert_eq!(resolve("pkg"), Target::Package("pkg".into()));
        assert_eq!(resolve("@/missing"), Target::Package("@/missing".into()));
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
            assert_eq!(
                resolve(dir.path(), specifier),
                Target::Builtin,
                "{specifier}"
            );
        }
    }

    #[test]
    fn a_bare_specifier_is_reduced_to_its_package_name() {
        let dir = project();
        let root = dir.path();
        assert_eq!(resolve(root, "lodash"), Target::Package("lodash".into()));
        assert_eq!(resolve(root, "lodash/fp"), Target::Package("lodash".into()));
        assert_eq!(
            resolve(root, "@scope/pkg"),
            Target::Package("@scope/pkg".into())
        );
        assert_eq!(
            resolve(root, "@scope/pkg/sub"),
            Target::Package("@scope/pkg".into())
        );
    }
}

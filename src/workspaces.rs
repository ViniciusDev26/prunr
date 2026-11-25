//! Monorepos: the packages a repository declares, whatever tool declares them.

use std::path::Path;

/// The directories (relative to `root`, with `/` separators) of the packages of the repository,
/// sorted. Empty for a repository of one package.
pub fn discover(root: &Path) -> Vec<String> {
    let _ = root;
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repository(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (path, content) in files {
            let path = dir.path().join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, content).unwrap();
        }
        dir
    }

    /// Two packages under `packages/`, one under `apps/`, and one where nothing declares it.
    const PACKAGES: &[(&str, &str)] = &[
        ("packages/a/package.json", r#"{ "name": "a" }"#),
        ("packages/b/package.json", r#"{ "name": "b" }"#),
        ("apps/web/package.json", r#"{ "name": "web" }"#),
        ("tools/x/package.json", r#"{ "name": "x" }"#),
        ("node_modules/dep/package.json", r#"{ "name": "dep" }"#),
    ];

    fn found(declaration: (&str, &str)) -> Vec<String> {
        let mut files = PACKAGES.to_vec();
        files.push(declaration);
        let dir = repository(&files);
        discover(dir.path())
    }

    fn dirs(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| item.to_string()).collect()
    }

    #[test]
    fn a_repository_of_one_package_has_no_workspaces() {
        let dir = repository(&[("package.json", "{}")]);
        assert!(discover(dir.path()).is_empty());
    }

    #[test]
    fn npm_yarn_and_bun_declare_them_in_package_json() {
        let expected = dirs(&["apps/web", "packages/a", "packages/b"]);
        assert_eq!(
            found(("package.json", r#"{ "workspaces": ["packages/*", "apps/*"] }"#)),
            expected
        );
        assert_eq!(
            found((
                "package.json",
                r#"{ "workspaces": { "packages": ["packages/*", "apps/web"], "nohoist": [] } }"#
            )),
            expected
        );
    }

    #[test]
    fn pnpm_declares_them_in_its_workspace_file_with_exclusions() {
        assert_eq!(
            found((
                "pnpm-workspace.yaml",
                "# the packages\npackages:\n  - 'packages/*'\n  - \"apps/**\"\n  - '!packages/b'\n\ncatalog:\n  react: ^19\n"
            )),
            dirs(&["apps/web", "packages/a"])
        );
        assert_eq!(
            found(("pnpm-workspace.yaml", "packages: ['packages/*', apps/web]\n")),
            dirs(&["apps/web", "packages/a", "packages/b"])
        );
    }

    #[test]
    fn lerna_declares_them_in_lerna_json() {
        assert_eq!(
            found(("lerna.json", r#"{ "packages": ["packages/*"] }"#)),
            dirs(&["packages/a", "packages/b"])
        );
    }

    #[test]
    fn rush_names_each_project_folder_in_rush_json_with_comments() {
        assert_eq!(
            found((
                "rush.json",
                r#"{
                    // Rush allows comments.
                    "projects": [
                        { "packageName": "a", "projectFolder": "packages/a" },
                        /* and blocks */
                        { "packageName": "web", "projectFolder": "apps/web" }
                    ],
                    "url": "https://example.com/not/a/comment"
                }"#
            )),
            dirs(&["apps/web", "packages/a"])
        );
    }

    #[test]
    fn nx_projects_are_the_folders_with_a_project_json() {
        assert_eq!(
            found(("apps/web/project.json", r#"{ "name": "web" }"#)),
            dirs(&["apps/web"])
        );
    }

    #[test]
    fn every_tool_adds_up_and_a_broken_file_is_skipped() {
        assert_eq!(
            found(("lerna.json", "{ nope")),
            Vec::<String>::new()
        );
        let mut files = PACKAGES.to_vec();
        files.push(("package.json", r#"{ "workspaces": ["packages/a"] }"#));
        files.push(("lerna.json", r#"{ "packages": ["apps/*"] }"#));
        files.push(("rush.json", "{ nope"));
        files.push(("pnpm-workspace.yaml", "packages:\n  - packages/b\n"));
        let dir = repository(&files);
        assert_eq!(
            discover(dir.path()),
            dirs(&["apps/web", "packages/a", "packages/b"])
        );
    }
}

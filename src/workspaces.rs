//! Monorepos: the packages a repository declares, whatever tool declares them.

use std::collections::BTreeSet;
use std::path::Path;

use serde_json::Value;

/// The directories (relative to `root`, with `/` separators) of the packages of the repository,
/// sorted. Empty for a repository of one package.
///
/// Each tool's declaration adds up: `workspaces` in `package.json` (npm, yarn, bun),
/// `pnpm-workspace.yaml`, `lerna.json`, `rush.json`, and the folders with an nx `project.json`.
/// turbo uses the package manager's. A file that does not parse adds nothing.
pub fn discover(root: &Path) -> Vec<String> {
    let mut patterns: Vec<String> = Vec::new();
    let manifest = read_json(root, "package.json");
    let workspaces = &manifest["workspaces"];
    patterns.extend(strings(workspaces));
    patterns.extend(strings(&workspaces["packages"]));
    if let Ok(text) = std::fs::read_to_string(root.join("pnpm-workspace.yaml")) {
        patterns.extend(yaml_packages(&text));
    }
    patterns.extend(strings(&read_json(root, "lerna.json")["packages"]));
    let rush = std::fs::read_to_string(root.join("rush.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&without_comments(&text)).ok())
        .unwrap_or_default();
    if let Value::Array(projects) = &rush["projects"] {
        patterns.extend(
            projects
                .iter()
                .filter_map(|p| p["projectFolder"].as_str().map(str::to_string)),
        );
    }
    let (exclude, include): (Vec<&String>, Vec<&String>) = patterns
        .iter()
        .partition(|pattern| pattern.starts_with('!'));
    let matcher = |patterns: &[&String], strip: &str| {
        let mut builder = globset::GlobSetBuilder::new();
        for pattern in patterns {
            let pattern = pattern.trim_start_matches(strip).trim_start_matches("./");
            if let Ok(glob) = globset::GlobBuilder::new(pattern.trim_end_matches('/'))
                .literal_separator(true)
                .build()
            {
                builder.add(glob);
            }
        }
        builder.build().expect("each glob was built")
    };
    let include = matcher(&include, "");
    let exclude = matcher(&exclude, "!");
    let mut found = BTreeSet::new();
    let walker = ignore::WalkBuilder::new(root)
        .require_git(false)
        .filter_entry(|entry| entry.file_name() != "node_modules")
        .build();
    for entry in walker.flatten() {
        let name = entry.file_name().to_string_lossy();
        let Some(dir) = entry
            .path()
            .parent()
            .and_then(|dir| dir.strip_prefix(root).ok())
        else {
            continue;
        };
        let dir = dir.to_string_lossy().replace('\\', "/");
        let declared = name == "package.json" && include.is_match(&dir) && !exclude.is_match(&dir);
        if !dir.is_empty() && (declared || name == "project.json") {
            found.insert(dir);
        }
    }
    found.into_iter().collect()
}

fn read_json(root: &Path, name: &str) -> Value {
    std::fs::read_to_string(root.join(name))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// The strings of a JSON array; nothing for anything else.
fn strings(value: &Value) -> Vec<String> {
    match value {
        Value::Array(items) => items
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

/// The `packages` list of a `pnpm-workspace.yaml`: a block list or a flow list.
fn yaml_packages(text: &str) -> Vec<String> {
    let unquote = |item: &str| {
        let item = item.split(" #").next().unwrap_or_default().trim();
        item.trim_matches(['\'', '"']).to_string()
    };
    let mut lines = text.lines();
    let Some(first) = lines.by_ref().find(|line| line.starts_with("packages:")) else {
        return Vec::new();
    };
    let inline = first["packages:".len()..].trim();
    if let Some(list) = inline.strip_prefix('[') {
        return list
            .trim_end_matches(']')
            .split(',')
            .map(unquote)
            .filter(|item| !item.is_empty())
            .collect();
    }
    let mut items = Vec::new();
    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if !line.starts_with([' ', '\t']) {
            break;
        }
        if let Some(item) = trimmed.strip_prefix('-') {
            items.push(unquote(item));
        }
    }
    items
}

/// JSON with its `//` and `/* */` comments taken out (rush.json allows them).
fn without_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut in_string = false;
    while let Some(c) = chars.next() {
        if in_string {
            out.push(c);
            match c {
                '\\' => out.extend(chars.next()),
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match (c, chars.peek()) {
            ('"', _) => {
                in_string = true;
                out.push(c);
            }
            ('/', Some('/')) => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            ('/', Some('*')) => {
                chars.next();
                let mut previous = ' ';
                for c in chars.by_ref() {
                    if previous == '*' && c == '/' {
                        break;
                    }
                    previous = c;
                }
            }
            _ => out.push(c),
        }
    }
    out
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
            found((
                "package.json",
                r#"{ "workspaces": ["packages/*", "apps/*"] }"#
            )),
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
            found((
                "pnpm-workspace.yaml",
                "packages: ['packages/*', apps/web]\n"
            )),
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
    fn odd_declarations_are_read_as_far_as_they_go() {
        assert_eq!(
            found((
                "package.json",
                r#"{ "workspaces": ["packages/[", "./packages/a/"] }"#
            )),
            dirs(&["packages/a"]),
            "an invalid glob matches nothing"
        );
        assert_eq!(
            found(("pnpm-workspace.yaml", "catalog:\n  react: ^19\n")),
            Vec::<String>::new()
        );
        assert_eq!(
            found((
                "pnpm-workspace.yaml",
                "packages:\n  # a comment\n  not-an-item\n  - packages/b # the second\n"
            )),
            dirs(&["packages/b"])
        );
        assert_eq!(
            found((
                "rush.json",
                r#"{ "note": "a \"quoted\" // not a comment", "projects": [{ "projectFolder": "apps/web" }] }"#
            )),
            dirs(&["apps/web"])
        );
    }

    #[test]
    fn every_tool_adds_up_and_a_broken_file_is_skipped() {
        assert_eq!(found(("lerna.json", "{ nope")), Vec::<String>::new());
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

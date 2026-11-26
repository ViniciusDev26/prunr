//! TypeScript: the packages a `tsconfig.json` loads (the configs it `extends`, its language
//! service `plugins`, its `types` and its `jsxImportSource`), following `extends` and
//! `references`, and the type tests `tsc` or tsd check (`test-d/`, `*.test-d.ts`).
//!
//! A project with a `tsconfig.json` enables it even when `typescript` comes from elsewhere.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::{Context, Contribution, Plugin, names};

pub struct TypeScript;

/// The files of type tests: tsd's and those a `tsconfig` includes only to be type-checked.
const TYPE_TESTS: &[&str] = &["**/test-d/**", "**/*.test-d.*"];

impl Plugin for TypeScript {
    fn name(&self) -> &'static str {
        "typescript"
    }

    fn enabled(&self, context: &Context) -> bool {
        context.declared.contains(self.name()) || context.root.join("tsconfig.json").is_file()
    }

    fn contribute(&self, context: &Context, contribution: &mut Contribution) {
        contribution
            .tests
            .extend(TYPE_TESTS.iter().map(|glob| glob.to_string()));
        let mut pending: Vec<PathBuf> = std::fs::read_dir(context.root)
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("tsconfig") && name.ends_with(".json"))
            })
            .collect();
        // The projects the scripts build: `tsc -p tsconfig.build.json`.
        for arguments in context.arguments("tsc") {
            for (index, argument) in arguments.iter().enumerate() {
                if argument == "-p" || argument == "--project" {
                    pending.extend(arguments.get(index + 1).map(|p| context.root.join(p)));
                }
            }
        }
        let mut seen = BTreeSet::new();
        while let Some(path) = pending.pop() {
            let path = match path.is_dir() {
                true => path.join("tsconfig.json"),
                false => path,
            };
            if !seen.insert(path.clone()) {
                continue;
            }
            let Some(config) = read(&path) else {
                continue;
            };
            let dir = path.parent().unwrap_or(Path::new(""));
            for base in names(&config["extends"]) {
                match base.starts_with(['.', '/']) {
                    true => pending.push(json_file(&dir.join(base))),
                    false => contribution.reference(base),
                }
            }
            for reference in config["references"].as_array().into_iter().flatten() {
                pending.extend(reference["path"].as_str().map(|p| dir.join(p)));
            }
            configure(&config["compilerOptions"], contribution);
        }
    }
}

/// What the `compilerOptions` of one config load.
fn configure(options: &Value, contribution: &mut Contribution) {
    for name in names(&options["types"]) {
        let package = match name.starts_with('@') || name.contains('/') {
            true => crate::resolve::package_name(name).to_string(),
            false => format!("@types/{name}"),
        };
        contribution.production_packages.insert(package);
    }
    if let Some(source) = options["jsxImportSource"].as_str() {
        let package = crate::resolve::package_name(source).to_string();
        contribution.production_packages.insert(package);
    }
    for plugin in options["plugins"].as_array().into_iter().flatten() {
        if let Some(name) = plugin["name"].as_str() {
            contribution.reference(name);
        }
    }
}

/// `extends: "./base"` names `./base.json` when no such file exists.
fn json_file(path: &Path) -> PathBuf {
    match path.is_file() {
        true => path.to_path_buf(),
        false => path.with_extension("json"),
    }
}

/// A tsconfig: JSON with comments and trailing commas. `None` when it cannot be read.
fn read(path: &Path) -> Option<Value> {
    let text = std::fs::read_to_string(path).ok()?;
    let text = crate::workspaces::without_comments(text.trim_start_matches('\u{feff}'));
    let trailing = regex::Regex::new(r",(\s*[}\]])").expect("the pattern compiles");
    serde_json::from_str(&trailing.replace_all(&text, "$1")).ok()
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use serde_json::json;

    #[test]
    fn a_tsconfig_names_the_packages_it_loads() {
        let dir = project(&[
            (
                "tsconfig.json",
                r#"{
                    // comments and trailing commas are fine
                    "extends": ["@tsconfig/node20/tsconfig.json", "./base"],
                    "compilerOptions": {
                        "types": ["node", "vitest/globals", "@cloudflare/workers-types"],
                        "plugins": [{ "name": "ts-plugin" }, {}],
                    },
                    "references": [{ "path": "./app" }, { "path": "./tsconfig.json" }, {}],
                }"#,
            ),
            (
                "base.json",
                r#"{ "compilerOptions": { "jsxImportSource": "preact" } }"#,
            ),
            ("tsconfig.broken.json", "{ nope"),
        ]);
        std::fs::create_dir_all(dir.path().join("app")).unwrap();
        std::fs::write(
            dir.path().join("app/tsconfig.json"),
            r#"{ "extends": ["../shared/tsconfig.json", "../base.json"] }"#,
        )
        .unwrap();
        let found = setup(dir.path(), &declared(&["typescript"]), &json!({}));
        assert_eq!(found.packages, declared(&["@tsconfig/node20", "ts-plugin"]));
        assert_eq!(
            found.production_packages,
            declared(&[
                "@cloudflare/workers-types",
                "@types/node",
                "preact",
                "vitest"
            ])
        );
        assert_eq!(found.tests, strings(&["**/test-d/**", "**/*.test-d.*"]));
    }

    #[test]
    fn a_project_a_script_builds_is_read_and_a_tsconfig_alone_enables_it() {
        let dir = project(&[
            ("tsconfig.json", "{}"),
            (
                "build.json",
                r#"{ "compilerOptions": { "types": ["bun"] } }"#,
            ),
        ]);
        let manifest = json!({ "scripts": { "build": "tsc -p build.json && tsc --project gone.json", "watch": "tsc -w -p" } });
        let found = setup(dir.path(), &declared(&[]), &manifest);
        assert_eq!(found.production_packages, declared(&["@types/bun"]));
        let dir = project(&[]);
        let found = setup(dir.path(), &declared(&[]), &json!({}));
        assert!(found.tests.is_empty());
    }
}

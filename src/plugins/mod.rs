//! Plugins: what a tool or framework (Jest, Vitest, Vite, React, Next.js, NestJS...) adds to
//! the graph by convention or through its configuration, which no `import` shows.
//!
//! Each plugin is compiled in and enables itself when `package.json` declares its package. A
//! plugin only reads files: it never runs the project's code. What all enabled plugins
//! contribute is added up before the checks run.

use std::collections::BTreeSet;
use std::path::Path;

use serde_json::Value;

mod drizzle;
mod jest;
mod mocha;
mod nest;
mod next;
mod playwright;
mod postcss;
mod react;
mod runners;
mod static_config;
mod vite;
mod vitest;

pub use static_config::static_config;

/// What a plugin can read about the project.
pub struct Context<'a> {
    pub root: &'a Path,
    /// Every package `package.json` declares.
    pub declared: &'a BTreeSet<String>,
    /// The whole `package.json`.
    pub manifest: &'a Value,
}

impl Context<'_> {
    /// The configuration files the `scripts` of `package.json` pass to `runner` with `--config`.
    fn configs_passed_to(&self, runner: &str) -> Vec<String> {
        let Value::Object(scripts) = &self.manifest["scripts"] else {
            return Vec::new();
        };
        scripts
            .values()
            .filter_map(Value::as_str)
            .flat_map(|script| crate::scripts::configs(script, runner))
            .collect()
    }
}

/// What the enabled plugins add to the graph.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Contribution {
    /// Globs, relative to the root, of test files.
    pub tests: Vec<String>,
    /// Test code the tools load (their configuration, setup files), as specifiers relative to
    /// the root (`./jest.config.js`).
    pub files: Vec<String>,
    /// Packages the tools load by name (presets, environments, reporters, CLIs): used by tests
    /// and tooling.
    pub packages: BTreeSet<String>,
    /// Globs, relative to the root, of the production entries a framework starts from by
    /// convention (Next.js routes, the NestJS `main`).
    pub production: Vec<String>,
    /// Packages a framework loads in production without an `import` (its peer dependencies).
    pub production_packages: BTreeSet<String>,
    /// Directories, relative to the package, where a bundler looks up a specifier that starts
    /// with `/` (Vite's public directory and root), tried in order when no such file exists.
    pub absolute_roots: Vec<String>,
    /// Commands the tools run (Playwright's web servers), read like the `scripts` of
    /// `package.json` that run test code.
    pub scripts: Vec<String>,
}

/// A tool or framework prunr knows.
pub trait Plugin {
    /// The plugin's name, which is also the package that enables it.
    fn name(&self) -> &'static str;

    /// Whether the project uses the tool: by default, when `package.json` declares the package.
    fn enabled(&self, context: &Context) -> bool {
        context.declared.contains(self.name())
    }

    /// Adds what the tool runs and loads.
    fn contribute(&self, context: &Context, contribution: &mut Contribution);
}

/// Every plugin, in the order they contribute.
pub const PLUGINS: &[&dyn Plugin] = &[
    &jest::Jest,
    &runners::Runners,
    &mocha::Mocha,
    &vitest::Vitest,
    &playwright::Playwright,
    &drizzle::DrizzleKit,
    &postcss::PostCss,
    &vite::Vite,
    &react::React,
    &next::Next,
    &nest::Nest,
];

/// What the plugins enabled in the project contribute.
pub fn contribution(context: &Context) -> Contribution {
    let mut contribution = Contribution::default();
    for plugin in PLUGINS {
        if plugin.enabled(context) {
            plugin.contribute(context, &mut contribution);
        }
    }
    contribution
}

/// The extensions a configuration file can have, in the order tools look for them.
const CONFIG_EXTENSIONS: &[&str] = &["js", "ts", "mjs", "cjs", "mts", "cts", "json"];

/// A relative path with `.` and `..` resolved and `/` separators.
pub(super) fn normalize(path: &Path) -> String {
    let mut parts: Vec<String> = Vec::new();
    for part in path.components() {
        match part {
            std::path::Component::ParentDir => {
                parts.pop();
            }
            std::path::Component::Normal(part) => parts.push(part.to_string_lossy().into()),
            _ => {}
        }
    }
    parts.join("/")
}

/// The strings an option holds: itself, or each item of an array; an item can be a
/// `[name, options]` pair.
fn names(option: &Value) -> Vec<&str> {
    let items = match option {
        Value::Array(items) => items.iter().collect(),
        option => vec![option],
    };
    items
        .into_iter()
        .filter_map(|item| match item {
            Value::Array(pair) => pair.first()?.as_str(),
            item => item.as_str(),
        })
        .collect()
}

impl Contribution {
    /// The configuration in the file `name` (relative to `root`), recorded as a file the tool
    /// loads. `None` when there is no such file; `Null` when it cannot be read statically.
    fn read(&mut self, root: &Path, name: &str) -> Option<Value> {
        let name = name.trim_start_matches("./");
        let source = std::fs::read_to_string(root.join(name)).ok()?;
        let file = format!("./{name}");
        if !self.files.contains(&file) {
            self.files.push(file);
        }
        let config = if name.ends_with(".json") {
            serde_json::from_str(&source).ok()
        } else {
            static_config(&source, Path::new(name))
        };
        Some(config.unwrap_or_default())
    }

    /// The first configuration file named `<stem>.<extension>` at `root`, read as [`Self::read`]
    /// does.
    fn config_file(&mut self, root: &Path, stems: &[&str]) -> Option<Value> {
        stems.iter().find_map(|stem| {
            CONFIG_EXTENSIONS
                .iter()
                .find_map(|extension| self.read(root, &format!("{stem}.{extension}")))
        })
    }

    /// A file (`./x`, `/x`, `<rootDir>/x`) or a package the configuration names.
    fn reference(&mut self, name: &str) {
        let name = match name.strip_prefix("<rootDir>/") {
            Some(path) => format!("./{path}"),
            None => name.to_string(),
        };
        if name.starts_with(['.', '/']) {
            self.files.push(name);
        } else {
            self.packages
                .insert(crate::resolve::package_name(&name).to_string());
        }
    }

    /// The test files the arguments of a runner name (a folder, a file, a glob), or `defaults`
    /// when they name none. An argument names a path only if it exists or has glob characters:
    /// the values of flags (`--reporter spec`, `--import tsx`) do not.
    fn test_arguments(&mut self, root: &Path, arguments: &[String], defaults: &[&str]) {
        let named: Vec<String> = arguments
            .iter()
            .filter(|argument| !argument.starts_with('-'))
            .map(|argument| argument.trim_start_matches("./").trim_end_matches('/'))
            .filter_map(|path| {
                if root.join(path).is_dir() {
                    Some(format!("{path}/**"))
                } else if root.join(path).is_file() || path.contains(['*', '?', '{']) {
                    Some(path.to_string())
                } else {
                    None
                }
            })
            .collect();
        if named.is_empty() {
            self.tests
                .extend(defaults.iter().map(|glob| glob.to_string()));
        } else {
            self.tests.extend(named);
        }
    }

    /// The test globs an option lists, or `defaults` when it lists none.
    fn tests(&mut self, patterns: &Value, defaults: &[&str]) {
        let patterns: Vec<String> = names(patterns)
            .into_iter()
            .filter(|pattern| !pattern.starts_with('!'))
            .map(|pattern| pattern.trim_start_matches("<rootDir>/").to_string())
            .collect();
        if patterns.is_empty() {
            self.tests.extend(defaults.iter().map(|d| d.to_string()));
        } else {
            self.tests.extend(patterns);
        }
    }
}

#[cfg(test)]
pub(crate) mod testing {
    use super::*;

    pub fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (path, content) in files {
            std::fs::write(dir.path().join(path), content).unwrap();
        }
        dir
    }

    pub fn declared(packages: &[&str]) -> BTreeSet<String> {
        packages.iter().map(|p| p.to_string()).collect()
    }

    pub fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| item.to_string()).collect()
    }

    /// What the plugins contribute for a project with these packages and `package.json`.
    pub fn setup(root: &Path, packages: &BTreeSet<String>, manifest: &Value) -> Contribution {
        contribution(&Context {
            root,
            declared: packages,
            manifest,
        })
    }

    pub const JEST_DEFAULTS: &[&str] = &[
        "**/__mocks__/**",
        "**/__tests__/**",
        "**/*.test.*",
        "**/*.spec.*",
        "**/test.*",
        "**/spec.*",
    ];
}

#[cfg(test)]
mod tests {
    use super::testing::*;
    use super::*;
    use serde_json::json;

    #[test]
    fn every_plugin_is_named_after_its_package() {
        let names: Vec<&str> = PLUGINS.iter().map(|plugin| plugin.name()).collect();
        assert_eq!(
            names,
            [
                "jest",
                "test-runners",
                "mocha",
                "vitest",
                "@playwright/test",
                "drizzle-kit",
                "postcss",
                "vite",
                "react",
                "next",
                "@nestjs/core"
            ]
        );
    }

    #[test]
    fn without_a_known_package_no_plugin_contributes() {
        let dir = project(&[("jest.config.js", "module.exports = {};")]);
        assert_eq!(
            setup(dir.path(), &declared(&["jasmine"]), &json!({})),
            Contribution::default()
        );
    }
    #[test]
    fn a_config_a_script_passes_is_read_too() {
        let dir = project(&[
            ("jest.config.js", "module.exports = {};"),
            (
                "jest.e2e.config.js",
                r#"module.exports = { testMatch: ["e2e/**/*.js"] };"#,
            ),
            (
                "vitest.bench.json",
                r#"{ "test": { "include": ["bench/**"] } }"#,
            ),
        ]);
        let manifest = json!({
            "scripts": {
                "e2e": "jest --config jest.e2e.config.js",
                "bench": "vitest -c vitest.bench.json",
                "gone": "jest --config ./gone.js && vitest -c gone.ts",
                "again": "jest --config jest.config.js",
                "other": 1
            }
        });
        let found = setup(dir.path(), &declared(&["jest", "vitest"]), &manifest);
        assert_eq!(
            found.files,
            strings(&[
                "./jest.config.js",
                "./jest.e2e.config.js",
                "./vitest.bench.json"
            ])
        );
        assert!(found.tests.contains(&"e2e/**/*.js".to_string()));
        assert!(found.tests.contains(&"bench/**".to_string()));
    }

    #[test]
    fn jest_and_vitest_add_up() {
        let dir = project(&[]);
        let found = setup(dir.path(), &declared(&["jest", "vitest"]), &json!({}));
        assert_eq!(found.tests.len(), JEST_DEFAULTS.len() + 1);
    }
}

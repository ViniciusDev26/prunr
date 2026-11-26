//! Tools described as data: where each keeps its configuration, which keys of it name packages,
//! entries or commands, and which files are its tests. A configuration in JavaScript is also a
//! file the tool loads, so what it imports is used too (the `globals` an `eslint.config.js`
//! imports).
//!
//! A tool is enabled by its package or by one of its configuration files, and with a
//! configuration its own package counts as used: editors and hooks run it without a script.

use std::path::Path;

use serde_json::Value;

use super::{Context, Contribution, Plugin, names};

/// A tool whose configuration names packages.
pub struct Tool {
    /// The package of the tool.
    pub name: &'static str,
    /// Its configuration files at the package root, in the order the tool looks for them.
    pub configs: &'static [&'static str],
    /// The key of `package.json` that can hold its configuration.
    pub manifest_key: &'static str,
    /// Keys of the configuration, at any depth (`overrides`, `env`), whose values name packages.
    pub keys: &'static [Key],
    /// Globs of its test files (`**/*.stories.*`, `cypress/**`).
    pub tests: &'static [&'static str],
    /// Keys whose values are production entries, relative to the package (tsup's `entry`).
    pub entry_keys: &'static [&'static str],
    /// Keys whose values are commands it runs (Prisma's `seed`); `*` for every value.
    pub command_keys: &'static [&'static str],
    /// Files of commands it runs, one per line (husky's hooks).
    pub command_files: &'static [&'static str],
}

/// A tool with nothing: each tool fills what it has.
const BASE: Tool = Tool {
    name: "",
    configs: &[],
    manifest_key: "",
    keys: &[],
    tests: &[],
    entry_keys: &[],
    command_keys: &[],
    command_files: &[],
};

/// A key of a configuration whose values are package names, maybe in the tool's shorthand.
pub struct Key {
    pub name: &'static str,
    /// The prefix a shorthand expands with (`eslint-config`: `airbnb` is
    /// `eslint-config-airbnb`), and the one it takes in a scope (`preset`: `@babel/env` is
    /// `@babel/preset-env`). Empty for full names.
    pub prefix: &'static str,
    pub scoped_prefix: &'static str,
}

const fn key(name: &'static str, prefix: &'static str, scoped_prefix: &'static str) -> Key {
    Key {
        name,
        prefix,
        scoped_prefix,
    }
}

pub const ESLINT: Tool = Tool {
    name: "eslint",
    configs: &[
        "eslint.config.js",
        "eslint.config.mjs",
        "eslint.config.cjs",
        "eslint.config.ts",
        "eslint.config.mts",
        "eslint.config.cts",
        ".eslintrc.js",
        ".eslintrc.cjs",
        ".eslintrc.yaml",
        ".eslintrc.yml",
        ".eslintrc.json",
        ".eslintrc",
    ],
    manifest_key: "eslintConfig",
    keys: &[
        key("extends", "eslint-config", "eslint-config"),
        key("plugins", "eslint-plugin", "eslint-plugin"),
        key("parser", "", ""),
    ],
    ..BASE
};

pub const PRETTIER: Tool = Tool {
    name: "prettier",
    configs: &[
        ".prettierrc",
        ".prettierrc.json",
        ".prettierrc.yaml",
        ".prettierrc.yml",
        ".prettierrc.json5",
        ".prettierrc.js",
        ".prettierrc.cjs",
        ".prettierrc.mjs",
        ".prettierrc.ts",
        "prettier.config.js",
        "prettier.config.cjs",
        "prettier.config.mjs",
        "prettier.config.ts",
    ],
    manifest_key: "prettier",
    keys: &[key("plugins", "", "")],
    ..BASE
};

pub const BABEL: Tool = Tool {
    name: "@babel/core",
    configs: &[
        "babel.config.js",
        "babel.config.cjs",
        "babel.config.mjs",
        "babel.config.cts",
        "babel.config.json",
        ".babelrc",
        ".babelrc.js",
        ".babelrc.cjs",
        ".babelrc.mjs",
        ".babelrc.json",
    ],
    manifest_key: "babel",
    keys: &[
        key("presets", "babel-preset", "preset"),
        key("plugins", "babel-plugin", "plugin"),
    ],
    ..BASE
};

pub const COMMITLINT: Tool = Tool {
    name: "@commitlint/cli",
    configs: &[
        "commitlint.config.js",
        "commitlint.config.cjs",
        "commitlint.config.mjs",
        "commitlint.config.ts",
        "commitlint.config.cts",
        ".commitlintrc",
        ".commitlintrc.json",
        ".commitlintrc.yaml",
        ".commitlintrc.yml",
        ".commitlintrc.js",
        ".commitlintrc.cjs",
        ".commitlintrc.mjs",
        ".commitlintrc.ts",
    ],
    manifest_key: "commitlint",
    keys: &[
        key("extends", "commitlint-config", "commitlint-config"),
        key("plugins", "commitlint-plugin", "commitlint-plugin"),
    ],
    ..BASE
};

pub const TAILWIND: Tool = Tool {
    name: "tailwindcss",
    configs: &[
        "tailwind.config.js",
        "tailwind.config.cjs",
        "tailwind.config.mjs",
        "tailwind.config.ts",
        "tailwind.config.cts",
        "tailwind.config.mts",
    ],
    ..BASE
};

pub const STORYBOOK: Tool = Tool {
    name: "storybook",
    configs: &[
        ".storybook/main.js",
        ".storybook/main.cjs",
        ".storybook/main.mjs",
        ".storybook/main.ts",
        ".storybook/main.cts",
        ".storybook/main.mts",
        ".storybook/preview.js",
        ".storybook/preview.jsx",
        ".storybook/preview.mjs",
        ".storybook/preview.ts",
        ".storybook/preview.tsx",
    ],
    keys: &[key("addons", "", ""), key("framework", "", "")],
    tests: &["**/*.stories.*"],
    ..BASE
};

pub const CYPRESS: Tool = Tool {
    name: "cypress",
    configs: &[
        "cypress.config.js",
        "cypress.config.cjs",
        "cypress.config.mjs",
        "cypress.config.ts",
        "cypress.config.cts",
        "cypress.config.mts",
    ],
    tests: &["cypress/**", "**/*.cy.*"],
    ..BASE
};

pub const HUSKY: Tool = Tool {
    name: "husky",
    command_files: &[
        ".husky/pre-commit",
        ".husky/prepare-commit-msg",
        ".husky/commit-msg",
        ".husky/post-commit",
        ".husky/pre-push",
        ".husky/pre-rebase",
        ".husky/post-checkout",
        ".husky/post-merge",
        ".husky/post-rewrite",
    ],
    ..BASE
};

pub const LINT_STAGED: Tool = Tool {
    name: "lint-staged",
    configs: &[
        ".lintstagedrc",
        ".lintstagedrc.json",
        ".lintstagedrc.yaml",
        ".lintstagedrc.yml",
        ".lintstagedrc.js",
        ".lintstagedrc.cjs",
        ".lintstagedrc.mjs",
        "lint-staged.config.js",
        "lint-staged.config.cjs",
        "lint-staged.config.mjs",
    ],
    manifest_key: "lint-staged",
    command_keys: &["*"],
    ..BASE
};

pub const PRISMA: Tool = Tool {
    name: "prisma",
    configs: &["prisma.config.ts", "prisma/schema.prisma"],
    manifest_key: "prisma",
    command_keys: &["seed"],
    ..BASE
};

pub const TSUP: Tool = Tool {
    name: "tsup",
    configs: &[
        "tsup.config.ts",
        "tsup.config.cts",
        "tsup.config.mts",
        "tsup.config.js",
        "tsup.config.cjs",
        "tsup.config.mjs",
        "tsup.config.json",
    ],
    manifest_key: "tsup",
    entry_keys: &["entry"],
    ..BASE
};

pub const ROLLUP: Tool = Tool {
    name: "rollup",
    configs: &[
        "rollup.config.js",
        "rollup.config.cjs",
        "rollup.config.mjs",
        "rollup.config.ts",
        "rollup.config.cts",
        "rollup.config.mts",
    ],
    entry_keys: &["input"],
    ..BASE
};

pub const WEBPACK: Tool = Tool {
    name: "webpack",
    configs: &[
        "webpack.config.js",
        "webpack.config.cjs",
        "webpack.config.mjs",
        "webpack.config.ts",
        "webpack.config.cts",
        "webpack.config.mts",
    ],
    keys: &[key("loader", "", ""), key("use", "", "")],
    entry_keys: &["entry"],
    ..BASE
};

impl Tool {
    fn configured(&self, context: &Context) -> bool {
        !context.manifest[self.manifest_key].is_null()
            || self
                .configs
                .iter()
                .chain(self.command_files)
                .any(|name| context.root.join(name).is_file())
    }
}

impl Plugin for Tool {
    fn name(&self) -> &'static str {
        self.name
    }

    fn enabled(&self, context: &Context) -> bool {
        context.declared.contains(self.name) || self.configured(context)
    }

    fn contribute(&self, context: &Context, contribution: &mut Contribution) {
        if !self.configured(context) {
            return;
        }
        contribution.packages.insert(self.name.to_string());
        contribution
            .tests
            .extend(self.tests.iter().map(|glob| glob.to_string()));
        for name in self.command_files {
            let source = std::fs::read_to_string(context.root.join(name)).unwrap_or_default();
            // Not the shebang, comments, or husky 8's `. "$(dirname -- "$0")/_/husky.sh"`.
            let commands = source.lines().map(str::trim).filter(|line| {
                !line.is_empty() && !line.starts_with('#') && !line.starts_with(". ")
            });
            contribution.scripts.extend(commands.map(str::to_string));
        }
        let mut configs = vec![context.manifest[self.manifest_key].clone()];
        for name in self.configs {
            configs.extend(load(context.root, name, contribution));
        }
        for config in &configs {
            self.uses(config, context, contribution);
        }
    }
}

impl Tool {
    /// The packages a configuration names: a string is a shared configuration; in an object,
    /// the values of the tool's keys, at any depth.
    fn uses(&self, config: &Value, context: &Context, contribution: &mut Contribution) {
        match config {
            Value::String(name) => {
                contribution.packages.extend(expand(name, &FULL, context));
            }
            Value::Array(items) => {
                for item in items {
                    self.uses_in(item, context, contribution);
                }
            }
            config => self.uses_in(config, context, contribution),
        }
    }

    fn uses_in(&self, config: &Value, context: &Context, contribution: &mut Contribution) {
        match config {
            Value::Object(object) => {
                for (name, value) in object {
                    if let Some(key) = self.keys.iter().find(|key| key.name == name) {
                        for item in named(value) {
                            contribution.packages.extend(expand(item, key, context));
                        }
                    }
                    if self.entry_keys.contains(&name.as_str()) {
                        contribution
                            .production
                            .extend(strings(value).into_iter().flat_map(entry));
                    }
                    if self
                        .command_keys
                        .iter()
                        .any(|key| *key == "*" || key == name)
                    {
                        let commands = strings(value).into_iter().map(str::to_string);
                        contribution.scripts.extend(commands);
                    }
                    self.uses_in(value, context, contribution);
                }
            }
            Value::Array(items) => {
                for item in items {
                    self.uses_in(item, context, contribution);
                }
            }
            _ => {}
        }
    }
}

/// The names in a value: a name, a `[name, options]` pair, an object with a `name`
/// (`{ name: "@storybook/react-vite", options }`), or a list of them.
fn named(value: &Value) -> Vec<&str> {
    let items: Vec<&Value> = match value {
        Value::Array(items) => items.iter().collect(),
        value => vec![value],
    };
    items
        .into_iter()
        .filter_map(|item| match item {
            Value::Object(object) => object.get("name")?.as_str(),
            item => names(item).first().copied(),
        })
        .collect()
}

/// Every string in a value, at any depth (the entries of webpack's `{ app: "./src/app" }`).
fn strings(value: &Value) -> Vec<&str> {
    match value {
        Value::String(text) => vec![text.as_str()],
        Value::Array(items) => items.iter().flat_map(strings).collect(),
        Value::Object(object) => object.values().flat_map(strings).collect(),
        _ => Vec::new(),
    }
}

/// The globs of an entry path: itself, and with any extension when it has none (`./src/app`).
fn entry(path: &str) -> Vec<String> {
    let path = path.trim_start_matches("./");
    match Path::new(path).extension() {
        Some(_) => vec![path.to_string()],
        None => vec![path.to_string(), format!("{path}.*")],
    }
}

/// The package a name in a configuration stands for: the first of its expansions the project
/// declares, else the tool's own expansion. `None` for a file and for a name built into the
/// tool (`eslint:recommended`).
fn expand(name: &str, key: &Key, context: &Context) -> Option<String> {
    // Babel's `module:x` is the package `x`; ESLint's `plugin:react/recommended` names the
    // plugin before the last `/`.
    let (name, key) = match (name.strip_prefix("module:"), name.strip_prefix("plugin:")) {
        (Some(package), _) => (package, &FULL),
        (_, Some(rest)) => (
            rest.rsplit_once('/').map_or(rest, |(plugin, _)| plugin),
            &PLUGIN,
        ),
        _ => (name, key),
    };
    if name.starts_with(['.', '/']) || name.contains(':') {
        return None;
    }
    let candidates = candidates(name, key);
    candidates
        .iter()
        .find(|candidate| context.declared.contains(*candidate))
        .or(candidates.first())
        .cloned()
}

const PLUGIN: Key = key("plugins", "eslint-plugin", "eslint-plugin");
const FULL: Key = key("", "", "");

/// The packages a name can be, in the order they are tried.
fn candidates(name: &str, key: &Key) -> Vec<String> {
    let package = crate::resolve::package_name(name).to_string();
    if key.prefix.is_empty() {
        return vec![package];
    }
    match name.strip_prefix('@') {
        Some(scoped) => {
            let (scope, rest) = scoped.split_once('/').unwrap_or((scoped, ""));
            let rest = rest.split('/').next().unwrap_or_default();
            if rest.is_empty() {
                return vec![format!("@{scope}/{}", key.prefix)];
            }
            vec![
                format!("@{scope}/{}-{rest}", key.prefix),
                format!("@{scope}/{}-{rest}", key.scoped_prefix),
                package,
            ]
        }
        None => {
            let first = name.split('/').next().unwrap_or(name);
            vec![format!("{}-{first}", key.prefix), package]
        }
    }
}

/// A configuration file: JavaScript (also recorded as a file the tool loads), JSON with
/// comments, or else YAML. `None` when there is no such file.
fn load(root: &Path, name: &str, contribution: &mut Contribution) -> Option<Value> {
    let javascript = [".js", ".cjs", ".mjs", ".jsx", ".ts", ".cts", ".mts", ".tsx"];
    if javascript.iter().any(|extension| name.ends_with(extension)) {
        return contribution.read(root, name);
    }
    let source = std::fs::read_to_string(root.join(name)).ok()?;
    let json = crate::workspaces::without_comments(&source);
    Some(serde_json::from_str(&json).unwrap_or_else(|_| super::mocha::yaml(&source)))
}

#[cfg(test)]
mod tests {
    use super::super::Context;
    use super::super::testing::*;
    use super::{expand, key};
    use serde_json::json;

    #[test]
    fn hooks_commands_entries_tests_and_named_objects_are_read() {
        let dir = project(&[
            (
                ".lintstagedrc",
                "{ \"*.js\": [\"eslint --fix\", \"prettier -w\"] }",
            ),
            (
                "webpack.config.js",
                r#"module.exports = { entry: { app: "./src/app", vendor: ["./src/v.js"], other: process.env.ENTRY }, module: { rules: [{ use: ["css-loader", { loader: "x-loader" }] }] } };"#,
            ),
        ]);
        std::fs::create_dir_all(dir.path().join(".husky")).unwrap();
        std::fs::create_dir_all(dir.path().join(".storybook")).unwrap();
        std::fs::write(
            dir.path().join(".husky/pre-push"),
            "#!/bin/sh\n. \"$(dirname -- \"$0\")/_/husky.sh\"\n\nnpm test\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join(".storybook/main.ts"),
            r#"export default { framework: { name: "@storybook/vue3-vite" }, addons: [{ name: "a" }, "b", ["c", {}], {}], stories: [] };"#,
        )
        .unwrap();
        let manifest = json!({ "prisma": { "seed": "node seed.js", "schema": 1 } });
        let found = setup(dir.path(), &declared(&[]), &manifest);
        assert_eq!(
            found.scripts,
            strings(&["npm test", "eslint --fix", "prettier -w", "node seed.js"])
        );
        assert_eq!(
            found.production,
            strings(&["src/app", "src/app.*", "src/v.js"])
        );
        assert_eq!(found.tests, strings(&["**/*.stories.*"]));
        for package in [
            "@storybook/vue3-vite",
            "a",
            "b",
            "c",
            "css-loader",
            "x-loader",
            "husky",
        ] {
            assert!(found.packages.contains(package), "{package}");
        }
    }

    #[test]
    fn a_key_without_a_prefix_names_whole_packages() {
        let dir = project(&[]);
        let context = Context {
            root: dir.path(),
            declared: &declared(&[]),
            manifest: &json!({}),
        };
        let parser = key("parser", "", "");
        assert_eq!(expand("@a/b/c", &parser, &context).as_deref(), Some("@a/b"));
        assert_eq!(expand("eslint:all", &parser, &context), None);
    }

    #[test]
    fn shorthands_expand_to_what_the_project_declares() {
        let dir = project(&[(
            ".eslintrc.json",
            r#"{
                "extends": ["airbnb/hooks", "@scope", "@vue/typescript/recommended", "./local.js", "module:x"],
                "plugins": ["@foo/bar", "react"],
                "overrides": [{ "parser": "vue-eslint-parser" }]
            }"#,
        )]);
        let found = setup(
            dir.path(),
            &declared(&["eslint", "@vue/eslint-config-typescript"]),
            &json!({}),
        );
        assert_eq!(
            found.packages,
            declared(&[
                "@foo/eslint-plugin-bar",
                "@scope/eslint-config",
                "@vue/eslint-config-typescript",
                "eslint",
                "eslint-config-airbnb",
                "eslint-plugin-react",
                "vue-eslint-parser",
                "x",
            ])
        );
    }

    #[test]
    fn a_string_configuration_is_a_shared_one_and_a_tool_without_one_adds_nothing() {
        let dir = project(&[(".prettierrc", "\"@company/prettier-config\"")]);
        let found = setup(dir.path(), &declared(&[]), &json!({}));
        assert_eq!(
            found.packages,
            declared(&["@company/prettier-config", "prettier"])
        );
        let manifest = json!({ "babel": [{ "presets": ["@babel/env"] }] });
        let found = setup(dir.path(), &declared(&["@babel/preset-env"]), &manifest);
        assert!(found.packages.contains("@babel/preset-env"));
        let dir = project(&[(
            ".eslintrc.yml",
            "extends:\n  - plugin:react/recommended\n  - module:@x/y\n",
        )]);
        let found = setup(dir.path(), &declared(&[]), &json!({}));
        assert_eq!(
            found.packages,
            declared(&["@x/y", "eslint", "eslint-plugin-react"])
        );
        let dir = project(&[]);
        let found = setup(dir.path(), &declared(&["eslint"]), &json!({}));
        assert!(found.packages.is_empty());
    }
}

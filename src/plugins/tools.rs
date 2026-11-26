//! Tools described as data: where each keeps its configuration, and which keys of it name
//! packages. A configuration in JavaScript is also a file the tool loads, so what it imports is
//! used too (the `globals` an `eslint.config.js` imports).
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
}

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
};

impl Tool {
    fn configured(&self, context: &Context) -> bool {
        !context.manifest[self.manifest_key].is_null()
            || self
                .configs
                .iter()
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
                        for item in names(value) {
                            contribution.packages.extend(expand(item, key, context));
                        }
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
    let javascript = [".js", ".cjs", ".mjs", ".ts", ".cts", ".mts"];
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

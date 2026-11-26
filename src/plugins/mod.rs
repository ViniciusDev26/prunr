//! Plugins: what a tool or framework (Jest, Vitest, Vite, React, Next.js, NestJS...) adds to
//! the graph by convention or through its configuration, which no `import` shows.
//!
//! Each plugin is compiled in and enables itself when `package.json` declares its package. A
//! plugin only reads files: it never runs the project's code. What all enabled plugins
//! contribute is added up before the checks run.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::Value;

mod astro;
mod chartjs_node_canvas;
mod compile;
mod drizzle;
mod jest;
mod mdx;
mod mocha;
mod nest;
mod next;
mod nuxt;
mod playwright;
mod postcss;
mod react;
mod react_router;
mod runners;
mod static_config;
mod svelte;
mod sveltekit;
mod tools;
mod typescript;
mod vite;
mod vitepress;
mod vitest;
mod vue;

pub use compile::{Compiled, Compiler};
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
    /// The `scripts` of `package.json`, and each script another one runs with extra arguments,
    /// with them (`npm run unit -- --config x`: the `unit` script, then `--config x`).
    fn scripts(&self) -> Vec<String> {
        let Value::Object(scripts) = &self.manifest["scripts"] else {
            return Vec::new();
        };
        let mut found: Vec<String> = scripts
            .values()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect();
        let forwarded: Vec<String> = found
            .iter()
            .flat_map(|script| crate::scripts::forwarded(script))
            .filter_map(|(name, arguments)| {
                let script = scripts.get(&name)?.as_str()?;
                let quoted: Vec<String> = arguments
                    .iter()
                    .map(|argument| match argument.contains(char::is_whitespace) {
                        true => format!("\"{argument}\""),
                        false => argument.clone(),
                    })
                    .collect();
                Some(format!("{script} {}", quoted.join(" ")))
            })
            .collect();
        found.extend(forwarded);
        found
    }

    /// The words a script runs `runner` with: its name, and in the tool's own repository the
    /// file its `bin` names (`node bin/mocha.js`).
    fn runner_words(&self, runner: &str) -> Vec<String> {
        let mut words = vec![runner.to_string()];
        if self.manifest["name"].as_str() == Some(runner) {
            let bin = match &self.manifest["bin"] {
                Value::String(path) => Some(path.as_str()),
                bins => bins[runner].as_str(),
            };
            words.extend(bin.map(|path| path.trim_start_matches("./").to_string()));
        }
        words
    }

    /// The arguments each command of the scripts passes to `runner`.
    fn arguments(&self, runner: &str) -> Vec<Vec<String>> {
        let words = self.runner_words(runner);
        self.scripts()
            .iter()
            .flat_map(|script| {
                words
                    .iter()
                    .flat_map(|word| crate::scripts::arguments(script, word))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// The configuration files the scripts pass to `runner` with `--config`.
    fn configs_passed_to(&self, runner: &str) -> Vec<String> {
        let words = self.runner_words(runner);
        self.scripts()
            .iter()
            .flat_map(|script| {
                words
                    .iter()
                    .flat_map(|word| crate::scripts::configs(script, word))
                    .collect::<Vec<_>>()
            })
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
    /// Names a tool resolves itself (Jest's `moduleNameMapper` and `__mocks__`, Vite's
    /// `resolve.alias`), tried before a specifier is resolved as Node.js would.
    pub aliases: Vec<Alias>,
    /// By extension (`vue`), what turns a file the parser cannot read into the module it holds.
    /// Those files are part of the project; their imports are never reported, since the markup
    /// around the scripts can use them.
    pub compilers: BTreeMap<String, Compiler>,
    /// Libraries that require packages named in the arguments of a call: `(library, key)` says
    /// that the strings under `key`, in a call of what the library exports, are packages it
    /// loads (`new ChartJSNodeCanvas({ plugins: { modern: ["chartjs-plugin-x"] } })`).
    pub loaded_by_name: Vec<(String, String)>,
}

/// A name a tool maps to a file (`./src/lib/x`, relative to the package) or to a package.
#[derive(Debug, Clone)]
pub struct Alias {
    pattern: Pattern,
    target: String,
}

#[derive(Debug, Clone)]
enum Pattern {
    /// The name itself, or the name followed by a subpath (`@` matches `@/utils/a`).
    Prefix(String),
    /// A regular expression whose groups the target uses as `$1`.
    Regex(regex::Regex),
}

impl PartialEq for Alias {
    fn eq(&self, other: &Self) -> bool {
        let key = |alias: &Alias| match &alias.pattern {
            Pattern::Prefix(prefix) => format!("prefix:{prefix}"),
            Pattern::Regex(regex) => format!("regex:{}", regex.as_str()),
        };
        key(self) == key(other) && self.target == other.target
    }
}

impl Eq for Alias {}

impl Alias {
    /// What `specifier` becomes, if the alias matches it.
    pub fn apply(&self, specifier: &str) -> Option<String> {
        match &self.pattern {
            Pattern::Prefix(prefix) => {
                let rest = specifier.strip_prefix(prefix.as_str())?;
                (rest.is_empty() || rest.starts_with('/'))
                    .then(|| format!("{}{rest}", self.target.trim_end_matches('/')))
            }
            // The whole name becomes the target, with the groups it matched.
            Pattern::Regex(regex) => regex.captures(specifier).map(|groups| {
                let mut target = String::new();
                groups.expand(&self.target, &mut target);
                target
            }),
        }
    }
}

/// A tool or framework prunr knows.
pub trait Plugin {
    /// The plugin's name, which is also the package that enables it.
    fn name(&self) -> &'static str;

    /// Whether the project uses the tool: by default, when `package.json` declares the package.
    fn enabled(&self, context: &Context) -> bool {
        // Also in the tool's own repository, which does not declare itself.
        context.declared.contains(self.name())
            || context.manifest["name"].as_str() == Some(self.name())
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
    &typescript::TypeScript,
    &next::Next,
    &nest::Nest,
    &vue::Vue,
    &svelte::Svelte,
    &astro::Astro,
    &mdx::Mdx,
    &sveltekit::SvelteKit,
    &nuxt::Nuxt,
    &react_router::ReactRouter,
    &vitepress::VitePress,
    &chartjs_node_canvas::ChartJsNodeCanvas,
    &tools::ESLINT,
    &tools::PRETTIER,
    &tools::BABEL,
    &tools::COMMITLINT,
    &tools::TAILWIND,
    &tools::STORYBOOK,
    &tools::CYPRESS,
    &tools::HUSKY,
    &tools::LINT_STAGED,
    &tools::PRISMA,
    &tools::TSUP,
    &tools::ROLLUP,
    &tools::WEBPACK,
];

/// What the plugins enabled in the project contribute. `toggles` turns a plugin off or on by
/// name, whatever the project declares.
pub fn contribution(context: &Context, toggles: &BTreeMap<String, bool>) -> Contribution {
    let mut contribution = Contribution::default();
    for plugin in PLUGINS {
        let enabled = toggles.get(plugin.name()).copied();
        if enabled.unwrap_or_else(|| plugin.enabled(context)) {
            plugin.contribute(context, &mut contribution);
        }
    }
    contribution
}

/// The folders that hold only test code when a runner reads specs from them.
const TEST_FOLDERS: &[&str] = &["test", "tests", "__tests__", "spec", "e2e", "test-d"];

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

/// A path a tool's configuration gives in a build folder (`./build/compiled/test/**`), in the
/// source it is compiled from: without the build folder and the folders after it that do not
/// exist (`test/**`). Any other path, as it is, without its leading `./`.
pub(super) fn from_source(root: &Path, path: &str) -> String {
    let path = path.trim_start_matches("./");
    let mut segments: Vec<&str> = path.split('/').collect();
    if segments.len() < 2 || !crate::project::BUILD_FOLDERS.contains(&segments[0]) {
        return path.to_string();
    }
    segments.remove(0);
    while segments.len() > 1 && !root.join(segments[0]).is_dir() {
        segments.remove(0);
    }
    segments.join("/")
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
        // A test folder the runner reads specs from holds test code only: helpers, fixtures.
        let folders: Vec<String> = named
            .iter()
            .filter_map(|path| path.split('/').next())
            .filter(|folder| TEST_FOLDERS.contains(folder))
            .map(|folder| format!("{folder}/**"))
            .collect();
        if named.is_empty() {
            self.tests
                .extend(defaults.iter().map(|glob| glob.to_string()));
        } else {
            self.tests.extend(named);
        }
        for folder in folders {
            if !self.tests.contains(&folder) {
                self.tests.push(folder);
            }
        }
    }

    /// An alias for a name and the names under it.
    fn alias(&mut self, name: &str, target: &str) {
        self.aliases.push(Alias {
            pattern: Pattern::Prefix(name.to_string()),
            target: target.to_string(),
        });
    }

    /// An alias for the names a regular expression matches; one that does not compile is none.
    fn alias_regex(&mut self, pattern: &str, target: &str) {
        // `$1a` would be a group named `1a` for the regex crate; `${1}a` is not.
        let groups = regex::Regex::new(r"\$(\d+)").expect("the pattern compiles");
        let target = groups.replace_all(target, "$${$1}").into_owned();
        if let Ok(regex) = regex::Regex::new(pattern) {
            self.aliases.push(Alias {
                pattern: Pattern::Regex(regex),
                target,
            });
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
        contribution(
            &Context {
                root,
                declared: packages,
                manifest,
            },
            &BTreeMap::new(),
        )
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
                "typescript",
                "next",
                "@nestjs/core",
                "vue",
                "svelte",
                "astro",
                "@mdx-js/mdx",
                "@sveltejs/kit",
                "nuxt",
                "@react-router/dev",
                "vitepress",
                "chartjs-node-canvas",
                "eslint",
                "prettier",
                "@babel/core",
                "@commitlint/cli",
                "tailwindcss",
                "storybook",
                "cypress",
                "husky",
                "lint-staged",
                "prisma",
                "tsup",
                "rollup",
                "webpack"
            ]
        );
    }

    #[test]
    fn a_tool_repository_enables_its_own_plugin() {
        let dir = project(&[]);
        let found = setup(dir.path(), &declared(&[]), &json!({ "name": "mocha" }));
        assert_eq!(found.tests, strings(&["test/*.*"]));
    }

    #[test]
    fn compilers_are_equal_when_they_are_the_same_function() {
        let a = Compiler(|source: &str| Compiled {
            source: source.to_string(),
            typescript: false,
        });
        assert_eq!(a, a);
        let b = Compiler(|_: &str| Compiled {
            source: String::new(),
            typescript: true,
        });
        assert_ne!(a, b);
        assert_eq!((b.0)("x").source, "");
        assert_eq!((a.0)("y").source, "y");
    }

    #[test]
    fn aliases_are_equal_when_their_pattern_and_target_are() {
        let mut a = Contribution::default();
        a.alias("@", "./src");
        a.alias_regex("^x$", "./x");
        let b = Contribution {
            aliases: a.aliases.clone(),
            ..Contribution::default()
        };
        assert_eq!(a, b);
        let mut c = Contribution::default();
        c.alias("@", "./lib");
        c.alias_regex("^y$", "./x");
        assert_ne!(a, c);
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

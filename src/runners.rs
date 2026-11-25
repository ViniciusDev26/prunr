//! Jest and Vitest: the test files they run and the files and packages their configuration
//! loads, read from the configuration without running it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    ArrayExpressionElement, AssignmentTarget, BindingPattern, Expression, ObjectPropertyKind,
    Statement,
};
use oxc_parser::Parser;
use oxc_span::SourceType;
use serde_json::Value;

/// What the test runners of a project run and load.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Setup {
    /// Globs, relative to the root, of the test files.
    pub tests: Vec<String>,
    /// Files the configuration loads, as specifiers relative to the root (`./jest.config.js`).
    pub files: Vec<String>,
    /// Packages the configuration names: presets, environments, transforms, reporters.
    pub packages: BTreeSet<String>,
}

/// The setup of Jest and Vitest at `root`, for those `package.json` declares (`declared`).
/// `manifest` is the whole `package.json`, whose `jest` key can hold Jest's configuration.
pub fn setup(root: &Path, declared: &BTreeSet<String>, manifest: &Value) -> Setup {
    let scripts: Vec<&str> = match &manifest["scripts"] {
        Value::Object(scripts) => scripts.values().filter_map(Value::as_str).collect(),
        _ => Vec::new(),
    };
    let passed = |runner: &str| -> Vec<String> {
        scripts
            .iter()
            .flat_map(|script| crate::scripts::configs(script, runner))
            .collect()
    };
    let mut setup = Setup::default();
    if declared.contains("jest") {
        let config = match config_file(root, &["jest.config"], &mut setup) {
            Some(config) => config,
            None => manifest["jest"].clone(),
        };
        setup.jest(&config);
        for name in passed("jest") {
            if let Some(config) = setup.read(root, &name) {
                setup.jest(&config);
            }
        }
    }
    if declared.contains("vitest") {
        let config = config_file(root, &["vitest.config", "vite.config"], &mut setup);
        setup.vitest(&config.unwrap_or_default()["test"]);
        for name in passed("vitest") {
            if let Some(config) = setup.read(root, &name) {
                setup.vitest(&config["test"]);
            }
        }
    }
    setup
}

/// The extensions a runner's configuration file can have, in the order runners look for them.
const CONFIG_EXTENSIONS: &[&str] = &["js", "ts", "mjs", "cjs", "mts", "cts", "json"];

/// Jest's default `testMatch`, without the extensions (only source files are in the graph).
const JEST_TESTS: &[&str] = &[
    "**/__tests__/**",
    "**/*.test.*",
    "**/*.spec.*",
    "**/test.*",
    "**/spec.*",
];

/// Jest's manual mocks, next to the modules they mock or at the root.
const MOCKS: &str = "**/__mocks__/**";

/// Vitest's default `include`.
const VITEST_TESTS: &[&str] = &["**/*.{test,spec}.*"];

/// The first configuration file named `<stem>.<extension>` at `root`, recorded in `setup` as a
/// file the runner loads, and what it exports. `Null` when it exists but cannot be read.
fn config_file(root: &Path, stems: &[&str], setup: &mut Setup) -> Option<Value> {
    stems.iter().find_map(|stem| {
        CONFIG_EXTENSIONS
            .iter()
            .find_map(|extension| setup.read(root, &format!("{stem}.{extension}")))
    })
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

impl Setup {
    /// The configuration in the file `name` (relative to `root`), recorded as a file the runner
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

    fn jest(&mut self, config: &Value) {
        // Manual mocks are test code wherever they are, whatever `testMatch` says.
        if !self.tests.iter().any(|test| test == MOCKS) {
            self.tests.push(MOCKS.into());
        }
        self.tests(&config["testMatch"], JEST_TESTS);
        for option in [
            "setupFiles",
            "setupFilesAfterEnv",
            "globalSetup",
            "globalTeardown",
            "preset",
            "snapshotSerializers",
            "watchPlugins",
        ] {
            for name in names(&config[option]) {
                self.reference(name);
            }
        }
        if let Value::Object(transforms) = &config["transform"] {
            for name in transforms.values().flat_map(names) {
                self.reference(name);
            }
        }
        for name in names(&config["reporters"]) {
            if !["default", "summary", "github-actions"].contains(&name) {
                self.reference(name);
            }
        }
        match config["testEnvironment"].as_str() {
            None | Some("node") => {}
            Some("jsdom") => self.reference("jest-environment-jsdom"),
            Some(path) if path.starts_with(['.', '/', '<']) => self.reference(path),
            Some(name) if name.starts_with("jest-environment-") => self.reference(name),
            Some(name) => {
                self.reference(name);
                self.reference(&format!("jest-environment-{name}"));
            }
        }
    }

    fn vitest(&mut self, test: &Value) {
        self.tests(&test["include"], VITEST_TESTS);
        for option in ["setupFiles", "globalSetup"] {
            for name in names(&test[option]) {
                self.reference(name);
            }
        }
        for name in names(&test["reporters"]) {
            if !VITEST_REPORTERS.contains(&name) {
                self.reference(name);
            }
        }
        match test["environment"].as_str() {
            None | Some("node") => {}
            Some("edge-runtime") => self.reference("@edge-runtime/vm"),
            Some(name) => self.reference(name),
        }
        match test["coverage"]["provider"].as_str() {
            Some(provider @ ("v8" | "istanbul")) => {
                self.reference(&format!("@vitest/coverage-{provider}"))
            }
            Some(custom) => self.reference(custom),
            None => {}
        }
    }
}

/// The reporters Vitest ships with.
const VITEST_REPORTERS: &[&str] = &[
    "default",
    "basic",
    "verbose",
    "dot",
    "json",
    "junit",
    "tap",
    "tap-flat",
    "html",
    "hanging-process",
    "github-actions",
    "blob",
    "tree",
];

/// The object a configuration file exports, as JSON: literals are kept, anything only known at
/// runtime is `null`. `None` when the file does not export an object.
pub fn static_config(source: &str, path: &std::path::Path) -> Option<Value> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(path).unwrap_or(SourceType::cjs());
    let parsed = Parser::new(&allocator, source, source_type).parse();
    if !parsed.diagnostics.is_empty() {
        return None;
    }
    let mut constants = BTreeMap::new();
    let mut exported = None;
    for statement in &parsed.program.body {
        match statement {
            Statement::VariableDeclaration(declaration) => {
                for declarator in &declaration.declarations {
                    if let (BindingPattern::BindingIdentifier(id), Some(init)) =
                        (&declarator.id, &declarator.init)
                    {
                        constants.insert(id.name.as_str(), init);
                    }
                }
            }
            Statement::ExportDefaultDeclaration(export) => {
                exported = export.declaration.as_expression();
            }
            Statement::ExpressionStatement(statement) => {
                if let Expression::AssignmentExpression(assignment) = &statement.expression
                    && let AssignmentTarget::StaticMemberExpression(target) = &assignment.left
                    && target.property.name == "exports"
                    && matches!(&target.object, Expression::Identifier(id) if id.name == "module")
                {
                    exported = Some(&assignment.right);
                }
            }
            _ => {}
        }
    }
    match value(exported?, &constants) {
        object @ Value::Object(_) => Some(object),
        _ => None,
    }
}

/// The JSON value of a literal expression; `null` for anything else.
fn value(expression: &Expression, constants: &BTreeMap<&str, &Expression>) -> Value {
    match expression.get_inner_expression() {
        Expression::StringLiteral(literal) => Value::String(literal.value.display().to_string()),
        Expression::TemplateLiteral(template) => {
            template.single_quasi().map_or(Value::Null, |quasi| {
                Value::String(quasi.display().to_string())
            })
        }
        Expression::NumericLiteral(number) => serde_json::json!(number.value),
        Expression::BooleanLiteral(boolean) => Value::Bool(boolean.value),
        Expression::ArrayExpression(array) => Value::Array(
            array
                .elements
                .iter()
                .map(|element| match element {
                    ArrayExpressionElement::SpreadElement(_)
                    | ArrayExpressionElement::Elision(_) => Value::Null,
                    element => value(element.to_expression(), constants),
                })
                .collect(),
        ),
        Expression::ObjectExpression(object) => Value::Object(
            object
                .properties
                .iter()
                .filter_map(|property| match property {
                    ObjectPropertyKind::ObjectProperty(property) => {
                        let key = property.key.static_name()?.display().to_string();
                        Some((key, value(&property.value, constants)))
                    }
                    ObjectPropertyKind::SpreadProperty(_) => None,
                })
                .collect(),
        ),
        Expression::CallExpression(call) => match call.arguments.first() {
            Some(argument) if call.arguments.len() == 1 => match argument.as_expression() {
                Some(argument) => value(argument, constants),
                None => Value::Null,
            },
            _ => Value::Null,
        },
        Expression::Identifier(id) => constants
            .get(id.name.as_str())
            .map_or(Value::Null, |init| value(init, constants)),
        _ => Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (path, content) in files {
            std::fs::write(dir.path().join(path), content).unwrap();
        }
        dir
    }

    fn declared(packages: &[&str]) -> BTreeSet<String> {
        packages.iter().map(|p| p.to_string()).collect()
    }

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| item.to_string()).collect()
    }

    #[test]
    fn without_a_runner_there_is_nothing() {
        let dir = project(&[("jest.config.js", "module.exports = {};")]);
        assert_eq!(
            setup(dir.path(), &declared(&["mocha"]), &json!({})),
            Setup::default()
        );
    }

    #[test]
    fn jest_reads_its_config_file() {
        let dir = project(&[(
            "jest.config.js",
            r#"module.exports = {
                testMatch: ["<rootDir>/test/**/*.test.js", "!**/fixtures/**"],
                preset: "ts-jest/presets/default",
                testEnvironment: "jsdom",
                setupFiles: ["<rootDir>/test/env.js"],
                setupFilesAfterEnv: ["@testing-library/jest-dom", "./test/after.js"],
                globalSetup: "./test/global-setup.js",
                globalTeardown: "/abs/teardown.js",
                transform: { "^.+\.tsx?$": ["ts-jest", {}], "^.+\.svg$": "jest-svg-transformer", "x": 1 },
                reporters: ["default", ["jest-junit", { outputDirectory: "out" }], "github-actions", 2],
                snapshotSerializers: "enzyme-to-json/serializer",
                watchPlugins: ["jest-watch-typeahead/filename"],
            };"#,
        )]);
        assert_eq!(
            setup(dir.path(), &declared(&["jest"]), &json!({})),
            Setup {
                tests: strings(&["**/__mocks__/**", "test/**/*.test.js"]),
                files: strings(&[
                    "./jest.config.js",
                    "./test/env.js",
                    "./test/after.js",
                    "./test/global-setup.js",
                    "/abs/teardown.js",
                ]),
                packages: declared(&[
                    "@testing-library/jest-dom",
                    "enzyme-to-json",
                    "jest-environment-jsdom",
                    "jest-junit",
                    "jest-svg-transformer",
                    "jest-watch-typeahead",
                    "ts-jest",
                ]),
            }
        );
    }

    const JEST_DEFAULTS: &[&str] = &[
        "**/__mocks__/**",
        "**/__tests__/**",
        "**/*.test.*",
        "**/*.spec.*",
        "**/test.*",
        "**/spec.*",
    ];

    #[test]
    fn jest_reads_the_jest_key_of_package_json_and_falls_back_to_its_defaults() {
        let dir = project(&[]);
        let manifest = json!({ "jest": { "testEnvironment": "node" } });
        assert_eq!(
            setup(dir.path(), &declared(&["jest"]), &manifest),
            Setup {
                tests: strings(JEST_DEFAULTS),
                ..Setup::default()
            }
        );
    }

    #[test]
    fn a_custom_jest_environment_is_its_package() {
        let dir = project(&[(
            "jest.config.json",
            r#"{ "testEnvironment": "puppeteer", "testMatch": [] }"#,
        )]);
        let setup = setup(dir.path(), &declared(&["jest"]), &json!({}));
        assert_eq!(setup.files, strings(&["./jest.config.json"]));
        assert_eq!(setup.tests, strings(JEST_DEFAULTS));
        assert_eq!(
            setup.packages,
            declared(&["jest-environment-puppeteer", "puppeteer"])
        );
        let dir = project(&[(
            "jest.config.js",
            r#"module.exports = { testEnvironment: "jest-environment-x" };"#,
        )]);
        let setup = super::setup(dir.path(), &declared(&["jest"]), &json!({}));
        assert_eq!(setup.packages, declared(&["jest-environment-x"]));
        let dir = project(&[(
            "jest.config.js",
            r#"module.exports = { testEnvironment: "./test/env.js" };"#,
        )]);
        let setup = super::setup(dir.path(), &declared(&["jest"]), &json!({}));
        assert_eq!(setup.files, strings(&["./jest.config.js", "./test/env.js"]));
    }

    #[test]
    fn a_jest_config_built_at_runtime_falls_back_to_the_defaults() {
        let dir = project(&[
            ("jest.config.ts", "export default async () => ({});"),
            ("jest.config.json", "{ nope"),
        ]);
        let setup = setup(dir.path(), &declared(&["jest"]), &json!({}));
        assert_eq!(setup.files, strings(&["./jest.config.ts"]));
        assert_eq!(setup.tests, strings(JEST_DEFAULTS));
        let dir = project(&[("jest.config.json", "{ nope")]);
        let setup = super::setup(dir.path(), &declared(&["jest"]), &json!({}));
        assert_eq!(setup.tests, strings(JEST_DEFAULTS));
    }

    #[test]
    fn vitest_reads_the_test_key_of_its_config() {
        let dir = project(&[(
            "vitest.config.ts",
            r#"import { defineConfig } from "vitest/config";
               export default defineConfig({
                 test: {
                   include: ["src/**/*.test.ts"],
                   setupFiles: "./test/setup.ts",
                   globalSetup: ["./test/global.ts"],
                   environment: "happy-dom",
                   coverage: { provider: "v8" },
                   reporters: ["verbose", "vitest-sonar-reporter"],
                 },
               });"#,
        )]);
        assert_eq!(
            setup(dir.path(), &declared(&["vitest"]), &json!({})),
            Setup {
                tests: strings(&["src/**/*.test.ts"]),
                files: strings(&["./vitest.config.ts", "./test/setup.ts", "./test/global.ts"]),
                packages: declared(&["@vitest/coverage-v8", "happy-dom", "vitest-sonar-reporter"]),
            }
        );
    }

    #[test]
    fn vitest_falls_back_to_the_vite_config_and_its_defaults() {
        let dir = project(&[(
            "vite.config.mjs",
            r#"export default { test: { environment: "edge-runtime", coverage: { provider: "./cov.js" } } };"#,
        )]);
        assert_eq!(
            setup(dir.path(), &declared(&["vitest"]), &json!({})),
            Setup {
                tests: strings(&["**/*.{test,spec}.*"]),
                files: strings(&["./vite.config.mjs", "./cov.js"]),
                packages: declared(&["@edge-runtime/vm"]),
            }
        );
        let dir = project(&[]);
        let setup = setup(dir.path(), &declared(&["vitest"]), &json!({}));
        assert_eq!(setup.tests, strings(&["**/*.{test,spec}.*"]));
        assert!(setup.files.is_empty());
        let dir = project(&[(
            "vitest.config.js",
            r#"export default { test: { environment: "node", coverage: { provider: "istanbul" } } };"#,
        )]);
        let setup = super::setup(dir.path(), &declared(&["vitest"]), &json!({}));
        assert_eq!(setup.packages, declared(&["@vitest/coverage-istanbul"]));
        let dir = project(&[(
            "vitest.config.js",
            r#"export default { test: { environment: "jsdom" } };"#,
        )]);
        let setup = super::setup(dir.path(), &declared(&["vitest"]), &json!({}));
        assert_eq!(setup.packages, declared(&["jsdom"]));
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
        let setup = setup(dir.path(), &declared(&["jest", "vitest"]), &manifest);
        assert_eq!(
            setup.files,
            strings(&[
                "./jest.config.js",
                "./jest.e2e.config.js",
                "./vitest.bench.json"
            ])
        );
        assert!(setup.tests.contains(&"e2e/**/*.js".to_string()));
        assert!(setup.tests.contains(&"bench/**".to_string()));
    }

    #[test]
    fn jest_and_vitest_add_up() {
        let dir = project(&[]);
        let setup = setup(dir.path(), &declared(&["jest", "vitest"]), &json!({}));
        assert_eq!(setup.tests.len(), JEST_DEFAULTS.len() + 1);
    }

    fn config(path: &str, source: &str) -> Option<Value> {
        static_config(source, std::path::Path::new(path))
    }

    #[test]
    fn module_exports_of_an_object_is_the_config() {
        assert_eq!(
            config(
                "jest.config.js",
                r#"module.exports = {
                    a: "x", b: ['y', `z`], c: { d: true, e: false }, f: 1.5, g: null,
                    h: someVariable, [computed]: 1, ...spread, i: [...rest, "j"]
                };"#
            ),
            Some(json!({
                "a": "x", "b": ["y", "z"], "c": { "d": true, "e": false }, "f": 1.5,
                "g": null, "h": null, "i": [null, "j"]
            }))
        );
    }

    #[test]
    fn export_default_through_define_config_and_type_assertions_is_the_config() {
        assert_eq!(
            config(
                "vitest.config.ts",
                r#"import { defineConfig } from "vitest/config";
                   export default defineConfig({ test: { include: ["a"] } }) satisfies object;"#
            ),
            Some(json!({ "test": { "include": ["a"] } }))
        );
    }

    #[test]
    fn a_config_held_in_a_constant_is_followed() {
        assert_eq!(
            config(
                "jest.config.ts",
                r#"const config = { preset: "ts-jest" } as const;
                   let other = 1, unused;
                   export default config;"#
            ),
            Some(json!({ "preset": "ts-jest" }))
        );
    }

    #[test]
    fn a_config_built_at_runtime_is_unknown() {
        for source in [
            "module.exports = async () => ({});",
            "export default defineConfig(() => ({}));",
            "export default missing;",
            "exports.preset = 'ts-jest';",
            "module.exports = { a: 1 };;; 1 + 1;",
            "export default merge(...configs);",
            "export default merge(a, b);",
            "export default merge();",
        ] {
            let expected = if source.contains("a: 1") {
                Some(json!({ "a": 1.0 }))
            } else {
                None
            };
            assert_eq!(config("jest.config.js", source), expected, "{source}");
        }
    }

    #[test]
    fn a_config_that_does_not_parse_is_unknown() {
        assert_eq!(config("jest.config.js", "module.exports = {"), None);
    }
}

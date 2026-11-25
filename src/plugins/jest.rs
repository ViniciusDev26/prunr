//! Jest: its test files, the files its configuration loads (`jest.config.*`, the `jest` key of
//! `package.json`, or one a script passes with `--config`), and the packages it names.

use serde_json::Value;

use super::{Context, Contribution, Plugin, names};

pub struct Jest;

/// Jest's default `testMatch`, without the extensions (only source files are in the graph).
const TESTS: &[&str] = &[
    "**/__tests__/**",
    "**/*.test.*",
    "**/*.spec.*",
    "**/test.*",
    "**/spec.*",
];

/// Jest's manual mocks, next to the modules they mock or at the root.
const MOCKS: &str = "**/__mocks__/**";

impl Plugin for Jest {
    fn name(&self) -> &'static str {
        "jest"
    }

    fn contribute(&self, context: &Context, contribution: &mut Contribution) {
        let config = match contribution.config_file(context.root, &["jest.config"]) {
            Some(config) => config,
            None => context.manifest["jest"].clone(),
        };
        configure(&config, contribution);
        for name in context.configs_passed_to("jest") {
            if let Some(config) = contribution.read(context.root, &name) {
                configure(&config, contribution);
            }
        }
    }
}

/// What one Jest configuration adds.
fn configure(config: &Value, contribution: &mut Contribution) {
    // Manual mocks are test code wherever they are, whatever `testMatch` says.
    if !contribution.tests.iter().any(|test| test == MOCKS) {
        contribution.tests.push(MOCKS.into());
    }
    contribution.tests(&config["testMatch"], TESTS);
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
            contribution.reference(name);
        }
    }
    if let Value::Object(transforms) = &config["transform"] {
        for name in transforms.values().flat_map(names) {
            contribution.reference(name);
        }
    }
    for name in names(&config["reporters"]) {
        if !["default", "summary", "github-actions"].contains(&name) {
            contribution.reference(name);
        }
    }
    match config["testEnvironment"].as_str() {
        None | Some("node") => {}
        Some("jsdom") => contribution.reference("jest-environment-jsdom"),
        Some(path) if path.starts_with(['.', '/', '<']) => contribution.reference(path),
        Some(name) if name.starts_with("jest-environment-") => contribution.reference(name),
        Some(name) => {
            contribution.reference(name);
            contribution.reference(&format!("jest-environment-{name}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use super::*;
    use serde_json::json;

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
            Contribution {
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
                ..Contribution::default()
            }
        );
    }

    #[test]
    fn jest_reads_the_jest_key_of_package_json_and_falls_back_to_its_defaults() {
        let dir = project(&[]);
        let manifest = json!({ "jest": { "testEnvironment": "node" } });
        assert_eq!(
            setup(dir.path(), &declared(&["jest"]), &manifest),
            Contribution {
                tests: strings(JEST_DEFAULTS),
                ..Contribution::default()
            }
        );
    }

    #[test]
    fn a_custom_jest_environment_is_its_package() {
        let dir = project(&[(
            "jest.config.json",
            r#"{ "testEnvironment": "puppeteer", "testMatch": [] }"#,
        )]);
        let found = setup(dir.path(), &declared(&["jest"]), &json!({}));
        assert_eq!(found.files, strings(&["./jest.config.json"]));
        assert_eq!(found.tests, strings(JEST_DEFAULTS));
        assert_eq!(
            found.packages,
            declared(&["jest-environment-puppeteer", "puppeteer"])
        );
        let dir = project(&[(
            "jest.config.js",
            r#"module.exports = { testEnvironment: "jest-environment-x" };"#,
        )]);
        let found = setup(dir.path(), &declared(&["jest"]), &json!({}));
        assert_eq!(found.packages, declared(&["jest-environment-x"]));
        let dir = project(&[(
            "jest.config.js",
            r#"module.exports = { testEnvironment: "./test/env.js" };"#,
        )]);
        let found = setup(dir.path(), &declared(&["jest"]), &json!({}));
        assert_eq!(found.files, strings(&["./jest.config.js", "./test/env.js"]));
    }

    #[test]
    fn a_jest_config_built_at_runtime_falls_back_to_the_defaults() {
        let dir = project(&[
            ("jest.config.ts", "export default async () => ({});"),
            ("jest.config.json", "{ nope"),
        ]);
        let found = setup(dir.path(), &declared(&["jest"]), &json!({}));
        assert_eq!(found.files, strings(&["./jest.config.ts"]));
        assert_eq!(found.tests, strings(JEST_DEFAULTS));
        let dir = project(&[("jest.config.json", "{ nope")]);
        let found = setup(dir.path(), &declared(&["jest"]), &json!({}));
        assert_eq!(found.tests, strings(JEST_DEFAULTS));
    }
}

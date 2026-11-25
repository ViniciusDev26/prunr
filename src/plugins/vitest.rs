//! Vitest: its test files and what the `test` key of its configuration (`vitest.config.*`,
//! `vite.config.*`, or one a script passes with `--config`) loads.

use serde_json::Value;

use super::{Context, Contribution, Plugin, names};

pub struct Vitest;

/// Vitest's default `include`.
const TESTS: &[&str] = &["**/*.{test,spec}.*"];

/// The reporters Vitest ships with.
const REPORTERS: &[&str] = &[
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

impl Plugin for Vitest {
    fn name(&self) -> &'static str {
        "vitest"
    }

    fn contribute(&self, context: &Context, contribution: &mut Contribution) {
        let config = contribution.config_file(context.root, &["vitest.config", "vite.config"]);
        configure(&config.unwrap_or_default()["test"], contribution);
        for name in context.configs_passed_to("vitest") {
            if let Some(config) = contribution.read(context.root, &name) {
                configure(&config["test"], contribution);
            }
        }
    }
}

/// What the `test` key of one Vitest configuration adds.
fn configure(test: &Value, contribution: &mut Contribution) {
    contribution.tests(&test["include"], TESTS);
    for option in ["setupFiles", "globalSetup"] {
        for name in names(&test[option]) {
            contribution.reference(name);
        }
    }
    for name in names(&test["reporters"]) {
        if !REPORTERS.contains(&name) {
            contribution.reference(name);
        }
    }
    match test["environment"].as_str() {
        None | Some("node") => {}
        Some("edge-runtime") => contribution.reference("@edge-runtime/vm"),
        Some(name) => contribution.reference(name),
    }
    match test["coverage"]["provider"].as_str() {
        Some(provider @ ("v8" | "istanbul")) => {
            contribution.reference(&format!("@vitest/coverage-{provider}"))
        }
        Some(custom) => contribution.reference(custom),
        None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use super::*;
    use serde_json::json;

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
            Contribution {
                tests: strings(&["src/**/*.test.ts"]),
                files: strings(&["./vitest.config.ts", "./test/setup.ts", "./test/global.ts"]),
                packages: declared(&["@vitest/coverage-v8", "happy-dom", "vitest-sonar-reporter"]),
                ..Contribution::default()
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
            Contribution {
                tests: strings(&["**/*.{test,spec}.*"]),
                files: strings(&["./vite.config.mjs", "./cov.js"]),
                packages: declared(&["@edge-runtime/vm"]),
                ..Contribution::default()
            }
        );
        let dir = project(&[]);
        let found = setup(dir.path(), &declared(&["vitest"]), &json!({}));
        assert_eq!(found.tests, strings(&["**/*.{test,spec}.*"]));
        assert!(found.files.is_empty());
        let dir = project(&[(
            "vitest.config.js",
            r#"export default { test: { environment: "node", coverage: { provider: "istanbul" } } };"#,
        )]);
        let found = setup(dir.path(), &declared(&["vitest"]), &json!({}));
        assert_eq!(found.packages, declared(&["@vitest/coverage-istanbul"]));
        let dir = project(&[(
            "vitest.config.js",
            r#"export default { test: { environment: "jsdom" } };"#,
        )]);
        let found = setup(dir.path(), &declared(&["vitest"]), &json!({}));
        assert_eq!(found.packages, declared(&["jsdom"]));
    }
}

//! Playwright: its test files (`testDir` and `testMatch`, also per project), the files and
//! reporters its configuration loads, and the web servers it starts before the tests.

use std::path::Path;

use serde_json::Value;

use super::{Context, Contribution, Plugin, names};

pub struct Playwright;

/// The reporters Playwright ships with.
const REPORTERS: &[&str] = &[
    "list", "line", "dot", "html", "json", "junit", "null", "github", "blob",
];

/// Playwright's default `testMatch`, without the extensions.
const TEST_MATCH: &str = "**/*.{spec,test}.*";

impl Plugin for Playwright {
    fn name(&self) -> &'static str {
        "@playwright/test"
    }

    fn contribute(&self, context: &Context, contribution: &mut Contribution) {
        let mut configs: Vec<(String, Value)> = Vec::new();
        for extension in super::CONFIG_EXTENSIONS {
            let name = format!("playwright.config.{extension}");
            if let Some(config) = contribution.read(context.root, &name) {
                configs.push((name, config));
                break;
            }
        }
        for name in context.configs_passed_to("playwright") {
            if let Some(config) = contribution.read(context.root, &name) {
                configs.push((name, config));
            }
        }
        for (name, config) in configs {
            let base = Path::new(name.trim_start_matches("./"))
                .parent()
                .unwrap_or(Path::new(""));
            configure(&config, base, contribution);
        }
    }
}

/// What one Playwright configuration, at `base`, adds.
fn configure(config: &Value, base: &Path, contribution: &mut Contribution) {
    let projects = match &config["projects"] {
        Value::Array(projects) => projects.iter().collect(),
        _ => Vec::new(),
    };
    for options in std::iter::once(config).chain(projects) {
        let dir = options["testDir"]
            .as_str()
            .or(config["testDir"].as_str())
            .unwrap_or(".");
        let dir = super::normalize(&base.join(dir));
        // Everything in a test folder is test code: helpers, fixtures, mock servers.
        if !dir.is_empty() {
            contribution.tests.push(format!("{dir}/**"));
        }
        let mut patterns = names(&options["testMatch"]);
        if patterns.is_empty() {
            patterns = names(&config["testMatch"]);
        }
        if patterns.is_empty() {
            patterns.push(TEST_MATCH);
        }
        for pattern in patterns {
            let pattern = match pattern.contains('/') {
                true => pattern.to_string(),
                false => format!("**/{pattern}"),
            };
            contribution.tests.push(match dir.as_str() {
                "" => pattern,
                dir => format!("{dir}/{pattern}"),
            });
        }
    }
    for option in ["globalSetup", "globalTeardown"] {
        for name in names(&config[option]) {
            let path = super::normalize(&base.join(name));
            contribution.reference(&format!("./{path}"));
        }
    }
    for name in names(&config["reporter"]) {
        if !REPORTERS.contains(&name) {
            contribution.reference(name);
        }
    }
    let servers = match &config["webServer"] {
        Value::Array(servers) => servers.iter().collect(),
        server => vec![server],
    };
    for server in servers {
        if let Some(command) = server["command"].as_str() {
            contribution.scripts.push(command.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use serde_json::json;

    #[test]
    fn playwright_reads_its_config() {
        let dir = project(&[(
            "playwright.config.ts",
            r#"export default {
                testDir: "./e2e",
                testMatch: "*.e2e.ts",
                globalSetup: ["./setup.ts"],
                globalTeardown: "./teardown.ts",
                reporter: [["html"], ["allure-playwright", {}], "list"],
                webServer: { command: "npm start" },
                projects: [{ name: "a" }, { testDir: "./smoke", testMatch: ["**/*.smoke.ts"] }],
            };"#,
        )]);
        let found = setup(dir.path(), &declared(&["@playwright/test"]), &json!({}));
        assert_eq!(
            found.tests,
            strings(&[
                "e2e/**",
                "e2e/**/*.e2e.ts",
                "e2e/**",
                "e2e/**/*.e2e.ts",
                "smoke/**",
                "smoke/**/*.smoke.ts"
            ])
        );
        assert_eq!(
            found.files,
            strings(&["./playwright.config.ts", "./setup.ts", "./teardown.ts"])
        );
        assert_eq!(found.packages, declared(&["allure-playwright"]));
        assert_eq!(found.scripts, strings(&["npm start"]));
    }

    #[test]
    fn a_config_a_script_passes_is_read_from_its_folder() {
        let dir = project(&[]);
        std::fs::create_dir_all(dir.path().join("e2e")).unwrap();
        std::fs::write(
            dir.path().join("e2e/pw.config.js"),
            r#"module.exports = { webServer: [{ command: "pnpm dev" }, { port: 1 }] };"#,
        )
        .unwrap();
        let manifest = json!({ "scripts": { "e2e": "playwright test --config=e2e/pw.config.js" } });
        let found = setup(dir.path(), &declared(&["@playwright/test"]), &manifest);
        assert_eq!(found.tests, strings(&["e2e/**", "e2e/**/*.{spec,test}.*"]));
        assert_eq!(found.files, strings(&["./e2e/pw.config.js"]));
        assert_eq!(found.scripts, strings(&["pnpm dev"]));
        let dir = project(&[("playwright.config.js", "module.exports = {};")]);
        let manifest = json!({ "scripts": { "e2e": "playwright test -c gone.ts" } });
        let found = setup(dir.path(), &declared(&["@playwright/test"]), &manifest);
        assert_eq!(found.tests, strings(&["**/*.{spec,test}.*"]));
        assert_eq!(found.files, strings(&["./playwright.config.js"]));
    }
}

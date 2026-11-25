//! Vitest: its test files and what the `test` key of its configuration (`vitest.config.*`,
//! `vite.config.*`, or one a script passes with `--config`) loads.

use std::path::Path;

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
        configure(context.root, "", &config.unwrap_or_default(), contribution);
        for name in context.configs_passed_to("vitest") {
            if let Some(config) = contribution.read(context.root, &name) {
                configure(context.root, &folder(&name), &config, contribution);
            }
        }
    }
}

/// The folder of a configuration file, relative to the root: empty for one at the root.
fn folder(name: &str) -> String {
    let path = Path::new(name.trim_start_matches("./"));
    super::normalize(path.parent().unwrap_or(Path::new("")))
}

/// What one Vitest configuration, in the folder `base`, adds: its `test` key, and the
/// configuration of each of its `test.projects` (a file, or a folder with its own config).
fn configure(root: &Path, base: &str, config: &Value, contribution: &mut Contribution) {
    let test = &config["test"];
    let at = |path: &str| {
        let path = path.trim_start_matches("./");
        match base {
            "" => path.to_string(),
            base => format!("{base}/{path}"),
        }
    };
    let first = contribution.tests.len();
    contribution.tests(&test["include"], TESTS);
    for glob in &mut contribution.tests[first..] {
        *glob = at(glob);
    }
    for option in ["setupFiles", "globalSetup"] {
        for name in names(&test[option]) {
            match name.starts_with('.') {
                true => contribution.reference(&format!("./{}", at(name))),
                false => contribution.reference(name),
            }
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
    for project in names(&test["projects"]) {
        let path = at(project);
        let files = match root.join(&path).is_file() {
            true => vec![path],
            false => folders(root, &path)
                .into_iter()
                .filter_map(|dir| {
                    let stems = ["vitest.config", "vite.config"];
                    stems.iter().find_map(|stem| {
                        super::CONFIG_EXTENSIONS
                            .iter()
                            .map(|extension| format!("{dir}/{stem}.{extension}"))
                            .find(|name| root.join(name).is_file())
                    })
                })
                .collect(),
        };
        for file in files {
            let config = contribution.read(root, &file).unwrap_or_default();
            configure(root, &folder(&file), &config, contribution);
        }
    }
}

/// The folders under `root` a glob (or a plain path) matches.
fn folders(root: &Path, pattern: &str) -> Vec<String> {
    let Ok(glob) = globset::GlobBuilder::new(pattern.trim_end_matches('/'))
        .literal_separator(true)
        .build()
    else {
        return Vec::new();
    };
    let glob = glob.compile_matcher();
    ignore::WalkBuilder::new(root)
        .require_git(false)
        .filter_entry(|entry| entry.file_name() != "node_modules")
        .build()
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| {
            let dir = entry.path().strip_prefix(root).ok()?;
            let dir = dir.to_string_lossy().replace('\\', "/");
            glob.is_match(&dir).then_some(dir)
        })
        .collect()
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
    fn each_project_is_read_from_its_folder() {
        let dir = project(&[]);
        for (path, content) in [
            (
                "vitest.config.ts",
                r#"export default { test: { projects: ["web/vitest.web.ts", "packages/*", "["] } };"#,
            ),
            (
                "web/vitest.web.ts",
                r#"export default { test: { include: ["src/**/*.spec.ts"], setupFiles: ["./setup.ts", "@testing-library/jest-dom"] } };"#,
            ),
            ("packages/a/vite.config.js", "export default {};"),
            ("packages/b/index.js", ""),
        ] {
            let path = dir.path().join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, content).unwrap();
        }
        let found = setup(dir.path(), &declared(&["vitest"]), &json!({}));
        assert_eq!(
            found.tests,
            strings(&[
                "**/*.{test,spec}.*",
                "web/src/**/*.spec.ts",
                "packages/a/**/*.{test,spec}.*"
            ])
        );
        assert_eq!(
            found.files,
            strings(&[
                "./vitest.config.ts",
                "./web/vitest.web.ts",
                "./web/setup.ts",
                "./packages/a/vite.config.js"
            ])
        );
        assert_eq!(found.packages, declared(&["@testing-library/jest-dom"]));
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

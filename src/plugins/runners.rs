//! The test runners configured by their command line and conventions: `node --test`, tap, AVA,
//! lab, tsd and borp. The folders, files or globs a script passes are test files; without any, the
//! runner's defaults are.
//!
//! `node --test` needs no package, so this plugin is always enabled; each other runner counts
//! only when `package.json` declares it.

use super::{Context, Contribution, Plugin};

pub struct Runners;

/// A runner: the binary a script calls, the package that provides it, a flag its command needs
/// to run tests, and the test files it finds on its own.
struct Runner {
    binary: &'static str,
    package: Option<&'static str>,
    flag: Option<&'static str>,
    defaults: &'static [&'static str],
}

const RUNNERS: &[Runner] = &[
    Runner {
        binary: "node",
        package: None,
        flag: Some("--test"),
        defaults: &[
            "**/*.test.*",
            "**/*-test.*",
            "**/*_test.*",
            "**/test-*.*",
            "**/test.*",
            "**/test/**",
        ],
    },
    Runner {
        binary: "tap",
        package: Some("tap"),
        flag: None,
        defaults: &["**/{test,tests,__tests__}/**", "**/*.{test,spec}.*"],
    },
    Runner {
        binary: "ava",
        package: Some("ava"),
        flag: None,
        defaults: &[
            "test.*",
            "{src,source}/test.*",
            "**/test-*.*",
            "**/*.{test,spec}.*",
            "**/{test,tests,__tests__}/**",
        ],
    },
    Runner {
        binary: "lab",
        package: Some("@hapi/lab"),
        flag: None,
        defaults: &["test/**"],
    },
    Runner {
        binary: "tsd",
        package: Some("tsd"),
        flag: None,
        defaults: &["test-d/**", "**/*.test-d.*", "index.test-d.*"],
    },
    Runner {
        binary: "borp",
        package: Some("borp"),
        flag: None,
        defaults: &["test/**/*.test.*"],
    },
];

impl Plugin for Runners {
    fn name(&self) -> &'static str {
        "test-runners"
    }

    fn enabled(&self, _context: &Context) -> bool {
        true
    }

    fn contribute(&self, context: &Context, contribution: &mut Contribution) {
        let serde_json::Value::Object(scripts) = &context.manifest["scripts"] else {
            return;
        };
        for script in scripts.values().filter_map(|script| script.as_str()) {
            for runner in RUNNERS {
                if runner
                    .package
                    .is_some_and(|package| !context.declared.contains(package))
                {
                    continue;
                }
                for arguments in crate::scripts::arguments(script, runner.binary) {
                    if runner
                        .flag
                        .is_some_and(|flag| !arguments.iter().any(|a| a == flag))
                    {
                        continue;
                    }
                    contribution.test_arguments(context.root, &arguments, runner.defaults);
                    // lab's assertion library, loaded by name.
                    if runner.binary == "lab"
                        && let Some(at) =
                            arguments.iter().position(|a| a == "-a" || a == "--assert")
                        && let Some(library) = arguments.get(at + 1)
                    {
                        contribution.reference(library);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use serde_json::json;

    #[test]
    fn a_runner_runs_the_paths_a_script_passes_or_its_defaults() {
        let dir = project(&[("a.test.js", "")]);
        std::fs::create_dir_all(dir.path().join("spec")).unwrap();
        let manifest = json!({ "scripts": {
            "unit": "node --import tsx --test spec/ a.test.js 'x/**/*.ts' --test-reporter=spec",
            "plain": "node index.js",
            "tap": "tap",
            "lab": "lab -a @hapi/code -t 100",
            "ava": "ava",
            "odd": 1
        } });
        let found = setup(dir.path(), &declared(&["tap", "@hapi/lab"]), &manifest);
        let mut tests = found.tests.clone();
        tests.sort();
        assert_eq!(
            tests,
            strings(&[
                "**/*.{test,spec}.*",
                "**/{test,tests,__tests__}/**",
                "a.test.js",
                "spec/**",
                "test/**",
                "x/**/*.ts",
            ])
        );
        assert_eq!(found.packages, declared(&["@hapi/code"]));
        let found = setup(dir.path(), &declared(&[]), &json!({}));
        assert!(found.tests.is_empty());
        let manifest = json!({ "scripts": { "test": "xo && tsd" } });
        let found = setup(dir.path(), &declared(&["tsd"]), &manifest);
        assert_eq!(
            found.tests,
            strings(&["test-d/**", "**/*.test-d.*", "index.test-d.*"])
        );
    }
}

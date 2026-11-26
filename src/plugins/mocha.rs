//! Mocha: the specs its configuration (`.mocharc.*`, the `mocha` key of `package.json`, or one
//! a script passes with `--config`) and its command line name, and the files and packages it
//! loads first (`require`, `file`) or reports with.

use std::path::Path;

use serde_json::Value;

use super::{Context, Contribution, Plugin, names};

pub struct Mocha;

/// Mocha's configuration files, in the order it looks for them.
const CONFIGS: &[&str] = &[
    ".mocharc.js",
    ".mocharc.cjs",
    ".mocharc.yaml",
    ".mocharc.yml",
    ".mocharc.jsonc",
    ".mocharc.json",
];

/// The reporters Mocha ships with.
const REPORTERS: &[&str] = &[
    "spec",
    "dot",
    "nyan",
    "tap",
    "landing",
    "list",
    "progress",
    "json",
    "json-stream",
    "min",
    "doc",
    "markdown",
    "xunit",
    "html",
];

/// Mocha's flags that take the next word as their value.
const VALUE_FLAGS: &[&str] = &[
    "--config",
    "-r",
    "--require",
    "--file",
    "-R",
    "--reporter",
    "-O",
    "--reporter-option",
    "-t",
    "--timeout",
    "-u",
    "--ui",
    "-g",
    "--grep",
    "--extension",
    "--ignore",
    "--slow",
    "-s",
];

/// Mocha's default spec, without the extensions.
const SPEC: &[&str] = &["test/*.*"];

impl Plugin for Mocha {
    fn name(&self) -> &'static str {
        "mocha"
    }

    fn contribute(&self, context: &Context, contribution: &mut Contribution) {
        let root = context.root;
        let mut configs = vec![context.manifest["mocha"].clone()];
        let files = CONFIGS
            .iter()
            .find(|name| root.join(name).is_file())
            .map(|name| name.to_string())
            .into_iter()
            .chain(context.configs_passed_to("mocha"));
        for name in files {
            let name = name.trim_start_matches("./").to_string();
            // A configuration in JavaScript is code the tests load.
            if !name.ends_with(".json") && !name.ends_with(".jsonc") && !name.contains(".y") {
                contribution.reference(&format!("./{name}"));
            }
            configs.push(read(root, &name));
        }
        let mut spec: Vec<String> = Vec::new();
        let mut recursive = false;
        for config in &configs {
            spec.extend(
                names(&config["spec"])
                    .into_iter()
                    .map(|path| super::from_source(root, path)),
            );
            recursive |= config["recursive"].as_bool() == Some(true);
            for option in ["require", "file"] {
                for name in names(&config[option]) {
                    load(root, name, contribution);
                }
            }
            for name in names(&config["reporter"]) {
                if !REPORTERS.contains(&name) {
                    load(root, name, contribution);
                }
            }
        }
        let mut ran = false;
        {
            for arguments in context.arguments("mocha") {
                ran = true;
                for (index, argument) in arguments.iter().enumerate() {
                    let value = arguments.get(index + 1);
                    match argument.as_str() {
                        "-r" | "--require" | "--file" => {
                            value.into_iter().for_each(|v| load(root, v, contribution))
                        }
                        "-R" | "--reporter" => value
                            .filter(|v| !REPORTERS.contains(&v.as_str()))
                            .into_iter()
                            .for_each(|v| load(root, v, contribution)),
                        "--recursive" => recursive = true,
                        _ => {}
                    }
                }
                // The values of the flags that take one are not specs (`--config c.yml`).
                let mut paths: Vec<String> = arguments
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| {
                        index
                            .checked_sub(1)
                            .is_none_or(|before| !VALUE_FLAGS.contains(&arguments[before].as_str()))
                    })
                    .map(|(_, argument)| argument.clone())
                    .collect();
                paths.extend(spec.iter().cloned());
                contribution.test_arguments(
                    root,
                    &paths,
                    if recursive { &["test/**"] } else { SPEC },
                );
            }
        }
        if !ran {
            contribution.test_arguments(root, &spec, if recursive { &["test/**"] } else { SPEC });
        }
    }
}

/// A Mocha configuration file: YAML, JSON or JavaScript, read without running it.
fn read(root: &Path, name: &str) -> Value {
    let source = std::fs::read_to_string(root.join(name)).unwrap_or_default();
    if name.ends_with(".yml") || name.ends_with(".yaml") {
        return yaml(&source);
    }
    if name.ends_with(".json") || name.ends_with(".jsonc") {
        let source = crate::workspaces::without_comments(&source);
        return serde_json::from_str(&source).unwrap_or_default();
    }
    super::static_config(&source, Path::new(name)).unwrap_or_default()
}

/// A file the configuration names (`test/setup`, `./test/setup.js`) or else a package.
fn load(root: &Path, name: &str, contribution: &mut Contribution) {
    let path = super::from_source(root, name);
    // A compiled `setup.js` is the `setup.ts` the source has.
    let stem = path
        .rsplit_once('.')
        .map_or(path.as_str(), |(stem, _)| stem);
    let found = std::iter::once(path.clone())
        .chain(
            super::super::project::source_extensions()
                .iter()
                .flat_map(|extension| {
                    [format!("{path}.{extension}"), format!("{stem}.{extension}")]
                }),
        )
        .find(|candidate| root.join(candidate).is_file());
    match found {
        Some(file) => contribution.reference(&format!("./{file}")),
        None => contribution.reference(name),
    }
}

/// The top-level keys of a YAML file, as far as Mocha's configurations use them: a scalar
/// (`require: should`) or a list (`spec:` followed by `- item` lines).
pub(super) fn yaml(source: &str) -> Value {
    let mut config = serde_json::Map::new();
    let mut list: Option<(String, Vec<Value>)> = None;
    let scalar = |value: &str| {
        let value = value.split(" #").next().unwrap_or_default().trim();
        Value::String(value.trim_matches(['"', '\'']).to_string())
    };
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(item) = trimmed.strip_prefix("- ")
            && line.starts_with([' ', '\t', '-'])
            && let Some((_, items)) = list.as_mut()
        {
            items.push(scalar(item));
            continue;
        }
        if let Some((key, items)) = list.take() {
            config.insert(key, Value::Array(items));
        }
        let Some((key, value)) = trimmed.split_once(':') else {
            continue;
        };
        match value.trim() {
            "" => list = Some((key.trim().to_string(), Vec::new())),
            "true" => {
                config.insert(key.trim().to_string(), Value::Bool(true));
            }
            value => {
                config.insert(key.trim().to_string(), scalar(value));
            }
        }
    }
    if let Some((key, items)) = list {
        config.insert(key, Value::Array(items));
    }
    Value::Object(config)
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use super::*;
    use serde_json::json;

    #[test]
    fn a_yaml_config_is_read_as_far_as_mocha_uses_it() {
        assert_eq!(
            yaml(
                "# comment\nspec:\n  - 'test/**/*.js' # the specs\n  - test/x.js\nrecursive: true\nrequire: should\nnot yaml\nfile:\n"
            ),
            json!({ "spec": ["test/**/*.js", "test/x.js"], "recursive": true, "require": "should", "file": [] })
        );
    }

    #[test]
    fn mocha_reads_its_config_and_command_line() {
        let dir = project(&[
            (
                ".mocharc.json",
                r#"{ "spec": "spec/**/*.js", "require": ["ts-node/register", "./setup.js"], "reporter": "mochawesome" }"#,
            ),
            ("setup.js", ""),
            ("helpers.js", ""),
        ]);
        std::fs::create_dir_all(dir.path().join("spec")).unwrap();
        let manifest = json!({
            "mocha": { "file": "helpers", "reporter": "spec" },
            "scripts": {
                "test": "mocha --recursive -r chai/register-expect -R dot --file",
                "ci": "mocha -R mocha-junit-reporter",
                "other": 1
            }
        });
        let found = setup(dir.path(), &declared(&["mocha"]), &manifest);
        assert_eq!(
            found.tests,
            strings(&["spec/**/*.js", "spec/**", "spec/**/*.js"])
        );
        assert_eq!(found.files, strings(&["./helpers.js", "./setup.js"]));
        assert_eq!(
            found.packages,
            declared(&["chai", "mocha-junit-reporter", "mochawesome", "ts-node"])
        );
    }

    #[test]
    fn a_spec_in_a_build_folder_and_a_config_with_comments_are_read_from_the_source() {
        let dir = project(&[(
            ".mocharc.json",
            r#"{
                // the specs are compiled first
                "file": ["./build/compiled/test/utils/setup.js"],
                "spec": ["./build/compiled/test/**/*.test.{js,ts}"]
            }"#,
        )]);
        std::fs::create_dir_all(dir.path().join("test/utils")).unwrap();
        std::fs::write(dir.path().join("test/utils/setup.ts"), "").unwrap();
        let found = setup(dir.path(), &declared(&["mocha"]), &json!({}));
        assert_eq!(found.tests, strings(&["test/**/*.test.{js,ts}", "test/**"]));
        assert_eq!(found.files, strings(&["./test/utils/setup.ts"]));
    }

    #[test]
    fn a_config_forwarded_to_a_script_that_runs_mocha_through_its_own_bin_is_read() {
        let dir = project(&[]);
        std::fs::create_dir_all(dir.path().join("test/unit")).unwrap();
        std::fs::write(
            dir.path().join("test/unit/mocharc.cjs"),
            "module.exports = { spec: ['test/unit/*.spec.cjs'] };",
        )
        .unwrap();
        let manifest = json!({
            "name": "mocha",
            "bin": { "mocha": "./bin/mocha.js" },
            "scripts": {
                "test-unit": "npm run -s run-tests -- --config test/unit/mocharc.cjs",
                "run-tests": "cross-env A=1 node bin/mocha.js --forbid-only"
            }
        });
        let found = setup(dir.path(), &declared(&[]), &manifest);
        assert!(
            found.tests.contains(&"test/unit/*.spec.cjs".to_string()),
            "{:?}",
            found.tests
        );
        assert!(
            found.files.contains(&"./test/unit/mocharc.cjs".to_string()),
            "{:?}",
            found.files
        );
        let manifest = json!({
            "name": "mocha",
            "bin": "bin/mocha.js",
            "scripts": {
                "unit": "pnpm run run-tests -- --grep \"slow case\" --config test/unit/mocharc.cjs",
                "run-tests": "node bin/mocha.js"
            }
        });
        let found = setup(dir.path(), &declared(&[]), &manifest);
        assert!(
            found.tests.contains(&"test/unit/*.spec.cjs".to_string()),
            "{:?}",
            found.tests
        );
    }

    #[test]
    fn without_a_config_mocha_runs_its_default_spec() {
        let dir = project(&[]);
        let found = setup(dir.path(), &declared(&["mocha"]), &json!({}));
        assert_eq!(found.tests, strings(&["test/*.*"]));
        let found = setup(
            dir.path(),
            &declared(&["mocha"]),
            &json!({ "mocha": { "recursive": true } }),
        );
        assert_eq!(found.tests, strings(&["test/**"]));
        let dir = project(&[(".mocharc.js", "module.exports = { spec: 'x/*.js' };")]);
        std::fs::write(dir.path().join("c.yml"), "recursive: true\n").unwrap();
        let manifest = json!({ "scripts": { "a": "mocha --config c.yml", "b": "mocha" } });
        let found = setup(dir.path(), &declared(&["mocha"]), &manifest);
        assert_eq!(found.tests, strings(&["x/*.js", "x/*.js"]));
        let manifest = json!({ "scripts": { "a": "jest" } });
        let found = setup(dir.path(), &declared(&["mocha"]), &manifest);
        assert_eq!(found.tests, strings(&["x/*.js"]));
        let dir = project(&[]);
        let manifest = json!({ "mocha": { "recursive": true }, "scripts": { "a": "jest" } });
        let found = setup(dir.path(), &declared(&["mocha"]), &manifest);
        assert_eq!(found.tests, strings(&["test/**"]));
    }

    #[test]
    fn the_values_of_the_watch_and_parallel_flags_are_not_specs() {
        let dir = project(&[]);
        for folder in ["lib", "test"] {
            std::fs::create_dir(dir.path().join(folder)).unwrap();
        }
        std::fs::write(dir.path().join("test/a.test.js"), "").unwrap();
        let manifest = json!({ "scripts": { "tdd": "mocha --watch --watch-files lib/**/*.js --watch-ignore lib/x.js -j 2 --retries 1 -f x -n inspect test/a.test.js" } });
        let found = setup(dir.path(), &declared(&["mocha"]), &manifest);
        assert_eq!(found.tests, strings(&["test/a.test.js", "test/**"]));
    }
}

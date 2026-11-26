//! `prunr init`: a first `prunr.json`, guessed from `package.json` and translated from a Knip
//! config when there is one. From then on the file is edited by hand; nothing is guessed when
//! the check runs.

use std::collections::BTreeSet;
use std::path::Path;

use serde::Deserialize;

use crate::config::{FILE_NAME, SCHEMA_URL};
use crate::project::Project;

/// Test globs `init` proposes when they match a file.
const TEST_GLOBS: &[&str] = &[
    "__tests__/**",
    "__mocks__/**",
    "test/**",
    "tests/**",
    "**/*.test.*",
    "**/*.spec.*",
];

/// Entries of a TypeScript project, whose `main` usually points to the compiled output.
const TYPESCRIPT_ENTRIES: &[&str] = &["index.ts", "src/index.ts", "src/main.ts"];

#[derive(Default, Deserialize)]
struct Manifest {
    main: Option<String>,
}

/// Knip's config files, in the order it looks for them; then the `knip` key of `package.json`.
const KNIP_CONFIGS: &[&str] = &[
    "knip.json",
    "knip.jsonc",
    ".knip.json",
    ".knip.jsonc",
    "knip.ts",
    "knip.js",
    "knip.config.ts",
    "knip.config.js",
];

/// What a Knip config translates to.
#[derive(Default)]
struct Knip {
    production: BTreeSet<String>,
    test: BTreeSet<String>,
    ignore: Vec<String>,
    left_out: Vec<String>,
}

/// The Knip config of the project and the file it is in, read without running it.
fn knip_config(root: &Path) -> Result<Option<(String, serde_json::Value)>, String> {
    for name in KNIP_CONFIGS {
        let Ok(text) = std::fs::read_to_string(root.join(name)) else {
            continue;
        };
        let config = match name.ends_with(".json") || name.ends_with(".jsonc") {
            true => {
                let text = crate::workspaces::without_comments(&text);
                let trailing = regex::Regex::new(r",(\s*[}\]])").expect("the pattern compiles");
                serde_json::from_str(&trailing.replace_all(&text, "$1")).ok()
            }
            false => crate::plugins::static_config(&text, Path::new(name)),
        };
        return match config {
            Some(config) => Ok(Some((name.to_string(), config))),
            None => Err(format!("invalid {name}")),
        };
    }
    let manifest: serde_json::Value = std::fs::read_to_string(root.join("package.json"))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    Ok(manifest
        .get("knip")
        .map(|config| ("package.json".to_string(), config.clone())))
}

/// Whether a Knip entry pattern names test code.
fn is_test(pattern: &str) -> bool {
    let folders = ["test", "tests", "__tests__", "spec", "e2e"];
    pattern.split('/').any(|segment| folders.contains(&segment))
        || [".test.", ".spec.", ".test-d."]
            .iter()
            .any(|infix| pattern.contains(infix))
}

/// Translates one level of a Knip config: the root, or a workspace at `prefix`. `at` names
/// where its keys are, for what is left out.
fn translate(config: &serde_json::Value, prefix: &str, at: &str, knip: &mut Knip) {
    let empty = serde_json::Map::new();
    for (key, value) in config.as_object().unwrap_or(&empty) {
        match key.as_str() {
            "$schema" => {}
            "entry" => {
                for pattern in crate::plugins::names(value) {
                    if pattern.starts_with('!') {
                        knip.left_out.push(format!("{at}entry {pattern}"));
                        continue;
                    }
                    // A trailing `!` marks a production entry in Knip.
                    let production = pattern.ends_with('!');
                    let pattern = pattern.trim_end_matches('!').trim_start_matches("./");
                    let pattern = format!("{prefix}{pattern}");
                    match production || !is_test(&pattern) {
                        true => knip.production.insert(pattern),
                        false => knip.test.insert(pattern),
                    };
                }
            }
            "ignoreDependencies" => knip
                .ignore
                .extend(crate::plugins::names(value).into_iter().map(str::to_string)),
            "workspaces" if prefix.is_empty() && at.is_empty() => {
                for (dir, workspace) in value.as_object().unwrap_or(&empty) {
                    let prefix = match dir.as_str() {
                        "." => String::new(),
                        dir => format!("{}/", dir.trim_end_matches('/')),
                    };
                    translate(workspace, &prefix, &format!("workspaces.{dir}."), knip);
                }
            }
            _ => knip.left_out.push(format!("{at}{key}")),
        }
    }
}

/// What `init` wrote.
#[derive(Debug, PartialEq, Eq)]
pub struct Created {
    pub contents: String,
    /// The Knip config it was translated from.
    pub from: Option<String>,
    /// What of that config has no equivalent in prunr.
    pub left_out: Vec<String>,
}

/// Writes `prunr.json` at `root` and returns what it wrote.
pub fn init(root: &Path) -> Result<Created, String> {
    if root.join(FILE_NAME).exists() {
        return Err(format!("{FILE_NAME} already exists"));
    }
    let manifest = match std::fs::read_to_string(root.join("package.json")) {
        Ok(text) => {
            serde_json::from_str(&text).map_err(|e| format!("invalid package.json: {e}"))?
        }
        Err(_) => Manifest::default(),
    };
    let knip_config = knip_config(root)?;
    let mut knip = Knip::default();
    if let Some((_, config)) = &knip_config {
        translate(config, "", "", &mut knip);
    }
    let project = Project::load(root).map_err(|e| e.to_string())?;
    let is_file = |path: &String| project.entries(std::slice::from_ref(path)).is_ok();

    let mut candidates = vec![manifest.main.unwrap_or_else(|| "index.js".into())];
    candidates.extend(project.root_entries.iter().cloned());
    candidates.extend(TYPESCRIPT_ENTRIES.iter().map(|entry| entry.to_string()));
    // The files the scripts run, Next.js routes and the NestJS main are entries already.
    let covered = |path: &String| {
        let id = path.trim_start_matches("./");
        project.production_files.iter().any(|file| file == id)
    };
    let mut production: BTreeSet<String> = candidates
        .into_iter()
        .filter(|path| is_file(path) && !covered(path))
        .collect();
    production.extend(knip.production);
    if production.is_empty() && project.production_files.is_empty() {
        return Err(format!(
            "no entry found: create {FILE_NAME} with entries.production by hand"
        ));
    }
    // Jest and Vitest find their own test files.
    let runner = !project.test_files.is_empty();
    let mut test: Vec<String> = TEST_GLOBS
        .iter()
        .map(|glob| glob.to_string())
        .filter(|glob| !runner && is_file(glob))
        .collect();
    test.extend(knip.test);

    let mut entries = serde_json::json!({ "production": production });
    if !test.is_empty() {
        entries["test"] = serde_json::json!(test);
    }
    let mut config = serde_json::json!({ "$schema": SCHEMA_URL, "entries": entries });
    if !knip.ignore.is_empty() {
        config["dependencies"] = serde_json::json!({ "ignore": knip.ignore });
    }
    let contents = serde_json::to_string_pretty(&config).expect("the config serializes") + "\n";
    std::fs::write(root.join(FILE_NAME), &contents).map_err(|e| e.to_string())?;
    Ok(Created {
        contents,
        from: knip_config.map(|(from, _)| from),
        left_out: knip.left_out,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (path, content) in files {
            let path = dir.path().join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, content).unwrap();
        }
        dir
    }

    fn written(dir: &tempfile::TempDir) -> serde_json::Value {
        let text = std::fs::read_to_string(dir.path().join("prunr.json")).unwrap();
        serde_json::from_str(&text).unwrap()
    }

    #[test]
    fn the_files_the_scripts_run_are_left_to_the_check() {
        let dir = project(&[
            (
                "package.json",
                r#"{
                    "main": "src/index.js",
                    "scripts": {
                        "start": "NODE_ENV=prod node --env-file-if-exists=.env --max-old-space-size=8192 src/index.js",
                        "worker": "node --watch workers/index.js",
                        "cli": "node scripts/cli.js",
                        "gone": "node scripts/gone.js",
                        "lint": "biome check ."
                    }
                }"#,
            ),
            ("src/index.js", ""),
            ("workers/index.js", ""),
            ("scripts/cli.js", ""),
            ("__tests__/index.test.js", ""),
            ("lib/util.spec.js", ""),
        ]);
        let created = init(dir.path()).unwrap();
        assert_eq!(created.from, None);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&created.contents).unwrap(),
            written(&dir)
        );
        assert_eq!(
            written(&dir),
            serde_json::json!({
                "$schema": "https://unpkg.com/@prunr/prunr/schema.json",
                "entries": {
                    "production": [],
                    "test": ["__tests__/**", "**/*.test.*", "**/*.spec.*"]
                }
            })
        );
    }

    #[test]
    fn the_entry_exports_names_is_found_in_the_source_a_build_compiles() {
        let dir = project(&[
            (
                "package.json",
                r#"{ "private": true, "exports": { "types": "./dist/source/index.d.ts", "default": "./dist/source/index.js" } }"#,
            ),
            ("source/index.ts", ""),
        ]);
        init(dir.path()).unwrap();
        assert_eq!(
            written(&dir)["entries"]["production"],
            serde_json::json!(["source/index.ts"])
        );
    }

    #[test]
    fn a_typescript_project_gets_its_sources_and_tests() {
        let dir = project(&[
            (
                "package.json",
                r#"{
                    "private": true,
                    "main": "dist/index.js",
                    "scripts": {
                        "dev": "tsx watch src/server.ts",
                        "seed": "ts-node --transpile-only scripts/seed.ts"
                    }
                }"#,
            ),
            ("src/index.ts", ""),
            ("src/server.ts", ""),
            ("scripts/seed.ts", ""),
            ("src/a.test.ts", ""),
            ("src/b.spec.tsx", ""),
        ]);
        init(dir.path()).unwrap();
        assert_eq!(
            written(&dir)["entries"],
            serde_json::json!({
                "production": ["src/index.ts"],
                "test": ["**/*.test.*", "**/*.spec.*"]
            })
        );
    }

    #[test]
    fn with_a_test_runner_no_test_entries_are_proposed() {
        let dir = project(&[
            (
                "package.json",
                r#"{ "private": true, "devDependencies": { "jest": "29" } }"#,
            ),
            ("index.js", ""),
            ("index.test.js", ""),
        ]);
        init(dir.path()).unwrap();
        assert_eq!(
            written(&dir)["entries"],
            serde_json::json!({ "production": ["index.js"] })
        );
    }

    #[test]
    fn a_framework_project_needs_no_entries_of_its_own() {
        let dir = project(&[
            (
                "package.json",
                r#"{ "dependencies": { "@nestjs/core": "11" } }"#,
            ),
            ("src/main.ts", ""),
        ]);
        init(dir.path()).unwrap();
        assert_eq!(
            written(&dir)["entries"],
            serde_json::json!({ "production": [] })
        );
    }

    #[test]
    fn without_main_index_js_is_the_entry() {
        let dir = project(&[("package.json", r#"{ "private": true }"#), ("index.js", "")]);
        init(dir.path()).unwrap();
        assert_eq!(
            written(&dir),
            serde_json::json!({
                "$schema": "https://unpkg.com/@prunr/prunr/schema.json",
                "entries": { "production": ["index.js"] }
            })
        );
    }

    #[test]
    fn without_package_json_index_js_is_the_entry() {
        let dir = project(&[("index.js", "")]);
        init(dir.path()).unwrap();
        assert_eq!(
            written(&dir)["entries"]["production"],
            serde_json::json!(["index.js"])
        );
    }

    #[test]
    fn a_knip_config_gives_its_entries_and_ignored_dependencies() {
        let dir = project(&[
            ("package.json", r#"{ "main": "index.js" }"#),
            ("index.js", ""),
            (
                "knip.jsonc",
                r#"{
                    // Knip's config, with comments and trailing commas
                    "$schema": "https://unpkg.com/knip@5/schema.json",
                    "entry": ["src/server.ts!", "./scripts/*.ts", "test/**/*.test.ts", "!src/gen/**"],
                    "ignoreDependencies": ["left-pad", 1],
                    "ignore": ["dist/**"],
                    "rules": { "types": "off" },
                }"#,
            ),
        ]);
        let created = init(dir.path()).unwrap();
        assert_eq!(
            written(&dir),
            serde_json::json!({
                "$schema": "https://unpkg.com/@prunr/prunr/schema.json",
                "entries": {
                    "production": ["scripts/*.ts", "src/server.ts"],
                    "test": ["test/**/*.test.ts"]
                },
                "dependencies": { "ignore": ["left-pad"] }
            })
        );
        assert_eq!(created.from.as_deref(), Some("knip.jsonc"));
        assert_eq!(
            created.left_out,
            ["entry !src/gen/**", "ignore", "rules"].map(String::from)
        );
    }

    #[test]
    fn the_knip_key_of_package_json_and_its_workspaces_are_read() {
        let dir = project(&[
            (
                "package.json",
                r#"{ "knip": { "workspaces": { ".": { "entry": "main.js" }, "packages/a": { "entry": ["cli.js"], "project": ["**"] }, "packages/b": 1 } } }"#,
            ),
            ("main.js", ""),
        ]);
        let created = init(dir.path()).unwrap();
        assert_eq!(
            written(&dir)["entries"],
            serde_json::json!({ "production": ["main.js", "packages/a/cli.js"] })
        );
        assert_eq!(created.from.as_deref(), Some("package.json"));
        assert_eq!(
            created.left_out,
            ["workspaces.packages/a.project"].map(String::from)
        );
    }

    #[test]
    fn a_knip_config_in_typescript_is_read_without_running_it() {
        let dir = project(&[("knip.ts", "export default { entry: ['src/app.ts'] };")]);
        let created = init(dir.path()).unwrap();
        assert_eq!(
            written(&dir)["entries"],
            serde_json::json!({ "production": ["src/app.ts"] })
        );
        assert_eq!(created.from.as_deref(), Some("knip.ts"));
        let dir = project(&[("knip.json", "{ nope")]);
        assert_eq!(init(dir.path()), Err("invalid knip.json".into()));
    }

    #[test]
    fn it_does_not_overwrite_an_existing_config() {
        let dir = project(&[("index.js", ""), ("prunr.json", "{}")]);
        assert_eq!(init(dir.path()), Err("prunr.json already exists".into()));
        assert_eq!(
            std::fs::read_to_string(dir.path().join("prunr.json")).unwrap(),
            "{}"
        );
    }

    #[test]
    fn without_any_entry_it_asks_for_one() {
        let dir = project(&[("package.json", r#"{ "main": "dist/gone.js" }"#)]);
        assert_eq!(
            init(dir.path()),
            Err("no entry found: create prunr.json with entries.production by hand".into())
        );
        assert!(!dir.path().join("prunr.json").exists());
    }

    #[test]
    fn an_invalid_package_json_is_an_error() {
        let dir = project(&[("package.json", "{ nope")]);
        let error = init(dir.path()).unwrap_err();
        assert!(error.starts_with("invalid package.json: "), "{error}");
    }

    #[test]
    fn an_invalid_package_json_of_a_workspace_is_an_error() {
        let dir = project(&[
            ("package.json", r#"{ "workspaces": ["packages/*"] }"#),
            ("packages/a/package.json", "{ nope"),
        ]);
        let error = init(dir.path()).unwrap_err();
        assert!(error.starts_with("invalid package.json: "), "{error}");
    }

    #[test]
    fn a_file_that_does_not_parse_does_not_stop_it() {
        let dir = project(&[
            ("index.js", "const = ;"),
            ("fixtures/broken.js", "const = ;"),
        ]);
        init(dir.path()).unwrap();
        assert_eq!(
            written(&dir)["entries"]["production"],
            serde_json::json!(["index.js"])
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_directory_it_cannot_write_to_is_an_error() {
        use std::os::unix::fs::PermissionsExt;
        let dir = project(&[("index.js", "")]);
        let readonly = std::fs::Permissions::from_mode(0o555);
        std::fs::set_permissions(dir.path(), readonly).unwrap();
        let result = init(dir.path());
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(result.unwrap_err().contains("ermission denied"));
    }
}

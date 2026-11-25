//! `prunr init`: a first `prunr.json`, guessed from `package.json`. From then on the file is
//! edited by hand; nothing is guessed when the check runs.

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

/// Writes `prunr.json` at `root` and returns its contents.
pub fn init(root: &Path) -> Result<String, String> {
    if root.join(FILE_NAME).exists() {
        return Err(format!("{FILE_NAME} already exists"));
    }
    let manifest = match std::fs::read_to_string(root.join("package.json")) {
        Ok(text) => {
            serde_json::from_str(&text).map_err(|e| format!("invalid package.json: {e}"))?
        }
        Err(_) => Manifest::default(),
    };
    let project = Project::load(root).map_err(|e| e.to_string())?;
    let is_file = |path: &String| project.entries(std::slice::from_ref(path)).is_ok();

    let mut candidates = vec![manifest.main.unwrap_or_else(|| "index.js".into())];
    candidates.extend(TYPESCRIPT_ENTRIES.iter().map(|entry| entry.to_string()));
    // The files the scripts run, Next.js routes and the NestJS main are entries already.
    let covered = |path: &String| {
        let id = path.trim_start_matches("./");
        project.production_files.iter().any(|file| file == id)
    };
    let production: BTreeSet<String> = candidates
        .into_iter()
        .filter(|path| is_file(path) && !covered(path))
        .collect();
    if production.is_empty() && project.production_files.is_empty() {
        return Err(format!(
            "no entry found: create {FILE_NAME} with entries.production by hand"
        ));
    }
    // Jest and Vitest find their own test files.
    let runner = !project.test_files.is_empty();
    let test: Vec<String> = TEST_GLOBS
        .iter()
        .map(|glob| glob.to_string())
        .filter(|glob| !runner && is_file(glob))
        .collect();

    let mut entries = serde_json::json!({ "production": production });
    if !test.is_empty() {
        entries["test"] = serde_json::json!(test);
    }
    let config = serde_json::json!({ "$schema": SCHEMA_URL, "entries": entries });
    let contents = serde_json::to_string_pretty(&config).expect("the config serializes") + "\n";
    std::fs::write(root.join(FILE_NAME), &contents).map_err(|e| e.to_string())?;
    Ok(contents)
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
        let contents = init(dir.path()).unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&contents).unwrap(),
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
    fn a_typescript_project_gets_its_sources_and_tests() {
        let dir = project(&[
            (
                "package.json",
                r#"{
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
            ("package.json", r#"{ "devDependencies": { "jest": "29" } }"#),
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
        let dir = project(&[("package.json", "{}"), ("index.js", "")]);
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
    fn a_file_that_does_not_parse_is_an_error() {
        let dir = project(&[("index.js", "const = ;")]);
        let error = init(dir.path()).unwrap_err();
        assert!(error.starts_with("index.js: "), "{error}");
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

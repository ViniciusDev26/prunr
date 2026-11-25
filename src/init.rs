//! `prunr init`: a first `prunr.json`, guessed from `package.json`. From then on the file is
//! edited by hand; nothing is guessed when the check runs.

use std::path::Path;

/// Test globs `init` proposes when they match a file.
const TEST_GLOBS: &[&str] = &[
    "__tests__/**",
    "__mocks__/**",
    "test/**",
    "tests/**",
    "**/*.test.js",
    "**/*.spec.js",
];

/// Writes `prunr.json` at `root` and returns its contents.
pub fn init(root: &Path) -> Result<String, String> {
    todo!()
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
    fn entries_come_from_main_and_the_node_scripts() {
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
                "$schema": "https://unpkg.com/prunr/schema.json",
                "entries": {
                    "production": ["scripts/cli.js", "src/index.js", "workers/index.js"],
                    "test": ["__tests__/**", "**/*.test.js", "**/*.spec.js"]
                }
            })
        );
    }

    #[test]
    fn without_main_index_js_is_the_entry() {
        let dir = project(&[("package.json", "{}"), ("index.js", "")]);
        init(dir.path()).unwrap();
        assert_eq!(
            written(&dir),
            serde_json::json!({
                "$schema": "https://unpkg.com/prunr/schema.json",
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
}

//! `prunr.json`: the project's configuration file.
//!
//! An option only exists here once a feature needs it.

use std::path::{Path, PathBuf};

use serde::Deserialize;

pub const FILE_NAME: &str = "prunr.json";

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub entries: Entries,
}

/// Paths or globs, relative to the project root.
#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entries {
    /// Where the program starts. Everything they reach, directly or not, is alive.
    pub production: Vec<String>,
    /// Test code. It is never reported, and what only it reaches is dead in production.
    #[serde(default)]
    pub test: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConfigError {
    Missing(PathBuf),
    Invalid(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(path) => write!(f, "{} not found", path.display()),
            Self::Invalid(reason) => write!(f, "invalid {FILE_NAME}: {reason}"),
        }
    }
}

impl Config {
    /// Reads and validates `prunr.json` at `root`.
    pub fn load(root: &Path) -> Result<Self, ConfigError> {
        let path = root.join(FILE_NAME);
        if !path.exists() {
            return Err(ConfigError::Missing(path));
        }
        let text =
            std::fs::read_to_string(&path).map_err(|e| ConfigError::Invalid(e.to_string()))?;
        serde_json::from_str(&text).map_err(|e| ConfigError::Invalid(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(config: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(FILE_NAME), config).unwrap();
        dir
    }

    #[test]
    fn reads_the_production_and_test_entries() {
        let dir =
            project(r#"{ "entries": { "production": ["index.js"], "test": ["**/*.test.js"] } }"#);
        let config = Config::load(dir.path()).unwrap();
        assert_eq!(config.entries.production, ["index.js"]);
        assert_eq!(config.entries.test, ["**/*.test.js"]);
    }

    #[test]
    fn test_entries_are_optional() {
        let dir = project(r#"{ "entries": { "production": ["index.js"] } }"#);
        let config = Config::load(dir.path()).unwrap();
        assert!(config.entries.test.is_empty());
    }

    #[test]
    fn an_unknown_key_in_entries_is_an_error() {
        let dir = project(r#"{ "entries": { "production": [], "tests": [] } }"#);
        let error = Config::load(dir.path()).unwrap_err();
        assert!(
            error.to_string().contains("unknown field `tests`"),
            "{error}"
        );
    }

    #[test]
    fn a_missing_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let error = Config::load(dir.path()).unwrap_err();
        assert_eq!(error, ConfigError::Missing(dir.path().join(FILE_NAME)));
        assert_eq!(
            error.to_string(),
            format!("{} not found", dir.path().join(FILE_NAME).display())
        );
    }

    #[test]
    fn malformed_json_is_an_error() {
        let dir = project("{ nope");
        let error = Config::load(dir.path()).unwrap_err();
        assert!(matches!(error, ConfigError::Invalid(_)), "{error:?}");
        assert!(
            error.to_string().starts_with("invalid prunr.json: "),
            "{error}"
        );
    }

    #[test]
    fn an_unknown_key_is_an_error() {
        let dir = project(r#"{ "entries": { "production": [] }, "entrys": [] }"#);
        let error = Config::load(dir.path()).unwrap_err();
        assert!(
            error.to_string().contains("unknown field `entrys`"),
            "{error}"
        );
    }

    #[test]
    fn entries_are_required() {
        let dir = project("{}");
        let error = Config::load(dir.path()).unwrap_err();
        assert!(
            error.to_string().contains("missing field `entries`"),
            "{error}"
        );
    }

    #[test]
    fn an_unreadable_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(FILE_NAME)).unwrap();
        let error = Config::load(dir.path()).unwrap_err();
        assert!(
            error.to_string().starts_with("invalid prunr.json: "),
            "{error}"
        );
    }
}

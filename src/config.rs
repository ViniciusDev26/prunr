//! `prunr.json`: the project's configuration file.
//!
//! An option only exists here once a feature needs it.

use std::path::{Path, PathBuf};

use serde::Deserialize;

pub const FILE_NAME: &str = "prunr.json";

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Files the program starts from, relative to the project root. Everything they reach,
    /// directly or not, is alive.
    pub entries: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConfigError {
    Missing(PathBuf),
    Invalid(String),
    EntryNotFound(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(path) => write!(f, "{} not found", path.display()),
            Self::Invalid(reason) => write!(f, "invalid {FILE_NAME}: {reason}"),
            Self::EntryNotFound(entry) => write!(f, "entry not found: {entry}"),
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
        let config: Self =
            serde_json::from_str(&text).map_err(|e| ConfigError::Invalid(e.to_string()))?;
        if let Some(entry) = config.entries.iter().find(|e| !root.join(e).is_file()) {
            return Err(ConfigError::EntryNotFound(entry.clone()));
        }
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(config: &str, files: &[&str]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(FILE_NAME), config).unwrap();
        for file in files {
            std::fs::write(dir.path().join(file), "").unwrap();
        }
        dir
    }

    #[test]
    fn reads_the_entries() {
        let dir = project(
            r#"{ "entries": ["index.js", "cli.js"] }"#,
            &["index.js", "cli.js"],
        );
        let config = Config::load(dir.path()).unwrap();
        assert_eq!(config.entries, ["index.js", "cli.js"]);
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
        let dir = project("{ nope", &[]);
        let error = Config::load(dir.path()).unwrap_err();
        assert!(matches!(error, ConfigError::Invalid(_)), "{error:?}");
        assert!(
            error.to_string().starts_with("invalid prunr.json: "),
            "{error}"
        );
    }

    #[test]
    fn an_unknown_key_is_an_error() {
        let dir = project(r#"{ "entries": [], "entrys": [] }"#, &[]);
        let error = Config::load(dir.path()).unwrap_err();
        assert!(
            error.to_string().contains("unknown field `entrys`"),
            "{error}"
        );
    }

    #[test]
    fn entries_are_required() {
        let dir = project("{}", &[]);
        let error = Config::load(dir.path()).unwrap_err();
        assert!(
            error.to_string().contains("missing field `entries`"),
            "{error}"
        );
    }

    #[test]
    fn an_entry_that_does_not_exist_is_an_error() {
        let dir = project(r#"{ "entries": ["index.js", "gone.js"] }"#, &["index.js"]);
        let error = Config::load(dir.path()).unwrap_err();
        assert_eq!(error, ConfigError::EntryNotFound("gone.js".into()));
        assert_eq!(error.to_string(), "entry not found: gone.js");
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

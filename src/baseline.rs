//! The baseline: findings a project already had when it adopted prunr. They do not fail the
//! check; a new finding does, and so does an entry that is no longer found, so the list only
//! shrinks.

use std::path::Path;

use crate::checks::Finding;

/// The findings in the baseline file at `path`; none when it does not exist yet.
pub fn read(path: &Path) -> Result<Vec<Finding>, String> {
    todo!()
}

/// Writes the findings that are errors, sorted, so the file is the same for the same findings.
pub fn write(path: &Path, findings: &[Finding]) -> Result<usize, String> {
    todo!()
}

/// Findings split against the baseline.
#[derive(Debug, PartialEq, Eq)]
pub struct Diff {
    /// Findings the baseline does not list.
    pub fresh: Vec<Finding>,
    /// Baseline entries no longer found.
    pub stale: Vec<Finding>,
}

pub fn diff(findings: Vec<Finding>, known: &[Finding]) -> Diff {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn export(file: &str, symbol: &str) -> Finding {
        Finding::UnusedExport {
            file: file.into(),
            symbol: symbol.into(),
            test_only: false,
        }
    }

    fn warning() -> Finding {
        Finding::UnresolvedRequire {
            file: "index.js".into(),
            specifier: "require(name)".into(),
        }
    }

    #[test]
    fn a_missing_baseline_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(read(&dir.path().join("baseline.json")), Ok(vec![]));
    }

    #[test]
    fn it_writes_errors_only_sorted_and_reads_them_back() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("baseline.json");
        let findings = [export("b.js", "x"), warning(), export("a.js", "y")];
        assert_eq!(write(&path, &findings), Ok(2));
        assert_eq!(
            read(&path),
            Ok(vec![export("a.js", "y"), export("b.js", "x")])
        );
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(
            text.starts_with("[\n  {\n    \"kind\": \"unused-export\""),
            "{text}"
        );
        assert!(text.ends_with("]\n"), "{text}");
    }

    #[test]
    fn writing_the_same_findings_twice_gives_the_same_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("baseline.json");
        write(&path, &[export("a.js", "y"), export("b.js", "x")]).unwrap();
        let first = std::fs::read(&path).unwrap();
        write(&path, &[export("b.js", "x"), export("a.js", "y")]).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), first);
    }

    #[test]
    fn an_invalid_baseline_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("baseline.json");
        std::fs::write(&path, "{ nope").unwrap();
        let error = read(&path).unwrap_err();
        assert!(
            error.starts_with("invalid baseline baseline.json: "),
            "{error}"
        );
    }

    #[test]
    fn a_baseline_it_cannot_write_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing-dir/baseline.json");
        assert!(write(&path, &[]).is_err());
    }

    #[test]
    fn known_findings_pass_new_ones_fail_and_fixed_ones_are_stale() {
        let known = [export("a.js", "known"), export("a.js", "fixed")];
        let findings = vec![export("a.js", "known"), export("a.js", "new"), warning()];
        assert_eq!(
            diff(findings, &known),
            Diff {
                fresh: vec![export("a.js", "new"), warning()],
                stale: vec![export("a.js", "fixed")],
            }
        );
    }
}

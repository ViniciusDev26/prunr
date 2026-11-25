//! How findings are printed.

use crate::checks::Finding;

/// One line per finding and a summary, for people.
pub fn human(findings: &[Finding]) -> String {
    todo!()
}

/// `{ "findings": [...] }`, for machines.
pub fn json(findings: &[Finding]) -> String {
    todo!()
}

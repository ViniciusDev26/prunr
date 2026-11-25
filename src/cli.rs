//! `prunr check [--json]`.

use std::io::Write;
use std::path::Path;

pub const USAGE: &str = "usage: prunr check [--json]";

/// Runs the command in `root` and returns the exit code: 0 without errors, 1 with findings
/// that are errors, 2 when the check could not run.
pub fn run(args: &[String], root: &Path, out: &mut impl Write, err: &mut impl Write) -> u8 {
    todo!()
}

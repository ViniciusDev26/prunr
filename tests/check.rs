//! `prunr check` run against the projects in `tests/fixtures/`, one per behavior.

use std::path::Path;
use std::process::Command;

struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

fn prunr(fixture: &str, args: &[&str]) -> Run {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(fixture);
    let output = Command::new(env!("CARGO_BIN_EXE_prunr"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    Run {
        code: output.status.code().unwrap(),
        stdout: String::from_utf8(output.stdout).unwrap(),
        stderr: String::from_utf8(output.stderr).unwrap(),
    }
}

fn check(fixture: &str) -> Run {
    prunr(fixture, &["check"])
}

#[test]
fn a_file_no_entry_reaches_is_unused() {
    let run = check("require-static");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-file         lib/unused.js\n\nerrors: 1, warnings: 0\n"
    );
}

#[test]
fn a_destructured_require_uses_only_its_properties() {
    let run = check("destructuring");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-export       math.js: sub\n\nerrors: 1, warnings: 0\n"
    );
}

#[test]
fn a_namespace_require_uses_only_the_members_it_reads() {
    let run = check("namespace");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-export       math.js: sub\n\nerrors: 1, warnings: 0\n"
    );
}

#[test]
fn property_assignments_are_exports() {
    let run = check("exports-assignment");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-export       math.js: sub\n\nerrors: 1, warnings: 0\n"
    );
}

#[test]
fn a_property_of_a_default_export_is_an_export() {
    let run = check("default-export");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-export       greet.js: default.shout\n\nerrors: 1, warnings: 0\n"
    );
}

#[test]
fn an_unused_static_method_is_an_unused_export() {
    let run = check("class-static");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-export       invoice.js: Invoice.legacyTotal\n\
         unused-export       invoice.js: Invoice.unusedHelper\n\
         \n\
         errors: 2, warnings: 0\n"
    );
}

#[test]
fn a_require_that_cannot_be_followed_is_a_warning() {
    let run = check("unresolved-require");
    assert_eq!(run.code, 0, "warnings do not fail the check");
    assert_eq!(
        run.stdout,
        "unresolved-require  index.js: ./missing (warning)\n\
         unresolved-require  index.js: require(name) (warning)\n\
         \n\
         errors: 0, warnings: 2\n"
    );
}

#[test]
fn dependencies_are_compared_with_package_json() {
    let run = check("dependencies");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-dependency   jest\n\
         unused-dependency   left-pad\n\
         missing-dependency  index.js: axios\n\
         \n\
         errors: 3, warnings: 0\n"
    );
}

#[test]
fn a_cycle_is_reported_once_from_its_first_file() {
    let run = check("circular");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "circular-import     a.js -> b.js -> c.js -> a.js\n\nerrors: 1, warnings: 0\n"
    );
}

#[test]
fn a_clean_project_passes() {
    let run = check("clean");
    assert_eq!(run.code, 0);
    assert_eq!(run.stdout, "No findings.\n");
}

#[test]
fn the_example_reports_every_kind_of_finding() {
    let run = check("example");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-file         src/legacy-report.js\n\
         unused-export       src/invoices.js: cancelInvoice\n\
         unused-export       src/money.js: Money.fromCents\n\
         unused-export       src/money.js: Money.invoiceFor\n\
         unused-dependency   left-pad\n\
         missing-dependency  src/index.js: dotenv\n\
         circular-import     src/invoices.js -> src/money.js -> src/invoices.js\n\
         unresolved-require  src/index.js: require(process.env.PLUGIN) (warning)\n\
         \n\
         errors: 7, warnings: 1\n"
    );
}

#[test]
fn json_output_is_for_machines() {
    let run = prunr("example", &["check", "--json"]);
    assert_eq!(run.code, 1);
    let expected = r#"{
  "findings": [
    { "kind": "unused-file", "file": "src/legacy-report.js" },
    { "kind": "unused-export", "file": "src/invoices.js", "symbol": "cancelInvoice" },
    { "kind": "unused-export", "file": "src/money.js", "symbol": "Money.fromCents" },
    { "kind": "unused-export", "file": "src/money.js", "symbol": "Money.invoiceFor" },
    { "kind": "unused-dependency", "package": "left-pad" },
    { "kind": "missing-dependency", "file": "src/index.js", "package": "dotenv" },
    { "kind": "circular-import", "cycle": ["src/invoices.js", "src/money.js", "src/invoices.js"] },
    { "kind": "unresolved-require", "file": "src/index.js", "specifier": "require(process.env.PLUGIN)" }
  ]
}"#;
    let actual: serde_json::Value = serde_json::from_str(&run.stdout).unwrap();
    let expected: serde_json::Value = serde_json::from_str(expected).unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn json_output_of_a_clean_project_is_an_empty_list() {
    let run = prunr("clean", &["check", "--json"]);
    assert_eq!(run.code, 0);
    let actual: serde_json::Value = serde_json::from_str(&run.stdout).unwrap();
    assert_eq!(actual, serde_json::json!({ "findings": [] }));
}

#[test]
fn a_missing_config_is_an_error() {
    let run = check("missing-config");
    assert_eq!(run.code, 2);
    assert_eq!(run.stdout, "");
    assert!(run.stderr.starts_with("error: "), "{}", run.stderr);
    assert!(
        run.stderr.contains("prunr.json not found"),
        "{}",
        run.stderr
    );
}

#[test]
fn a_file_that_does_not_parse_is_an_error() {
    let run = check("syntax-error");
    assert_eq!(run.code, 2);
    assert!(
        run.stderr.starts_with("error: index.js: "),
        "{}",
        run.stderr
    );
}

#[test]
fn an_unknown_command_is_a_usage_error() {
    for args in [&[][..], &["nope"], &["check", "--nope"]] {
        let run = prunr("clean", args);
        assert_eq!(run.code, 2, "{args:?}");
        assert_eq!(run.stderr, "usage: prunr check [--json]\n", "{args:?}");
    }
}

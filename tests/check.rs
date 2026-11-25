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
        "unused-file           lib/unused.js\n\nerrors: 1, warnings: 0\n"
    );
}

#[test]
fn a_destructured_require_uses_only_its_properties() {
    let run = check("destructuring");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-export         math.js: sub\n\nerrors: 1, warnings: 0\n"
    );
}

#[test]
fn a_namespace_require_uses_only_the_members_it_reads() {
    let run = check("namespace");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-export         math.js: sub\n\nerrors: 1, warnings: 0\n"
    );
}

#[test]
fn property_assignments_are_exports() {
    let run = check("exports-assignment");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-export         math.js: sub\n\nerrors: 1, warnings: 0\n"
    );
}

#[test]
fn a_property_of_a_default_export_is_an_export() {
    let run = check("default-export");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-export         greet.js: default.shout\n\nerrors: 1, warnings: 0\n"
    );
}

#[test]
fn an_unused_static_method_is_an_unused_export() {
    let run = check("class-static");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-export         invoice.js: Invoice.legacyTotal\n\
         unused-export         invoice.js: Invoice.unusedHelper\n\
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
        "unresolved-require    index.js: ./missing (warning)\n\
         unresolved-require    index.js: require(name) (warning)\n\
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
        "unused-dependency     jest\n\
         unused-dependency     left-pad\n\
         missing-dependency    index.js: axios\n\
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
        "circular-import       a.js -> b.js -> c.js -> a.js\n\nerrors: 1, warnings: 0\n"
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
        "unused-file           src/legacy-report.js\n\
         unused-export         src/invoices.js: cancelInvoice\n\
         unused-export         src/money.js: Money.fromCents\n\
         unused-export         src/money.js: Money.invoiceFor\n\
         unused-dependency     left-pad\n\
         missing-dependency    src/index.js: dotenv\n\
         circular-import       src/invoices.js -> src/money.js -> src/invoices.js\n\
         unresolved-require    src/index.js: require(process.env.PLUGIN) (warning)\n\
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
        assert_eq!(
            run.stderr,
            "usage: prunr check [--json] | prunr baseline | prunr init | prunr schema | prunr --version\n",
            "{args:?}"
        );
    }
}

#[test]
fn an_entry_that_is_not_a_source_file_is_an_error() {
    let run = check("entry-not-source");
    assert_eq!(run.code, 2);
    assert_eq!(
        run.stderr,
        "error: entry matches no source file: data.json\n"
    );
}

#[test]
fn what_only_tests_use_is_dead_in_production() {
    let run = check("test-only");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-file           lib/legacy.js (only tests use it)\n\
         unused-export         lib/math.js: mul\n\
         unused-export         lib/math.js: sub (only tests use it)\n\
         misplaced-dependency  supertest: only tests use it, move it to devDependencies\n\
         \n\
         errors: 4, warnings: 0\n"
    );
}

#[test]
fn json_marks_what_only_tests_use() {
    let run = prunr("test-only", &["check", "--json"]);
    let actual: serde_json::Value = serde_json::from_str(&run.stdout).unwrap();
    let expected = serde_json::json!({ "findings": [
        { "kind": "unused-file", "file": "lib/legacy.js", "testOnly": true },
        { "kind": "unused-export", "file": "lib/math.js", "symbol": "mul" },
        { "kind": "unused-export", "file": "lib/math.js", "symbol": "sub", "testOnly": true },
        { "kind": "misplaced-dependency", "package": "supertest" },
    ]});
    assert_eq!(actual, expected);
}

#[test]
fn ignored_dependencies_are_left_out_of_the_dependency_checks() {
    let run = check("dependencies-ignore");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-dependency     left-pad\n\nerrors: 1, warnings: 0\n"
    );
}

fn run_in(dir: &Path, args: &[&str]) -> Run {
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

#[test]
fn init_writes_a_config_that_check_can_run() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("package.json"), r#"{ "main": "index.js" }"#).unwrap();
    std::fs::write(dir.path().join("index.js"), "").unwrap();
    let run = run_in(dir.path(), &["init"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(run.stdout, "created prunr.json\n");
    assert_eq!(run_in(dir.path(), &["check"]).stdout, "No findings.\n");
    let again = run_in(dir.path(), &["init"]);
    assert_eq!(again.code, 2);
    assert_eq!(again.stderr, "error: prunr.json already exists\n");
}

#[test]
fn schema_prints_the_published_schema() {
    let run = prunr("clean", &["schema"]);
    assert_eq!(run.code, 0);
    let published =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("schema.json")).unwrap();
    assert_eq!(run.stdout, published);
}

#[test]
fn a_cycle_is_the_same_entry_whichever_file_reaches_it_first() {
    let run = check("circular-from-b");
    assert_eq!(
        run.stdout,
        "circular-import       a.js -> b.js -> c.js -> a.js\n\nerrors: 1, warnings: 0\n"
    );
}

#[test]
fn the_baseline_hides_known_findings_and_flags_fixed_ones() {
    let run = check("baseline");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-export         math.js: mul\n\
         stale-baseline        unused-export math.js: div (fixed: run `prunr baseline`)\n\
         \n\
         errors: 2, warnings: 0\n"
    );
}

#[test]
fn json_lists_stale_baseline_entries_apart() {
    let run = prunr("baseline", &["check", "--json"]);
    let actual: serde_json::Value = serde_json::from_str(&run.stdout).unwrap();
    let expected = serde_json::json!({
        "findings": [{ "kind": "unused-export", "file": "math.js", "symbol": "mul" }],
        "stale": [{ "kind": "unused-export", "file": "math.js", "symbol": "div" }],
    });
    assert_eq!(actual, expected);
}

/// A copy of a fixture, for commands that write to it.
fn copy(fixture: &str) -> tempfile::TempDir {
    let from = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(fixture);
    let dir = tempfile::tempdir().unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        std::fs::copy(entry.path(), dir.path().join(entry.file_name())).unwrap();
    }
    dir
}

#[test]
fn baseline_records_the_current_findings() {
    let dir = copy("baseline");
    let run = run_in(dir.path(), &["baseline"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(run.stdout, "wrote .prunr-baseline.json: 2 findings\n");
    let first = std::fs::read(dir.path().join(".prunr-baseline.json")).unwrap();
    assert_eq!(run_in(dir.path(), &["check"]).stdout, "No findings.\n");

    run_in(dir.path(), &["baseline"]);
    assert_eq!(
        std::fs::read(dir.path().join(".prunr-baseline.json")).unwrap(),
        first
    );

    let math = std::fs::read_to_string(dir.path().join("math.js")).unwrap();
    std::fs::write(dir.path().join("math.js"), format!("// moved\n\n{math}")).unwrap();
    let moved = run_in(dir.path(), &["check"]);
    assert_eq!(
        moved.code, 0,
        "moving code does not invalidate the baseline"
    );
}

#[test]
fn baseline_needs_a_baseline_file_in_the_config() {
    let run = prunr("clean", &["baseline"]);
    assert_eq!(run.code, 2);
    assert_eq!(
        run.stderr,
        "error: set baseline in prunr.json to use prunr baseline\n"
    );
}

#[test]
fn an_invalid_baseline_is_an_error() {
    let dir = copy("baseline");
    std::fs::write(dir.path().join(".prunr-baseline.json"), "{ nope").unwrap();
    let run = run_in(dir.path(), &["check"]);
    assert_eq!(run.code, 2);
    assert!(
        run.stderr
            .starts_with("error: invalid baseline .prunr-baseline.json: "),
        "{}",
        run.stderr
    );
}

#[test]
fn unused_locals_are_left_to_biome() {
    let run = check("unused-locals");
    assert_eq!(run.code, 0);
    assert_eq!(run.stdout, "No findings.\n");
}

#[test]
fn a_require_whose_binding_is_never_used_is_an_unused_import() {
    let run = check("unused-import");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-export         math.js: sub\n\
         unused-import         index.js: fs from node:fs\n\
         unused-import         index.js: lodash from lodash\n\
         unused-import         index.js: sub from ./math\n\
         \n\
         errors: 4, warnings: 0\n"
    );
}

#[test]
fn version_prints_the_crate_version() {
    let run = prunr("clean", &["--version"]);
    assert_eq!(run.code, 0);
    assert_eq!(run.stdout, format!("prunr {}\n", env!("CARGO_PKG_VERSION")));
}

#[test]
fn a_require_with_a_pattern_uses_every_file_it_can_reach() {
    let run = check("dynamic-require-pattern");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-file           handlers/deep/too-deep.js\n\
         unused-file           jobs/helper.js\n\
         unresolved-require    index.js: require(`${name}/plugin`) (warning)\n\
         unresolved-require    index.js: require(`./missing/${name}`) (warning)\n\
         \n\
         errors: 2, warnings: 2\n"
    );
}

#[test]
fn a_package_the_scripts_run_is_used() {
    let run = check("script-dependencies");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-dependency     left-pad\n\nerrors: 1, warnings: 0\n"
    );
}

#[test]
fn esm_and_commonjs_share_one_graph() {
    let run = check("esm");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-file           src/unused.js\n\
         unused-export         src/greet.js: whisper\n\
         unused-export         src/legacy-named.cjs: other\n\
         unused-export         src/legacy.cjs: stop\n\
         unused-export         src/math.mjs: sub\n\
         unused-export         src/math.mjs: unusedBinding\n\
         unused-import         src/index.js: unusedBinding from ./math.mjs\n\
         \n\
         errors: 7, warnings: 0\n"
    );
}

#[test]
fn a_typescript_project_is_read_like_the_compiler_resolves_it() {
    let run = check("typescript");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-export         src/format.ts: unusedFormat\n\
         unused-export         src/math.ts: sub\n\
         unused-export         src/types.ts: Unused\n\
         unused-export         src/ui/button.tsx: IconButton\n\
         unused-export         src/ui/card.tsx: Card\n\
         unused-dependency     @types/express\n\
         \n\
         errors: 6, warnings: 0\n"
    );
}

#[test]
fn a_reexport_forwards_uses_to_where_the_export_is_defined() {
    let run = check("reexports");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-export         src/greek.js: beta\n\
         unused-export         src/internal.js: other\n\
         unused-export         src/tools/hammer.js: nail\n\
         unused-export         src/tools/index.js: local\n\
         unused-export         src/tools/legacy.cjs: stop\n\
         unused-export         src/tools/saw.js: saw\n\
         \n\
         errors: 6, warnings: 0\n"
    );
}

#[test]
fn a_types_package_is_used_with_the_package_it_types() {
    let run = check("types-packages");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-dependency     @types/unused-lib\n\nerrors: 1, warnings: 0\n"
    );
}

#[test]
fn jest_finds_its_tests_and_what_its_config_loads() {
    let run = check("jest");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-export         src/math.js: sub (only tests use it)\n\
         misplaced-dependency  jest-junit: only tests use it, move it to devDependencies\n\
         \n\
         errors: 2, warnings: 0\n"
    );
}

#[test]
fn vitest_finds_its_tests_and_what_its_config_loads() {
    let run = check("vitest");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-file           src/legacy.spec.ts\n\nerrors: 1, warnings: 0\n"
    );
}

#[test]
fn a_comment_ignores_an_export_or_a_file_and_must_ignore_something() {
    let run = check("ignore-comments");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-export         src/legacy.cjs: gone\n\
         unused-export         src/lib.js: dead\n\
         unused-ignore         src/index.js:8: ignores nothing\n\
         unused-ignore         src/lib.js:12: ignores nothing\n\
         unused-ignore         src/lib.js:17: ignores nothing\n\
         unused-ignore         src/used-file.js:1: ignores nothing\n\
         \n\
         errors: 6, warnings: 0\n"
    );
}

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
fn a_file_that_does_not_parse_and_is_reached_is_a_warning() {
    let run = check("syntax-error");
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert!(
        run.stdout.starts_with("unparsed-file         index.js: "),
        "{}",
        run.stdout
    );
    assert!(
        run.stdout
            .ends_with("(warning)\n\nerrors: 0, warnings: 1\n"),
        "{}",
        run.stdout
    );
}

#[test]
fn jsx_in_js_files_top_level_return_and_unreached_broken_files_do_not_stop_the_check() {
    let run = check("unparsable");
    assert_eq!(run.code, 1, "{}", run.stderr);
    assert_eq!(
        run.stdout,
        "unused-file           fixtures/broken.js\n\nerrors: 1, warnings: 0\n"
    );
}

#[test]
fn an_unknown_command_is_a_usage_error() {
    for args in [
        &[][..],
        &["nope"],
        &["check", "--nope"],
        &["check", "--json", "--github"],
        &["check", "--since"],
        &["fix", "--nope"],
    ] {
        let run = prunr("clean", args);
        assert_eq!(run.code, 2, "{args:?}");
        assert_eq!(
            run.stderr,
            "usage: prunr check [--json | --github | --sarif | --codeclimate | --markdown] [--production] [--since <ref>] | prunr fix [--dry-run] | prunr why <file | file:export | package> | prunr baseline | prunr init | prunr schema | prunr --version\n",
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

#[test]
fn github_output_annotates_each_finding_where_it_is() {
    let run = prunr("example", &["check", "--github"]);
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "::error file=src/legacy-report.js,title=unused-file::src/legacy-report.js\n\
         ::error file=src/invoices.js,line=11,title=unused-export::src/invoices.js: cancelInvoice\n\
         ::error file=src/money.js,line=10,title=unused-export::src/money.js: Money.fromCents\n\
         ::error file=src/money.js,line=13,title=unused-export::src/money.js: Money.invoiceFor\n\
         ::error file=package.json,line=4,title=unused-dependency::left-pad\n\
         ::error file=src/index.js,line=2,title=missing-dependency::src/index.js: dotenv\n\
         ::error file=src/invoices.js,line=1,title=circular-import::src/invoices.js -> src/money.js -> src/invoices.js\n\
         ::warning file=src/index.js,line=9,title=unresolved-require::src/index.js: require(process.env.PLUGIN)\n\
         \n\
         errors: 7, warnings: 1\n"
    );
}

#[test]
fn github_output_annotates_stale_baseline_entries_and_the_rest_of_the_findings() {
    let run = prunr("baseline", &["check", "--github"]);
    assert_eq!(
        run.stdout,
        "::error file=math.js,line=3,title=unused-export::math.js: mul\n\
         ::error title=stale-baseline::unused-export math.js: div (fixed: run `prunr baseline`)\n\
         \n\
         errors: 2, warnings: 0\n"
    );
    let run = prunr("clean", &["check", "--github"]);
    assert_eq!(run.stdout, "No findings.\n");
}

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args([
            "-c",
            "user.name=prunr",
            "-c",
            "user.email=prunr@example.com",
        ])
        .args([
            "-c",
            "commit.gpgsign=false",
            "-c",
            "init.defaultBranch=main",
        ])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(status.status.success(), "{status:?}");
}

/// A git repository with the `clean` fixture in `app/`, committed.
fn repository() -> tempfile::TempDir {
    let repo = tempfile::tempdir().unwrap();
    let app = repo.path().join("app");
    std::fs::create_dir(&app).unwrap();
    let clean = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/clean");
    for entry in std::fs::read_dir(&clean).unwrap() {
        let entry = entry.unwrap();
        std::fs::copy(entry.path(), app.join(entry.file_name())).unwrap();
    }
    std::fs::write(repo.path().join(".gitignore"), "node_modules/\n").unwrap();
    let modules = app.join("node_modules/lodash");
    std::fs::create_dir_all(&modules).unwrap();
    std::fs::write(modules.join("package.json"), r#"{ "name": "lodash" }"#).unwrap();
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "clean"]);
    repo
}

#[test]
fn since_reports_only_what_is_new_since_a_commit() {
    let repo = repository();
    let app = repo.path().join("app");
    std::fs::write(app.join("legacy.js"), "exports.old = 1;\n").unwrap();
    git(repo.path(), &["add", "."]);
    git(
        repo.path(),
        &["commit", "-qm", "a dead file, already there"],
    );
    let run = run_in(&app, &["check", "--since", "HEAD"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(run.stdout, "No findings.\n");
    std::fs::write(app.join("other.js"), "exports.fresh = 1;\n").unwrap();
    let run = run_in(&app, &["check", "--since", "HEAD", "--json"]);
    assert_eq!(run.code, 1);
    let actual: serde_json::Value = serde_json::from_str(&run.stdout).unwrap();
    assert_eq!(
        actual,
        serde_json::json!({ "findings": [{ "kind": "unused-file", "file": "other.js" }] })
    );
    assert_eq!(
        run_in(&app, &["check"]).code,
        1,
        "without --since, both are reported"
    );
}

#[test]
fn since_finds_an_export_a_change_elsewhere_left_unused() {
    let repo = repository();
    let app = repo.path().join("app");
    let index = std::fs::read_to_string(app.join("index.js")).unwrap();
    assert!(index.contains("require("), "{index}");
    let before = run_in(&app, &["check", "--since", "HEAD"]);
    assert_eq!(before.stdout, "No findings.\n");
    std::fs::write(app.join("index.js"), "").unwrap();
    let run = run_in(&app, &["check", "--since", "HEAD"]);
    assert_eq!(run.code, 1);
    assert!(run.stdout.starts_with("unused-file"), "{}", run.stdout);
}

#[test]
fn since_needs_a_commit_git_knows() {
    let repo = repository();
    let run = run_in(&repo.path().join("app"), &["check", "--since", "nope"]);
    assert_eq!(run.code, 2);
    assert!(
        run.stderr.starts_with("error: cannot check out nope: "),
        "{}",
        run.stderr
    );
    let outside = copy("clean");
    let run = run_in(outside.path(), &["check", "--since", "HEAD"]);
    assert_eq!(run.code, 2);
    assert!(
        run.stderr
            .starts_with("error: --since needs a git repository: "),
        "{}",
        run.stderr
    );
}

#[test]
fn since_a_commit_without_the_project_reports_everything() {
    let repo = tempfile::tempdir().unwrap();
    std::fs::write(repo.path().join("README.md"), "empty\n").unwrap();
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "before the project"]);
    let app = repo.path().join("app");
    let example = copy("destructuring");
    std::fs::create_dir(&app).unwrap();
    for entry in std::fs::read_dir(example.path()).unwrap() {
        let entry = entry.unwrap();
        if entry.path().is_file() {
            std::fs::copy(entry.path(), app.join(entry.file_name())).unwrap();
        }
    }
    let run = run_in(&app, &["check", "--since", "HEAD"]);
    assert_eq!(run.code, 1);
    assert_eq!(run.stdout, run_in(&app, &["check"]).stdout);
}

#[test]
fn since_without_git_is_an_error() {
    let dir = copy("clean");
    let output = Command::new(env!("CARGO_BIN_EXE_prunr"))
        .args(["check", "--since", "HEAD"])
        .current_dir(dir.path())
        .env("PATH", "")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.starts_with("error: --since needs a git repository: "),
        "{stderr}"
    );
}

#[test]
fn github_output_annotates_an_ignore_comment_at_its_line() {
    let run = prunr("ignore-comments", &["check", "--github"]);
    assert!(
        run.stdout.contains(
            "::error file=src/lib.js,line=12,title=unused-ignore::src/lib.js:12: ignores nothing\n"
        ),
        "{}",
        run.stdout
    );
}

#[test]
fn baseline_without_a_config_is_an_error() {
    let run = prunr("missing-config", &["baseline"]);
    assert_eq!(run.code, 2);
    assert!(
        run.stderr.contains("prunr.json not found"),
        "{}",
        run.stderr
    );
}

/// A copy of a fixture with its subdirectories.
fn copy_tree(fixture: &str) -> tempfile::TempDir {
    fn copy_dir(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).unwrap();
        for entry in std::fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            let target = to.join(entry.file_name());
            if entry.path().is_dir() {
                copy_dir(&entry.path(), &target);
            } else {
                std::fs::copy(entry.path(), target).unwrap();
            }
        }
    }
    let dir = tempfile::tempdir().unwrap();
    copy_dir(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(fixture),
        dir.path(),
    );
    dir
}

fn read(dir: &Path, file: &str) -> String {
    std::fs::read_to_string(dir.join(file)).unwrap()
}

#[test]
fn fix_removes_what_check_reports_until_nothing_more_can_go() {
    let dir = copy_tree("fix");
    let run = run_in(dir.path(), &["fix"]);
    assert_eq!(run.code, 1, "{}", run.stderr);
    assert_eq!(
        run.stdout,
        "fixed unused-file           src/dead.js\n\
         fixed unused-file           src/only-dead.js\n\
         fixed unused-export         src/greet.mjs: local\n\
         fixed unused-export         src/greet.mjs: unusedConst\n\
         fixed unused-export         src/legacy.js: default\n\
         fixed unused-export         src/math.js: sub\n\
         fixed unused-import         src/greet.mjs: writeFile from node:fs/promises\n\
         fixed unused-import         src/index.js: fs from fs\n\
         fixed unused-import         src/index.js: legacy from ./legacy\n\
         fixed unused-import         src/index.js: unusedHelper from ./math\n\
         fixed unused-ignore         src/greet.mjs:9: ignores nothing\n\
         fixed unused-dependency     left-pad\n\
         fixed misplaced-dependency  supertest: only tests use it, move it to devDependencies\n\
         fixed unused-file           src/legacy.js\n\
         \n\
         unused-export         src/math.js: mul (only tests use it)\n\
         \n\
         errors: 1, warnings: 0\n"
    );
    let root = dir.path();
    assert!(!root.join("src/dead.js").exists());
    assert!(!root.join("src/only-dead.js").exists());
    assert!(
        !root.join("src/legacy.js").exists(),
        "the second round removes it"
    );
    assert_eq!(
        read(root, "src/index.js"),
        "const { add } = require(\"./math\");\n\
         const _ = require(\"lodash\");\n\
         const { greet, stillUsed } = require(\"./greet.mjs\");\n\
         \n\
         console.log(add(1, 2), _.chunk, greet(), stillUsed);\n"
    );
    assert_eq!(
        read(root, "src/math.js"),
        "exports.add = (a, b) => a + b;\nexports.mul = (a, b) => a * b;\n"
    );
    assert_eq!(
        read(root, "src/greet.mjs"),
        "import { readFile } from \"node:fs/promises\";\n\
         \n\
         export function greet() {\n  return readFile;\n}\n\
         \n\
         export const stillUsed = 2;\n"
    );
    assert_eq!(
        read(root, "package.json"),
        "{\n  \"name\": \"fix\",\n  \"private\": true,\n  \"dependencies\": {\n    \"lodash\": \"^4.17.0\"\n  },\n  \"devDependencies\": {\n    \"jest\": \"^29.0.0\",\n    \"supertest\": \"^7.0.0\"\n  },\n  \"scripts\": { \"test\": \"jest\" }\n}\n"
    );
    let again = run_in(root, &["fix"]);
    assert_eq!(
        again.stdout,
        "unused-export         src/math.js: mul (only tests use it)\n\nerrors: 1, warnings: 0\n",
        "a second run has nothing left to fix"
    );
}

#[test]
fn fix_dry_run_says_what_it_would_do_and_changes_nothing() {
    let dir = copy_tree("fix");
    let before = read(dir.path(), "src/index.js");
    let run = run_in(dir.path(), &["fix", "--dry-run"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert!(
        run.stdout
            .starts_with("would fix unused-file           src/dead.js\n"),
        "{}",
        run.stdout
    );
    assert!(
        !run.stdout.contains("unused-file           src/legacy.js"),
        "the dry run stops at the first round: {}",
        run.stdout
    );
    assert_eq!(read(dir.path(), "src/index.js"), before);
    assert!(dir.path().join("src/dead.js").exists());
}

#[test]
fn fix_leaves_baseline_findings_alone() {
    let dir = copy("baseline");
    let math = read(dir.path(), "math.js");
    let run = run_in(dir.path(), &["fix"]);
    assert!(
        run.stdout
            .starts_with("fixed unused-export         math.js: mul\n"),
        "{}",
        run.stdout
    );
    assert!(read(dir.path(), "math.js").contains("exports.sub"));
    assert_ne!(read(dir.path(), "math.js"), math);
}

#[test]
fn fix_leaves_what_it_cannot_remove_safely() {
    let dir = copy_tree("fix-partial");
    let run = run_in(dir.path(), &["fix"]);
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "fixed unused-export         lib.js: b\n\
         fixed unused-export         lib.js: c\n\
         fixed unused-export         lib.js: onlyUnused\n\
         fixed unused-import         index.js: b from ./lib.js\n\
         fixed unused-import         index.js: c from ./lib.js\n\
         fixed unused-import         index.js: onlyUnused from ./lib.js\n\
         \n\
         unused-export         anon.js: default\n\
         unused-export         lib.js: shared\n\
         unused-import         cjs.cjs: z from ./other.js\n\
         \n\
         errors: 3, warnings: 0\n",
        "an export keyword shared with a used name, a default with side effects and a require \
         in a declaration of several names stay"
    );
    assert_eq!(
        read(dir.path(), "index.js"),
        "import def, { a } from \"./lib.js\";\n\
         import whole from \"./lib.js\";\n\
         import * as ns from \"./other.js\";\n\
         import \"./anon.js\";\n\
         import \"./cjs.cjs\";\n\
         \n\
         console.log(def, a, whole, ns.x);\n"
    );
    assert_eq!(
        read(dir.path(), "lib.js"),
        "export const a = 1, shared = 2;\nexport default 42;\n"
    );
}

#[test]
fn fix_without_a_config_is_an_error() {
    let run = prunr("missing-config", &["fix"]);
    assert_eq!(run.code, 2);
    assert!(
        run.stderr.contains("prunr.json not found"),
        "{}",
        run.stderr
    );
}

#[cfg(unix)]
#[test]
fn fix_says_which_file_it_could_not_change() {
    use std::os::unix::fs::PermissionsExt;
    let dir = copy_tree("require-static");
    let lib = dir.path().join("lib");
    std::fs::set_permissions(&lib, std::fs::Permissions::from_mode(0o555)).unwrap();
    let run = run_in(dir.path(), &["fix"]);
    std::fs::set_permissions(&lib, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(run.code, 2);
    assert!(
        run.stderr.starts_with("error: ") && run.stderr.contains("lib/unused.js: "),
        "{}",
        run.stderr
    );
}

#[test]
fn a_jsdoc_import_type_uses_its_module() {
    let run = check("jsdoc-imports");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-file           lib/not-jsdoc.js\n\
         unused-export         lib/types.js: Draft\n\
         \n\
         errors: 2, warnings: 0\n"
    );
}

#[test]
fn fix_deletes_the_declaration_an_export_leaves_unused() {
    let dir = copy_tree("fix-declarations");
    let run = run_in(dir.path(), &["fix"]);
    assert_eq!(run.code, 1, "{}{}", run.stdout, run.stderr);
    assert!(
        run.stdout
            .ends_with("\nunused-export         esm.js: notUsed\n\nerrors: 1, warnings: 0\n"),
        "an export keyword shared with a used name stays: {}",
        run.stdout
    );
    assert_eq!(
        read(dir.path(), "esm.js"),
        "/**\n * The header of the file stays.\n */\n\
         \n\
         export function used() {\n  return 1;\n}\n\
         \n\
         const sideEffect = Math.random();\n\
         \n\
         export const usedToo = 5, notUsed = 6;\n"
    );
    assert_eq!(
        read(dir.path(), "cjs.cjs"),
        "const kept = 1;\n\
         const also = kept;\n\
         \n\
         const computed = Date.now();\n\
         \n\
         module.exports = { kept, also };\n"
    );
}

#[test]
fn next_routes_are_entries_by_convention() {
    let run = check("nextjs");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-file           components/Dead.tsx\n\
         unused-export         components/Button.tsx: unusedVariant\n\
         \n\
         errors: 2, warnings: 0\n"
    );
}

#[test]
fn nest_starts_at_its_main_and_tests_end_to_end() {
    let run = check("nestjs");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-file           src/orphan.service.ts\n\nerrors: 1, warnings: 0\n"
    );
}

const WORKSPACES_FINDINGS: &str = "unused-file           packages/lib/internal.js\n\
     unused-export         packages/shared/src/index.js: unused\n\
     unused-dependency     left-pad (packages/shared)\n\
     misplaced-dependency  supertest (packages/shared): only tests use it, move it to devDependencies\n\
     missing-dependency    packages/app/src/index.js: lodash\n\
     \n\
     errors: 5, warnings: 0\n";

#[test]
fn npm_workspaces_are_one_graph_with_dependencies_per_package() {
    let run = check("workspaces-npm");
    assert_eq!(run.code, 1);
    assert_eq!(run.stdout, WORKSPACES_FINDINGS);
}

#[test]
fn pnpm_workspaces_are_one_graph_with_dependencies_per_package() {
    let run = check("workspaces-pnpm");
    assert_eq!(run.code, 1);
    assert_eq!(run.stdout, WORKSPACES_FINDINGS);
}

#[test]
fn json_names_the_package_of_a_dependency_finding() {
    let run = prunr("workspaces-npm", &["check", "--json"]);
    let actual: serde_json::Value = serde_json::from_str(&run.stdout).unwrap();
    assert!(
        actual["findings"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!({
                "kind": "unused-dependency",
                "package": "left-pad",
                "workspace": "packages/shared"
            })),
        "{actual}"
    );
}

#[test]
fn github_annotates_a_dependency_of_a_package_in_its_package_json() {
    let run = prunr("workspaces-npm", &["check", "--github"]);
    assert!(
        run.stdout.contains(
            "::error file=packages/shared/package.json,line=7,title=unused-dependency::left-pad (packages/shared)\n"
        ),
        "{}",
        run.stdout
    );
}

#[test]
fn fix_edits_the_package_json_of_the_package() {
    let dir = copy_tree("workspaces-npm");
    let run = run_in(dir.path(), &["fix"]);
    assert_eq!(run.code, 1, "{}", run.stderr);
    assert!(
        run.stdout
            .contains("fixed unused-dependency     left-pad (packages/shared)\n"),
        "{}",
        run.stdout
    );
    let shared = read(dir.path(), "packages/shared/package.json");
    assert!(
        !shared.contains("left-pad") && shared.contains("lodash"),
        "{shared}"
    );
}

#[test]
fn an_invalid_package_json_of_a_package_is_an_error_that_names_it() {
    let dir = copy_tree("workspaces-npm");
    std::fs::write(dir.path().join("packages/lib/package.json"), "{ nope").unwrap();
    let run = run_in(dir.path(), &["check"]);
    assert_eq!(run.code, 2);
    assert!(
        run.stderr
            .starts_with("error: invalid package.json: packages/lib/package.json: "),
        "{}",
        run.stderr
    );
}

#[test]
fn vite_starts_at_the_scripts_of_its_index_html() {
    let run = check("vite-react");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-file           src/components/Dead.tsx\n\
         unused-export         src/components/Button.tsx: unusedSize\n\
         unresolved-require    src/components/Button.tsx: /gone.svg (warning)\n\
         \n\
         errors: 2, warnings: 1\n"
    );
}

#[test]
fn the_files_scripts_run_are_entries() {
    let run = check("script-entries");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-file           input.js\n\
         unused-file           packages/tools/src/dead.ts\n\
         unused-file           src/dead.js\n\
         unused-file           src/never.js\n\
         unused-file           src/only-e2e.js (only tests use it)\n\
         \n\
         errors: 5, warnings: 0\n"
    );
}

#[test]
fn playwright_runs_its_test_dir_setup_reporters_and_web_servers() {
    let run = check("playwright");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-file           src/dead.js\n\nerrors: 1, warnings: 0\n"
    );
}

#[test]
fn drizzle_kit_loads_its_config_and_schema() {
    let run = check("drizzle");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-file           src/db/dead.ts\n\nerrors: 1, warnings: 0\n"
    );
}

#[test]
fn postcss_loads_the_plugins_its_config_names() {
    let run = check("postcss");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-dependency     unused-lib\n\nerrors: 1, warnings: 0\n"
    );
}

#[test]
fn vitest_reads_the_config_of_each_project() {
    let run = check("vitest-projects");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-file           apps/web/src/only-tests.js (only tests use it)\n\
         \n\
         errors: 1, warnings: 0\n"
    );
}

#[test]
fn a_package_another_used_package_needs_as_a_peer_is_used() {
    let run = check("peer-dependencies");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-dependency     orphan-peer\n\
         unused-dependency     unused-host\n\
         misplaced-dependency  test-helper-peer: only tests use it, move it to devDependencies\n\
         \n\
         errors: 3, warnings: 0\n"
    );
}

#[test]
fn the_stylesheets_a_file_imports_use_packages_too() {
    let run = check("css-imports");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-dependency     unused-lib\n\nerrors: 1, warnings: 0\n"
    );
}

#[test]
fn the_output_is_the_same_with_any_number_of_threads() {
    for fixture in ["example", "workspaces-pnpm", "script-entries", "vite-react"] {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(fixture);
        let one = Command::new(env!("CARGO_BIN_EXE_prunr"))
            .arg("check")
            .env("RAYON_NUM_THREADS", "1")
            .current_dir(dir)
            .output()
            .unwrap();
        assert_eq!(
            String::from_utf8(one.stdout).unwrap(),
            check(fixture).stdout,
            "{fixture}"
        );
    }
}

#[test]
fn a_package_any_package_json_above_the_file_declares_is_declared() {
    let run = check("nested-packages");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "missing-dependency    tests/smoke/cjs/a.test.js: chai\n\nerrors: 1, warnings: 0\n"
    );
}

#[test]
fn why_shows_the_chain_from_an_entry_to_a_file() {
    let run = prunr("example", &["why", "src/money.js"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(
        run.stdout,
        "src/money.js is used in production:\n\
         \x20 src/index.js (entry)\n\
         \x20 -> src/money.js (src/index.js:4)\n"
    );
    let run = prunr("example", &["why", "src/legacy-report.js"]);
    assert_eq!(
        run.stdout,
        "src/legacy-report.js is not reached by any entry, and no file imports it.\n"
    );
}

#[test]
fn why_lists_the_files_that_use_an_export() {
    let run = prunr("example", &["why", "src/money.js:Money.format"]);
    assert_eq!(
        run.stdout,
        "src/money.js: Money.format is used by:\n\
         \x20 src/invoices.js:1 (production)\n"
    );
    let run = prunr("example", &["why", "src/invoices.js:cancelInvoice"]);
    assert_eq!(
        run.stdout,
        "src/invoices.js: cancelInvoice is used by no file.\n"
    );
}

#[test]
fn why_lists_what_uses_a_package() {
    let run = prunr("example", &["why", "express"]);
    assert_eq!(
        run.stdout,
        "express is used by:\n\
         \x20 src/index.js:1 (production)\n"
    );
    let run = prunr("example", &["why", "left-pad"]);
    assert_eq!(run.stdout, "left-pad is used by no file, script or tool.\n");
    let empty = tempfile::tempdir().unwrap();
    let run = run_in(empty.path(), &["why", "x"]);
    assert_eq!(run.code, 2);
    assert!(run.stderr.contains("prunr.json"), "{}", run.stderr);
    let run = prunr("example", &["why", "nope"]);
    assert_eq!(run.code, 2);
    assert_eq!(
        run.stderr,
        "error: nope is not a file of the project nor a package it declares or imports\n"
    );
}

#[test]
fn why_explains_test_files_dead_importers_and_what_needs_a_package() {
    let dir = tempfile::tempdir().unwrap();
    for (path, content) in [
        (
            "package.json",
            r#"{
                "workspaces": ["web"],
                "scripts": { "lint": "eslint ." },
                "devDependencies": { "eslint": "9", "jest": "29", "ts-jest": "29" },
                "jest": { "preset": "ts-jest" }
            }"#,
        ),
        (
            "prunr.json",
            r#"{ "entries": { "production": ["index.js"], "test": ["a.test.js"] } }"#,
        ),
        (
            "web/package.json",
            r#"{ "dependencies": { "next": "15", "react": "19" } }"#,
        ),
        (
            "node_modules/next/package.json",
            r#"{ "peerDependencies": { "react": "19" } }"#,
        ),
        ("web/index.js", "require('react');"),
        ("index.js", "require('undeclared');"),
        ("a.test.js", "require('./helper').x();"),
        ("helper.js", "exports.x = () => {};"),
        ("dead.js", "require('./orphan');"),
        ("orphan.js", ""),
    ] {
        let path = dir.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }
    let why = |target: &str| run_in(dir.path(), &["why", target]).stdout;
    assert_eq!(
        why("helper.js"),
        "helper.js is used only by tests:\n  a.test.js (test entry)\n  -> helper.js (a.test.js:1)\n"
    );
    assert_eq!(
        why("helper.js:x"),
        "helper.js: x is used by:\n  a.test.js:1 (test)\n"
    );
    assert_eq!(
        why("orphan.js"),
        "orphan.js is not reached by any entry. Imported by:\n  dead.js:1 (unreached)\n"
    );
    assert_eq!(
        why("eslint"),
        "eslint is used by:\n  a script of package.json\n"
    );
    assert_eq!(
        why("ts-jest"),
        "ts-jest is used by:\n  a tool configured in the project (test)\n"
    );
    assert_eq!(
        why("react"),
        "react is used by:\n  web/index.js:1 (production)\n  a framework of web (production)\n  a peer dependency of next\n"
    );
    assert_eq!(
        why("undeclared"),
        "undeclared is used by:\n  index.js:1 (production)\n"
    );
}

#[test]
fn sarif_reports_each_finding_as_a_result_of_its_rule() {
    let run = prunr("example", &["check", "--sarif"]);
    assert_eq!(run.code, 1);
    let sarif: serde_json::Value = serde_json::from_str(&run.stdout).unwrap();
    assert_eq!(sarif["version"], "2.1.0");
    let run0 = &sarif["runs"][0];
    assert_eq!(run0["tool"]["driver"]["name"], "prunr");
    let rules: Vec<&str> = run0["tool"]["driver"]["rules"]
        .as_array()
        .unwrap()
        .iter()
        .map(|rule| rule["id"].as_str().unwrap())
        .collect();
    assert!(rules.contains(&"unused-export") && rules.contains(&"unresolved-require"));
    let results = run0["results"].as_array().unwrap();
    assert_eq!(results.len(), 8);
    let export = results
        .iter()
        .find(|result| result["ruleId"] == "unused-export")
        .unwrap();
    assert_eq!(export["level"], "error");
    assert_eq!(export["message"]["text"], "src/invoices.js: cancelInvoice");
    let location = &export["locations"][0]["physicalLocation"];
    assert_eq!(location["artifactLocation"]["uri"], "src/invoices.js");
    assert_eq!(location["region"]["startLine"], 11);
    let warning = results
        .iter()
        .find(|result| result["ruleId"] == "unresolved-require")
        .unwrap();
    assert_eq!(warning["level"], "warning");
}

#[test]
fn codeclimate_reports_gitlab_code_quality_issues() {
    let run = prunr("example", &["check", "--codeclimate"]);
    assert_eq!(run.code, 1);
    let issues: serde_json::Value = serde_json::from_str(&run.stdout).unwrap();
    let issues = issues.as_array().unwrap();
    assert_eq!(issues.len(), 8);
    let dependency = issues
        .iter()
        .find(|issue| issue["check_name"] == "unused-dependency")
        .unwrap();
    assert_eq!(dependency["description"], "left-pad");
    assert_eq!(dependency["severity"], "major");
    assert_eq!(dependency["location"]["path"], "package.json");
    assert_eq!(dependency["location"]["lines"]["begin"], 4);
    let fingerprints: std::collections::BTreeSet<&str> = issues
        .iter()
        .map(|issue| issue["fingerprint"].as_str().unwrap())
        .collect();
    assert_eq!(fingerprints.len(), 8, "fingerprints are unique");
    let warning = issues
        .iter()
        .find(|issue| issue["check_name"] == "unresolved-require")
        .unwrap();
    assert_eq!(warning["severity"], "minor");
}

#[test]
fn markdown_reports_a_table_for_a_pull_request_comment() {
    let run = prunr("example", &["check", "--markdown"]);
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "### prunr: 7 errors, 1 warning\n\
         \n\
         | Finding | Where | What |\n\
         |---|---|---|\n\
         | `unused-file` | `src/legacy-report.js` | src/legacy-report.js |\n\
         | `unused-export` | `src/invoices.js:11` | src/invoices.js: cancelInvoice |\n\
         | `unused-export` | `src/money.js:10` | src/money.js: Money.fromCents |\n\
         | `unused-export` | `src/money.js:13` | src/money.js: Money.invoiceFor |\n\
         | `unused-dependency` | `package.json:4` | left-pad |\n\
         | `missing-dependency` | `src/index.js:2` | src/index.js: dotenv |\n\
         | `circular-import` | `src/invoices.js:1` | src/invoices.js -> src/money.js -> src/invoices.js |\n\
         | `unresolved-require` (warning) | `src/index.js:9` | src/index.js: require(process.env.PLUGIN) |\n"
    );
    let clean = prunr("clean", &["check", "--markdown"]);
    assert_eq!(clean.stdout, "### prunr: no findings\n");
}

#[test]
fn the_structured_reports_list_stale_baseline_entries_too() {
    let run = prunr("baseline", &["check", "--markdown"]);
    assert_eq!(
        run.stdout,
        "### prunr: 2 errors, 0 warnings\n\
         \n\
         | Finding | Where | What |\n\
         |---|---|---|\n\
         | `unused-export` | `math.js:3` | math.js: mul |\n\
         | `stale-baseline` | `prunr.json` | unused-export math.js: div (fixed: run `prunr baseline`) |\n"
    );
    let sarif: serde_json::Value =
        serde_json::from_str(&prunr("baseline", &["check", "--sarif"]).stdout).unwrap();
    let stale = &sarif["runs"][0]["results"][1];
    assert_eq!(stale["ruleId"], "stale-baseline");
    assert!(stale["locations"][0]["physicalLocation"]["region"].is_null());
    let issues: serde_json::Value =
        serde_json::from_str(&prunr("baseline", &["check", "--codeclimate"]).stdout).unwrap();
    assert_eq!(issues[1]["location"]["lines"]["begin"], 1);
}

#[test]
fn production_mode_reports_only_what_goes_to_production() {
    let run = check("production-mode");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-file           src/dead.js\n\
         unused-import         test/a.test.js: helper from ./helper\n\
         unused-dependency     left-pad\n\
         unused-dependency     lodash\n\
         missing-dependency    test/a.test.js: undeclared-test-lib\n\
         circular-import       test/cycle-a.js -> test/cycle-b.js -> test/cycle-a.js\n\
         \n\
         errors: 6, warnings: 0\n"
    );
    let run = prunr("production-mode", &["check", "--production"]);
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-file           src/dead.js\n\
         unused-dependency     lodash\n\
         \n\
         errors: 2, warnings: 0\n"
    );
}

#[test]
fn the_root_package_can_import_the_packages_of_the_repository_without_declaring_them() {
    let run = check("workspace-root-imports");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "missing-dependency    packages/app/src/index.js: @x/lib\n\nerrors: 1, warnings: 0\n"
    );
}

#[test]
fn mocha_runs_the_specs_its_config_and_scripts_name() {
    let run = check("mocha");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-file           lib/dead.js\n\nerrors: 1, warnings: 0\n"
    );
}

#[test]
fn node_test_tap_and_lab_run_their_default_test_files() {
    let run = check("node-test");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-file           src/only-tests.js (only tests use it)\n\nerrors: 1, warnings: 0\n"
    );
}

#[test]
fn declaration_files_non_packages_vendored_packages_and_test_fixtures_are_not_findings() {
    let run = check("test-data");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-file           client/src/index.ts (only tests use it)\n\
         unused-file           src/dead.js\n\
         \n\
         errors: 2, warnings: 0\n"
    );
}

#[test]
fn jest_mocks_and_module_name_mapper_resolve_their_names() {
    let run = check("aliases/jest");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-file           src/lib/math.js (only tests use it)\n\nerrors: 1, warnings: 0\n"
    );
}

#[test]
fn vite_resolve_alias_resolves_its_names() {
    let run = check("aliases/vite");
    assert_eq!(run.code, 0, "{}", run.stdout);
    assert_eq!(run.stdout, "No findings.\n");
}

#[test]
fn the_tsconfig_uses_the_packages_it_extends_types_and_plugins_and_runs_type_tests() {
    let run = check("typescript-config");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-dependency     unused-dev\n\nerrors: 1, warnings: 0\n"
    );
}

#[test]
fn the_scripts_of_vue_svelte_astro_and_mdx_files_are_part_of_the_graph() {
    let run = check("single-file-components");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-file           src/Orphan.vue\nunused-export         src/util.ts: unused\n\nerrors: 2, warnings: 0\n"
    );
}

#[test]
fn the_packages_eslint_prettier_babel_and_commitlint_configs_name_are_used() {
    let run = check("tool-configs");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-dependency     unused-dev\n\nerrors: 1, warnings: 0\n"
    );
}

#[test]
fn a_package_compiles_only_the_files_its_own_plugins_compile() {
    let run = check("compilers-per-package");
    assert_eq!(run.code, 0, "{}", run.stdout);
    assert_eq!(run.stdout, "No findings.\n");
}

#[test]
fn sveltekit_routes_hooks_and_aliases_are_part_of_the_graph() {
    let run = check("sveltekit");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-file           src/lib/orphan.js\nunused-export         src/lib/format.js: unused\n\nerrors: 2, warnings: 0\n"
    );
}

#[test]
fn nuxt_pages_components_composables_and_aliases_are_part_of_the_graph() {
    let run = check("nuxt");
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "unused-file           orphan.ts\n\nerrors: 1, warnings: 0\n"
    );
}

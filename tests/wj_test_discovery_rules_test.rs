//! `wj test --list` discovery rules (black box, no cargo): one test per sharp edge.
//!
//! Rule: every `.wj` under the package root (minus hidden/target/build/node_modules) is a
//! candidate; tests are `@test`/`test_*` functions at top level, in inline `mod`s, or as
//! receiver-less `impl` functions. Discovery never silently finds nothing and never lets one bad
//! file hide the others.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;

fn write(root: &Path, rel: &str, body: &str) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, body).unwrap();
}

fn wj_test(root: &Path, args: &[&str]) -> (bool, String) {
    let out: Output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .current_dir(root)
        .arg("test")
        .args(args)
        .output()
        .expect("run wj test");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.success(), text)
}

fn listing(root: &Path) -> String {
    let (ok, text) = wj_test(root, &["--list"]);
    assert!(ok, "--list failed:\n{text}");
    text
}

#[test]
fn at_test_in_ordinary_src_file_is_discovered() {
    let tmp = TempDir::new().unwrap();
    write(tmp.path(), "src/math.wj", "@test\nfn adds_two() {}\nfn helper() {}\n");
    let text = listing(tmp.path());
    assert!(text.contains("math.wj") && text.contains("adds_two"), "{text}");
    assert!(!text.contains("helper"), "{text}");
}

#[test]
fn test_prefixed_function_in_ordinary_file_is_discovered() {
    let tmp = TempDir::new().unwrap();
    write(tmp.path(), "src/calc.wj", "fn test_sum() {}\n");
    assert!(listing(tmp.path()).contains("test_sum"));
}

#[test]
fn files_named_tests_suffix_and_test_prefix_are_discovered() {
    let tmp = TempDir::new().unwrap();
    write(tmp.path(), "src/parse_tests.wj", "fn test_a() {}\n");
    write(tmp.path(), "src/test_lexer.wj", "fn test_b() {}\n");
    let text = listing(tmp.path());
    assert!(text.contains("parse_tests.wj") && text.contains("test_lexer.wj"), "{text}");
}

#[test]
fn tests_inside_inline_mod_are_discovered() {
    let tmp = TempDir::new().unwrap();
    write(tmp.path(), "src/lib_a.wj", "mod checks {\n    @test\n    fn inner_case() {}\n}\n");
    assert!(listing(tmp.path()).contains("checks::inner_case"));
}

#[test]
fn receiverless_impl_tests_are_discovered() {
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        "src/suite.wj",
        "struct Suite {}\nimpl Suite {\n    @test\n    fn case_one() {}\n}\n",
    );
    assert!(listing(tmp.path()).contains("Suite::case_one"));
}

#[test]
fn same_file_stem_in_two_directories_gets_distinct_modules() {
    let tmp = TempDir::new().unwrap();
    write(tmp.path(), "a/util_test.wj", "fn test_a() {}\n");
    write(tmp.path(), "b/util_test.wj", "fn test_b() {}\n");
    let text = listing(tmp.path());
    assert!(text.contains("module a__util_test"), "{text}");
    assert!(text.contains("module b__util_test"), "{text}");
}

#[test]
fn build_target_and_hidden_directories_are_skipped() {
    let tmp = TempDir::new().unwrap();
    write(tmp.path(), "src/real.wj", "fn test_real() {}\n");
    for dir in ["build", "target", ".cache", "node_modules"] {
        write(tmp.path(), &format!("{dir}/gen_test.wj"), "fn test_generated() {}\n");
    }
    let text = listing(tmp.path());
    assert!(text.contains("test_real") && !text.contains("test_generated"), "{text}");
}

#[test]
fn one_unparsable_file_is_reported_without_hiding_the_rest() {
    let tmp = TempDir::new().unwrap();
    write(tmp.path(), "src/ok_test.wj", "fn test_ok() {}\n");
    write(tmp.path(), "src/bad_test.wj", "fn test_bad( {{{\n");
    write(tmp.path(), "src/worse_test.wj", "struct {{{\n");
    let (ok, text) = wj_test(tmp.path(), &["--list"]);
    assert!(!ok, "unparsable test files must fail the run:\n{text}");
    assert!(text.contains("2 file(s)"), "aggregated count missing:\n{text}");
    assert!(text.contains("bad_test.wj") && text.contains("worse_test.wj"), "{text}");
}

#[test]
fn zero_tests_is_an_error_unless_allow_empty() {
    let tmp = TempDir::new().unwrap();
    write(tmp.path(), "src/plain.wj", "fn helper() {}\n");
    let (ok, text) = wj_test(tmp.path(), &[]);
    assert!(!ok, "0 tests must not exit 0:\n{text}");
    assert!(text.contains("@test") && text.contains("--allow-empty"), "{text}");
    let (ok, text) = wj_test(tmp.path(), &["--allow-empty"]);
    assert!(ok, "--allow-empty must exit 0:\n{text}");
}

#[test]
fn files_present_but_no_test_functions_is_an_error() {
    let tmp = TempDir::new().unwrap();
    write(tmp.path(), "tests/helpers_test.wj", "fn helper() {}\n");
    let (ok, text) = wj_test(tmp.path(), &[]);
    assert!(!ok, "{text}");
}

#[test]
fn filter_matching_nothing_is_an_error_not_a_green_run() {
    let tmp = TempDir::new().unwrap();
    write(tmp.path(), "src/a_test.wj", "fn test_alpha() {}\n");
    let (ok, text) = wj_test(tmp.path(), &["--filter", "no_such_test"]);
    assert!(!ok, "{text}");
    assert!(text.contains("matched none"), "{text}");
}

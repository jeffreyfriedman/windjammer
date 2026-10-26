#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "interpreter_tests",
))]

//! Gate: `wj test --interpret` runs exactly the tests `wj test` discovers (one shared discovery
//! implementation), over a fresh `wj new --template lib` project and a project exercising every
//! discovery shape (top-level, `_test.wj` files, inline `mod`, receiver-less `impl` functions,
//! `@test_cases`).

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn wj(dir: &Path, args: &[&str]) -> (String, String, i32) {
    let o = Command::new(WJ).args(args).current_dir(dir).output().unwrap();
    (
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
        o.status.code().unwrap_or(-1),
    )
}

/// Test names from `wj test --list` (indented lines under each file header).
fn listed(dir: &Path) -> BTreeSet<String> {
    let (out, err, code) = wj(dir, &["test", "--list"]);
    assert_eq!(code, 0, "list failed: {err}");
    out.lines().filter(|l| l.starts_with("    ")).map(|l| l.trim().to_string()).collect()
}

/// Test names from the `--interpret` report (`test <file>::<name> ... ok`).
fn interpreted(dir: &Path, extra: &[&str]) -> (BTreeSet<String>, i32, String) {
    let mut args = vec!["test", "--interpret"];
    args.extend_from_slice(extra);
    let (out, err, code) = wj(dir, &args);
    let names = out
        .lines()
        .filter_map(|l| l.strip_prefix("test "))
        .filter_map(|l| l.rsplit_once(" ... ").map(|(n, _)| n.to_string()))
        .map(|n| match n.split_once(".wj::") {
            Some((_, name)) => name.to_string(),
            None => n,
        })
        .collect();
    (names, code, format!("{out}{err}"))
}

#[test]
fn lib_template_tests_are_found_and_pass_in_both_modes() {
    let tmp = TempDir::new().unwrap();
    let (_, err, code) = wj(tmp.path(), &["new", "demo", "--template", "lib"]);
    assert_eq!(code, 0, "{err}");
    let proj = tmp.path().join("demo");
    let want = listed(&proj);
    assert_eq!(want.len(), 4, "template ships four tests: {want:?}");
    let (got, code, text) = interpreted(&proj, &[]);
    assert_eq!(got, want, "interpret mode must run exactly the discovered tests\n{text}");
    assert_eq!(code, 0, "{text}");
}

#[test]
fn every_discovery_shape_is_shared() {
    let tmp = TempDir::new().unwrap();
    let d = tmp.path();
    fs::write(
        d.join("lib.wj"),
        r#"
fn double(x: i64) -> i64 { x * 2 }

fn test_top_level() {
    assert_eq(double(4), 8)
}

@test
fn checks_decorated() {
    assert_eq(double(0), 0)
}

@test_cases([(1, 2), (5, 10)])
fn test_cases_double(x: i64, want: i64) {
    assert_eq(double(x), want)
}

mod inner {
    fn test_in_mod() {
        assert_eq(1 + 1, 2)
    }
}

struct Fixture {}

impl Fixture {
    fn test_assoc() {
        assert_eq(3 - 1, 2)
    }
}

fn helper_not_a_test() {}
"#,
    )
    .unwrap();
    fs::write(d.join("extra_test.wj"), "fn test_in_test_file() {\n    assert(true)\n}\n").unwrap();
    let want = listed(d);
    for expected in ["test_top_level", "checks_decorated", "test_cases_double", "inner::test_in_mod", "Fixture::test_assoc", "test_in_test_file"] {
        assert!(want.contains(expected), "compiled discovery must list {expected}: {want:?}");
    }
    assert!(!want.contains("helper_not_a_test"));
    let (got, code, text) = interpreted(d, &[]);
    assert_eq!(got, want, "{text}");
    assert_eq!(code, 0, "{text}");
}

#[test]
fn filter_and_empty_behaviour_match_the_compiled_runner() {
    let tmp = TempDir::new().unwrap();
    let d = tmp.path();
    fs::write(d.join("a.wj"), "fn test_alpha() {}\nfn test_beta() {}\n").unwrap();
    let (got, code, text) = interpreted(d, &["--filter", "alpha"]);
    assert_eq!(got, BTreeSet::from(["test_alpha".to_string()]), "{text}");
    assert_eq!(code, 0);
    // a filter that matches nothing is an error in both modes
    let (_, code_i, text_i) = interpreted(d, &["--filter", "zzz"]);
    let (_, err_c, code_c) = wj(d, &["test", "--filter", "zzz"]);
    assert_ne!(code_i, 0, "{text_i}");
    assert_ne!(code_c, 0, "{err_c}");
    // no tests at all: same error text
    let empty = TempDir::new().unwrap();
    fs::write(empty.path().join("m.wj"), "fn main() {}\n").unwrap();
    let (_, code_i, text_i) = interpreted(empty.path(), &[]);
    let (_, err_c, code_c) = wj(empty.path(), &["test"]);
    assert_ne!(code_i, 0);
    assert_ne!(code_c, 0);
    assert!(text_i.contains("no tests found"), "{text_i}");
    assert!(err_c.contains("no tests found"), "{err_c}");
}

#[test]
fn a_failing_interpreted_test_fails_the_run() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("f.wj"), "fn test_bad() {\n    assert_eq(1, 2)\n}\n").unwrap();
    let (names, code, text) = interpreted(tmp.path(), &[]);
    assert_eq!(names, BTreeSet::from(["test_bad".to_string()]));
    assert_eq!(code, 1, "{text}");
    assert!(text.contains("FAILED"));
}

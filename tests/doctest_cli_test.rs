//! ADR-0018 feature 6: `wj test --doc` runs ```wj examples in doc comments (interpreter), with `// expect:` lines.
//! Oracles are written by hand in this file; failing expectations must fail the run.

use std::process::Command;
use tempfile::TempDir;

fn wj_test_doc(src: &str) -> (bool, String) {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("lib.wj"), src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["test", "--doc"])
        .arg(dir.path())
        .output()
        .unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).to_string(),
    )
}

const DOUBLE: &str = "/// Doubles.\n///\n/// ```wj\n/// println(\"${double(21)}\")\n/// // expect: 42\n/// ```\nfn double(x: int) -> int {\n    x * 2\n}\n";

#[test]
fn doctest_with_matching_expectation_passes() {
    let (ok, out) = wj_test_doc(DOUBLE);
    assert!(ok, "{out}");
    assert!(out.contains("(fn double) ... ok"), "{out}");
    assert!(
        out.contains("doctests: 1 passed, 0 failed, 0 ignored"),
        "{out}"
    );
}

#[test]
fn doctest_with_wrong_expectation_fails() {
    let (ok, out) = wj_test_doc(&DOUBLE.replace("expect: 42", "expect: 41"));
    assert!(!ok, "{out}");
    assert!(out.contains("FAILED") && out.contains("\"41\""), "{out}");
}

#[test]
fn doctest_without_expectation_must_not_panic() {
    let src = "/// ```wj\n/// let v: Vec<int> = []\n/// println(\"${v[3]}\")\n/// ```\nfn f() {}\n";
    let (ok, out) = wj_test_doc(src);
    assert!(!ok, "an out-of-bounds index in a doctest must fail: {out}");
}

#[test]
fn ignored_and_should_fail_fences() {
    let src = "/// ```wj,ignore\n/// this is not code\n/// ```\n///\n/// ```wj,should_fail\n/// let x = (\n/// ```\nfn f() {}\n";
    let (ok, out) = wj_test_doc(src);
    assert!(ok, "{out}");
    assert!(out.contains("1 passed, 0 failed, 1 ignored"), "{out}");
}

#[test]
fn module_main_is_not_run_and_module_comments_are_not_expectations() {
    let src = format!(
        "{DOUBLE}\n// expect: not-an-expectation\nfn main() {{\n    println(\"module main\")\n}}\n"
    );
    let (ok, out) = wj_test_doc(&src);
    assert!(ok, "{out}");
}

#[test]
fn std_enumerable_doctests_pass() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["test", "--doc"])
        .arg(root.join("std/enumerable.wj"))
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{text}");
    assert!(text.contains("7 passed"), "{text}");
}

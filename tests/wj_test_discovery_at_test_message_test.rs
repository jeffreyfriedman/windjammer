#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
))]

//! Empty-state help mentions `@test` and `tests/` + `*_test.wj`.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn wj_test_empty_state_mentions_at_test_and_tests_dir() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    fs::create_dir_all(root.join("src")).unwrap();

    let wj = env!("CARGO_BIN_EXE_wj");
    let output = Command::new(wj)
        .current_dir(root)
        .arg("test")
        .output()
        .expect("wj test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        output.status.success(),
        "empty test run should succeed:\n{combined}"
    );
    assert!(
        combined.contains("@test"),
        "empty-state help must mention @test:\n{combined}"
    );
    assert!(
        combined.contains("tests/") && combined.contains("_test.wj"),
        "empty-state help must mention tests/ and *_test.wj:\n{combined}"
    );
}

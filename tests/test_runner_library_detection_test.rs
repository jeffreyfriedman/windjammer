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

//! TDD: `wj test` must not add a library path dependency when the project has no `.wj` sources
//! (e.g. the windjammer compiler repo has `src/*.rs` only).

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_wj_test_runs_without_wj_library_sources() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // Rust-only `src/` (like windjammer compiler layout) — no `.wj` library to compile.
    let src = root.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("lib.rs"), "pub fn x() {}\n").unwrap();

    let tests = root.join("tests");
    fs::create_dir_all(&tests).unwrap();
    fs::write(
        tests.join("smoke_test.wj"),
        r#"pub fn test_smoke() {
    assert_eq(1, 1)
}
"#,
    )
    .unwrap();

    let wj = env!("CARGO_BIN_EXE_wj");
    let output = Command::new(wj)
        .args(["test", tests.join("smoke_test.wj").to_str().unwrap()])
        .current_dir(root)
        .output()
        .expect("run wj test");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stdout}{stderr}");

    assert!(
        !combined.contains("failed to read") || !combined.contains("lib/Cargo.toml"),
        "should not reference missing lib/Cargo.toml when no .wj library exists:\n{combined}"
    );
    assert!(
        output.status.success() || combined.contains("1 passed"),
        "wj test should succeed for standalone test file:\n{combined}"
    );
}

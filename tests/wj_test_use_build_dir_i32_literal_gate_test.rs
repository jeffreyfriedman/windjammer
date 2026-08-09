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

//! FAILING REPRO — `wj test --use-build-dir` must type `@test` int literals to match
//! explicit `i32` (or `i64`) formals on the prebuilt library.
//!
//! Observed tip bug: `pub fn double(x: i32)` + `double(3)` in a `.wj` test emits
//! `double(3_i64)`, so cargo fails with E0308 before any assertion runs.
//!
//! Language-only; no product/repo names.
//! Gap doc: `docs/USER_LIBRARY_TEST_GAPS.md` §6 / dogfood acceptance.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn wj_test_use_build_dir_i32_formal_accepts_int_literal() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let src = root.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("mod.wj"),
        r#"pub fn double(x: i32) -> i32 {
    x * 2
}
"#,
    )
    .unwrap();

    fs::write(
        root.join("Cargo.toml"),
        r#"[package]
name = "prebuilt-i32-lib"
version = "0.1.0"
edition = "2021"

[lib]
name = "prebuilt_i32_lib"
path = "build/lib.rs"
"#,
    )
    .unwrap();

    let build = root.join("build");
    let wj = env!("CARGO_BIN_EXE_wj");

    let build_out = Command::new(wj)
        .current_dir(root)
        .args([
            "build",
            "src/mod.wj",
            "--library",
            "--module-file",
            "-o",
            build.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("wj build");
    assert!(
        build_out.status.success(),
        "prebuild failed:\n{}{}",
        String::from_utf8_lossy(&build_out.stdout),
        String::from_utf8_lossy(&build_out.stderr)
    );

    let tests = root.join("tests");
    fs::create_dir_all(&tests).unwrap();
    fs::write(
        tests.join("double_test.wj"),
        r#"use prebuilt_i32_lib::double

@test
fn test_double() {
    assert_eq(double(3), 6)
}
"#,
    )
    .unwrap();

    let test_out = Command::new(wj)
        .current_dir(root)
        .args(["test", "--use-build-dir", "build", "--no-runtime-copy"])
        .output()
        .expect("wj test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&test_out.stdout),
        String::from_utf8_lossy(&test_out.stderr)
    );

    assert!(
        test_out.status.success(),
        "wj test --use-build-dir must accept int literal for i32 formal \
         (must not emit 3_i64):\n{combined}"
    );
    assert!(
        combined.contains("test_double") || combined.contains("1 passed"),
        "expected test to run:\n{combined}"
    );
    assert!(
        !combined.contains("3_i64") || test_out.status.success(),
        "emitted 3_i64 against i32 formal:\n{combined}"
    );
}

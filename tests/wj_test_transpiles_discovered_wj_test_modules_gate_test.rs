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

//! FAILING REPRO — `wj test` must transpile discovered `*_test.wj` into the temp
//! test crate (not only emit `pub mod …` in `lib.rs`).
//!
//! Observed tip bug: discovery finds `tests/foo_test.wj` and generates
//! `pub mod foo_test;` but cargo fails with E0583 (`file not found for module`)
//! because `foo_test.rs` was never written into the temp tree.
//!
//! Language-only; no product/repo names.
//! Gap doc: `docs/USER_LIBRARY_TEST_GAPS.md` (discovery → transpile).

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn wj_test_transpiles_discovered_wj_test_modules_into_temp_crate() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let src = root.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("mod.wj"),
        r#"pub fn double(x: int) -> int {
    x * 2
}
"#,
    )
    .unwrap();

    fs::write(
        root.join("Cargo.toml"),
        r#"[package]
name = "prebuilt-lib"
version = "0.1.0"
edition = "2021"

[lib]
name = "prebuilt_lib"
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
        r#"use prebuilt_lib::double

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
        !combined.contains("E0583") && !combined.contains("file not found for module"),
        "wj test must transpile discovered *_test.wj into the temp crate \
         (must not leave pub mod without .rs):\n{combined}"
    );
    assert!(
        test_out.status.success(),
        "wj test --use-build-dir must run discovered .wj tests:\n{combined}"
    );
    assert!(
        combined.contains("test_double") || combined.contains("1 passed"),
        "expected test to run:\n{combined}"
    );
}

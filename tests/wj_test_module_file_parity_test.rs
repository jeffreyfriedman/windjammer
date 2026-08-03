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

//! `wj test --module-file --library` must compile multipass libs like `wj build`.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn wj_test_module_file_compiles_multipass_library() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let src = root.join("src");
    fs::create_dir_all(&src).unwrap();

    fs::write(
        src.join("mod.wj"),
        r#"pub mod math;
pub use math::add;
"#,
    )
    .unwrap();

    fs::write(
        src.join("math.wj"),
        r#"pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#,
    )
    .unwrap();

    fs::write(
        root.join("Cargo.toml"),
        r#"[package]
name = "parity-lib"
version = "0.1.0"
edition = "2021"

[lib]
name = "parity_lib"
path = "build/lib.rs"
"#,
    )
    .unwrap();

    let tests = root.join("tests");
    fs::create_dir_all(&tests).unwrap();
    fs::write(
        tests.join("math_test.wj"),
        r#"use parity_lib::add

@test
fn test_add() {
    assert_eq(add(2, 3), 5)
}
"#,
    )
    .unwrap();

    let wj = env!("CARGO_BIN_EXE_wj");
    let output = Command::new(wj)
        .current_dir(root)
        .args(["test", "--module-file", "--library", "--no-runtime-copy"])
        .output()
        .expect("wj test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        output.status.success(),
        "wj test --module-file should pass:\n{combined}"
    );
    assert!(
        combined.contains("test_add") || combined.contains("1 passed"),
        "expected test to run:\n{combined}"
    );
}

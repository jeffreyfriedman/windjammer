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

//! TDD: `tests/*.rs` Rust tests auto-register via generated `tests/lib.rs` + lib.rs hook.

use std::fs;
use std::process::Command;

#[test]
fn test_rust_integration_tests_auto_registered() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    fs::create_dir_all(root.join("tests")).unwrap();
    fs::write(
        root.join("tests/my_feature_test.rs"),
        "#[test]\nfn works() { assert!(true); }\n",
    )
    .unwrap();
    fs::write(root.join("lib.rs"), "pub fn api() {}\n").unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"demo-crate\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"lib.rs\"\n",
    )
    .unwrap();

    windjammer::sync_rust_integration_tests(root).unwrap();

    let tests_lib = fs::read_to_string(root.join("tests/lib.rs")).unwrap();
    assert!(
        tests_lib.contains("pub mod my_feature_test;"),
        "missing auto mod: {tests_lib}"
    );

    let lib_rs = fs::read_to_string(root.join("lib.rs")).unwrap();
    assert!(
        lib_rs.contains("WINDJAMMER_AUTO_RUST_TESTS_BEGIN"),
        "lib.rs hook missing: {lib_rs}"
    );

    let cargo = fs::read_to_string(root.join("Cargo.toml")).unwrap();
    assert!(
        cargo.contains("autotests = false"),
        "should disable duplicate integration binaries: {cargo}"
    );

    let status = Command::new("cargo")
        .args(["test", "--lib", "works"])
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success(), "cargo test --lib should pass");
}

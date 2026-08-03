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

//! `wj test --use-project-cargo` preserves dependency features from project Cargo.toml.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn setup_stub_ui(stub_root: &std::path::Path) {
    fs::create_dir_all(stub_root.join("src")).unwrap();
    fs::write(
        stub_root.join("Cargo.toml"),
        r#"[package]
name = "stub-ui"
version = "0.1.0"
edition = "2021"

[features]
default = []
web = []
desktop = []

[lib]
name = "stub_ui"
path = "src/lib.rs"
"#,
    )
    .unwrap();
    fs::write(stub_root.join("src/lib.rs"), "pub fn marker() -> &'static str { \"web\" }\n")
        .unwrap();
}

#[test]
fn wj_test_use_project_cargo_preserves_dep_features() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let stub = root.join("stub-ui");
    setup_stub_ui(&stub);

    let src = root.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("mod.wj"),
        r#"pub fn hello() -> string {
    "hi"
}
"#,
    )
    .unwrap();

    let stub_rel = "stub-ui";
    fs::write(
        root.join("Cargo.toml"),
        &format!(
            r#"[package]
name = "feature-lib"
version = "0.1.0"
edition = "2021"

[lib]
name = "feature_lib"
path = "build/lib.rs"

[dependencies]
stub-ui = {{ path = "{stub_rel}", features = ["web"] }}
"#
        ),
    )
    .unwrap();

    let tests = root.join("tests");
    fs::create_dir_all(&tests).unwrap();
    fs::write(
        tests.join("hello_test.wj"),
        r#"use feature_lib::hello

@test
fn test_hello() {
    assert_eq(hello(), "hi")
}
"#,
    )
    .unwrap();

    let wj = env!("CARGO_BIN_EXE_wj");
    let output = Command::new(wj)
        .current_dir(root)
        .env("WJ_TEST_KEEP_TEMP", "1")
        .args([
            "test",
            "--use-project-cargo",
            "--no-runtime-copy",
            "--module-file",
            "--library",
        ])
        .output()
        .expect("wj test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        output.status.success(),
        "wj test --use-project-cargo should pass:\n{combined}"
    );

    let temp_line = combined
        .lines()
        .find(|l| l.starts_with("WJ_TEST_KEEP_TEMP:"))
        .expect("WJ_TEST_KEEP_TEMP path in output");
    let temp_dir = temp_line.trim_start_matches("WJ_TEST_KEEP_TEMP:").trim();
    let lib_cargo = fs::read_to_string(format!("{temp_dir}/lib/Cargo.toml"))
        .expect("lib Cargo.toml in temp");

    assert!(
        lib_cargo.contains("stub-ui") && lib_cargo.contains("web"),
        "project Cargo features must be preserved:\n{lib_cargo}"
    );
    assert!(
        !lib_cargo.contains("desktop"),
        "must not force desktop feature:\n{lib_cargo}"
    );
}

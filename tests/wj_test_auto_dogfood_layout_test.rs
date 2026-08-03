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

//! Bare `wj test` auto-detects dogfood layout (`Cargo.toml` `[lib] path = "build/lib.rs"`).

use std::fs;
use std::process::Command;
use std::time::SystemTime;
use tempfile::TempDir;

fn mtime(path: &std::path::Path) -> SystemTime {
    fs::metadata(path)
        .unwrap()
        .modified()
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

#[test]
fn wj_test_auto_detects_use_build_dir_from_cargo_lib_path() {
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
name = "auto-layout-lib"
version = "0.1.0"
edition = "2021"

[lib]
name = "auto_layout_lib"
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

    let before = mtime(&build.join("lib.rs"));

    let tests = root.join("tests");
    fs::create_dir_all(&tests).unwrap();
    fs::write(
        tests.join("double_test.wj"),
        r#"use auto_layout_lib::double

@test
fn test_double() {
    assert_eq(double(3), 6)
}
"#,
    )
    .unwrap();

    // No --use-build-dir / --use-project-cargo — inference must kick in.
    let test_out = Command::new(wj)
        .current_dir(root)
        .args(["test", "--no-runtime-copy"])
        .output()
        .expect("wj test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&test_out.stdout),
        String::from_utf8_lossy(&test_out.stderr)
    );

    assert!(
        test_out.status.success(),
        "bare wj test should auto-detect build/:\n{combined}"
    );

    let after = mtime(&build.join("lib.rs"));
    assert_eq!(before, after, "auto-detected build dir must not recompile lib.rs");
}

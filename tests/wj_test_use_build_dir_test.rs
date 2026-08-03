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

//! `wj test --use-build-dir` links a pre-built outbound tree without recompiling it.

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
fn wj_test_use_build_dir_skips_lib_recompile() {
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
    assert!(build.join("lib.rs").exists(), "build/lib.rs missing");

    let before = mtime(build.join("lib.rs").as_path());

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
        .args([
            "test",
            "--use-build-dir",
            "build",
            "--no-runtime-copy",
        ])
        .output()
        .expect("wj test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&test_out.stdout),
        String::from_utf8_lossy(&test_out.stderr)
    );

    assert!(
        test_out.status.success(),
        "wj test --use-build-dir should pass:\n{combined}"
    );

    let after = mtime(build.join("lib.rs").as_path());
    assert_eq!(
        before, after,
        "build/lib.rs mtime must be unchanged (library was not recompiled)"
    );
}

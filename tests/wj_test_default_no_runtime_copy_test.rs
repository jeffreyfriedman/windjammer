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

//! `wj.toml` path dep on windjammer-runtime → default `--no-runtime-copy`.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn wj_test_infers_no_runtime_copy_from_wj_toml() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let runtime_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("crates/windjammer-runtime");
    let runtime_str = runtime_path.to_string_lossy().replace('\\', "/");

    fs::write(
        root.join("wj.toml"),
        format!(
            r#"[package]
name = "infer-runtime"
version = "0.1.0"

[dependencies]
windjammer-runtime = {{ path = "{runtime_str}" }}
"#
        ),
    )
    .unwrap();

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
        .current_dir(root)
        .env("WJ_TEST_KEEP_TEMP", "1")
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
        "wj test should infer no-runtime-copy:\n{combined}"
    );

    let temp_line = combined
        .lines()
        .find(|l| l.starts_with("WJ_TEST_KEEP_TEMP:"))
        .expect("WJ_TEST_KEEP_TEMP path in output");
    let temp_dir = temp_line.trim_start_matches("WJ_TEST_KEEP_TEMP:").trim();

    let harness_cargo =
        fs::read_to_string(format!("{temp_dir}/Cargo.toml")).expect("harness Cargo.toml");
    assert!(
        harness_cargo.contains(&runtime_str),
        "harness must reference runtime path dep:\n{harness_cargo}"
    );
    assert!(
        !PathBuf::from(temp_dir)
            .join("crates/windjammer-runtime")
            .exists(),
        "must not copy runtime into temp tree when wj.toml has path dep"
    );
}

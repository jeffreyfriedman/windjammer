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

//! `wj test --no-runtime-copy` uses a path dep instead of copying windjammer-runtime.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn wj_test_no_runtime_copy_uses_path_dep() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

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

    let runtime_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("crates/windjammer-runtime");
    let runtime_str = runtime_path.to_string_lossy().replace('\\', "/");

    let wj = env!("CARGO_BIN_EXE_wj");
    let output = Command::new(wj)
        .current_dir(root)
        .env("WJ_TEST_KEEP_TEMP", "1")
        .args([
            "test",
            "--no-runtime-copy",
            "--runtime-path",
            &runtime_str,
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
        "wj test --no-runtime-copy should pass:\n{combined}"
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
        "must not copy runtime into temp tree"
    );
}

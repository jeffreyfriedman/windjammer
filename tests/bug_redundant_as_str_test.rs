#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
/// TDD Test: Redundant .as_str() on inferred &str parameters
///
/// Bug: When a string parameter is inferred as &str, calling .as_str() on it
/// generated code that tried to call the unstable String::as_str() method.
///
/// Fix: Reject `.as_str()` at compile time (no-rust-leakage rule). The compiler
/// emits a helpful error directing users to use `match ext { ... }` instead.
use std::process::Command;

#[test]
fn test_as_str_is_rejected_by_wj_cli() {
    // When source contains .as_str(), wj build should FAIL with helpful error
    let source = r#"
enum AssetType {
    Texture,
    Audio,
    Unknown,
}

impl AssetType {
    pub fn from_extension(ext: string) -> AssetType {
        match ext.as_str() {
            "png" => AssetType::Texture,
            "wav" => AssetType::Audio,
            _ => AssetType::Unknown,
        }
    }
}
"#;

    let _tmp = tempfile::tempdir().unwrap();

    let temp_dir = _tmp
        .path()
        .join(format!("wj_as_str_test_{}", std::process::id()));

    fs::create_dir_all(&temp_dir).unwrap();
    let temp_file = temp_dir.join("test_as_str.wj");
    let output_dir = temp_dir.join("out");
    fs::create_dir_all(&output_dir).unwrap();
    fs::write(&temp_file, source).unwrap();

    let output = Command::new(test_utils::wj_binary())
        .args([
            "build",
            "--output",
            output_dir.to_str().unwrap(),
            "--target",
            "rust",
            temp_file.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run wj");

    // Build should FAIL - .as_str() is forbidden
    assert!(
        !output.status.success(),
        "wj build should fail when source contains .as_str()"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should have helpful error (not rustc E0658)
    assert!(
        stderr.contains("`.as_str()` is forbidden") || stdout.contains("`.as_str()` is forbidden"),
        "Should emit helpful 'forbidden' error. Stderr:\n{}\nStdout:\n{}",
        stderr,
        stdout
    );
}

#[test]
fn test_match_on_string_directly() {
    // Idiomatic Windjammer: match ext directly (no .as_str())
    let source = r#"
enum FileType {
    Text,
    Binary,
}

impl FileType {
    pub fn classify(ext: string) -> FileType {
        match ext {
            "txt" => FileType::Text,
            "bin" => FileType::Binary,
            _ => FileType::Binary,
        }
    }
}
"#;

    let _tmp2 = tempfile::tempdir().unwrap();

    let temp_dir = _tmp2
        .path()
        .join(format!("wj_match_string_test_{}", std::process::id()));

    fs::create_dir_all(&temp_dir).unwrap();
    let temp_file = temp_dir.join("test_match_string.wj");
    let output_dir = temp_dir.join("out");
    fs::create_dir_all(&output_dir).unwrap();
    fs::write(&temp_file, source).unwrap();

    let output = Command::new(test_utils::wj_binary())
        .args([
            "build",
            "--output",
            output_dir.to_str().unwrap(),
            "--target",
            "rust",
            "--no-cargo",
            temp_file.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run wj");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "Should compile without errors. Stderr:\n{}\nStdout:\n{}",
        stderr,
        stdout
    );

    // Verify generated .rs file exists
    let generated_rs = output_dir.join("test_match_string.rs");
    assert!(
        generated_rs.exists(),
        "Generated .rs file should exist at {:?}",
        generated_rs
    );
}

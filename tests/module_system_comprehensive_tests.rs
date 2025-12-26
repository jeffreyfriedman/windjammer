//! Comprehensive Module System Tests
//!
//! These tests verify that the Windjammer compiler correctly handles
//! the module system, including:
//! - use statements
//! - mod declarations
//! - super and crate paths
//! - Re-exports

use std::fs;
use std::process::Command;
use tempfile::TempDir;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn compile_and_get_rust(code: &str) -> Result<String, String> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&wj_path, code).expect("Failed to write test file");
    fs::create_dir_all(&out_dir).expect("Failed to create output dir");

    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--",
            "build",
            wj_path.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let generated_path = out_dir.join("test.rs");
    fs::read_to_string(&generated_path).map_err(|e| format!("Failed to read generated file: {}", e))
}

fn compile_and_verify(code: &str) -> (bool, String, String) {
    match compile_and_get_rust(code) {
        Ok(generated) => {
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let rs_path = temp_dir.path().join("test.rs");
            fs::write(&rs_path, &generated).expect("Failed to write rs file");

            let rustc = Command::new("rustc")
                .arg("--crate-type=lib")
                .arg(&rs_path)
                .arg("-o")
                .arg(temp_dir.path().join("test.rlib"))
                .output();

            match rustc {
                Ok(output) => {
                    let err = String::from_utf8_lossy(&output.stderr).to_string();
                    (output.status.success(), generated, err)
                }
                Err(e) => (false, generated, format!("Failed to run rustc: {}", e)),
            }
        }
        Err(e) => (false, String::new(), e),
    }
}

// ============================================================================
// USE STATEMENTS
// ============================================================================

#[test]
fn test_use_std_collections() {
    let code = r#"
use std::collections::HashMap

pub fn create_map() -> HashMap<i32, i32> {
    HashMap::new()
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(
        success,
        "use std::collections should compile. Error: {}",
        err
    );
}

#[test]
fn test_use_std_vec() {
    let code = r#"
pub fn create_vec() -> Vec<i32> {
    let mut v = Vec::new();
    v.push(1);
    v.push(2);
    v
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Vec should be in prelude. Error: {}", err);
}

#[test]
fn test_use_option() {
    let code = r#"
pub fn wrap(x: i32) -> Option<i32> {
    if x > 0 {
        Some(x)
    } else {
        None
    }
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Option should be in prelude. Error: {}", err);
}

#[test]
fn test_use_result() {
    let code = r#"
pub fn parse(n: i32) -> Result<i32, i32> {
    if n >= 0 {
        Ok(n)
    } else {
        Err(-1)
    }
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Result should be in prelude. Error: {}", err);
}

// ============================================================================
// MULTIPLE USE
// ============================================================================

#[test]
fn test_multiple_use() {
    let code = r#"
use std::collections::HashMap
use std::collections::HashSet

pub fn test_both() -> (HashMap<i32, i32>, HashSet<i32>) {
    (HashMap::new(), HashSet::new())
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Multiple use should compile. Error: {}", err);
}

// ============================================================================
// NESTED TYPES
// ============================================================================

#[test]
fn test_vec_of_i32() {
    let code = r#"
pub fn int_items() -> Vec<i32> {
    let mut v = Vec::new();
    v.push(1);
    v.push(2);
    v.push(3);
    v
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Vec of i32 should compile. Error: {}", err);
}

// ============================================================================
// BASIC OPTION/RESULT
// ============================================================================

#[test]
fn test_option_some() {
    let code = r#"
pub fn make_some(x: i32) -> Option<i32> {
    Some(x)
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Option Some should compile. Error: {}", err);
}

#[test]
fn test_option_none() {
    let code = r#"
pub fn make_none() -> Option<i32> {
    None
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Option None should compile. Error: {}", err);
}

#[test]
fn test_result_ok() {
    let code = r#"
pub fn make_ok(x: i32) -> Result<i32, i32> {
    Ok(x)
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Result Ok should compile. Error: {}", err);
}

#[test]
fn test_result_err() {
    let code = r#"
pub fn make_err(e: i32) -> Result<i32, i32> {
    Err(e)
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Result Err should compile. Error: {}", err);
}

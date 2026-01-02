//! Comprehensive Codegen Closure Tests
//!
//! These tests verify that the Windjammer compiler correctly generates
//! Rust code for closures, including:
//! - Basic closures
//! - Capture modes (move, borrow)
//! - Closure types
//! - Higher-order functions

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
// BASIC CLOSURES
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_closure_simple() {
    let code = r#"
pub fn use_closure() -> i32 {
    let add_one = |x| x + 1;
    add_one(5)
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Simple closure should compile. Error: {}", err);
}

#[test]
fn test_closure_multiple_params() {
    let code = r#"
pub fn use_closure() -> i32 {
    let add = |a, b| a + b;
    add(3, 4)
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(
        success,
        "Multiple param closure should compile. Error: {}",
        err
    );
}

#[test]
fn test_closure_no_params() {
    let code = r#"
pub fn use_closure() -> i32 {
    let get_five = || 5;
    get_five()
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "No param closure should compile. Error: {}", err);
}

// ============================================================================
// CLOSURE CAPTURE
// ============================================================================

#[test]
fn test_closure_capture_immutable() {
    let code = r#"
pub fn use_closure() -> i32 {
    let x = 10;
    let get_x = || x;
    get_x()
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Capture immutable should compile. Error: {}", err);
}

#[test]
fn test_closure_capture_multiple() {
    let code = r#"
pub fn use_closure() -> i32 {
    let x = 10;
    let y = 20;
    let sum = || x + y;
    sum()
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Capture multiple should compile. Error: {}", err);
}

// ============================================================================
// CLOSURE WITH ITERATOR METHODS
// ============================================================================

#[test]
fn test_closure_map() {
    let code = r#"
pub fn double_all(items: Vec<i32>) -> Vec<i32> {
    items.iter().map(|x| x * 2).collect()
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Closure map should compile. Error: {}", err);
}

#[test]
fn test_closure_filter() {
    let code = r#"
pub fn positive_only(items: Vec<i32>) -> Vec<i32> {
    items.iter().filter(|x| **x > 0).cloned().collect()
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Closure filter should compile. Error: {}", err);
}

#[test]
fn test_closure_fold() {
    let code = r#"
pub fn sum_all(items: Vec<i32>) -> i32 {
    items.iter().fold(0, |acc, x| acc + x)
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Closure fold should compile. Error: {}", err);
}

#[test]
fn test_closure_for_each() {
    let code = r#"
pub fn print_all(items: Vec<i32>) {
    items.iter().for_each(|x| println!("{}", x))
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Closure for_each should compile. Error: {}", err);
}

// ============================================================================
// CLOSURE CHAINING
// ============================================================================

#[test]
fn test_closure_chain() {
    let code = r#"
pub fn process(items: Vec<i32>) -> i32 {
    items.iter()
        .map(|x| x * 2)
        .filter(|x| *x > 5)
        .sum()
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Closure chain should compile. Error: {}", err);
}

#[test]
fn test_closure_complex_chain() {
    let code = r#"
pub fn transform(items: Vec<i32>) -> Vec<i32> {
    items.iter()
        .filter(|x| **x > 0)
        .map(|x| x * 2)
        .map(|x| x + 1)
        .collect()
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Complex chain should compile. Error: {}", err);
}

// ============================================================================
// CLOSURE WITH BLOCKS
// ============================================================================

#[test]
fn test_closure_block_body() {
    let code = r#"
pub fn use_closure() -> i32 {
    let compute = |x| {
        let doubled = x * 2;
        let plus_one = doubled + 1;
        plus_one
    };
    compute(5)
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Closure block body should compile. Error: {}", err);
}

// ============================================================================
// SORT AND COMPARE CLOSURES
// ============================================================================

#[test]
fn test_closure_sort_by() {
    let code = r#"
pub fn sort_descending(items: &mut Vec<i32>) {
    items.sort_by(|a, b| b.cmp(a))
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Closure sort_by should compile. Error: {}", err);
}

// ============================================================================
// FIND AND ANY/ALL
// ============================================================================

#[test]
fn test_closure_count() {
    // Test count() which returns usize (simpler case)
    let code = r#"
pub fn count_positive(items: &Vec<i32>) -> usize {
    items.iter().filter(|x| **x > 0).count()
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Closure count should compile. Error: {}", err);
}

#[test]
fn test_closure_any() {
    let code = r#"
pub fn has_positive(items: &Vec<i32>) -> bool {
    items.iter().any(|x| *x > 0)
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Closure any should compile. Error: {}", err);
}

#[test]
fn test_closure_all() {
    let code = r#"
pub fn all_positive(items: &Vec<i32>) -> bool {
    items.iter().all(|x| *x > 0)
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Closure all should compile. Error: {}", err);
}

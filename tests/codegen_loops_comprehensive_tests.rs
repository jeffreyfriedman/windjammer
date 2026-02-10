//! Comprehensive Codegen Loop Tests
//!
//! These tests verify that the Windjammer compiler correctly generates
//! Rust code for loops, including:
//! - for-in loops with automatic .iter() inference
//! - while loops
//! - loop with break/continue
//! - Iterator method chaining

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn get_wj_compiler() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_wj"))
}

fn compile_and_get_rust(code: &str) -> Result<String, String> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&wj_path, code).expect("Failed to write test file");
    fs::create_dir_all(&out_dir).expect("Failed to create output dir");

    let output = Command::new(get_wj_compiler())
        .args([
            "build",
            wj_path.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
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
// BASIC FOR LOOPS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_for_range() {
    let code = r#"
pub fn sum_to_n(n: i32) -> i32 {
    let mut sum = 0
    for i in 0..n {
        sum += i
    }
    sum
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "For range should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_for_range_inclusive() {
    let code = r#"
pub fn sum_to_n_inclusive(n: i32) -> i32 {
    let mut sum = 0
    for i in 0..=n {
        sum += i
    }
    sum
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(
        success,
        "For range inclusive should compile. Error: {}",
        err
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_for_vec_iter() {
    let code = r#"
pub fn sum_vec(items: Vec<i32>) -> i32 {
    let mut sum = 0
    for item in items {
        sum += item
    }
    sum
}
"#;
    let (success, _generated, err) = compile_and_verify(code);

    // Should auto-infer .iter() or iterate properly
    assert!(success, "For vec iter should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_for_vec_owned() {
    let code = r#"
pub fn process_owned(items: Vec<i32>) -> Vec<i32> {
    let mut result = Vec::new()
    for item in items {
        result.push(item * 2)
    }
    result
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "For vec owned should compile. Error: {}", err);
}

// ============================================================================
// FOR LOOP MUTATION
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_for_iter_mut() {
    // iter_mut() for in-place mutation
    let code = r#"
pub fn double_all(items: &mut Vec<i32>) {
    for item in items.iter_mut() {
        *item = *item * 2
    }
}
"#;
    let (success, generated, err) = compile_and_verify(code);
    assert!(
        success,
        "For iter_mut should compile. Generated:\n{}\nError: {}",
        generated, err
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_for_loop_var_mutation() {
    let code = r#"
pub fn increment_all(items: Vec<i32>) -> Vec<i32> {
    let mut result = Vec::new()
    for item in items {
        let incremented = item + 1
        result.push(incremented)
    }
    result
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(
        success,
        "For loop var mutation should compile. Error: {}",
        err
    );
}

// ============================================================================
// WHILE LOOPS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_while_basic() {
    let code = r#"
pub fn countdown(n: i32) -> i32 {
    let mut count = 0
    let mut remaining = n
    while remaining > 0 {
        count += 1
        remaining -= 1
    }
    count
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "While basic should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_while_let() {
    let code = r#"
pub fn sum_optional(items: Vec<i32>) -> i32 {
    let mut sum = 0
    let mut iter = items.into_iter()
    while let Some(item) = iter.next() {
        sum += item
    }
    sum
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "While let should compile. Error: {}", err);
}

// ============================================================================
// LOOP WITH BREAK/CONTINUE
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_loop_with_break() {
    let code = r#"
pub fn find_first(items: &Vec<i32>, target: i32) -> i32 {
    let mut index = 0
    for item in items {
        if *item == target {
            break
        }
        index += 1
    }
    index
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Loop with break should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_loop_with_continue() {
    let code = r#"
pub fn sum_positive(items: &Vec<i32>) -> i32 {
    let mut sum = 0
    for item in items {
        if *item < 0 {
            continue
        }
        sum += *item
    }
    sum
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Loop with continue should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_infinite_loop() {
    let code = r#"
pub fn find_value() -> i32 {
    let mut n = 0
    loop {
        n += 1
        if n > 10 {
            break
        }
    }
    n
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(
        success,
        "Infinite loop with break should compile. Error: {}",
        err
    );
}

// ============================================================================
// NESTED LOOPS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_nested_for() {
    // Nested for loops with ranges
    let code = r#"
pub fn multiply(a: i32, b: i32) -> i32 {
    let mut result = 0
    for _i in 0..a {
        for _j in 0..b {
            result += 1
        }
    }
    result
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Nested for should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_nested_while() {
    let code = r#"
pub fn multiply(a: i32, b: i32) -> i32 {
    let mut result = 0
    let mut i = 0
    while i < a {
        let mut j = 0
        while j < b {
            result += 1
            j += 1
        }
        i += 1
    }
    result
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Nested while should compile. Error: {}", err);
}

// ============================================================================
// ITERATOR METHODS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_iter_map() {
    let code = r#"
pub fn double_all(items: Vec<i32>) -> Vec<i32> {
    items.iter().map(|x| x * 2).collect()
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Iter map should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_iter_filter() {
    // Use iter().cloned() for simpler type handling
    let code = r#"
pub fn only_positive(items: Vec<i32>) -> Vec<i32> {
    items.iter().filter(|x| **x > 0).cloned().collect()
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Iter filter should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_iter_fold() {
    let code = r#"
pub fn sum_all(items: Vec<i32>) -> i32 {
    items.iter().fold(0, |acc, x| acc + x)
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Iter fold should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_iter_chain() {
    let code = r#"
pub fn double_filter_sum(items: Vec<i32>) -> i32 {
    items.iter()
        .map(|x| x * 2)
        .filter(|x| *x > 5)
        .sum()
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Iter chain should compile. Error: {}", err);
}

// ============================================================================
// ENUMERATE
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_enumerate_basic() {
    let code = r#"
pub fn find_index(items: Vec<i32>, target: i32) -> i32 {
    for (i, item) in items.iter().enumerate() {
        if *item == target {
            return i as i32
        }
    }
    -1
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Enumerate basic should compile. Error: {}", err);
}

// ============================================================================
// STRUCT ITERATION
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_iterate_struct_vec() {
    let code = r#"
@derive(Clone, Debug)
pub struct Item {
    value: i32,
}

pub fn sum_items(items: Vec<Item>) -> i32 {
    let mut sum = 0
    for item in items {
        sum += item.value
    }
    sum
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Iterate struct vec should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_iterate_and_clone() {
    let code = r#"
@derive(Clone, Debug)
pub struct Item {
    value: i32,
}

pub fn copy_all(items: &Vec<Item>) -> Vec<Item> {
    let mut result = Vec::new()
    for item in items {
        result.push(item.clone())
    }
    result
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Iterate and clone should compile. Error: {}", err);
}

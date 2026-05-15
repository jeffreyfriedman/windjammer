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

//! Comprehensive Module System Tests
//!
//! These tests verify that the Windjammer compiler correctly handles
//! the module system, including:
//! - use statements
//! - mod declarations
//! - super and crate paths
//! - Re-exports

#[path = "../common/test_utils.rs"]
mod test_utils;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

// ============================================================================
// USE STATEMENTS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_use_std_collections() {
    let code = r#"
use std::collections::HashMap

pub fn create_map() -> HashMap<i32, i32> {
    HashMap::new()
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(
        success,
        "use std::collections should compile. Error: {}",
        err
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_use_std_vec() {
    let code = r#"
pub fn create_vec() -> Vec<i32> {
    let mut v = Vec::new();
    v.push(1);
    v.push(2);
    v
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Vec should be in prelude. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Option should be in prelude. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Result should be in prelude. Error: {}", err);
}

// ============================================================================
// MULTIPLE USE
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_multiple_use() {
    let code = r#"
use std::collections::HashMap
use std::collections::HashSet

pub fn test_both() -> (HashMap<i32, i32>, HashSet<i32>) {
    (HashMap::new(), HashSet::new())
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Multiple use should compile. Error: {}", err);
}

// ============================================================================
// NESTED TYPES
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Vec of i32 should compile. Error: {}", err);
}

// ============================================================================
// BASIC OPTION/RESULT
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_option_some() {
    let code = r#"
pub fn make_some(x: i32) -> Option<i32> {
    Some(x)
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Option Some should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_option_none() {
    let code = r#"
pub fn make_none() -> Option<i32> {
    None
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Option None should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_result_ok() {
    let code = r#"
pub fn make_ok(x: i32) -> Result<i32, i32> {
    Ok(x)
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Result Ok should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_result_err() {
    let code = r#"
pub fn make_err(e: i32) -> Result<i32, i32> {
    Err(e)
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Result Err should compile. Error: {}", err);
}

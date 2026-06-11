#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

//! Comprehensive Codegen Closure Tests
//!
//! These tests verify that the Windjammer compiler correctly generates
//! Rust code for closures, including:
//! - Basic closures
//! - Capture modes (move, borrow)
//! - Closure types
//! - Higher-order functions

#[path = "common/test_utils.rs"]
mod test_utils;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Simple closure should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_closure_multiple_params() {
    let code = r#"
pub fn use_closure() -> i32 {
    let add = |a, b| a + b;
    add(3, 4)
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(
        success,
        "Multiple param closure should compile. Error: {}",
        err
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_closure_no_params() {
    let code = r#"
pub fn use_closure() -> i32 {
    let get_five = || 5;
    get_five()
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "No param closure should compile. Error: {}", err);
}

// ============================================================================
// CLOSURE CAPTURE
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_closure_capture_immutable() {
    let code = r#"
pub fn use_closure() -> i32 {
    let x = 10;
    let get_x = || x;
    get_x()
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Capture immutable should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_closure_capture_multiple() {
    let code = r#"
pub fn use_closure() -> i32 {
    let x = 10;
    let y = 20;
    let sum = || x + y;
    sum()
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Capture multiple should compile. Error: {}", err);
}

// ============================================================================
// CLOSURE WITH ITERATOR METHODS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_closure_map() {
    let code = r#"
pub fn double_all(items: Vec<i32>) -> Vec<i32> {
    items.map(|x| x * 2).collect()
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Closure map should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_closure_filter() {
    let code = r#"
pub fn positive_only(items: Vec<i32>) -> Vec<i32> {
    items.filter(|x| *x > 0).collect()
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Closure filter should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_closure_fold() {
    let code = r#"
pub fn sum_all(items: Vec<i32>) -> i32 {
    items.fold(0, |acc, x| acc + x)
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Closure fold should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_closure_for_each() {
    let code = r#"
pub fn print_all(items: Vec<i32>) {
    items.for_each(|x| println!("{}", x))
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Closure for_each should compile. Error: {}", err);
}

// ============================================================================
// CLOSURE CHAINING
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_closure_chain() {
    let code = r#"
pub fn process(items: Vec<i32>) -> i32 {
    items
        .map(|x| x * 2)
        .filter(|x| *x > 5)
        .sum()
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Closure chain should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_closure_complex_chain() {
    let code = r#"
pub fn transform(items: Vec<i32>) -> Vec<i32> {
    items
        .filter(|x| *x > 0)
        .map(|x| x * 2)
        .map(|x| x + 1)
        .collect()
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Complex chain should compile. Error: {}", err);
}

// ============================================================================
// CLOSURE WITH BLOCKS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Closure block body should compile. Error: {}", err);
}

// ============================================================================
// SORT AND COMPARE CLOSURES
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_closure_sort_by() {
    let code = r#"
pub fn sort_descending(mut items: Vec<i32>) {
    items.sort_by(|a, b| b.cmp(a))
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Closure sort_by should compile. Error: {}", err);
}

// ============================================================================
// FIND AND ANY/ALL
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_closure_count() {
    // Test count() which returns usize (simpler case)
    let code = r#"
pub fn count_positive(items: Vec<i32>) -> usize {
    items.filter(|x| *x > 0).count()
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Closure count should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_closure_any() {
    let code = r#"
pub fn has_positive(items: Vec<i32>) -> bool {
    items.any(|x| *x > 0)
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Closure any should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_closure_all() {
    let code = r#"
pub fn all_positive(items: Vec<i32>) -> bool {
    items.all(|x| *x > 0)
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Closure all should compile. Error: {}", err);
}

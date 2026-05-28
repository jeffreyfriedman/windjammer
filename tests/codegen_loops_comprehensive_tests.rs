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

//! Comprehensive Codegen Loop Tests
//!
//! These tests verify that the Windjammer compiler correctly generates
//! Rust code for loops, including:
//! - for-in loops with automatic .iter() inference
//! - while loops
//! - loop with break/continue
//! - Iterator method chaining

#[path = "common/test_utils.rs"]
mod test_utils;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);

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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
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
    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Iterate and clone should compile. Error: {}", err);
}

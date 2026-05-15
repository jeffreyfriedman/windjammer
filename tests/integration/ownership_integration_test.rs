//! Ownership Tracker Integration Tests
//!
//! Verifies that the ownership tracking system correctly populates and
//! influences code generation for parameters, for-loops, and match patterns.
//! Philosophy: "Safety Without Ceremony" - automatic ownership tracking.

#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_parameter_ownership_tracked_borrowed() {
    let src = r#"
pub struct Data { pub value: int }
pub fn process(data: Data) -> int {
    data.value
}
"#;
    let result = test_utils::compile_single_result(src).expect("compile");
    assert!(
        result.contains("data.value"),
        "Should use data.value directly"
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_parameter_owned_vec() {
    let src = r#"
pub fn sum(items: Vec<int>) -> int {
    let mut total = 0
    for item in items {
        total += item
    }
    total
}
"#;
    let result = test_utils::compile_single_result(src).expect("compile");
    assert!(result.contains("items: Vec<i64>") || result.contains("items: Vec<i32>"));
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_match_option_owned() {
    let src = r#"
pub fn unwrap(opt: Option<int>) -> int {
    match opt {
        Some(val) => val,
        None => 0
    }
}
"#;
    let result = test_utils::compile_single_result(src).expect("compile");
    assert!(result.contains("Some(val)"));
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_if_let_owned() {
    let src = r#"
pub fn get_default(opt: Option<int>) -> int {
    if let Some(x) = opt {
        x
    } else {
        0
    }
}
"#;
    let result = test_utils::compile_single_result(src).expect("compile");
    assert!(result.contains("if let Some(x) = opt"));
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_ownership_tracker_does_not_break_existing() {
    let src = r#"
pub fn identity(x: int) -> int { x }
pub fn main() {
    let x = identity(42)
}
"#;
    let result = test_utils::compile_single_result(src).expect("compile");
    assert!(result.contains("fn identity"));
}

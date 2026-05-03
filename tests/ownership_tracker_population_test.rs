//! Verification tests for ownership tracker population.
//!
//! Ensures ALL variable bindings are registered with the ownership tracker:
//! - Let bindings (simple and destructuring)
//! - For loop variables
//! - Match arm bindings
//! - If-let bindings
//! - Function parameters (verified in function_generation)

#[path = "test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_binding_registered() {
    // If tracker not populated, will get E0614 or E0308 when using x in y = x + 1
    let src = r#"
pub fn process() -> i32 {
    let x = 5
    let y = x + 1
    y
}
"#;

    let result = test_utils::compile_single_result(src).expect("compile");
    assert!(result.contains("let x = 5"), "Expected let x = 5 in output");
    assert!(
        result.contains("let y = x + 1"),
        "Expected let y = x + 1 in output"
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_from_param_registered() {
    // let x = param where param is borrowed - x should be usable
    let src = r#"
pub fn process(data: str) -> int {
    let x = data
    x.len() as int
}
"#;

    let result = test_utils::compile_single_result(src).expect("compile");
    assert!(result.contains("let x = data"), "Expected let binding");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_for_loop_variable_registered() {
    let src = r#"
pub fn sum_items() -> i32 {
    let items = [1, 2, 3]
    let mut total = 0
    for item in items {
        total = total + item
    }
    total
}
"#;

    let result = test_utils::compile_single_result(src).expect("compile");
    assert!(result.contains("for "), "Expected for loop");
    assert!(result.contains("item"), "Expected loop variable");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_match_arm_binding_registered() {
    let src = r#"
pub fn process(opt: Option<i32>) -> i32 {
    match opt {
        Some(x) => x + 1,
        None => 0
    }
}
"#;

    let result = test_utils::compile_single_result(src).expect("compile");
    assert!(
        result.contains("Some(x)"),
        "Expected match arm with binding"
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_if_let_binding_registered() {
    let src = r#"
pub fn process(opt: Option<i32>) -> i32 {
    if let Some(x) = opt {
        x + 1
    } else {
        0
    }
}
"#;

    let result = test_utils::compile_single_result(src).expect("compile");
    assert!(
        result.contains("if let Some(x)"),
        "Expected if let with binding"
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_tuple_destructuring_registered() {
    let src = r#"
pub fn process() -> i32 {
    let t = (1, 2, 3)
    let (a, b, c) = t
    a + b + c
}
"#;

    let result = test_utils::compile_single_result(src).expect("compile");
    assert!(
        result.contains("let (a, b, c)"),
        "Expected tuple destructuring"
    );
}

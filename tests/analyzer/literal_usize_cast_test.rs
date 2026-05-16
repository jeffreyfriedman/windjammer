#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

/// TDD Test: Integer literals don't need `as usize` cast in index and usize arithmetic
///
/// Bug: The compiler generates `arr[0 as usize]` instead of `arr[0]`, and
/// `items.len() - 1 as usize` instead of `items.len() - 1`.
/// Rust infers integer literal types from context, so these casts are unnecessary
/// and trigger clippy warnings.
///
/// Root Cause: The index expression handler and binary expression handler always
/// add `as usize` for integer literals, without checking if the context already
/// provides the correct type inference.
///
/// Fix: Skip `as usize` when the expression is an integer literal (non-negative),
/// since Rust will infer it as `usize` from context.
#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_no_usize_cast_on_integer_literal_index() {
    // arr[0] should NOT generate arr[0 as usize]
    let source = r#"
pub fn first_element(arr: Vec<i32>) -> i32 {
    arr[0]
}

pub fn third_element(arr: Vec<i32>) -> i32 {
    arr[2]
}
"#;

    let generated = test_utils::compile_single(source);
    println!("Generated:\n{}", generated);

    assert!(
        !generated.contains("0 as usize"),
        "Integer literal 0 in index position should not have `as usize` cast.\nGenerated:\n{}",
        generated
    );
    assert!(
        !generated.contains("2 as usize"),
        "Integer literal 2 in index position should not have `as usize` cast.\nGenerated:\n{}",
        generated
    );
    // But it SHOULD still compile (literals infer to usize in index context)
    assert!(
        generated.contains("arr[0]") || generated.contains("[0]"),
        "Should index with bare literal.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_no_usize_cast_on_literal_in_usize_arithmetic() {
    // items.len() - 1 should NOT generate items.len() - 1 as usize
    let source = r#"
pub fn last_index(items: Vec<i32>) -> usize {
    items.len() - 1
}
"#;

    let generated = test_utils::compile_single(source);
    println!("Generated:\n{}", generated);

    assert!(
        !generated.contains("1 as usize"),
        "Integer literal in arithmetic with usize should not have `as usize` cast.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_variable_index_still_gets_usize_cast() {
    // arr[idx] where idx is i32 SHOULD get `as usize` since i32 != usize
    let source = r#"
pub fn element_at(arr: Vec<i32>, idx: i32) -> i32 {
    arr[idx]
}
"#;

    let generated = test_utils::compile_single(source);
    println!("Generated:\n{}", generated);

    assert!(
        generated.contains("idx as usize"),
        "Variable index should still get `as usize` cast when type is i32.\nGenerated:\n{}",
        generated
    );
}

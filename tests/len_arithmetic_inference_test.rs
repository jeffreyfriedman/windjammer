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

// TDD: Integer literals in vec.len() +/- literal must infer as usize (not i32).
//
// Bug: items.len() - 1 → 1_i32 → E0277 cannot subtract i32 from usize

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_len_minus_literal_infers_usize() {
    let test_wj = r#"
fn last_index(items: Vec<i32>) -> usize {
    items.len() - 1
}
"#;

    let rust = test_utils::compile_single(test_wj);

    assert!(
        rust.contains("1_usize"),
        "Expected '1_usize' in len() subtraction. Generated:\n{}",
        rust
    );
    assert!(
        !rust.contains("1_i32"),
        "Should not default to i32 when subtracting from usize\n{}",
        rust
    );
}

#[test]
fn test_len_plus_literal_infers_usize() {
    let test_wj = r#"
fn capacity_with_buffer(items: Vec<i32>) -> usize {
    items.len() + 10
}
"#;

    let rust = test_utils::compile_single(test_wj);

    assert!(
        rust.contains("10_usize"),
        "Expected '10_usize' in len() addition. Generated:\n{}",
        rust
    );
}

#[test]
fn test_len_minus_in_comparison() {
    let test_wj = r#"
fn check_bounds(items: Vec<i32>, i: usize) -> bool {
    i < items.len() - 1
}
"#;

    let rust = test_utils::compile_single(test_wj);

    assert!(
        rust.contains("1_usize"),
        "Expected '1_usize' in len()-1 comparison. Generated:\n{}",
        rust
    );
}

#[test]
fn test_len_arithmetic_in_assignment() {
    let test_wj = r#"
fn set_last_index(items: Vec<i32>) {
    let mut idx: usize = 0
    idx = items.len() - 1
}
"#;

    let rust = test_utils::compile_single(test_wj);

    assert!(
        rust.contains("1_usize"),
        "Expected '1_usize' when assigning to usize var. Generated:\n{}",
        rust
    );
}

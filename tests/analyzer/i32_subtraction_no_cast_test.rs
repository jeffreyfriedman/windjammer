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

//! TDD: Subtraction of two i32 fields should not insert any `as u32` casts.
//! Reproduces a pre-existing bug where the compiler incorrectly casts one operand
//! of an i32 subtraction to u32, causing type mismatches in downstream usage.

#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_i32_minus_i32_no_u32_cast() {
    let code = r#"
pub struct Item {
    pub max_stack: i32,
}

pub struct Stack {
    pub item: Item,
    pub quantity: i32,
}

pub fn compute_space(stack: Stack) -> i32 {
    let can_add = stack.item.max_stack - stack.quantity
    can_add
}
"#;

    let output = test_utils::compile_single(code);

    assert!(
        !output.contains("as u32"),
        "i32 - i32 subtraction should NOT generate any `as u32` cast.\nGenerated:\n{}",
        output
    );
}

#[test]
fn test_i32_comparison_no_u32_cast() {
    let code = r#"
pub fn check(a: i32, b: i32) -> bool {
    let diff = a - b
    diff >= 0
}
"#;

    let output = test_utils::compile_single(code);

    assert!(
        !output.contains("as u32"),
        "i32 comparison should NOT generate `as u32` cast.\nGenerated:\n{}",
        output
    );
}

#[test]
fn test_i32_compound_subtract_no_u32_cast() {
    let code = r#"
pub struct Counter {
    pub value: i32,
}

impl Counter {
    pub fn subtract(self, amount: i32) {
        self.value = self.value - amount
    }
}
"#;

    let output = test_utils::compile_single(code);

    assert!(
        !output.contains("as u32"),
        "i32 compound subtraction should NOT generate `as u32` cast.\nGenerated:\n{}",
        output
    );
}

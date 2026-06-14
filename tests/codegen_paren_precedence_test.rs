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

//! TDD: Same-precedence `*`/`/` on the RHS of `/` must keep parentheses in Rust output.
//!
//! Without parens, `x / (2.0 * y)` becomes `x / 2.0 * y` which parses as `(x / 2.0) * y`.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_div_rhs_mul_same_precedence_keeps_parens() {
    let source = r#"
pub fn scaled_div(x: f32, y: f32) -> f32 {
    let result = x / (2.0 * y)
    result
}
"#;
    let rust = test_utils::compile_single(source);
    assert!(
        rust.contains("(2.0_f32 * y)"),
        "RHS of `/` must stay grouped so Rust does not parse `(x / 2.0) * y`. Generated:\n{}",
        rust
    );
    assert!(
        !rust.contains("x / 2.0_f32 * y") && !rust.contains("x / 2.0 * y"),
        "must not emit flat division-then-multiply without grouping. Generated:\n{}",
        rust
    );
}

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

#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_u32_compared_to_i32_inserts_cast() {
    let source = r#"
pub fn cmp_u32_i32(a: u32, b: i32) -> bool {
    a > b
}
"#;

    let out = test_utils::compile_single(source);
    assert!(
        out.contains("b as u32") || out.contains("(b as u32)"),
        "expected `b as u32` (promote to u32), got:\n{}",
        out
    );
}

#[test]
fn test_i32_compared_to_literal_uses_i32_suffix() {
    let source = r#"
pub fn cmp_i32_lit(x: i32) -> bool {
    x > 0
}
"#;

    let out = test_utils::compile_single(source);
    assert!(
        out.contains("0_i32") || out.contains("0i32"),
        "expected int literal suffix for i32 context, got:\n{}",
        out
    );
}

#[test]
fn test_u32_plus_i32_inserts_cast() {
    let source = r#"
pub fn add_u32_i32(a: u32, b: i32) -> u32 {
    a + b
}
"#;

    let out = test_utils::compile_single(source);
    assert!(
        out.contains("b as u32") || out.contains("(b as u32)"),
        "expected `b as u32` for u32 + i32, got:\n{}",
        out
    );
}

#[test]
fn test_len_compared_to_i32_variable() {
    let source = r#"
pub fn f(items: Vec<i32>, i: i32) -> bool {
    items.len() > i
}
"#;

    let out = test_utils::compile_single(source);
    assert!(
        out.contains(".len() as i64") || out.contains(".len()) as i64"),
        "expected .len() cast to i64 for safe comparison with signed int, got:\n{}",
        out
    );
}

/// Regression: do not force `N_usize` on literals that must stay in u32/i32 context.
#[test]
fn test_u32_comparison_literal_not_usize_suffixed() {
    let source = r#"
pub fn f(x: u32) -> bool {
    x > 0
}
"#;

    let out = test_utils::compile_single(source);
    assert!(
        !out.contains("0_usize"),
        "expected no 0_usize in u32 compare (E0308 regression), got:\n{}",
        out
    );
    assert!(
        out.contains("0_u32") || out.contains("x > 0"),
        "expected 0_u32 or contextual bare 0 with u32 lhs, got:\n{}",
        out
    );
}

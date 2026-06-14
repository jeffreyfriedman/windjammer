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

// (x as f32 + offset) - offset must be cast to f32 for f32 + i32

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_cast_plus_int_should_cast_int() {
    let source = r#"
pub fn test() -> usize {
    let x = 10
    let offset = 5
    (x as f32 + offset) as usize
}
"#;

    let result = test_utils::compile_single(source);
    assert!(
        result.contains("x as f32") && result.contains("offset as f32"),
        "Cast + int should cast int operand to f32. Got:\n{}",
        result
    );

    let __result = test_utils::verify_rust_compiles(&result);
    let rustc_ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(
        rustc_ok,
        "Should compile without E0277 mixed arithmetic. stderr: {}\n\nGenerated:\n{}",
        stderr, result
    );
}

/// (offset + x as f32) - offset must be cast to f32 when right is Cast to f32
#[test]
fn test_int_plus_cast_should_cast_int() {
    let source = r#"
pub fn test() -> usize {
    let x = 10
    let offset = 5
    (offset + x as f32) as usize
}
"#;

    let result = test_utils::compile_single(source);
    assert!(
        result.contains("offset as f32") && result.contains("x as f32"),
        "Int + Cast should cast int operand to f32. Got:\n{}",
        result
    );

    let __result = test_utils::verify_rust_compiles(&result);
    let rustc_ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(
        rustc_ok,
        "Should compile without E0277 mixed arithmetic. stderr: {}\n\nGenerated:\n{}",
        stderr, result
    );
}

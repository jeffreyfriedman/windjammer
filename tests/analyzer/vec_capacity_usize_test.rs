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

// TDD: Vec::with_capacity / push literal typing (usize, f64 unification)

#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_vec_with_capacity_literal() {
    let test_wj = r#"
fn test() {
    let mut data = Vec::with_capacity(10)
    data.push(42)
}
"#;

    let (rust_code, err) = test_utils::compile_via_cli_with_stderr(test_wj);
    assert!(
        err.is_empty() && !rust_code.is_empty(),
        "Compilation failed: {err}"
    );

    assert!(
        rust_code.contains("with_capacity(10_usize)") || rust_code.contains("with_capacity(10)"),
        "Vec::with_capacity should use usize or plain 10\nGenerated:\n{rust_code}"
    );
    assert!(
        !rust_code.contains("with_capacity(10_i32)"),
        "Should not use: with_capacity(10_i32)\n{rust_code}"
    );
}

#[test]
fn test_vec_with_capacity_variable() {
    let test_wj = r#"
fn test() {
    let size: int = 10
    let mut data = Vec::with_capacity(size)
    data.push(42)
}
"#;

    let (rust_code, err) = test_utils::compile_via_cli_with_stderr(test_wj);
    assert!(
        err.is_empty() && !rust_code.is_empty(),
        "Compilation failed: {err}"
    );

    assert!(rust_code.contains("with_capacity(") && rust_code.contains("as usize"));
}

#[test]
fn test_vec_push_float_unification() {
    let test_wj = r#"
fn test(alpha: f64) {
    let mut data = Vec::new()
    data.push(alpha)
    data.push(0.5)
    data.push(32.0)
}
"#;

    let (rust_code, err) = test_utils::compile_via_cli_with_stderr(test_wj);
    assert!(
        err.is_empty() && !rust_code.is_empty(),
        "Compilation failed: {err}"
    );

    assert!(
        (rust_code.contains("0.5_f64") || (rust_code.contains("0.5") && rust_code.contains("f64")))
            && rust_code.contains("push("),
        "literals should match f64 context\n{rust_code}"
    );
    let has_f32 = rust_code.contains("_f32");
    let has_f64 = rust_code.contains("_f64");
    if has_f32 && has_f64 {
        panic!("mixed f32/f64 in one Vec: {rust_code}");
    }
}

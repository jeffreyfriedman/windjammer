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

/// TDD Test: Float inference for method return types in binary operations
///
/// Bug: t.sin() * 0.8 where t: f32 generates 0.8_f64 instead of 0.8_f32
/// Pattern: MethodCall with f32 return type used in binary operation with float literal
/// Expected: Literal should be constrained to method's return type (f32)
///
/// Example from windjammer-game:
/// ```windjammer
/// pub fn set_time_of_day(self, hours: f32) {
///     let t = (hours - 6.0) / 12.0
///     let elev = t.sin() * 0.8  // 0.8 should be f32, not f64
/// }
/// ```
#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_method_return_in_binary_op_simple() {
    let source = r#"pub fn compute(t: f32) -> f32 {
    t.sin() * 0.8
}
"#;

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    // Should generate 0.8_f32 since sin() returns f32
    assert!(
        output.contains("0.8_f32"),
        "Expected '0.8_f32' in generated code, got:\n{}",
        output
    );

    // Should NOT generate 0.8_f64
    assert!(
        !output.contains("0.8_f64"),
        "Should not contain '0.8_f64', but it does:\n{}",
        output
    );
}

#[test]
fn test_method_return_chained() {
    let source = r#"pub fn compute(t: f32) -> f32 {
    t.sin().abs() * 0.5
}
"#;

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    // Chained methods: sin() returns f32, abs() returns f32
    assert!(
        output.contains("0.5_f32"),
        "Expected '0.5_f32' in generated code"
    );

    assert!(
        !output.contains("_f64"),
        "Should not contain any '_f64' literals:\n{}",
        output
    );
}

#[test]
fn test_method_return_complex_expr() {
    let source = r#"pub fn compute(x: f32, y: f32) -> f32 {
    (x.sin() * 2.0 + y.cos() * 3.0) * 0.5
}
"#;

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    // All literals should be f32
    assert!(
        output.contains("2.0_f32"),
        "Expected '2.0_f32' in generated code"
    );
    assert!(
        output.contains("3.0_f32"),
        "Expected '3.0_f32' in generated code"
    );
    assert!(
        output.contains("0.5_f32"),
        "Expected '0.5_f32' in generated code"
    );

    // Should NOT generate any f64
    assert!(
        !output.contains("_f64"),
        "Should not contain any '_f64' literals:\n{}",
        output
    );
}

#[test]
fn test_method_return_with_let() {
    let source = r#"pub fn compute(t: f32) {
    let elev = t.sin() * 0.8
    let azim = t.cos() * 3.14159
}
"#;

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    // Both literals should be f32
    assert!(
        output.contains("0.8_f32"),
        "Expected '0.8_f32' in generated code"
    );
    assert!(
        output.contains("3.14159_f32"),
        "Expected '3.14159_f32' in generated code"
    );

    assert!(
        !output.contains("_f64"),
        "Should not contain any '_f64' literals:\n{}",
        output
    );
}

// Helper function to compile Windjammer source and get generated Rust

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

/// TDD Test: If/Else Arm Type Unification
///
/// Bug: if/else branches with float literals don't unify types
/// Pattern: if condition { f32_expr } else { 0.0 } generates 0.0_f64
/// Root Cause: Else branch literal not constrained by if branch type
/// Expected: All arms should infer to same type
#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_if_else_with_literal_in_else() {
    let source = r#"
pub fn normalize(len: f32) -> f32 {
    if len > 0.0 {
        1.0 / len
    } else {
        0.0
    }
}
"#;

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    // The else branch 0.0 should be f32 to match if branch (returns f32)
    assert!(
        output.contains("} else { 0.0_f32 }") || output.contains("else {\n        0.0_f32"),
        "Expected '0.0_f32' in else branch, got: {}",
        output
    );
    assert!(
        !output.contains("0.0_f64"),
        "Should not contain '_f64': {}",
        output
    );
}

#[test]
fn test_if_else_both_literals() {
    let source = r#"
pub fn clamp_zero_one(x: f32) -> f32 {
    if x < 0.0 {
        0.0
    } else if x > 1.0 {
        1.0
    } else {
        x
    }
}
"#;

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    // All literals should be f32 (return type is f32, param x is f32)
    assert!(
        !output.contains("_f64"),
        "Should not contain '_f64': {}",
        output
    );
}

#[test]
fn test_if_else_expression_vs_literal() {
    let source = r#"
pub fn safe_divide(a: f32, b: f32) -> f32 {
    if b != 0.0 {
        a / b
    } else {
        0.0
    }
}
"#;

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    // else branch 0.0 should match if branch (a / b is f32)
    assert!(
        output.contains("0.0_f32"),
        "Expected '0.0_f32', got: {}",
        output
    );
}

#[test]
fn test_nested_if_else_literals() {
    let source = r#"
pub fn compute(x: f32) -> f32 {
    if x < 0.0 {
        if x < -10.0 {
            -10.0
        } else {
            x
        }
    } else {
        0.0
    }
}
"#;

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    // All literals should be f32
    assert!(
        !output.contains("_f64"),
        "Should not contain '_f64': {}",
        output
    );
}

#[test]
fn test_if_without_else() {
    let source = r#"
pub fn maybe_zero(x: f32) -> f32 {
    let mut result = x
    if result < 0.0 {
        result = 0.0
    }
    result
}
"#;

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    // All literals should be f32
    assert!(
        !output.contains("_f64"),
        "Should not contain '_f64': {}",
        output
    );
}

// Helper function to compile Windjammer code and return generated Rust

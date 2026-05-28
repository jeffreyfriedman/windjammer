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

/// TDD Test: Float inference for match arm unification
///
/// Bug: Match arms don't unify float literal types
/// Pattern: match Some(v) => *v (f32), None => literal defaults to f64
/// Expected: Both arms should have same type (unify to f32)
///
/// Example from breach-protocol:
/// ```windjammer
/// let g_score: HashMap<(i32, i32), f32> = HashMap::new();
/// let score = match g_score.get(&(x, y)) {
///     Some(v) => *v,       // Returns f32
///     None => 999999.0,    // Should be f32, generates f64!
/// };
/// ```
///
/// Error: error[E0308]: `match` arms have incompatible types
///        expected `f32`, found `f64`
#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_match_option_with_f32() {
    let source = r#"use std::collections::HashMap

pub fn get_score(scores: HashMap<i32, f32>, key: i32) -> f32 {
    match scores.get(&key) {
        Some(v) => *v,
        None => 999999.0,
    }
}
"#;

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    // The None arm literal should be f32 (matches Some arm type)
    assert!(
        output.contains("999999.0_f32") || output.contains("999999_f32"),
        "Expected '999999.0_f32' (to match Some arm type f32), got: {}",
        output
    );
    assert!(
        !output.contains("999999.0_f64") && !output.contains("999999_f64"),
        "Should not contain '_f64' in None arm: {}",
        output
    );
}

#[test]
fn test_match_result_with_f32() {
    // TDD: Testing literal patterns in match (0.0 => ...)
    let source = r#"pub fn safe_divide(a: f32, b: f32) -> f32 {
    match b {
        0.0 => 0.0,
        _ => a / b,
    }
}
"#;

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    // The 0.0 literals should be f32 (param types)
    assert!(
        output.contains("0.0_f32"),
        "Expected '0.0_f32' in match arms"
    );
}

#[test]
fn test_match_different_literals() {
    let source = r#"pub fn clamp(x: f32) -> f32 {
    match x {
        v if v < 0.0 => 0.0,
        v if v > 1.0 => 1.0,
        v => v,
    }
}
"#;

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    // All literals should be f32 (param/return type)
    assert!(
        output.contains("0.0_f32") && output.contains("1.0_f32"),
        "Expected f32 literals in all match arms"
    );
}

#[test]
fn test_match_with_default_value() {
    let source = r#"pub struct Point {
    pub x: f32,
    pub y: f32,
}

pub fn get_x_or_default(maybe_point: Option<Point>) -> f32 {
    match maybe_point {
        Some(p) => p.x,
        None => -1.0,
    }
}
"#;

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    // The -1.0 should be f32 (matches field type and return type)
    assert!(
        output.contains("-1.0_f32") || output.contains("- 1.0_f32"),
        "Expected '-1.0_f32' in None arm"
    );
}

// Helper function

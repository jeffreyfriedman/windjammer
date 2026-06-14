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

/// TDD Test: Match Wildcard Arm Type Unification
///
/// Bug: Wildcard (_) match arm literal doesn't unify with other arms
/// Pattern: match x { Some(y) => y, _ => 0.0 } generates 0.0_f64
/// Root Cause: Wildcard pattern not included in arm unification
/// Expected: All match arms (including _) should infer to same type
#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_match_option_with_wildcard_literal() {
    let source = r#"
pub fn get_value(opt: Option<f32>) -> f32 {
    match opt {
        Some(v) => v,
        _ => 0.0,
    }
}
"#;

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    // The wildcard arm 0.0 should be f32
    assert!(
        output.contains("0.0_f32"),
        "Expected '0.0_f32' in wildcard arm, got: {}",
        output
    );
    assert!(
        !output.contains("0.0_f64"),
        "Should not contain '0.0_f64': {}",
        output
    );
}

#[test]
fn test_match_result_with_wildcard() {
    let source = r#"
pub fn unwrap_or_zero(res: Result<f32, string>) -> f32 {
    match res {
        Ok(val) => val,
        _ => 0.0,
    }
}
"#;

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    assert!(
        output.contains("0.0_f32"),
        "Expected '0.0_f32', got: {}",
        output
    );
}

#[test]
fn test_match_enum_with_wildcard() {
    let source = r#"
enum Node {
    Lerp { factor: f32 },
    Additive { factor: f32 },
    Identity,
}

pub fn get_factor(node: Node) -> f32 {
    match node {
        Node::Lerp { factor } => factor,
        Node::Additive { factor } => factor,
        _ => 0.0,
    }
}
"#;

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    assert!(
        output.contains("0.0_f32"),
        "Expected '0.0_f32', got: {}",
        output
    );
}

#[test]
fn test_match_wildcard_with_different_literal() {
    let source = r#"
pub fn get_value_or_one(opt: Option<f32>) -> f32 {
    match opt {
        Some(v) => v,
        _ => 1.0,
    }
}
"#;

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    assert!(
        output.contains("1.0_f32"),
        "Expected '1.0_f32', got: {}",
        output
    );
}

#[test]
fn test_match_none_explicit_vs_wildcard() {
    let source = r#"
pub fn explicit_none(opt: Option<f32>) -> f32 {
    match opt {
        Some(v) => v,
        None => 0.0,
    }
}

pub fn wildcard_none(opt: Option<f32>) -> f32 {
    match opt {
        Some(v) => v,
        _ => 0.0,
    }
}
"#;

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    // Both explicit None and wildcard _ should infer to f32
    assert!(
        output.matches("0.0_f32").count() >= 2,
        "Expected at least 2 instances of '0.0_f32', got: {}",
        output
    );
}

// Helper function to compile Windjammer code and return generated Rust

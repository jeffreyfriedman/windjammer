/// TDD Test: If/Else Arm Type Unification
///
/// Bug: if/else branches with float literals don't unify types
/// Pattern: if condition { f32_expr } else { 0.0 } generates 0.0_f64
/// Root Cause: Else branch literal not constrained by if branch type
/// Expected: All arms should infer to same type
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

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

    let output = compile_and_get_rust(source);

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

    let output = compile_and_get_rust(source);

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

    let output = compile_and_get_rust(source);

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

    let output = compile_and_get_rust(source);

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

    let output = compile_and_get_rust(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    // All literals should be f32
    assert!(
        !output.contains("_f64"),
        "Should not contain '_f64': {}",
        output
    );
}

// Helper function to compile Windjammer code and return generated Rust
fn compile_and_get_rust(source: &str) -> String {
    let _ = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let source_file = tmp.path().join("test.wj");
    std::fs::write(&source_file, source).unwrap();

    windjammer::build_project(
        &source_file,
        tmp.path(),
        windjammer::CompilationTarget::Rust,
        false,
    )
    .expect("Failed to run wj compiler");

    std::fs::read_to_string(tmp.path().join("test.rs")).expect("Failed to read generated Rust file")
}

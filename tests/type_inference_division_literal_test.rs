/// TDD Test: Division with Literal Numerator
///
/// Bug: let inv_det = 1.0 / det generates 1.0_f64 instead of matching det type
/// Pattern: literal / variable should infer literal from variable type
/// Root Cause: Binary ops not constraining literals to match typed operands
/// Expected: 1.0 should be f32 if det is f32

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn test_one_divided_by_variable() {
    let source = r#"
pub fn reciprocal(x: f32) -> f32 {
    1.0 / x
}
"#;

    let output = compile_and_get_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    // 1.0 should be f32 (to match x parameter)
    assert!(
        output.contains("1.0_f32 / x"),
        "Expected '1.0_f32', got: {}",
        output
    );
}

#[test]
fn test_literal_divided_by_field() {
    let source = r#"
struct Config {
    scale: f32,
}

impl Config {
    pub fn get_inverse() -> f32 {
        1.0 / self.scale
    }
}
"#;

    let output = compile_and_get_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    assert!(
        output.contains("1.0_f32"),
        "Expected '1.0_f32', got: {}",
        output
    );
}

#[test]
fn test_inv_det_pattern() {
    // This is the EXACT pattern from mat4.wj
    let source = r#"
pub fn invert(det: f32) -> f32 {
    let inv_det = 1.0 / det
    inv_det
}
"#;

    let output = compile_and_get_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    // 1.0 should be f32, inv_det should be f32
    assert!(
        output.contains("1.0_f32"),
        "Expected '1.0_f32', got: {}",
        output
    );
    assert!(
        !output.contains("1.0_f64"),
        "Should not contain '_f64': {}",
        output
    );
}

#[test]
fn test_inv_det_used_in_multiplication() {
    // Full mat4 inverse pattern
    let source = r#"
struct Result {
    value: f32,
}

pub fn compute(det: f32, c00: f32) -> Result {
    let inv_det = 1.0 / det
    Result {
        value: c00 * inv_det,
    }
}
"#;

    let output = compile_and_get_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    // 1.0 should be f32, and c00 * inv_det should work
    assert!(
        output.contains("1.0_f32"),
        "Expected '1.0_f32', got: {}",
        output
    );
    assert!(
        !output.contains("1.0_f64"),
        "Should not contain '_f64': {}",
        output
    );
}

// Helper function to compile Windjammer code and return generated Rust
fn compile_and_get_rust(source: &str) -> String {
    let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let test_dir = format!("/tmp/division_literal_test_{}_{}", std::process::id(), counter);
    
    std::fs::create_dir_all(&test_dir).unwrap();
    
    let source_file = PathBuf::from(&test_dir).join("test.wj");
    std::fs::write(&source_file, source).unwrap();
    
    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(&[
            "build",
            source_file.to_str().unwrap(),
            "--target", "rust",
            "--output", &test_dir,
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj compiler");
    
    assert!(
        output.status.success(),
        "Compilation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    
    let rust_file = PathBuf::from(&test_dir).join("test.rs");
    std::fs::read_to_string(&rust_file)
        .expect("Failed to read generated Rust file")
}

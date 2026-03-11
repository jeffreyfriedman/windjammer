/// TDD Test: Mat4 Inverse Pattern (Complex Real-World Case)
///
/// Bug: Mat4::inverse() generates 1.0_f64 / det instead of 1.0_f32
/// Pattern: Complex method with self parameter, field access, and division
/// Root Cause: Variable type inference not propagating through complex chains
/// Expected: det should be f32 (all Mat4 fields are f32)

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn test_mat4_simplified_inverse_pattern() {
    let source = r#"
pub struct Mat4 {
    pub m00: f32, pub m01: f32,
    pub m10: f32, pub m11: f32,
}

impl Mat4 {
    pub fn inverse() -> Mat4 {
        let a = self.m00
        let b = self.m01
        let c = self.m10
        let d = self.m11
        
        // Determinant: ad - bc
        let det = a * d - b * c
        
        // Reciprocal of determinant
        let inv_det = 1.0 / det
        
        // Inverted matrix (swapped diagonal, negated off-diagonal, scaled by inv_det)
        Mat4 {
            m00: d * inv_det,
            m01: 0.0 - (b * inv_det),
            m10: 0.0 - (c * inv_det),
            m11: a * inv_det,
        }
    }
}
"#;

    let output = compile_and_get_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    // All fields are f32, so det should be f32, and 1.0 should be f32
    assert!(
        output.contains("1.0_f32 / det") || output.contains("1.0_f32/det"),
        "Expected '1.0_f32 / det', got:\n{}",
        output
    );
    assert!(
        !output.contains("1.0_f64"),
        "Should not contain '1.0_f64':\n{}",
        output
    );
}

#[test]
fn test_det_variable_type_from_fields() {
    let source = r#"
pub struct Mat4 {
    pub m00: f32, pub m01: f32,
}

impl Mat4 {
    pub fn compute() -> f32 {
        let a = self.m00
        let b = self.m01
        let det = a * b
        det
    }
}
"#;

    let output = compile_and_get_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    // det should be inferred as f32
    // This verifies the variable declaration gets the right type
    assert!(
        !output.contains("_f64"),
        "Should not contain '_f64':\n{}",
        output
    );
}

#[test]
fn test_division_after_field_multiplication() {
    let source = r#"
pub struct Data {
    pub x: f32,
    pub y: f32,
}

impl Data {
    pub fn ratio() -> f32 {
        let product = self.x * self.y
        1.0 / product
    }
}
"#;

    let output = compile_and_get_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    assert!(
        output.contains("1.0_f32"),
        "Expected '1.0_f32', got:\n{}",
        output
    );
}

// Helper function to compile Windjammer code and return generated Rust
fn compile_and_get_rust(source: &str) -> String {
    let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let test_dir = format!("/tmp/mat4_inverse_test_{}_{}", std::process::id(), counter);
    
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

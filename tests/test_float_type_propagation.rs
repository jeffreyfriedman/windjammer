//! TDD: Float literal type propagation in binary operations
//!
//! Goal: Propagate types through expressions so `pos.x + 10.0` infers `10.0` as f32
//! when `pos.x` is f32.
//!
//! Root cause: Float literals in binary ops default to f64, causing E0277/E0308
//! when mixed with f32 operands. Fix: Constraint-based inference propagates
//! known types (FieldAccess, Identifier, etc.) to float literals via MustMatch.

use std::path::PathBuf;
use tempfile::TempDir;
use windjammer::{build_project, CompilationTarget};

fn compile_to_rust(source: &str) -> Result<String, String> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_dir = temp_dir.path();
    let input_file = test_dir.join("test.wj");
    let output_dir = test_dir.join("build");

    std::fs::create_dir_all(&output_dir).expect("Failed to create output dir");
    std::fs::write(&input_file, source).expect("Failed to write source file");

    build_project(&input_file, &output_dir, CompilationTarget::Rust, true)
        .map_err(|e| format!("Windjammer compilation failed: {}", e))?;

    let output_file = output_dir.join("test.rs");
    let rust_code = std::fs::read_to_string(&output_file)
        .map_err(|e| format!("Failed to read generated file: {}", e))?;

    Ok(rust_code)
}

fn verify_rust_compiles(rust_code: &str) -> Result<(), String> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_dir = temp_dir.path();
    let rust_file = test_dir.join("test.rs");
    std::fs::write(&rust_file, rust_code).expect("Failed to write Rust file");

    let check = std::process::Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            rust_file.to_str().unwrap(),
            "-o",
            test_dir.join("test.rlib").to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run rustc");

    if check.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&check.stderr).to_string())
    }
}

#[test]
fn test_binary_op_propagates_f32_to_literal() {
    let source = r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub fn move_player(pos: Vec3, delta: f32) -> Vec3 {
    Vec3 {
        x: pos.x + 10.0,
        y: pos.y + 5.0,
        z: pos.z + delta,
    }
}
"#;

    let rust_code = compile_to_rust(source).unwrap();

    // Should generate f32 literals
    assert!(
        rust_code.contains("10.0_f32") || rust_code.contains("10.0f32"),
        "pos.x + 10.0 should generate 10.0_f32, got:\n{}",
        rust_code
    );
    assert!(
        rust_code.contains("5.0_f32") || rust_code.contains("5.0f32"),
        "pos.y + 5.0 should generate 5.0_f32, got:\n{}",
        rust_code
    );

    verify_rust_compiles(&rust_code).expect("Generated Rust should compile");
}

#[test]
fn test_binary_op_propagates_f64_to_literal() {
    let source = r#"
pub fn calculate(timestamp: f64) -> f64 {
    timestamp + 1.0
}
"#;

    let rust_code = compile_to_rust(source).unwrap();
    assert!(
        rust_code.contains("1.0_f64") || rust_code.contains("1.0f64"),
        "timestamp + 1.0 where timestamp: f64 should generate 1.0_f64, got:\n{}",
        rust_code
    );
}

#[test]
fn test_assignment_propagates_type() {
    let source = r#"
pub fn test() {
    let mut x: f32 = 0.0
    x = x + 1.0
}
"#;

    let rust_code = compile_to_rust(source).unwrap();
    assert!(
        rust_code.contains("1.0_f32") || rust_code.contains("1.0f32"),
        "x = x + 1.0 where x: f32 should generate 1.0_f32, got:\n{}",
        rust_code
    );
    verify_rust_compiles(&rust_code).expect("Generated Rust should compile");
}

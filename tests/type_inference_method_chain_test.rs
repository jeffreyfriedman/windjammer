/// TDD Test: Method Chain Type Inference
///
/// Bug: (value + amount).min(100.0) generates 100.0_f64 instead of _f32
/// Root Cause: Method return type not propagating to chained method arguments
/// Expected: .min() and .max() arguments should match receiver type

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn test_min_with_field() {
    let source = r#"
struct Detection {
    level: f32,
}

impl Detection {
    pub fn increase(amount: f32) {
        self.level = (self.level + amount).min(100.0)
    }
}
"#;

    let output = compile_and_get_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    // The .min(100.0) argument should be f32
    assert!(
        output.contains("100.0_f32") || output.contains("100_f32"),
        "Expected '100.0_f32' in .min() call, got: {}",
        output
    );
    assert!(
        !output.contains("100.0_f64") && !output.contains("100_f64"),
        "Should not contain '_f64': {}",
        output
    );
}

#[test]
fn test_max_with_field() {
    let source = r#"
struct Detection {
    level: f32,
}

impl Detection {
    pub fn decrease(amount: f32) {
        self.level = (self.level - amount).max(0.0)
    }
}
"#;

    let output = compile_and_get_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    // The .max(0.0) argument should be f32
    assert!(
        output.contains("0.0_f32"),
        "Expected '0.0_f32' in .max() call, got: {}",
        output
    );
}

#[test]
fn test_clamp_with_min_max_chain() {
    let source = r#"
pub fn clamp(value: f32, min_val: f32, max_val: f32) -> f32 {
    value.max(min_val).min(max_val)
}
"#;

    let output = compile_and_get_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    // All parameters are f32, so method chain should use f32
    // (This test verifies the chain works, though params already constrain it)
    assert!(
        !output.contains("_f64"),
        "Should not contain any '_f64': {}",
        output
    );
}

#[test]
fn test_min_max_with_literal_receiver() {
    let source = r#"
pub fn get_clamped(x: f32) -> f32 {
    x.min(1.0).max(0.0)
}
"#;

    let output = compile_and_get_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    // Both literals should be f32 (param x is f32)
    assert!(
        output.contains("1.0_f32") && output.contains("0.0_f32"),
        "Expected both literals as f32, got: {}",
        output
    );
}

#[test]
fn test_min_max_without_param_context() {
    let source = r#"
pub fn example() -> f32 {
    let x = 5.0
    x.min(10.0).max(0.0)
}
"#;

    let output = compile_and_get_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    // Return type f32 should propagate backwards through chain
    assert!(
        output.contains("5.0_f32") && output.contains("10.0_f32") && output.contains("0.0_f32"),
        "Expected all literals as f32 (from return type), got: {}",
        output
    );
}

// Helper function to compile Windjammer code and return generated Rust
fn compile_and_get_rust(source: &str) -> String {
    let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let test_dir = format!("/tmp/method_chain_test_{}_{}", std::process::id(), counter);
    
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

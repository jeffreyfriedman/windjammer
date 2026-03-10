/// TDD Test: Float inference for mixed f32/f64 binary operations
///
/// Bug: Binary operations like `x * 0.5` generate mixed types (f32 * f64)
/// Pattern: One operand is correctly inferred as f32, but the other defaults to f64
/// Expected: Both operands should unify to the same type
///
/// Error Pattern from game (83 occurrences):
/// error[E0277]: cannot multiply `f32` by `f64`
///
/// Examples from breach-protocol:
/// - `self.x * 0.5` where `self.x: f32`
/// - `velocity * dt` where `velocity: Vec3` (f32 components), `dt: f64`
/// - `radius * 2.0` where `radius: f32`

use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_field_times_literal() {
    let source = r#"pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn scaled(self, factor: f32) -> Point {
        Point {
            x: self.x * 0.5,
            y: self.y * 2.0,
        }
    }
}
"#;

    let output = compile_and_get_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    // Both literals should be f32 (field types)
    assert!(
        output.contains("0.5_f32"),
        "Expected '0.5_f32' in: {}",
        output
    );
    assert!(
        output.contains("2.0_f32"),
        "Expected '2.0_f32' in: {}",
        output
    );
    assert!(
        !output.contains("_f64"),
        "Should not contain '_f64' in: {}",
        output
    );
}

#[test]
fn test_param_times_literal() {
    let source = r#"pub fn scale(x: f32) -> f32 {
    x * 2.0
}
"#;

    let output = compile_and_get_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    assert!(
        output.contains("2.0_f32"),
        "Expected '2.0_f32' (param is f32)"
    );
}

#[test]
fn test_variable_times_literal() {
    let source = r#"pub fn compute() -> f32 {
    let velocity: f32 = 10.0
    let dt = 0.016
    velocity * dt
}
"#;

    let output = compile_and_get_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    // dt should be inferred as f32 from velocity * dt
    assert!(
        output.contains("0.016_f32"),
        "Expected 'dt' literal to be f32"
    );
}

#[test]
fn test_chained_binary_ops() {
    let source = r#"pub fn area(width: f32, height: f32) -> f32 {
    width * height * 0.5
}
"#;

    let output = compile_and_get_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    assert!(
        output.contains("0.5_f32"),
        "Expected '0.5_f32' (chained with f32 params)"
    );
}

// Helper function to compile Windjammer source and get generated Rust
fn compile_and_get_rust(source: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    
    let temp_dir = std::env::temp_dir();
    let unique_id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let test_name = format!("mixed_float_test_{}_{}", std::process::id(), unique_id);
    let test_file = temp_dir.join(format!("{}.wj", test_name));
    let output_dir = temp_dir.join(&test_name);
    let output_file = output_dir.join(format!("{}.rs", test_name));
    
    // Write source to temporary file
    std::fs::write(&test_file, source).expect("Failed to write test file");
    
    // Compile with wj (use local build)
    let wj_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/release/wj");
    
    let status = Command::new(&wj_path)
        .arg("build")
        .arg(&test_file)
        .arg("--output")
        .arg(&output_dir)
        .arg("--no-cargo")
        .status()
        .expect("Failed to execute wj compiler");
    
    assert!(status.success(), "Compilation failed");
    
    // Read generated Rust
    let rust_code = std::fs::read_to_string(&output_file)
        .expect("Failed to read generated Rust file");
    
    // Cleanup
    let _ = std::fs::remove_file(&test_file);
    let _ = std::fs::remove_dir_all(&output_dir);
    
    rust_code
}

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

use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_method_return_in_binary_op_simple() {
    let source = r#"pub fn compute(t: f32) -> f32 {
    t.sin() * 0.8
}
"#;

    let output = compile_and_get_rust(source);
    
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

    let output = compile_and_get_rust(source);
    
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

    let output = compile_and_get_rust(source);
    
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

    let output = compile_and_get_rust(source);
    
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
fn compile_and_get_rust(source: &str) -> String {
    let temp_dir = std::env::temp_dir();
    let test_name = format!("method_return_test_{}", std::process::id());
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
        .arg("-o")
        .arg(&output_dir)
        .arg("--no-cargo")  // Skip cargo build to avoid dependency issues
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

/// TDD Test: Float inference for constant-folded expressions
///
/// Bug: 1.0 / 60.0 in struct literal generates f64 even when field is f32
/// Pattern: Binary operation gets constant-folded, but the folded value isn't constrained
/// Expected: Folded literal should match the struct field type (f32)
///
/// Example from windjammer-game:
/// ```windjammer
/// pub struct GameConfig {
///     pub timestep: f32,
/// }
/// pub fn create() -> GameConfig {
///     GameConfig { timestep: 1.0 / 60.0 }  // Should be f32, not f64
/// }
/// ```

use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_const_fold_in_struct_literal() {
    let source = r#"pub struct Config {
    pub timestep: f32,
}

pub fn create() -> Config {
    Config {
        timestep: 1.0 / 60.0,
    }
}
"#;

    let output = compile_and_get_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    // The constant-folded value should be f32
    assert!(
        output.contains("_f32") && !output.contains("_f64"),
        "Expected '_f32' suffix (not '_f64') in generated code:\n{}",
        output
    );
}

#[test]
fn test_const_fold_simple() {
    let source = r#"pub fn compute() -> f32 {
    1.0 / 2.0
}
"#;

    let output = compile_and_get_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    // Return type is f32, so folded value should be f32
    assert!(
        output.contains("_f32"),
        "Expected '_f32' in generated code"
    );
    assert!(
        !output.contains("_f64"),
        "Should not contain '_f64':\n{}",
        output
    );
}

// Helper function to compile Windjammer source and get generated Rust
fn compile_and_get_rust(source: &str) -> String {
    let temp_dir = std::env::temp_dir();
    let test_name = format!("const_fold_test_{}", std::process::id());
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

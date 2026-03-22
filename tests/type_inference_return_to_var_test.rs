/// TDD Test: Float inference through return type to variable assignment
///
/// Bug: Vec::new() with no type annotation, push tuples with floats, implicit return
/// Pattern: Return type constrains variable, variable constrains collection elements
/// Expected: Bidirectional flow from return type through Vec to tuple literals
///
/// Example from windjammer-game (astar_grid.wj):
/// ```windjammer
/// fn get_neighbors(self, x: i32, y: i32) -> Vec<(i32, i32, f32)> {
///     let mut result = Vec::new()  // No type annotation
///     result.push((x, y, 1.414))   // 1.414 should be f32 (from return type)
///     result                       // Implicit return
/// }
/// ```

use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_return_type_to_vec_simple() {
    let source = r#"pub fn make_vec() -> Vec<f32> {
    let mut result = Vec::new()
    result.push(1.414)
    result
}
"#;

    let output = compile_and_get_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    assert!(
        output.contains("1.414_f32"),
        "Expected '1.414_f32' in generated code, got:\n{}",
        output
    );
    assert!(
        !output.contains("1.414_f64"),
        "Should not contain '1.414_f64', but it does:\n{}",
        output
    );
}

#[test]
fn test_return_type_to_vec_tuple() {
    let source = r#"pub fn make_coords() -> Vec<(i32, i32, f32)> {
    let mut result = Vec::new()
    result.push((1, 2, 1.414))
    result.push((3, 4, 2.718))
    result
}
"#;

    let output = compile_and_get_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    assert!(
        output.contains("1.414_f32"),
        "Expected '1.414_f32' in generated code"
    );
    assert!(
        output.contains("2.718_f32"),
        "Expected '2.718_f32' in generated code"
    );
    assert!(
        !output.contains("_f64"),
        "Should not contain any '_f64' literals:\n{}",
        output
    );
}

#[test]
fn test_return_type_to_hashmap() {
    let source = r#"use std::collections::HashMap

pub fn make_scores() -> HashMap<i32, f32> {
    let mut result = HashMap::new()
    result.insert(1, 3.14)
    result.insert(2, 2.71)
    result
}
"#;

    let output = compile_and_get_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    assert!(
        output.contains("3.14_f32"),
        "Expected '3.14_f32' in generated code"
    );
    assert!(
        output.contains("2.71_f32"),
        "Expected '2.71_f32' in generated code"
    );
    assert!(
        !output.contains("_f64"),
        "Should not contain any '_f64' literals:\n{}",
        output
    );
}

#[test]
fn test_return_type_complex_tuple() {
    let source = r#"pub fn compute() -> Vec<(i32, i32, f32)> {
    let mut result = Vec::new()
    let x = 10
    let y = 20
    result.push((x, y, x as f32 * 1.5))
    result
}
"#;

    let output = compile_and_get_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    assert!(
        output.contains("1.5_f32"),
        "Expected '1.5_f32' in generated code:\n{}",
        output
    );
}

// Helper function to compile Windjammer source and get generated Rust
fn compile_and_get_rust(source: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    
    let temp_dir = std::env::temp_dir();
    let unique_id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let test_name = format!("return_to_var_test_{}_{}", std::process::id(), unique_id);
    let test_file = temp_dir.join(format!("{}.wj", test_name));
    let output_dir = temp_dir.join(&test_name);
    let output_file = output_dir.join(format!("{}.rs", test_name));
    
    std::fs::write(&test_file, source).expect("Failed to write test file");
    
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
    
    let rust_code = std::fs::read_to_string(&output_file)
        .expect("Failed to read generated Rust file");
    
    let _ = std::fs::remove_file(&test_file);
    let _ = std::fs::remove_dir_all(&output_dir);
    
    rust_code
}

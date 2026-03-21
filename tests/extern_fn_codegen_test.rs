// TDD Test: Windjammer Compiler Bug - extern fn not being transpiled
//
// Bug: extern fn declarations in .wj files are not appearing in generated .rs files
// Expected: extern fn should be transpiled to Rust's extern "C" { pub fn ... }
//
// Reproduction:
// 1. Create .wj file with: extern fn test_function(x: i32) -> i32
// 2. Transpile to Rust
// 3. Check generated .rs file contains: pub fn test_function(x: i32) -> i32

use std::fs;
use std::process::Command;

#[test]
fn test_extern_fn_transpiles_to_rust() {
    // Create a minimal .wj file with extern fn
    let test_wj = r#"
// Test extern function declarations
extern fn test_simple_function(x: i32) -> i32
extern fn test_no_return(value: f32)
extern fn test_multiple_params(a: u32, b: u32, c: f32) -> bool
extern fn test_string_param(path: string) -> u32
"#;
    
    let test_file = "/tmp/test_extern_fn.wj";
    fs::write(test_file, test_wj).expect("Failed to write test file");
    
    // Transpile with wj compiler
    let output = Command::new("./target/release/wj")
        .args(&["build", "--no-cargo", test_file])
        .output()
        .expect("Failed to run wj compiler");
    
    if !output.status.success() {
        eprintln!("Compiler output: {}", String::from_utf8_lossy(&output.stderr));
        panic!("Compilation failed");
    }
    
    // Read generated Rust file
    let rs_file = "/tmp/build/test_extern_fn.rs";
    let rust_code = fs::read_to_string(rs_file)
        .expect("Failed to read generated .rs file");
    
    // Verify extern functions are present
    assert!(rust_code.contains("extern \"C\""), 
        "Generated Rust should have extern \"C\" block");
    
    assert!(rust_code.contains("pub fn test_simple_function(x: i32) -> i32"),
        "Should transpile: extern fn test_simple_function(x: i32) -> i32");
    
    assert!(rust_code.contains("pub fn test_no_return(value: f32)"),
        "Should transpile: extern fn test_no_return(value: f32)");
    
    assert!(rust_code.contains("pub fn test_multiple_params(a: u32, b: u32, c: f32) -> bool"),
        "Should transpile: extern fn test_multiple_params(a: u32, b: u32, c: f32) -> bool");
    
    // String should become FfiString in Rust
    assert!(rust_code.contains("test_string_param") && rust_code.contains("FfiString"),
        "String parameters should become FfiString in generated Rust");
    
    // Cleanup
    let _ = fs::remove_file(test_file);
    let _ = fs::remove_dir_all("/tmp/build");
    
    println!("✅ extern fn transpilation test PASSED");
}

#[test]
fn test_extern_fn_in_extern_block() {
    // Verify that multiple extern fn declarations are grouped in a single extern "C" block
    let test_wj = r#"
extern fn func_a(x: i32) -> i32
extern fn func_b(y: f32) -> f32
extern fn func_c()
"#;
    
    let test_file = "/tmp/test_extern_block.wj";
    fs::write(test_file, test_wj).expect("Failed to write test file");
    
    let output = Command::new("./target/release/wj")
        .args(&["build", "--no-cargo", test_file])
        .output()
        .expect("Failed to run wj compiler");
    
    if !output.status.success() {
        panic!("Compilation failed");
    }
    
    let rs_file = "/tmp/build/test_extern_block.rs";
    let rust_code = fs::read_to_string(rs_file)
        .expect("Failed to read generated .rs file");
    
    // Should have exactly one extern "C" block containing all functions
    let extern_count = rust_code.matches("extern \"C\"").count();
    assert_eq!(extern_count, 1, "Should have exactly one extern \"C\" block");
    
    // All functions should be inside it
    assert!(rust_code.contains("pub fn func_a"),
        "func_a should be in extern block");
    assert!(rust_code.contains("pub fn func_b"),
        "func_b should be in extern block");
    assert!(rust_code.contains("pub fn func_c"),
        "func_c should be in extern block");
    
    // Cleanup
    let _ = fs::remove_file(test_file);
    let _ = fs::remove_dir_all("/tmp/build");
    
    println!("✅ extern block grouping test PASSED");
}

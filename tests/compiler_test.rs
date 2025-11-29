// Comprehensive Compiler Test Suite
// This ensures we can safely refactor the compiler without breaking functionality

use std::fs;
use std::path::Path;
use std::process::Command;

/// Test helper: Compile a .wj file and compare output with expected .rs file
fn test_codegen(test_name: &str) {
    let test_dir = Path::new("tests/codegen");
    let input_file = test_dir.join(format!("{}.wj", test_name));
    let expected_file = test_dir.join(format!("{}.expected.rs", test_name));
    
    assert!(input_file.exists(), "Test file not found: {:?}", input_file);
    assert!(expected_file.exists(), "Expected file not found: {:?}", expected_file);
    
    // Compile the Windjammer file
    let output = Command::new("cargo")
        .args(&["run", "--bin", "wj", "--", "build", input_file.to_str().unwrap(), "--no-cargo"])
        .output()
        .expect("Failed to run wj compiler");
    
    assert!(output.status.success(), "Compilation failed: {}", String::from_utf8_lossy(&output.stderr));
    
    // Read generated output
    let generated_file = Path::new("build").join(format!("{}.rs", test_name));
    let generated = fs::read_to_string(&generated_file)
        .expect("Failed to read generated file");
    
    // Read expected output
    let expected = fs::read_to_string(&expected_file)
        .expect("Failed to read expected file");
    
    // Normalize whitespace for comparison (ignore formatting differences)
    let normalize = |s: &str| s.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    
    let generated_normalized = normalize(&generated);
    let expected_normalized = normalize(&expected);
    
    if generated_normalized != expected_normalized {
        eprintln!("=== GENERATED ===\n{}\n", generated);
        eprintln!("=== EXPECTED ===\n{}\n", expected);
        panic!("Generated code does not match expected output for test: {}", test_name);
    }
}

/// Test helper: Verify that generated code compiles with Rust compiler
fn test_rust_compiles(test_name: &str) {
    let generated_file = Path::new("build").join(format!("{}.rs", test_name));
    
    // Create a temporary Cargo project
    let temp_dir = std::env::temp_dir().join(format!("wj_test_{}", test_name));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();
    
    // Copy generated file
    fs::copy(&generated_file, temp_dir.join("lib.rs")).unwrap();
    
    // Create Cargo.toml
    let cargo_toml = r#"
[package]
name = "test"
version = "0.1.0"
edition = "2021"

[lib]
path = "lib.rs"
"#;
    fs::write(temp_dir.join("Cargo.toml"), cargo_toml).unwrap();
    
    // Compile with Rust
    let output = Command::new("cargo")
        .args(&["build", "--lib"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to run cargo");
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Generated Rust code failed to compile for test {}: \n{}", test_name, stderr);
    }
    
    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_implicit_return_after_let() {
    test_codegen("implicit_return_after_let");
    test_rust_compiles("implicit_return_after_let");
}

#[test]
fn test_basic_struct() {
    test_codegen("basic_struct");
    test_rust_compiles("basic_struct");
}

#[test]
fn test_basic_enum() {
    test_codegen("basic_enum");
    test_rust_compiles("basic_enum");
}

#[test]
fn test_trait_impl() {
    test_codegen("trait_impl");
    test_rust_compiles("trait_impl");
}

#[test]
fn test_generic_function() {
    test_codegen("generic_function");
    test_rust_compiles("generic_function");
}

#[test]
fn test_ownership_inference() {
    test_codegen("ownership_inference");
    test_rust_compiles("ownership_inference");
}

#[test]
fn test_auto_mut_inference() {
    test_codegen("auto_mut_inference");
    test_rust_compiles("auto_mut_inference");
}

#[test]
fn test_builder_pattern() {
    test_codegen("builder_pattern");
    test_rust_compiles("builder_pattern");
}

#[test]
fn test_auto_derive() {
    test_codegen("auto_derive");
    test_rust_compiles("auto_derive");
}

#[test]
fn test_mod_support() {
    test_codegen("mod_support");
    test_rust_compiles("mod_support");
}

#[test]
fn test_extern_fn() {
    test_codegen("extern_fn");
    test_rust_compiles("extern_fn");
}

#[test]
fn test_generic_extern_fn() {
    test_codegen("generic_extern_fn");
    test_rust_compiles("generic_extern_fn");
}



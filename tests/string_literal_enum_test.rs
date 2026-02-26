//! Test: String literal to String conversion in enum construction
//! Bug: Codegen produces `Speaker::NPC("string")` which generates `&str`
//! Fix: Should generate `Speaker::NPC("string".to_string())` or `Speaker::NPC(String::from("string"))`

use std::process::Command;
use std::fs;
use std::path::PathBuf;

#[test]
fn test_string_literal_enum_auto_convert() {
    // Compile the test file
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let wj_binary = PathBuf::from(manifest_dir).join("target/release/wj");
    let test_file = PathBuf::from(manifest_dir).join("tests/string_literal_enum_test.wj");
    let output_dir = PathBuf::from("/tmp/test_string_literal_enum");
    
    // Clean output directory
    let _ = fs::remove_dir_all(&output_dir);
    fs::create_dir_all(&output_dir).unwrap();
    
    // Run wj build
    let output = Command::new(&wj_binary)
        .args(["build", test_file.to_str().unwrap(), "--output", output_dir.to_str().unwrap(), "--target", "rust"])
        .output()
        .expect("Failed to run wj build");
    
    if !output.status.success() {
        panic!("wj build failed:\n{}", String::from_utf8_lossy(&output.stderr));
    }
    
    // Read generated Rust file
    let generated_rs = output_dir.join("string_literal_enum_test.rs");
    let generated_code = fs::read_to_string(&generated_rs)
        .expect("Failed to read generated Rust file");
    
    println!("Generated code:\n{}", generated_code);
    
    // Verify string literal is converted to String
    // Should contain either:
    // - Speaker::NPC("Alice".to_string())
    // - Speaker::NPC(String::from("Alice"))
    assert!(
        generated_code.contains("\"Alice\".to_string()") || 
        generated_code.contains("String::from(\"Alice\")"),
        "Generated code should convert string literal to String"
    );
    
    // Verify it compiles with rustc
    let compile_output = Command::new("rustc")
        .args(["--crate-type", "bin", "--edition", "2021"])
        .arg(&generated_rs)
        .arg("-o")
        .arg(output_dir.join("test_binary"))
        .output()
        .expect("Failed to run rustc");
    
    if !compile_output.status.success() {
        panic!("rustc failed:\n{}", String::from_utf8_lossy(&compile_output.stderr));
    }
}

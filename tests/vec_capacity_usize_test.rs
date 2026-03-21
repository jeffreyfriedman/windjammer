// TDD Test: Vec::with_capacity should accept usize, not i32
//
// Bug: Vec::with_capacity(10) generates with_capacity(10_i32)
// Rust signature: Vec::with_capacity(capacity: usize)
//
// Fix: When method parameter is usize, constrain int literals to usize

use std::fs;
use std::process::Command;

#[test]
fn test_vec_with_capacity_literal() {
    let test_wj = r#"
fn test() {
    let mut data = Vec::with_capacity(10)
    data.push(42)
}
"#;
    
    let test_file = "/tmp/test_vec_capacity.wj";
    fs::write(test_file, test_wj).expect("Failed to write test file");
    
    let output = Command::new("./target/release/wj")
        .args(&["build", test_file, "-o", "./build", "--no-cargo"])
        .output()
        .expect("Failed to run wj compiler");
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Compilation failed: {}", stderr);
    }
    
    let rs_file = "./build/test_vec_capacity.rs";
    let rust_code = fs::read_to_string(rs_file)
        .expect("Failed to read generated .rs file");
    
    println!("Generated Rust:\n{}", rust_code);
    
    // Should generate with_capacity(10_usize) or with_capacity(10)
    // Rust infers literals as usize from signature context
    assert!(
        rust_code.contains("with_capacity(10_usize)") || 
        rust_code.contains("with_capacity(10)"),
        "Vec::with_capacity should accept usize literal\nGenerated:\n{}", 
        rust_code
    );
    
    // Should NOT generate with_capacity(10_i32)
    assert!(
        !rust_code.contains("with_capacity(10_i32)"),
        "Should NOT generate: with_capacity(10_i32)\nGenerated:\n{}", 
        rust_code
    );
    
    // Cleanup
    let _ = fs::remove_file(test_file);
    
    println!("✅ Vec::with_capacity usize test PASSED");
}

#[test]
fn test_vec_with_capacity_variable() {
    let test_wj = r#"
fn test(size: int) {
    let mut data = Vec::with_capacity(size)
    data.push(42)
}
"#;
    
    let test_file = "/tmp/test_vec_capacity_var.wj";
    fs::write(test_file, test_wj).expect("Failed to write test file");
    
    let output = Command::new("./target/release/wj")
        .args(&["build", test_file, "-o", "./build", "--no-cargo"])
        .output()
        .expect("Failed to run wj compiler");
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Compilation failed: {}", stderr);
    }
    
    let rs_file = "./build/test_vec_capacity_var.rs";
    let rust_code = fs::read_to_string(rs_file)
        .expect("Failed to read generated .rs file");
    
    println!("Generated Rust:\n{}", rust_code);
    
    // Should cast variable to usize
    assert!(
        rust_code.contains("with_capacity(size as usize)"),
        "Vec::with_capacity should cast int parameter to usize\nGenerated:\n{}", 
        rust_code
    );
    
    // Cleanup
    let _ = fs::remove_file(test_file);
    
    println!("✅ Vec::with_capacity with variable test PASSED");
}

#[test]
fn test_vec_push_float_unification() {
    let test_wj = r#"
fn test(alpha: f64) {
    let mut data = Vec::new()
    data.push(alpha)
    data.push(0.5)
    data.push(32.0)
}
"#;
    
    let test_file = "/tmp/test_vec_push_float.wj";
    fs::write(test_file, test_wj).expect("Failed to write test file");
    
    let output = Command::new("./target/release/wj")
        .args(&["build", test_file, "-o", "./build", "--no-cargo"])
        .output()
        .expect("Failed to run wj compiler");
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Compilation failed: {}", stderr);
    }
    
    let rs_file = "./build/test_vec_push_float.rs";
    let rust_code = fs::read_to_string(rs_file)
        .expect("Failed to read generated .rs file");
    
    println!("Generated Rust:\n{}", rust_code);
    
    // All literals should be f64 to match the first push(alpha: f64)
    assert!(
        rust_code.contains("0.5_f64") || rust_code.contains("0.5") && rust_code.contains("push(alpha)"),
        "Vec::push literals should unify to first element type (f64)\nGenerated:\n{}", 
        rust_code
    );
    
    // Should NOT have mixed f32/f64
    let has_f32 = rust_code.contains("_f32");
    let has_f64 = rust_code.contains("_f64") || rust_code.contains("alpha");
    assert!(
        !(has_f32 && has_f64),
        "Should NOT mix f32 and f64 in same Vec\nGenerated:\n{}", 
        rust_code
    );
    
    // Cleanup
    let _ = fs::remove_file(test_file);
    
    println!("✅ Vec::push float unification test PASSED");
}

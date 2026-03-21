// TDD Test: Range type unification in for loops
//
// Bug: for i in 0..vec.len() generates 0_i32..vec.len() (usize)
// This creates a Range<T> where T is ambiguous (i32 vs usize)
//
// Fix: When range bounds have different integer types, unify them to a common type
//      Prefer usize for ranges ending with .len()

use std::fs;
use std::process::Command;

#[test]
fn test_range_with_vec_len() {
    let test_wj = r#"
fn test(items: Vec<i32>) {
    for i in 0..items.len() {
        println!("{}", i)
    }
}
"#;
    
    let test_file = "/tmp/test_range_len.wj";
    fs::write(test_file, test_wj).expect("Failed to write test file");
    
    let output = Command::new("./target/release/wj")
        .args(&["build", test_file, "-o", "./build", "--no-cargo"])
        .output()
        .expect("Failed to run wj compiler");
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Compilation failed: {}", stderr);
    }
    
    let rs_file = "./build/test_range_len.rs";
    let rust_code = fs::read_to_string(rs_file)
        .expect("Failed to read generated .rs file");
    
    println!("Generated Rust:\n{}", rust_code);
    
    // Verify range has unified types (both usize)
    assert!(
        rust_code.contains("0_usize..items.len()") ||
        rust_code.contains("0..items.len()"), // Rust infers 0 as usize from context
        "Should unify range types: 0_usize..items.len() or 0..items.len()\nGenerated:\n{}", 
        rust_code
    );
    
    // Verify does NOT generate mismatched types
    assert!(
        !rust_code.contains("0_i32..items.len()"),
        "Should NOT generate: 0_i32..items.len() (type mismatch)\nGenerated:\n{}", 
        rust_code
    );
    
    // Cleanup
    let _ = fs::remove_file(test_file);
    
    println!("✅ Range with vec.len() test PASSED - correct type unification!");
}

#[test]
fn test_range_with_field_len() {
    let test_wj = r#"
struct Container {
    items: Vec<String>
}

impl Container {
    fn process(self) {
        for i in 0..self.items.len() {
            println!("{}", i)
        }
    }
}
"#;
    
    let test_file = "/tmp/test_range_field_len.wj";
    fs::write(test_file, test_wj).expect("Failed to write test file");
    
    let output = Command::new("./target/release/wj")
        .args(&["build", test_file, "-o", "./build", "--no-cargo"])
        .output()
        .expect("Failed to run wj compiler");
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Compilation failed: {}", stderr);
    }
    
    let rs_file = "./build/test_range_field_len.rs";
    let rust_code = fs::read_to_string(rs_file)
        .expect("Failed to read generated .rs file");
    
    println!("Generated Rust:\n{}", rust_code);
    
    // Should unify to usize (matching .len() return type)
    assert!(
        rust_code.contains("0..self.items.len()") ||
        rust_code.contains("0_usize..self.items.len()"),
        "Should unify range types for field.len()\nGenerated:\n{}", 
        rust_code
    );
    
    // Should NOT have type mismatch
    assert!(
        !rust_code.contains("0_i32..self.items.len()"),
        "Should NOT generate: 0_i32..self.items.len()\nGenerated:\n{}", 
        rust_code
    );
    
    // Cleanup
    let _ = fs::remove_file(test_file);
    
    println!("✅ Range with field.len() test PASSED - correct type unification!");
}

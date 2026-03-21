// TDD Test: Array elements of Copy types should not get & when passed to functions
//
// Bug: roots[j] generates &roots[j as usize] when function expects u32 by value
// Root cause: Ownership inference adds & for array indexing regardless of target type
// Rust: u32 is Copy, function takes by value, no & needed
//
// Fix: Check if target parameter is Copy type before adding &

use std::fs;
use std::process::Command;

#[test]
fn test_array_element_function_call_copy_type() {
    let test_wj = r#"
fn process_id(id: u32) {
    println!("{}", id)
}

fn process_all(ids: [u32; 5]) {
    let mut i = 0
    while i < ids.len() {
        process_id(ids[i])  // Should generate: process_id(ids[i as usize]), NOT &ids[...]
        i = i + 1
    }
}
"#;
    
    let test_file = "/tmp/test_array_fn_call.wj";
    fs::write(test_file, test_wj).expect("Failed to write test file");
    
    let output = Command::new("./target/release/wj")
        .args(&["build", test_file, "-o", "./build", "--no-cargo"])
        .output()
        .expect("Failed to run wj compiler");
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Compilation failed: {}", stderr);
    }
    
    let rs_file = "./build/test_array_fn_call.rs";
    let rust_code = fs::read_to_string(rs_file)
        .expect("Failed to read generated .rs file");
    
    println!("Generated Rust:\n{}", rust_code);
    
    // Should NOT add & for Copy type array element
    assert!(
        !rust_code.contains("process_id(&ids["),
        "Should NOT add & for Copy type (u32) array element\nGenerated:\n{}", 
        rust_code
    );
    
    // Should pass by value
    assert!(
        rust_code.contains("process_id(ids["),
        "Should pass Copy type array element by value\nGenerated:\n{}", 
        rust_code
    );
    
    // Cleanup
    let _ = fs::remove_file(test_file);
    
    println!("✅ Array element function call (Copy type) test PASSED");
}

#[test]
fn test_method_call_with_array_element() {
    let test_wj = r#"
struct Processor {
    data: Vec<i32>
}

impl Processor {
    fn update_bone(self, bone_id: u32) {
        println!("Bone: {}", bone_id)
    }
    
    fn process_bones(self, bone_ids: [u32; 3]) {
        let mut i = 0
        while i < bone_ids.len() {
            self.update_bone(bone_ids[i])  // Should NOT add &
            i = i + 1
        }
    }
}
"#;
    
    let test_file = "/tmp/test_method_array.wj";
    fs::write(test_file, test_wj).expect("Failed to write test file");
    
    let output = Command::new("./target/release/wj")
        .args(&["build", test_file, "-o", "./build", "--no-cargo"])
        .output()
        .expect("Failed to run wj compiler");
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Compilation failed: {}", stderr);
    }
    
    let rs_file = "./build/test_method_array.rs";
    let rust_code = fs::read_to_string(rs_file)
        .expect("Failed to read generated .rs file");
    
    println!("Generated Rust:\n{}", rust_code);
    
    // Should NOT add & before array indexing
    assert!(
        !rust_code.contains("update_bone(&bone_ids["),
        "Should NOT add & for u32 array element in method call\nGenerated:\n{}", 
        rust_code
    );
    
    // Cleanup
    let _ = fs::remove_file(test_file);
    
    println!("✅ Method call with array element test PASSED");
}

#[test]
fn test_vec_indexing_function_call_copy_type() {
    let test_wj = r#"
fn process_id(id: u32) {
    println!("{}", id)
}

fn process_vec(ids: Vec<u32>) {
    let mut i = 0
    while i < ids.len() {
        process_id(ids[i])  // Should NOT add & for Vec<u32>[i]
        i = i + 1
    }
}
"#;
    
    let test_file = "/tmp/test_vec_index.wj";
    fs::write(test_file, test_wj).expect("Failed to write test file");
    
    let output = Command::new("./target/release/wj")
        .args(&["build", test_file, "-o", "./build", "--no-cargo"])
        .output()
        .expect("Failed to run wj compiler");
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Compilation failed: {}", stderr);
    }
    
    let rs_file = "./build/test_vec_index.rs";
    let rust_code = fs::read_to_string(rs_file)
        .expect("Failed to read generated .rs file");
    
    println!("Generated Rust:\n{}", rust_code);
    
    // Should NOT add & for Vec<u32>[i] when passing to function expecting u32
    assert!(
        !rust_code.contains("process_id(&ids["),
        "Should NOT add & for Vec<u32>[i] (Copy type)\nGenerated:\n{}", 
        rust_code
    );
    
    // Should pass by value
    assert!(
        rust_code.contains("process_id(ids["),
        "Should pass Vec<u32>[i] by value\nGenerated:\n{}", 
        rust_code
    );
    
    // Cleanup
    let _ = fs::remove_file(test_file);
    
    println!("✅ Vec indexing function call test PASSED");
}

#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

// TDD Test: Array elements of Copy types should not get & when passed to functions
//
// Bug: roots[j] generates &roots[j as usize] when function expects u32 by value
// Root cause: Ownership inference adds & for array indexing regardless of target type
// Rust: u32 is Copy, function takes by value, no & needed
//
// Fix: Check if target parameter is Copy type before adding &

use std::fs;
use std::process::Command;
use tempfile::TempDir;

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

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("test.wj");
    fs::write(&test_file, test_wj).expect("Failed to write test file");

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            test_file.to_str().unwrap(),
            "-o",
            temp_dir.path().to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj compiler");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Compilation failed: {}", stderr);
    }

    let rs_file = temp_dir.path().join("test.rs");
    let rust_code = fs::read_to_string(&rs_file).expect("Failed to read generated .rs file");

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

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("test.wj");
    fs::write(&test_file, test_wj).expect("Failed to write test file");

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            test_file.to_str().unwrap(),
            "-o",
            temp_dir.path().to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj compiler");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Compilation failed: {}", stderr);
    }

    let rs_file = temp_dir.path().join("test.rs");
    let rust_code = fs::read_to_string(&rs_file).expect("Failed to read generated .rs file");

    println!("Generated Rust:\n{}", rust_code);

    // Should NOT add & before array indexing
    assert!(
        !rust_code.contains("update_bone(&bone_ids["),
        "Should NOT add & for u32 array element in method call\nGenerated:\n{}",
        rust_code
    );

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

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("test.wj");
    fs::write(&test_file, test_wj).expect("Failed to write test file");

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            test_file.to_str().unwrap(),
            "-o",
            temp_dir.path().to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj compiler");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Compilation failed: {}", stderr);
    }

    let rs_file = temp_dir.path().join("test.rs");
    let rust_code = fs::read_to_string(&rs_file).expect("Failed to read generated .rs file");

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

    println!("✅ Vec indexing function call test PASSED");
}

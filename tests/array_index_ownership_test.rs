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

// TDD Test: Array indexing should respect Copy types (no & needed)
//
// Bug: buf.push(&array[i]) generates push(&f32) when should be push(f32)
// Root cause: Ownership inference adding & for array access of Copy types
// Rust: Vec<f32>::push takes f32 by value (Copy), not &f32
//
// Fix: Array indexing of Copy types should return T, not &T

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_push_array_element_copy_type() {
    let test_wj = r#"
fn collect_values(params: [f32; 3]) -> Vec<f32> {
    let mut buf = Vec::new()
    buf.push(params[0])
    buf.push(params[1])
    buf.push(params[2])
    buf
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

    // Should generate push(params[0]), NOT push(&params[0])
    // f32 is Copy, so Vec<f32>::push takes f32 by value
    assert!(
        rust_code.contains("buf.push(params[0") && !rust_code.contains("buf.push(&params[0"),
        "Should push Copy type by value, not by reference\nGenerated:\n{}",
        rust_code
    );
    println!("✅ Array element push (Copy type) test PASSED");
}

#[test]
fn test_push_array_element_with_index_expression() {
    let test_wj = r#"
fn collect_from_index(data: [f32; 10]) -> Vec<f32> {
    let mut result = Vec::new()
    result.push(data[0])
    result.push(data[1])
    result
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

    // Should NOT have & before array indexing for Copy types
    assert!(
        !rust_code.contains("push(&data["),
        "Should NOT add & for Copy type array access\nGenerated:\n{}",
        rust_code
    );
    println!("✅ Array element push with expression test PASSED");
}

#[test]
fn test_field_access_array_element() {
    let test_wj = r#"
struct Node {
    transform_params: [f32; 3]
}

fn emit_instruction(node: Node, buf: Vec<f32>) {
    buf.push(node.transform_params[0])
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

    // Should generate: buf.push(node.transform_params[0])
    // NOT: buf.push(&node.transform_params[0])
    assert!(
        !rust_code.contains("push(&node.transform_params["),
        "Should NOT add & for Copy type field array access\nGenerated:\n{}",
        rust_code
    );
    println!("✅ Field array element access test PASSED");
}

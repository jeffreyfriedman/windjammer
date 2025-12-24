// TDD Test: Compiler should auto-cast usize to i64 in return statements
// Functions returning int should accept .len() (usize) without explicit casts

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_code(code: &str) -> Result<String, String> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_dir = temp_dir.path();
    let input_file = test_dir.join("test.wj");
    fs::write(&input_file, code).expect("Failed to write source file");

    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--",
            "build",
            input_file.to_str().unwrap(),
            "--output",
            test_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let generated_file = test_dir.join("test.rs");
    let generated = fs::read_to_string(&generated_file).expect("Failed to read generated file");

    Ok(generated)
}

#[test]
fn test_return_vec_len_should_cast_to_int() {
    // BUG: Compiler doesn't auto-cast .len() to i64 in return
    let code = r#"
    pub fn get_length(items: Vec<i32>) -> int {
        return items.len()
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // Should auto-cast .len() (usize) to i64
    assert!(
        generated.contains("items.len() as i64") || generated.contains("(items.len() as i64)"),
        "Should auto-cast .len() to i64 when returning int, got:\n{}",
        generated
    );
}

#[test]
fn test_return_len_from_method() {
    // Real case from components.rs
    let code = r#"
    pub struct ComponentArray {
        pub dense: Vec<i32>,
    }
    
    impl ComponentArray {
        pub fn len(&self) -> int {
            return self.dense.len()
        }
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // Should auto-cast
    assert!(
        generated.contains("self.dense.len() as i64")
            || generated.contains("(self.dense.len() as i64)"),
        "Should auto-cast .len() to i64, got:\n{}",
        generated
    );
}

#[test]
fn test_implicit_return_len_should_cast() {
    // Test implicit returns (no return keyword)
    let code = r#"
    pub fn count(items: Vec<i32>) -> int {
        items.len()
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // Should auto-cast implicit return
    assert!(
        generated.contains("items.len() as i64"),
        "Should auto-cast implicit return of .len() to i64, got:\n{}",
        generated
    );
}

#[test]
fn test_return_usize_variable_to_int() {
    // When a usize variable is returned as int
    let code = r#"
    pub fn process(items: Vec<i32>) -> int {
        let count: usize = items.len()
        return count
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // Should auto-cast usize variable to i64
    assert!(
        generated.contains("return count as i64"),
        "Should auto-cast usize variable to i64, got:\n{}",
        generated
    );
}

#[test]
fn test_return_computed_usize_to_int() {
    // Return expression with usize operations
    let code = r#"
    pub fn get_half_length(items: Vec<i32>) -> int {
        return items.len() / 2
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // Should cast the entire expression
    assert!(
        generated.contains("as i64") || generated.contains("as usize"),
        "Should handle usize arithmetic in return, got:\n{}",
        generated
    );
}

// TDD Test: Compiler should auto-cast .len() to i64 when comparing with int variables
// WINDJAMMER PHILOSOPHY: Compiler handles type compatibility automatically

use std::process::Command;
use std::fs;

fn compile_code(code: &str) -> Result<String, String> {
    let test_dir = "tests/generated/int_usize_comparison_test";
    fs::create_dir_all(test_dir).expect("Failed to create test dir");
    let input_file = format!("{}/test.wj", test_dir);
    fs::write(&input_file, code).expect("Failed to write source file");

    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--",
            "build",
            &input_file,
            "--output",
            test_dir,
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        fs::remove_dir_all(test_dir).ok();
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let generated_file = format!("{}/test.rs", test_dir);
    let generated = fs::read_to_string(&generated_file)
        .expect("Failed to read generated file");

    fs::remove_dir_all(test_dir).ok();

    Ok(generated)
}

#[test]
fn test_int_var_compared_with_len_should_cast_len() {
    // Real case from query.rs: self.index (int) >= self.entities.len() (usize)
    let code = r#"
    pub struct Iterator {
        pub index: int,
        pub items: Vec<i32>,
    }
    
    impl Iterator {
        pub fn has_next(&self) -> bool {
            return self.index < self.items.len()
        }
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");
    
    // Should cast .len() to i64 for comparison with int field
    assert!(
        generated.contains("self.index < (self.items.len() as i64)") ||
        generated.contains("(self.items.len() as i64)"),
        "Should cast .len() to i64 when comparing with int field, got:\n{}",
        generated
    );
}

#[test]
fn test_int_local_var_compared_with_len() {
    let code = r#"
    pub fn check_bounds(items: Vec<i32>, pos: int) -> bool {
        return pos >= items.len()
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");
    
    // Should cast .len() to i64
    assert!(
        generated.contains("items.len() as i64"),
        "Should cast .len() to i64 when comparing with int parameter, got:\n{}",
        generated
    );
}

#[test]
fn test_usize_var_compared_with_len_no_cast() {
    // When both are usize, NO cast needed
    let code = r#"
    pub fn check(items: Vec<i32>, index: usize) -> bool {
        return index < items.len()
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");
    
    // Should NOT cast when both are usize
    assert!(
        !generated.contains("as i64") || !generated.contains("index"),
        "Should NOT cast when both sides are usize, got:\n{}",
        generated
    );
}

#[test]
fn test_int_field_compared_with_len_in_if() {
    let code = r#"
    pub struct State {
        pub current: int,
        pub data: Vec<i32>,
    }
    
    impl State {
        pub fn is_done(&self) -> bool {
            if self.current >= self.data.len() {
                return true
            }
            return false
        }
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");
    
    // Should cast .len() to i64 in if condition
    assert!(
        generated.contains("self.data.len() as i64"),
        "Should cast .len() to i64 in if condition, got:\n{}",
        generated
    );
}



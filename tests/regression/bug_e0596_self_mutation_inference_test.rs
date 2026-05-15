// TDD Test: E0596 - Infer &mut self from field mutations
//
// Bug: Methods that mutate self fields (self.count = ..., self.items.push(...))
// were generating &self instead of &mut self, causing "cannot borrow as mutable" (E0596).
//
// Root cause: Compiler's ownership inference too conservative.
// Fix: Ensure both analyzer and codegen correctly detect all mutation patterns.
//
// Success: Generated Rust compiles with rustc (no E0596).

#[path = "../common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn rustc_compile(rust_code: &str, _test_name: &str) -> Result<(), String> {
    let test_dir = TempDir::new().expect("Failed to create temp dir");
    let rust_file = test_dir.path().join("test.rs");
    fs::write(&rust_file, rust_code).expect("Failed to write Rust file");

    let output = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            "-o",
            test_dir.path().join("libtest.rlib").to_str().unwrap(),
            rust_file.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run rustc");

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[test]
fn test_infer_mut_self_from_field_assignment() {
    // Direct field mutation: self.count = self.count + 1
    let source = r#"
pub struct Counter {
    pub count: i32,
}

impl Counter {
    pub fn new() -> Counter {
        Counter { count: 0 }
    }

    pub fn increment(self) {
        self.count = self.count + 1
    }
}
"#;

    let rust_code =
        test_utils::compile_single_result(source).expect("Windjammer compilation should succeed");

    assert!(
        rust_code.contains("increment(&mut self)") || rust_code.contains("&mut self"),
        "Should infer &mut self from field mutation. Generated:\n{}",
        rust_code
    );

    let result = rustc_compile(&rust_code, "field_assignment");
    assert!(
        result.is_ok(),
        "Generated Rust should compile. Error:\n{}",
        result.err().unwrap()
    );
}

#[test]
fn test_infer_mut_self_from_compound_assignment() {
    // Compound assignment: self.points += amount
    let source = r#"
pub struct Score {
    pub points: i32,
}

impl Score {
    pub fn new() -> Score {
        Score { points: 0 }
    }

    pub fn add_points(self, amount: i32) {
        self.points += amount
    }
}
"#;

    let rust_code =
        test_utils::compile_single_result(source).expect("Windjammer compilation should succeed");

    assert!(
        rust_code.contains("add_points(&mut self)") || rust_code.contains("&mut self"),
        "Should infer &mut self from compound assignment. Generated:\n{}",
        rust_code
    );

    let result = rustc_compile(&rust_code, "compound_assignment");
    assert!(
        result.is_ok(),
        "Generated Rust should compile. Error:\n{}",
        result.err().unwrap()
    );
}

#[test]
fn test_infer_mut_self_from_vec_push() {
    // Method call on field: self.items.push(item)
    let source = r#"
pub struct List {
    pub items: Vec<i32>,
}

impl List {
    pub fn new() -> List {
        List { items: Vec::new() }
    }

    pub fn add(self, item: i32) {
        self.items.push(item)
    }
}
"#;

    let rust_code =
        test_utils::compile_single_result(source).expect("Windjammer compilation should succeed");

    assert!(
        rust_code.contains("add(&mut self)") || rust_code.contains("&mut self"),
        "Should infer &mut self from Vec::push. Generated:\n{}",
        rust_code
    );
    let result = rustc_compile(&rust_code, "vec_push");
    assert!(
        result.is_ok(),
        "Generated Rust should compile. Error:\n{}",
        result.err().unwrap()
    );
}

#[test]
fn test_infer_mut_self_from_index_assignment() {
    // Index assignment: self.items[i] = value
    let source = r#"
pub struct Buffer {
    pub data: Vec<i32>,
}

impl Buffer {
    pub fn new() -> Buffer {
        Buffer { data: Vec::new() }
    }

    pub fn set(self, index: usize, value: i32) {
        if index < self.data.len() {
            self.data[index] = value
        }
    }
}
"#;

    let rust_code =
        test_utils::compile_single_result(source).expect("Windjammer compilation should succeed");

    assert!(
        rust_code.contains("set(&mut self)") || rust_code.contains("&mut self"),
        "Should infer &mut self from index assignment. Generated:\n{}",
        rust_code
    );

    let result = rustc_compile(&rust_code, "index_assignment");
    assert!(
        result.is_ok(),
        "Generated Rust should compile. Error:\n{}",
        result.err().unwrap()
    );
}

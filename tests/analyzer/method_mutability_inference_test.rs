//! TDD Test: Method Mutability Inference (E0596)
//!
//! Bug: Methods that mutate self fields generate &self instead of &mut self,
//! causing "cannot borrow as mutable" (E0596).
//!
//! Root cause: Ownership inference not detecting all mutation patterns.
//! Fix: Ensure analyzer correctly detects self.field.push, self.field = value,
//! nested mutations, and that read-only methods stay &self.
//!
//! Success: All tests pass, generated Rust compiles with rustc.

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
fn test_method_with_vec_push_infers_mut_self() {
    let source = r#"
pub struct Container {
    items: Vec<i32>,
}

impl Container {
    pub fn add(self, item: i32) {
        self.items.push(item)
    }
}
"#;

    let result =
        test_utils::compile_single_result(source).expect("Windjammer compilation should succeed");
    assert!(
        result.contains("fn add(&mut self"),
        "Should infer &mut self for self.items.push. Got:\n{}",
        result
    );

    let rust_result = rustc_compile(&result, "vec_push");
    assert!(
        rust_result.is_ok(),
        "Generated Rust should compile. Error:\n{}",
        rust_result.err().unwrap()
    );
}

#[test]
fn test_method_with_field_assignment_infers_mut_self() {
    let source = r#"
pub struct Counter {
    count: i32,
}

impl Counter {
    pub fn increment(self) {
        self.count = self.count + 1
    }
}
"#;

    let result =
        test_utils::compile_single_result(source).expect("Windjammer compilation should succeed");
    assert!(
        result.contains("fn increment(&mut self") || result.contains("&mut self"),
        "Should infer &mut self for self.count = value. Got:\n{}",
        result
    );

    let rust_result = rustc_compile(&result, "field_assignment");
    assert!(
        rust_result.is_ok(),
        "Generated Rust should compile. Error:\n{}",
        rust_result.err().unwrap()
    );
}

#[test]
fn test_method_with_nested_mutation_infers_mut_self() {
    let source = r#"
pub struct Inner {
    items: Vec<i32>,
}

pub struct Outer {
    inner: Inner,
}

impl Inner {
    pub fn add(self, item: i32) {
        self.items.push(item)
    }
}

impl Outer {
    pub fn add(self, item: i32) {
        self.inner.add(item)
    }
}
"#;

    let result =
        test_utils::compile_single_result(source).expect("Windjammer compilation should succeed");
    assert!(
        result.contains("fn add(&mut self") || result.contains("&mut self"),
        "Should infer &mut self for self.inner.add (nested mutation). Got:\n{}",
        result
    );

    let rust_result = rustc_compile(&result, "nested_mutation");
    assert!(
        rust_result.is_ok(),
        "Generated Rust should compile. Error:\n{}",
        rust_result.err().unwrap()
    );
}

#[test]
fn test_method_with_nested_field_push_infers_mut_self() {
    let source = r#"
pub struct Inner {
    items: Vec<i32>,
}

pub struct Outer {
    inner: Inner,
}

impl Outer {
    pub fn add(self, item: i32) {
        self.inner.items.push(item)
    }
}
"#;

    let result =
        test_utils::compile_single_result(source).expect("Windjammer compilation should succeed");
    assert!(
        result.contains("fn add(&mut self") || result.contains("&mut self"),
        "Should infer &mut self for self.inner.items.push (nested field). Got:\n{}",
        result
    );

    let rust_result = rustc_compile(&result, "nested_field_push");
    assert!(
        rust_result.is_ok(),
        "Generated Rust should compile. Error:\n{}",
        rust_result.err().unwrap()
    );
}

#[test]
fn test_method_read_only_infers_ref_self() {
    let source = r#"
pub struct Container {
    items: Vec<i32>,
}

impl Container {
    pub fn len(self) -> i32 {
        self.items.len() as i32
    }
}
"#;

    let result =
        test_utils::compile_single_result(source).expect("Windjammer compilation should succeed");
    assert!(
        result.contains("fn len(&self") && !result.contains("fn len(&mut self"),
        "Read-only method should infer &self. Got:\n{}",
        result
    );

    let rust_result = rustc_compile(&result, "read_only");
    assert!(
        rust_result.is_ok(),
        "Generated Rust should compile. Error:\n{}",
        rust_result.err().unwrap()
    );
}

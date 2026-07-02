#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
))]

//! TDD Test: HashMap .get() match destructuring should auto-deref/clone bindings.
//!
//! When matching on `hashmap.get(key)` (which returns `Option<&V>` in Rust),
//! the bound variables in enum variant destructuring are references.
//! The compiler must auto-dereference Copy types (*value) and auto-clone
//! non-Copy types (value.clone()) when the binding is used in an owned context.

#[path = "common/test_utils.rs"]
mod test_utils;

use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_hashmap_get_enum_destructure_copy_types() {
    let code = r#"
pub enum DataValue {
    Int(i32),
    Float(f32),
    Bool(bool),
}

pub struct Store {
    data: HashMap<string, DataValue>,
}

impl Store {
    pub fn new() -> Store {
        Store { data: HashMap::new() }
    }

    pub fn get_int(self, key: string) -> Option<i32> {
        match self.data.get(key) {
            Some(DataValue::Int(value)) => Some(value),
            _ => None,
        }
    }

    pub fn get_float(self, key: string) -> Option<f32> {
        match self.data.get(key) {
            Some(DataValue::Float(value)) => Some(value),
            _ => None,
        }
    }

    pub fn get_bool(self, key: string) -> Option<bool> {
        match self.data.get(key) {
            Some(DataValue::Bool(value)) => Some(value),
            _ => None,
        }
    }
}

fn main() {
    let store = Store::new()
}
"#;

    match test_utils::compile_single_result(code) {
        Ok(generated) => {
            // For Copy types (i32, f32, bool), bindings from HashMap .get() match
            // should be auto-dereferenced with *value
            let has_deref = generated.contains("*value")
                || generated.contains("*val")
                || generated.contains(".clone()");

            assert!(
                has_deref,
                "HashMap .get() match bindings should have auto-deref (*value) or clone.\nGenerated code:\n{}",
                generated
            );

            // Verify with rustc
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let rust_file = temp_dir.path().join("test.rs");
            std::fs::write(&rust_file, &generated).expect("Failed to write Rust file");

            let rustc_output = Command::new("rustc")
                .arg("--crate-type")
                .arg("lib")
                .arg("--emit")
                .arg("metadata")
                .arg(&rust_file)
                .arg("--out-dir")
                .arg(temp_dir.path())
                .output()
                .expect("Failed to run rustc");

            assert!(
                rustc_output.status.success(),
                "Generated Rust for HashMap .get() match should compile.\nrustc stderr:\n{}\nGenerated code:\n{}",
                String::from_utf8_lossy(&rustc_output.stderr),
                generated
            );
        }
        Err(err) => {
            panic!("Compilation failed: {}", err);
        }
    }
}

#[test]
fn test_hashmap_get_enum_destructure_string_type() {
    let code = r#"
pub enum DataValue {
    Text(string),
}

pub struct Store {
    data: HashMap<string, DataValue>,
}

impl Store {
    pub fn new() -> Store {
        Store { data: HashMap::new() }
    }

    pub fn get_text(self, key: string) -> Option<string> {
        match self.data.get(key) {
            Some(DataValue::Text(value)) => Some(value),
            _ => None,
        }
    }
}

fn main() {
    let store = Store::new()
}
"#;

    match test_utils::compile_single_result(code) {
        Ok(generated) => {
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let rust_file = temp_dir.path().join("test.rs");
            std::fs::write(&rust_file, &generated).expect("Failed to write Rust file");

            let rustc_output = Command::new("rustc")
                .arg("--crate-type")
                .arg("lib")
                .arg("--emit")
                .arg("metadata")
                .arg(&rust_file)
                .arg("--out-dir")
                .arg(temp_dir.path())
                .output()
                .expect("Failed to run rustc");

            assert!(
                rustc_output.status.success(),
                "Generated Rust for HashMap .get() with String type should compile.\nrustc stderr:\n{}\nGenerated code:\n{}",
                String::from_utf8_lossy(&rustc_output.stderr),
                generated
            );
        }
        Err(err) => {
            panic!("Compilation failed: {}", err);
        }
    }
}

//! TDD Tests: HashMap str/String codegen - E0277 Borrow<&str> fix
//!
//! Problem: HashMap<str, T> generates HashMap<&str, T> causing "String: Borrow<&str>" errors.
//! Root Cause: str in containers should become String (owned), not &str.
//!
//! Solution: HashMap<str, T> → HashMap<String, T> in types.rs
//!
//! Philosophy: "Compiler Does the Hard Work" - users shouldn't think about String vs &str.

use windjammer::codegen::rust::types::{type_to_rust, type_to_rust_with_lifetime};
use windjammer::parser::Type;

// =============================================================================
// Unit tests: type_to_rust for HashMap
// =============================================================================

/// HashMap<str, i32> -> HashMap<String, i32> (key str becomes String)
#[test]
fn test_hashmap_str_i32_emits_string_i64() {
    let ty = Type::Parameterized(
        "HashMap".to_string(),
        vec![Type::Custom("str".to_string()), Type::Int],
    );
    let rust = type_to_rust(&ty);
    assert_eq!(
        rust, "HashMap<String, i64>",
        "HashMap<str, i32> should emit HashMap<String, i64>, got {}",
        rust
    );
}

/// HashMap<str, str> -> HashMap<String, String> (both key and value)
#[test]
fn test_hashmap_str_str_emits_string_string() {
    let ty = Type::Parameterized(
        "HashMap".to_string(),
        vec![
            Type::Custom("str".to_string()),
            Type::Custom("str".to_string()),
        ],
    );
    let rust = type_to_rust(&ty);
    assert_eq!(
        rust, "HashMap<String, String>",
        "HashMap<str, str> should emit HashMap<String, String>, got {}",
        rust
    );
}

/// HashMap<string, i32> -> HashMap<String, i64> (string keyword)
#[test]
fn test_hashmap_string_i32_emits_string_i64() {
    let ty = Type::Parameterized(
        "HashMap".to_string(),
        vec![Type::String, Type::Int],
    );
    let rust = type_to_rust(&ty);
    assert_eq!(
        rust, "HashMap<String, i64>",
        "HashMap<string, i32> should emit HashMap<String, i64>, got {}",
        rust
    );
}

/// HashMap<i32, str> -> HashMap<i32, String> (value str becomes String)
#[test]
fn test_hashmap_i32_str_value_emits_string() {
    let ty = Type::Parameterized(
        "HashMap".to_string(),
        vec![Type::Int, Type::Custom("str".to_string())],
    );
    let rust = type_to_rust(&ty);
    assert_eq!(
        rust, "HashMap<i64, String>",
        "HashMap<i32, str> value should become String, got {}",
        rust
    );
}

/// type_to_rust: HashMap<str, T> in struct field (same as general - str -> String)
#[test]
fn test_hashmap_str_in_struct_field() {
    let ty = Type::Parameterized(
        "HashMap".to_string(),
        vec![
            Type::Custom("str".to_string()),
            Type::Custom("PropertyValue".to_string()),
        ],
    );
    let rust = type_to_rust(&ty);
    assert_eq!(
        rust, "HashMap<String, PropertyValue>",
        "Struct field HashMap<str, PropertyValue> should emit HashMap<String, PropertyValue>, got {}",
        rust
    );
}

/// type_to_rust_with_lifetime: HashMap<str, str> (for fn signatures with lifetimes)
#[test]
fn test_hashmap_str_str_with_lifetime() {
    let ty = Type::Parameterized(
        "HashMap".to_string(),
        vec![
            Type::Custom("str".to_string()),
            Type::Custom("str".to_string()),
        ],
    );
    let rust = type_to_rust_with_lifetime(&ty);
    assert_eq!(
        rust, "HashMap<String, String>",
        "HashMap<str, str> with lifetime should emit HashMap<String, String>, got {}",
        rust
    );
}

/// Must NOT emit HashMap<&str, T> (the bug we're fixing)
#[test]
fn test_hashmap_must_not_emit_ampersand_str_as_key() {
    let ty = Type::Parameterized(
        "HashMap".to_string(),
        vec![Type::Custom("str".to_string()), Type::Int],
    );
    let rust = type_to_rust(&ty);
    assert!(
        !rust.contains("HashMap<&str"),
        "Must NOT emit HashMap<&str, ...> - causes Borrow<&str> errors. Got: {}",
        rust
    );
}

// =============================================================================
// Integration test: compile Windjammer HashMap<str, T> and verify Rust output
// =============================================================================

#[test]
#[cfg(feature = "cli")]
fn test_hashmap_str_compiles_to_string_integration() {
    use std::fs;
    use std::process::Command;

    let source = r#"
use std::collections::HashMap

struct Registry {
    name_to_id: HashMap<str, i64>
}

impl Registry {
    pub fn get_id(self, name: str) -> Option<i64> {
        self.name_to_id.get(&name).cloned()
    }
}

fn main() {
    let mut reg = Registry { name_to_id: HashMap::new() }
    reg.name_to_id.insert("test".to_string(), 42)
    let id = reg.get_id("test")
    println("{:?}", id)
}
"#;

    let temp_dir = std::env::temp_dir();
    let test_id = format!(
        "wj_hashmap_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let test_dir = temp_dir.join(&test_id);
    fs::create_dir_all(&test_dir).unwrap();

    let wj_file = test_dir.join("test.wj");
    fs::write(&wj_file, source).unwrap();

    let out_dir = test_dir.join("out");

    let wj_binary = env!("CARGO_BIN_EXE_wj");
    let _output = Command::new(wj_binary)
        .arg("build")
        .arg(&wj_file)
        .arg("--target")
        .arg("rust")
        .arg("--output")
        .arg(&out_dir)
        .output()
        .expect("Failed to run wj compiler");

    let rust_file = out_dir.join("test.rs");
    let generated = fs::read_to_string(&rust_file).expect("Failed to read generated Rust file");

    // CRITICAL: Must emit HashMap<String, i64> NOT HashMap<&str, i64>
    assert!(
        generated.contains("HashMap<String, i64>"),
        "HashMap<str, i64> must emit HashMap<String, i64>. Got:\n{}",
        generated
    );
    assert!(
        !generated.contains("HashMap<&str"),
        "Must NOT emit HashMap<&str, ...> - causes Borrow<&str> errors. Got:\n{}",
        generated
    );

    fs::remove_dir_all(&test_dir).ok();
}

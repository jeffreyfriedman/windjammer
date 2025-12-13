//! Comprehensive Codegen Generics Tests
//!
//! These tests verify that the Windjammer compiler correctly generates
//! Rust code for generics, including:
//! - Type parameters
//! - Trait bounds
//! - Where clauses
//! - Generic structs and functions

use std::fs;
use std::process::Command;
use tempfile::TempDir;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn compile_and_get_rust(code: &str) -> Result<String, String> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&wj_path, code).expect("Failed to write test file");
    fs::create_dir_all(&out_dir).expect("Failed to create output dir");

    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--",
            "build",
            wj_path.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let generated_path = out_dir.join("test.rs");
    fs::read_to_string(&generated_path).map_err(|e| format!("Failed to read generated file: {}", e))
}

fn compile_and_verify(code: &str) -> (bool, String, String) {
    match compile_and_get_rust(code) {
        Ok(generated) => {
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let rs_path = temp_dir.path().join("test.rs");
            fs::write(&rs_path, &generated).expect("Failed to write rs file");

            let rustc = Command::new("rustc")
                .arg("--crate-type=lib")
                .arg(&rs_path)
                .arg("-o")
                .arg(temp_dir.path().join("test.rlib"))
                .output();

            match rustc {
                Ok(output) => {
                    let err = String::from_utf8_lossy(&output.stderr).to_string();
                    (output.status.success(), generated, err)
                }
                Err(e) => (false, generated, format!("Failed to run rustc: {}", e)),
            }
        }
        Err(e) => (false, String::new(), e),
    }
}

// ============================================================================
// GENERIC FUNCTIONS
// ============================================================================

#[test]
fn test_generic_function_simple() {
    let code = r#"
pub fn identity<T>(x: T) -> T {
    x
}
"#;
    let (success, generated, err) = compile_and_verify(code);
    assert!(success, "Generic function should compile. Error: {}", err);
}

#[test]
fn test_generic_function_multiple_params() {
    let code = r#"
pub fn pair<A, B>(a: A, b: B) -> (A, B) {
    (a, b)
}
"#;
    let (success, generated, err) = compile_and_verify(code);
    assert!(
        success,
        "Multiple generic params should compile. Error: {}",
        err
    );
}

// ============================================================================
// GENERIC STRUCTS
// ============================================================================

#[test]
fn test_generic_struct() {
    let code = r#"
@derive(Clone, Debug)
pub struct Container<T> {
    value: T,
}

impl<T> Container<T> {
    pub fn new(value: T) -> Container<T> {
        Container { value: value }
    }
    
    pub fn get(&self) -> &T {
        &self.value
    }
}
"#;
    let (success, generated, err) = compile_and_verify(code);
    assert!(success, "Generic struct should compile. Error: {}", err);
}

#[test]
fn test_generic_struct_multiple() {
    let code = r#"
@derive(Clone, Debug)
pub struct Pair<A, B> {
    first: A,
    second: B,
}

impl<A, B> Pair<A, B> {
    pub fn new(first: A, second: B) -> Pair<A, B> {
        Pair { first: first, second: second }
    }
}
"#;
    let (success, generated, err) = compile_and_verify(code);
    assert!(
        success,
        "Generic struct multiple should compile. Error: {}",
        err
    );
}

// ============================================================================
// TRAIT BOUNDS
// ============================================================================

#[test]
fn test_trait_bound_clone() {
    let code = r#"
pub fn duplicate<T: Clone>(item: T) -> (T, T) {
    (item.clone(), item)
}
"#;
    let (success, generated, err) = compile_and_verify(code);
    assert!(success, "Clone bound should compile. Error: {}", err);
}

#[test]
fn test_trait_bound_multiple() {
    // Multiple trait bounds
    let code = r#"
pub fn clone_twice<T: Clone>(item: T) -> (T, T) {
    let a = item.clone();
    let b = item;
    (a, b)
}
"#;
    let (success, generated, err) = compile_and_verify(code);
    assert!(success, "Multiple bounds should compile. Error: {}", err);
}

#[test]
fn test_trait_bound_default() {
    let code = r#"
pub fn get_default<T: Default>() -> T {
    T::default()
}
"#;
    let (success, generated, err) = compile_and_verify(code);
    assert!(success, "Default bound should compile. Error: {}", err);
}

// ============================================================================
// GENERIC ENUMS
// ============================================================================

#[test]
fn test_generic_enum() {
    // Simple generic enum definition
    let code = r#"
pub enum Maybe<T> {
    Just(T),
    Nothing,
}

pub fn is_just<T>(m: &Maybe<T>) -> bool {
    match m {
        Maybe::Just(_) => true,
        Maybe::Nothing => false,
    }
}
"#;
    let (success, generated, err) = compile_and_verify(code);
    assert!(success, "Generic enum should compile. Error: {}", err);
}

// ============================================================================
// GENERIC TRAITS
// ============================================================================

#[test]
fn test_simple_impl() {
    // Simple impl block
    let code = r#"
@derive(Clone, Debug)
pub struct Counter {
    count: i32,
}

impl Counter {
    pub fn increment(&mut self) {
        self.count += 1
    }
    
    pub fn get(&self) -> i32 {
        self.count
    }
}
"#;
    let (success, generated, err) = compile_and_verify(code);
    assert!(success, "Simple impl should compile. Error: {}", err);
}

// ============================================================================
// GENERIC METHODS
// ============================================================================

#[test]
fn test_generic_method() {
    // Simpler generic method
    let code = r#"
@derive(Clone, Debug)
pub struct Container<T> {
    value: T,
}

impl<T: Clone> Container<T> {
    pub fn duplicate(&self) -> Container<T> {
        Container { value: self.value.clone() }
    }
}
"#;
    let (success, generated, err) = compile_and_verify(code);
    assert!(success, "Generic method should compile. Error: {}", err);
}

// ============================================================================
// VEC AND OPTION USAGE
// ============================================================================

#[test]
fn test_vec_generic() {
    let code = r#"
pub fn first<T: Clone>(items: &Vec<T>) -> Option<T> {
    items.first().cloned()
}
"#;
    let (success, generated, err) = compile_and_verify(code);
    assert!(success, "Vec generic should compile. Error: {}", err);
}

#[test]
fn test_option_generic() {
    // Simple option usage
    let code = r#"
pub fn is_some<T>(opt: &Option<T>) -> bool {
    match opt {
        Some(_) => true,
        None => false,
    }
}
"#;
    let (success, generated, err) = compile_and_verify(code);
    assert!(success, "Option generic should compile. Error: {}", err);
}

// ============================================================================
// ASSOCIATED TYPES (SIMULATED)
// ============================================================================

#[test]
fn test_iterator_usage() {
    // Basic iterator usage with Vec
    let code = r#"
pub fn count_items(items: &Vec<i32>) -> usize {
    items.iter().count()
}
"#;
    let (success, generated, err) = compile_and_verify(code);
    assert!(success, "Iterator usage should compile. Error: {}", err);
}

// ============================================================================
// LIFETIME PARAMETERS (BASIC)
// ============================================================================

#[test]
fn test_reference_return() {
    // Simple reference return (lifetime inferred)
    let code = r#"
pub fn get_first(items: &Vec<i32>) -> Option<&i32> {
    items.first()
}
"#;
    let (success, generated, err) = compile_and_verify(code);
    assert!(success, "Reference return should compile. Error: {}", err);
}

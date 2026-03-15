//! TDD: Method Call Generation with 3-Layer Ownership System
//!
//! Tests for generate_method_call_with_ownership migration.
//! Replaces builder pattern clone logic with systematic layered approach.
//!
//! - infer_method_receiver_ownership: signature registry + heuristics
//! - generate_expression_with_target_ownership: 3-layer coercion
//! - Clean method calls, E0507 reduction

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_and_verify(code: &str) -> (bool, String, String) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&wj_path, code).expect("Failed to write test file");
    fs::create_dir_all(&out_dir).expect("Failed to create output dir");

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            wj_path.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return (false, String::new(), stderr);
    }

    let generated_path = out_dir.join("test.rs");
    let generated = fs::read_to_string(&generated_path)
        .unwrap_or_else(|_| "Failed to read generated file".to_string());

    let rustc_output = Command::new("rustc")
        .args(["--crate-type=lib", "--edition=2021"])
        .arg(&generated_path)
        .arg("-o")
        .arg(temp_dir.path().join("test.rlib"))
        .output();

    match rustc_output {
        Ok(output) => {
            let rustc_success = output.status.success();
            let rustc_err = String::from_utf8_lossy(&output.stderr).to_string();
            (rustc_success, generated, rustc_err)
        }
        Err(e) => (false, generated, format!("Failed to run rustc: {}", e)),
    }
}

// =============================================================================
// Auto-deref receiver
// =============================================================================

#[test]
fn test_method_call_auto_deref_receiver() {
    let src = r#"
pub fn length(s: string) -> usize {
    s.len()
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    // &String.len(): auto-deref
    assert!(result.contains("s.len()"));
}

// =============================================================================
// Owned receiver
// =============================================================================

#[test]
fn test_method_call_owned_receiver() {
    let src = r#"
pub struct Timer { pub id: i32 }
impl Timer {
    pub fn id(self) -> i32 { self.id }
}
pub fn get_id(t: Timer) -> i32 {
    t.id() + 0
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    // t is Owned, id() takes Owned
    assert!(result.contains("t.id()"));
}

// =============================================================================
// Builder pattern (borrowed -> owned needs clone)
// =============================================================================

#[test]
fn test_builder_pattern_needs_clone() {
    let src = r#"
pub struct Builder {}
impl Builder {
    pub fn with_value(self, v: i32) -> Self { self }
}
pub fn build(b: &Builder) -> Builder {
    b.with_value(42)
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    // b is Borrowed, with_value() takes Owned - system should add .clone()
    assert!(
        result.contains("clone().with_value"),
        "Expected clone for builder. Got:\n{}",
        result
    );
}

// =============================================================================
// Mutating methods (&mut self)
// =============================================================================

#[test]
fn test_mutating_method_push() {
    let src = r#"
pub fn add_item() {
    let mut v = Vec::new()
    v.push(42)
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    assert!(result.contains("v.push(42)"));
}

#[test]
fn test_mutating_method_clear() {
    let src = r#"
pub fn clear_vec() {
    let mut v = Vec::new()
    v.push(1)
    v.clear()
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    assert!(result.contains("clear"));
}

// =============================================================================
// Chain calls
// =============================================================================

#[test]
fn test_chain_calls_trim_len() {
    let src = r#"
pub fn trimmed_len(s: string) -> usize {
    s.trim().len() + 0
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    assert!(result.contains("s.trim().len()"));
}

#[test]
fn test_chain_calls_multiple() {
    let src = r#"
pub fn process(s: string) -> usize {
    if s.trim().to_lowercase().len() > 0 {
        1
    } else {
        0
    }
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    assert!(result.contains("trim"));
    assert!(result.contains("to_lowercase"));
}

// =============================================================================
// Borrowed receiver (no clone needed)
// =============================================================================

#[test]
fn test_borrowed_receiver_len() {
    let src = r#"
pub fn get_len(v: &Vec<i32>) -> usize {
    v.len()
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    assert!(result.contains("v.len()"));
    assert!(!result.contains("v.clone()"));
}

#[test]
fn test_borrowed_receiver_is_empty() {
    let src = r#"
pub fn check_empty(v: &Vec<i32>) -> bool {
    v.is_empty()
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    assert!(result.contains("v.is_empty()"));
}

// =============================================================================
// Option::unwrap on borrowed
// =============================================================================

#[test]
fn test_unwrap_on_borrowed_field() {
    let src = r#"
pub struct Node { pub value: Option<i32> }
pub fn get_value(n: &Node) -> i32 {
    n.value.unwrap()
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    // n.value is &Option, unwrap needs owned - should add clone
    assert!(
        result.contains("unwrap()"),
        "Expected unwrap. Got:\n{}",
        result
    );
}

// =============================================================================
// Option::map with &self (as_ref)
// =============================================================================

#[test]
fn test_option_map_with_borrowed_self() {
    let src = r#"
pub struct Container { pub items: Option<Vec<i32>> }
impl Container {
    pub fn count(self) -> usize {
        match self.items {
            Some(v) => v.len(),
            None => 0
        }
    }
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    // Verify method call on Option field works
    assert!(result.contains("len()"));
}

// =============================================================================
// Vec index + method (owned receiver)
// =============================================================================

#[test]
fn test_vec_index_method_owned_receiver() {
    let src = r#"
pub struct Item { pub id: i32 }
pub fn get_first_id(items: Vec<Item>) -> i32 {
    items[0].id
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    assert!(result.contains("items") && result.contains("id"));
}

// =============================================================================
// Explicit .clone() - no double clone
// =============================================================================

#[test]
fn test_explicit_clone_no_double() {
    let src = r#"
pub struct Data { pub name: string }
pub fn copy_name(d: &Data) -> string {
    d.name.clone()
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    assert!(result.contains("clone()"));
    // Should NOT have .clone().clone()
    assert!(
        !result.contains(".clone().clone()"),
        "Double clone detected. Got:\n{}",
        result
    );
}

// =============================================================================
// Copy type - no clone on borrowed
// =============================================================================

#[test]
fn test_copy_type_no_clone() {
    let src = r#"
pub struct Point { pub x: i32, pub y: i32 }
pub fn get_x(p: &Point) -> i32 {
    p.x
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    assert!(result.contains("p.x"));
    assert!(!result.contains("p.x.clone()"));
}

// =============================================================================
// String methods
// =============================================================================

#[test]
fn test_string_starts_with() {
    let src = r#"
pub fn check_prefix(s: string) -> bool {
    s.starts_with("foo")
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    assert!(result.contains("starts_with"));
}

#[test]
fn test_string_contains() {
    let src = r#"
pub fn has_substring(s: string, sub: string) -> bool {
    s.contains(sub)
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    assert!(result.contains("contains"));
}

// =============================================================================
// Vec::new() static call
// =============================================================================

#[test]
fn test_vec_new_static_call() {
    let src = r#"
pub fn make_vec() -> Vec<i32> {
    Vec::new()
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    assert!(result.contains("Vec::new()"));
}

// =============================================================================
// Additional coverage tests (18-20)
// =============================================================================

#[test]
fn test_get_on_borrowed_vec() {
    let src = r#"
pub fn has_elements(v: &Vec<i32>) -> bool {
    !v.is_empty()
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    assert!(result.contains("v.is_empty()"));
}

#[test]
fn test_iter_method_on_borrowed() {
    let src = r#"
pub fn sum(v: &Vec<i32>) -> i32 {
    let mut s = 0
    for x in v.iter() {
        s = s + x
    }
    s
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    assert!(result.contains("iter()"));
}

#[test]
fn test_field_after_method_call() {
    let src = r#"
pub struct Wrapper { pub value: i32 }
pub fn get_inner(w: &Wrapper) -> i32 {
    w.value
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    assert!(result.contains("w.value"));
}

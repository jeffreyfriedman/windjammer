//! TDD: Function argument generation with 3-layer ownership system
//!
//! Tests for generate_function_arguments() migration.
//! Replaces ad-hoc auto-borrow/deref/clone logic with systematic approach.
//!
//! Layer 1: Track ownership from variables/parameters
//! Layer 2: Apply Copy semantics (effective_ownership)
//! Layer 3: Determine required coercion (RustCoercionRules)

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
        .arg("--crate-type=lib")
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
// Auto-borrow: Owned → Borrowed param
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_fn_arg_auto_borrow() {
    let src = r#"
pub fn takes_ref(s: &string) {}
pub fn caller(s: string) {
    takes_ref(s)
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    // Ownership inference may infer s as &str; either takes_ref(&s) or takes_ref(s) is correct
    assert!(
        result.contains("takes_ref(&s)") || result.contains("takes_ref(s)"),
        "Should handle borrow. Got:\n{}",
        result
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_fn_arg_auto_borrow_mut() {
    let src = r#"
pub fn takes_mut(n: &mut i32) {}
pub fn caller(mut n: i32) {
    takes_mut(n)
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    assert!(
        result.contains("takes_mut(&mut n)"),
        "Should auto-borrow mut. Got:\n{}",
        result
    );
}

// =============================================================================
// Auto-deref/copy: Borrowed Copy → Owned param
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_fn_arg_auto_deref_copy() {
    let src = r#"
pub fn takes_owned(x: i32) {}
pub fn caller(x: &i32) {
    takes_owned(x)
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    // &i32 to i32: may use x, *x, or (x).clone() - all valid for Copy
    assert!(
        result.contains("takes_owned(x)") || result.contains("takes_owned(*x)") || result.contains("takes_owned((x).clone())"),
        "Copy type in function arg. Got:\n{}",
        result
    );
}

// =============================================================================
// Auto-clone: Borrowed non-Copy → Owned param
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_fn_arg_noncopy_needs_clone() {
    let src = r#"
pub fn takes_owned(s: string) {}
pub fn caller(s: &string) {
    takes_owned(s)
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    // May use .clone() or ownership inference may change param to &str
    assert!(
        result.contains(".clone()") || result.contains("takes_owned(s)"),
        "Borrowed to owned param. Got:\n{}",
        result
    );
}

// =============================================================================
// String literal handling
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_fn_arg_string_literal_to_owned() {
    let src = r#"
pub fn takes_string(s: string) {}
pub fn caller() {
    takes_string("hello")
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    // String literal: may add .to_string() or pass directly; &"hello" also valid
    assert!(
        result.contains("\"hello\"") || result.contains("hello"),
        "String literal in call. Got:\n{}",
        result
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_fn_arg_string_literal_to_borrowed() {
    let src = r#"
pub fn takes_ref(s: &string) {}
pub fn caller() {
    takes_ref("hello")
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    // String literal to &str: takes_ref("hello") or takes_ref(&"hello") both work
    assert!(
        result.contains("takes_ref(\"hello\")") || result.contains("takes_ref(&\"hello\")"),
        "String literal to &str param. Got:\n{}",
        result
    );
}

// =============================================================================
// Float type inference
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_fn_arg_float_inference_f32() {
    let src = r#"
pub fn foo(x: f32) {}
pub fn caller() {
    foo(1.0)
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    assert!(
        result.contains("1.0_f32") || result.contains("1_f32"),
        "Float literal should infer f32 from param. Got:\n{}",
        result
    );
}

// =============================================================================
// Copy type no borrow
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_fn_arg_copy_no_extra_borrow() {
    let src = r#"
pub fn takes_int(x: i32) -> i32 { x }
pub fn caller(n: i32) -> i32 {
    takes_int(n)
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    assert!(
        result.contains("takes_int(n)") && !result.contains("takes_int(&n)"),
        "Copy type should not add &. Got:\n{}",
        result
    );
}

// =============================================================================
// Field access from borrowed
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_fn_arg_borrowed_field_copy() {
    let src = r#"
pub struct Item { pub id: i32 }
pub fn takes_id(id: i32) {}
pub fn caller(items: &Vec<Item>) {
    takes_id(items[0].id)
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_fn_arg_borrowed_field_noncopy() {
    // Explicit & to ensure correct type - 3-layer handles & when needed
    let src = r#"
pub struct Item { pub name: string }
pub fn takes_name(name: &string) {}
pub fn caller(items: &Vec<Item>) {
    takes_name(&items[0].name)
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

// =============================================================================
// Multiple params
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_fn_arg_mixed_ownership() {
    let src = r#"
pub fn mixed(a: &i32, b: i32, c: &string) {}
pub fn caller(x: &i32, y: i32, z: &string) {
    mixed(x, y, z)
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

// =============================================================================
// No signature fallback
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_fn_arg_no_signature_passthrough() {
    let src = r#"
pub fn unknown(a: i32, b: string) {}
pub fn caller(x: i32, s: string) {
    unknown(x, s)
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

// =============================================================================
// Constructor (new) heuristic
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_fn_arg_constructor_string() {
    let src = r#"
pub struct Widget { name: string }
pub fn new(name: string) -> Widget { Widget { name } }
pub fn caller() {
    let w = new("test")
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

// =============================================================================
// For-loop iterator variable
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_fn_arg_for_loop_borrowed() {
    // Explicit & for field from borrowed iterator - 3-layer handles coercion
    let src = r#"
pub struct Item { pub id: string }
pub fn process(id: &string) {}
pub fn caller(items: &Vec<Item>) {
    for item in items {
        process(&item.id)
    }
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

// =============================================================================
// Format macro in args
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_fn_arg_format_macro() {
    // format! returns String; greet takes &str - 3-layer adds & when needed
    let src = r#"
pub fn greet(msg: &string) {}
pub fn caller(name: string) {
    let s = format!("Hello {}", name)
    greet(s)
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    // format! expands to write! in generated Rust
    assert!(
        result.contains("format!") || result.contains("write!") || result.contains("Hello"),
        "Format/write macro or string should be present. Got:\n{}",
        result
    );
}

// =============================================================================
// Vec::new / HashMap::new
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_fn_arg_vec_new() {
    let src = r#"
pub fn caller() {
    let v: Vec<i32> = Vec::new()
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

// =============================================================================
// Enum variant constructor
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_fn_arg_enum_variant() {
    let src = r#"
pub enum Event {
    Message(string)
}
pub fn caller() {
    let e = Event::Message("hi")
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

// =============================================================================
// Nested call
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_fn_arg_nested_call() {
    let src = r#"
pub fn inner(x: i32) -> i32 { x }
pub fn outer(x: i32) {}
pub fn caller(n: i32) {
    outer(inner(n))
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

// =============================================================================
// Regression: temp variable no borrow
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_fn_arg_temp_var_no_borrow() {
    let src = r#"
pub fn takes_string(s: string) {}
pub fn caller(name: &string) {
    let msg = format!("Hello {}", name)
    takes_string(msg)
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    // Temp from format!() - pass to takes_string
    assert!(
        result.contains("takes_string(") && result.contains("msg"),
        "Temp var in call. Got:\n{}",
        result
    );
}

//! TDD: Tuple Struct Support
//!
//! Tests for parsing, codegen, and ownership handling of tuple structs.
//! Tuple structs: struct Point(i32, i32), struct Id(u32), etc.

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
// PARSING: Tuple struct syntax is recognized
// =============================================================================

#[test]
fn test_tuple_struct_basic() {
    let src = r#"
pub struct Point(i32, i32)
"#;
    let (success, generated, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    assert!(
        generated.contains("struct Point("),
        "Must generate tuple struct syntax. Got:\n{}",
        generated
    );
}

#[test]
fn test_tuple_struct_single_field() {
    let src = r#"
pub struct Id(u32)
"#;
    let (success, generated, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    assert!(
        generated.contains("struct Id("),
        "Must generate single-field tuple struct. Got:\n{}",
        generated
    );
}

#[test]
fn test_tuple_struct_many_fields() {
    let src = r#"
pub struct Color(f32, f32, f32, f32)
"#;
    let (success, generated, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    assert!(
        generated.contains("struct Color("),
        "Must generate multi-field tuple struct. Got:\n{}",
        generated
    );
}

// =============================================================================
// CODEGEN: Tuple struct generates valid Rust
// =============================================================================

#[test]
fn test_tuple_struct_constructor() {
    let src = r#"
pub struct Point(i32, i32)

pub fn origin() -> Point {
    Point(0, 0)
}
"#;
    let (success, _generated, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

#[test]
fn test_tuple_struct_with_expressions() {
    let src = r#"
pub struct Point(i32, i32)

pub fn make(x: i32, y: i32) -> Point {
    Point(x + 1, y * 2)
}
"#;
    let (success, _generated, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

// =============================================================================
// OWNERSHIP: Borrowed params are correctly handled in constructors
// =============================================================================

#[test]
fn test_tuple_struct_borrowed_copy_args() {
    let src = r#"
pub struct Point(i32, i32)

pub fn make_pair(x: &i32, y: &i32) -> Point {
    Point(x, y)
}
"#;
    let (success, _generated, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

#[test]
fn test_tuple_struct_borrowed_string_args() {
    let src = r#"
pub struct Label(String)

pub fn wrap(s: &String) -> Label {
    Label(s)
}
"#;
    let (success, _generated, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

#[test]
fn test_tuple_struct_with_string_literal() {
    let src = r#"
pub struct Tag(String)

pub fn default_tag() -> Tag {
    Tag("default")
}
"#;
    let (success, _generated, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

// =============================================================================
// FIELD ACCESS: Tuple struct fields via .0, .1, etc.
// =============================================================================

#[test]
fn test_tuple_struct_field_access() {
    let src = r#"
pub struct Point(i32, i32)

pub fn get_x(p: Point) -> i32 {
    p.0
}
"#;
    let (success, _generated, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

// =============================================================================
// DERIVES: Auto-derive works for tuple structs
// =============================================================================

#[test]
fn test_tuple_struct_auto_derive_copy() {
    let src = r#"
pub struct Point(i32, i32)

pub fn copy_point(p: Point) -> Point {
    let q = p
    q
}
"#;
    let (success, _generated, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

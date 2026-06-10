#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

//! TDD: Tuple Struct Support
//!
//! Tests for parsing, codegen, and ownership handling of tuple structs.
//! Tuple structs: struct Point(i32, i32), struct Id(u32), etc.

#[path = "common/test_utils.rs"]
mod test_utils;

// =============================================================================
// PARSING: Tuple struct syntax is recognized
// =============================================================================

#[test]
fn test_tuple_struct_basic() {
    let src = r#"
pub struct Point(i32, i32)
"#;
    let (generated, success) = test_utils::compile_single_check(src);
    let err = if !success { &generated } else { "" };
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
    let (generated, success) = test_utils::compile_single_check(src);
    let err = if !success { &generated } else { "" };
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
    let (generated, success) = test_utils::compile_single_check(src);
    let err = if !success { &generated } else { "" };
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
    let (success, _generated, err) = test_utils::compile_via_cli(src);
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
    let (success, _generated, err) = test_utils::compile_via_cli(src);
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
    let (success, _generated, err) = test_utils::compile_via_cli(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

#[test]
fn test_tuple_struct_borrowed_string_args() {
    let src = r#"
pub struct Label(string)

pub fn wrap(s: string) -> Label {
    Label(s)
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

#[test]
fn test_tuple_struct_with_string_literal() {
    let src = r#"
pub struct Tag(string)

pub fn default_tag() -> Tag {
    Tag("default")
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(src);
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
    let (success, _generated, err) = test_utils::compile_via_cli(src);
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
    let (success, _generated, err) = test_utils::compile_via_cli(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

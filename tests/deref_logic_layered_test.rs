//! TDD: 3-Layered Ownership System for Deref/Clone Logic
//!
//! Tests for generate_expression_with_target_ownership migration.
//! Replaces scattered deref/clone decisions with systematic layered approach.
//!
//! Layer 1: Track ownership from variables/parameters (ownership_tracker)
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
// Struct Literal - Target: Owned
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_struct_literal_copy_field() {
    let src = r#"
pub struct Point { pub x: i32, pub y: i32 }
pub fn make(x: &i32, y: &i32) -> Point {
    Point { x: x, y: y }
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    // x, y are &i32 (Borrowed). i32 is Copy, so auto-copy in struct literal
    assert!(
        result.contains("Point { x: x, y: y }") || result.contains("Point { x, y }"),
        "Expected Point {{ x: x, y: y }} or shorthand. Got:\n{}",
        result
    );
    assert!(!result.contains("*x"), "Copy types should not need explicit deref");
    assert!(!result.contains(".clone()"), "Copy types should not need clone");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_struct_literal_noncopy_field_needs_clone() {
    let src = r#"
pub struct Data { pub name: string }
pub fn copy_name(d: &Data) -> string {
    d.name
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    // d.name is &String (Borrowed). String is non-Copy, needs .clone()
    assert!(
        result.contains("d.name.clone()") || result.contains("(d.name).clone()"),
        "Non-Copy field from borrowed struct needs .clone(). Got:\n{}",
        result
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_struct_literal_owned_field_no_coercion() {
    let src = r#"
pub struct Wrapper { pub val: i32 }
pub fn wrap(v: i32) -> Wrapper {
    Wrapper { val: v }
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    assert!(result.contains("Wrapper { val: v }") || result.contains("Wrapper { val, .. }"));
}

// =============================================================================
// Function Arguments - Target: from param type
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_function_arg_auto_borrow() {
    let src = r#"
pub fn takes_ref(s: &string) {}
pub fn caller(s: string) {
    takes_ref(s)
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    // s is Owned, param needs Borrowed. Should add &
    assert!(
        result.contains("takes_ref(&s)"),
        "Should auto-borrow owned to &param. Got:\n{}",
        result
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_function_arg_copy_no_borrow() {
    let src = r#"
pub fn takes_int(x: i32) -> i32 { x }
pub fn caller(r: &i32) -> i32 {
    takes_int(r)
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    // r is &i32 (Borrowed). i32 is Copy. Rust auto-copies in function arg.
    assert!(
        result.contains("takes_int(r)") || result.contains("takes_int(*r)"),
        "Copy type in function arg. Got:\n{}",
        result
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_function_arg_owned_needs_clone() {
    let src = r#"
pub fn takes_string(s: string) -> usize { s.len() }
pub fn caller(r: &string) -> usize {
    takes_string(r)
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    assert!(
        result.contains(".clone()"),
        "Non-Copy borrowed to owned param needs clone. Got:\n{}",
        result
    );
}

// =============================================================================
// Comparisons - Target: auto-deref
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_comparison_auto_deref() {
    let src = r#"
pub fn compare(x: &i32, y: &i32) -> bool {
    x == y
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    // x, y are &i32. Comparison auto-derefs. No explicit * needed.
    assert!(
        result.contains("x == y"),
        "Comparison should work without explicit deref. Got:\n{}",
        result
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_comparison_mixed_owned_borrowed() {
    let src = r#"
pub fn cmp(a: i32, b: &i32) -> bool {
    a == b
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

// =============================================================================
// Binary Operations - Target: Owned for Copy
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_binary_op_auto_copy() {
    let src = r#"
pub fn add(x: &i32, y: &i32) -> i32 {
    x + y
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    // x, y are &i32. i32 is Copy. Binary op auto-copies.
    assert!(
        result.contains("x + y"),
        "Copy types in binary op. Got:\n{}",
        result
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_binary_op_int_arithmetic() {
    let src = r#"
pub fn sum(a: &i32, b: &i32) -> i32 {
    a + b + 1
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

// =============================================================================
// Method Calls - Target: from receiver type
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_method_call_borrowed_receiver() {
    let src = r#"
pub struct Counter { pub value: i32 }
impl Counter {
    pub fn get(self) -> i32 { self.value }
}
pub fn use_counter(c: &Counter) -> i32 {
    c.get()
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    // c is &Counter. get() takes owned self. Need .clone() or pass by ref.
    // Counter may need Clone. Check for clone or ref.
    assert!(
        result.contains("c.get()") || result.contains("c.clone().get()") || result.contains("(*c).get()"),
        "Method call on borrowed. Got:\n{}",
        result
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_method_call_owned_receiver() {
    let src = r#"
pub fn len(s: string) -> usize {
    s.len()
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    assert!(result.contains("s.len()"));
}

// =============================================================================
// For-loop iterator variables
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_for_loop_copy_field() {
    let src = r#"
pub struct Item { pub id: i32 }
pub fn sum_ids(items: &Vec<Item>) -> i32 {
    let mut sum = 0
    for item in items {
        sum = sum + item.id
    }
    sum
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    // item is &Item from iter. item.id is Copy. No clone needed.
    assert!(
        result.contains("item.id"),
        "Copy field from iter var. Got:\n{}",
        result
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_for_loop_noncopy_field() {
    let src = r#"
pub struct Item { pub name: string }
pub fn collect_names(items: &Vec<Item>) -> Vec<string> {
    let mut names = Vec::new()
    for item in items {
        names.push(item.name)
    }
    names
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    assert!(
        result.contains(".clone()"),
        "Non-Copy field from iter needs clone. Got:\n{}",
        result
    );
}

// =============================================================================
// Match/if-let bindings
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_match_arm_copy_binding() {
    let src = r#"
pub fn get_val(opt: &Option<i32>) -> i32 {
    match opt {
        Some(v) => v,
        None => 0
    }
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_if_let_borrowed_binding() {
    let src = r#"
pub fn check(opt: &Option<i32>) -> bool {
    if let Some(x) = opt {
        x > 0
    } else {
        false
    }
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

// =============================================================================
// Edge cases and regression prevention
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_nested_struct_literal() {
    let src = r#"
pub struct Inner { pub x: i32 }
pub struct Outer { pub inner: Inner }
pub fn make(x: &i32) -> Outer {
    Outer { inner: Inner { x: x } }
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_field_access_chain() {
    let src = r#"
pub struct A { pub b: B }
pub struct B { pub c: i32 }
pub fn get(a: &A) -> i32 {
    a.b.c
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_vec_index_borrowed() {
    let src = r#"
pub fn first(items: &Vec<i32>) -> i32 {
    items[0]
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_option_map_borrowed() {
    let src = r#"
pub fn double(opt: &Option<i32>) -> Option<i32> {
    opt.map(|x| x * 2)
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_concat_borrowed() {
    let src = r#"
pub fn greet(name: &string) -> string {
    "Hello, " + name
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_explicit_clone_preserved() {
    let src = r#"
pub fn dup(s: &string) -> string {
    s.clone()
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    assert!(result.contains("clone"));
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_mut_ref_param() {
    let src = r#"
pub fn read_mut(n: &mut i32) -> i32 {
    let x = n
    x
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_tuple_struct_copy() {
    let src = r#"
pub struct Point(i32, i32)
pub fn make_pair(x: &i32, y: &i32) -> Point {
    Point(x, y)
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_return_borrowed_field() {
    let src = r#"
pub struct Data { pub x: i32 }
pub fn get_x(d: &Data) -> i32 {
    d.x
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_assign_from_borrowed() {
    let src = r#"
pub fn assign(r: &i32) -> i32 {
    let x = r
    x
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_multiple_borrowed_params() {
    let src = r#"
pub fn add_three(a: &i32, b: &i32, c: &i32) -> i32 {
    a + b + c
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_borrowed_string_len() {
    let src = r#"
pub fn len(s: &string) -> usize {
    s.len()
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_pass_borrowed_to_fn() {
    let src = r#"
pub fn double(x: i32) -> i32 { x * 2 }
pub fn apply(r: &i32) -> i32 {
    double(r)
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_enum_variant_copy_field() {
    let src = r#"
pub enum E {
    V { x: i32 }
}
pub fn get(e: &E) -> i32 {
    match e {
        E::V { x } => x
    }
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_self_field_borrowed() {
    let src = r#"
pub struct S { pub x: i32 }
impl S {
    pub fn get(self) -> i32 { self.x }
    pub fn get_ref(self) -> i32 {
        let r = &self.x
        r
    }
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_format_macro_arg() {
    let src = r#"
pub fn msg(name: &string) -> string {
    format!("Hello {}", name)
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_borrowed_in_condition() {
    let src = r#"
pub fn check(r: &bool) -> bool {
    if r {
        true
    } else {
        false
    }
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_borrowed_in_binary_logic() {
    let src = r#"
pub fn and(a: &bool, b: &bool) -> bool {
    a && b
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_array_index_copy() {
    let src = r#"
pub fn first(arr: &[i32; 3]) -> i32 {
    arr[0]
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_derive_copy_struct() {
    let src = r#"
@derive(Copy)
pub struct Vec2 { pub x: f32, pub y: f32 }
pub fn add(a: &Vec2, b: &Vec2) -> Vec2 {
    Vec2 { x: a.x + b.x, y: a.y + b.y }
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
}

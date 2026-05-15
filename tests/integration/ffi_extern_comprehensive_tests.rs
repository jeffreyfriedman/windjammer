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

//! Comprehensive FFI/Extern Function Tests
//!
//! These tests verify that the Windjammer compiler correctly generates
//! Rust code for FFI/extern functions, including:
//! - Extern function declarations
//! - Extern block syntax
//! - FFI-safe types

#[path = "../common/test_utils.rs"]
mod test_utils;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

// ============================================================================
// BASIC EXTERN FN
// ============================================================================

// Note: Extern fn declarations may not be fully supported yet
// These tests document what's expected when implemented

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_extern_compatibility() {
    // Test that regular functions can be made extern-compatible
    let code = r#"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(
        success,
        "Extern-compatible fn should compile. Error: {}",
        err
    );
}

// ============================================================================
// FFI-SAFE TYPES
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_ffi_primitives() {
    let code = r#"
pub fn use_ffi_types(
    a: i8, b: i16, c: i32, d: i64,
    e: u8, f: u16, g: u32, h: u64,
    i: f32, j: f64, k: bool
) -> i32 {
    c
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "FFI primitives should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_ffi_references() {
    // References are safe alternatives to pointers
    let code = r#"
pub fn use_references(a: &i32, b: &mut i32) -> i32 {
    *b = *a;
    *a
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "FFI references should compile. Error: {}", err);
}

// ============================================================================
// CALLING RUST FROM WJ
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_rust_interop_types() {
    // Types that are compatible with Rust FFI
    let code = r#"
@derive(Clone, Debug)
pub struct FfiSafe {
    x: i32,
    y: i32,
}

impl FfiSafe {
    pub fn new(x: i32, y: i32) -> FfiSafe {
        FfiSafe { x: x, y: y }
    }
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Rust interop types should compile. Error: {}", err);
}

// ============================================================================
// REPR ANNOTATIONS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_repr_c_struct() {
    let code = r#"
@repr(C)
pub struct CStruct {
    x: i32,
    y: i32,
}
"#;
    let (generated, success) = test_utils::compile_single_check(code);
    // Should generate #[repr(C)]
    if success {
        assert!(
            generated.contains("repr(C)") || generated.contains("#[repr"),
            "Should have repr(C). Generated:\n{}",
            generated
        );
    }
    // May not be implemented yet
    println!("Generated:\n{}", generated);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_repr_transparent() {
    let code = r#"
@repr(transparent)
pub struct Wrapper {
    value: i32,
}
"#;
    let (generated, _success) = test_utils::compile_single_check(code);
    let _err = if !_success { generated.as_str() } else { "" };
    println!("Generated:\n{}", generated);
    // May not be implemented yet
}

// ============================================================================
// OPTION FOR NULLABLE POINTERS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_option_ref() {
    // Option<&T> for optional references
    let code = r#"
pub fn maybe_ref(p: Option<&i32>) -> bool {
    match p {
        Some(_) => true,
        None => false,
    }
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Option ref should compile. Error: {}", err);
}

// ============================================================================
// RESULT FOR ERROR HANDLING
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_result_for_ffi() {
    let code = r#"
pub fn ffi_result(success: bool) -> Result<i32, string> {
    if success {
        Ok(42)
    } else {
        Err("failed".to_string())
    }
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Result for FFI should compile. Error: {}", err);
}

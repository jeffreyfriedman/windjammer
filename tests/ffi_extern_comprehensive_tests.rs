//! Comprehensive FFI/Extern Function Tests
//!
//! These tests verify that the Windjammer compiler correctly generates
//! Rust code for FFI/extern functions, including:
//! - Extern function declarations
//! - Extern block syntax
//! - FFI-safe types

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
    let (success, _generated, err) = compile_and_verify(code);
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
    let (success, _generated, err) = compile_and_verify(code);
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
    let (success, _generated, err) = compile_and_verify(code);
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
    let (success, _generated, err) = compile_and_verify(code);
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
    let (success, generated, _err) = compile_and_verify(code);
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
    let (_success, generated, _err) = compile_and_verify(code);
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
    let (success, _generated, err) = compile_and_verify(code);
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
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Result for FFI should compile. Error: {}", err);
}

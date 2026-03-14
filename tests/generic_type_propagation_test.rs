//! TDD: Generic type parameter propagation in Rust codegen
//!
//! Bug: E0425 - "cannot find type 'T' in this scope" (19 errors in windjammer-game)
//! Root cause: Codegen doesn't preserve generic type parameters when generating Rust
//!
//! Philosophy: "Compiler does hard work" - type parameter propagation is mechanical

use std::path::PathBuf;
use std::process::Command;

/// Helper to compile a test fixture and return the generated Rust code
fn compile_fixture(fixture_name: &str) -> Result<String, String> {
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(format!("{}.wj", fixture_name));

    let output_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_output")
        .join(fixture_name);
    std::fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;

    let compiler_output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            fixture_path.to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .map_err(|e| format!("Failed to run compiler: {}", e))?;

    if !compiler_output.status.success() {
        return Err(format!(
            "Compiler failed: {}",
            String::from_utf8_lossy(&compiler_output.stderr)
        ));
    }

    let rust_file = output_dir.join(format!("{}.rs", fixture_name));
    std::fs::read_to_string(rust_file).map_err(|e| format!("Failed to read generated code: {}", e))
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_generic_function_preserves_type_parameter() {
    let generated = compile_fixture("generic_type_propagation").expect("Compilation failed");

    // Verify generated Rust has <T> preserved in function signature
    assert!(
        generated.contains("fn identity<T>") || generated.contains("pub fn identity<T>"),
        "Generic function should preserve <T> in signature. Generated:\n{}",
        generated
    );

    // Verify parameter and return type use T
    assert!(
        generated.contains("value: T") || generated.contains("value: T)"),
        "Parameter type T should be preserved. Generated:\n{}",
        generated
    );

    // Verify rustc compiles it
    let output_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_output")
        .join("generic_type_propagation");
    let rs_file = output_dir.join("generic_type_propagation.rs");

    let rustc_output = Command::new("rustc")
        .args([rs_file.to_str().unwrap(), "--crate-type=lib", "--edition=2021"])
        .output()
        .expect("rustc failed");

    assert!(
        rustc_output.status.success(),
        "Generated Rust should compile. rustc stderr:\n{}",
        String::from_utf8_lossy(&rustc_output.stderr)
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_generic_struct_preserves_type_parameter() {
    let generated = compile_fixture("generic_struct_impl").expect("Compilation failed");

    // Verify struct has <T>
    assert!(
        generated.contains("struct Container<T>"),
        "Generic struct should preserve <T>. Generated:\n{}",
        generated
    );

    // Verify impl has <T>
    assert!(
        generated.contains("impl<T> Container<T>"),
        "Generic impl should preserve impl<T>. Generated:\n{}",
        generated
    );

    // Verify method return type uses T
    assert!(
        generated.contains("-> Container<T>") || generated.contains("-> T"),
        "Method return types should use T. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_generic_impl_method_preserves_type_parameter() {
    let generated = compile_fixture("generic_method").expect("Compilation failed");

    // Method add_entity has its own <T> (not from impl - Scene has no type params)
    assert!(
        generated.contains("fn add_entity<T>") || generated.contains("pub fn add_entity<T>"),
        "Method with own type param should preserve <T>. Generated:\n{}",
        generated
    );

    assert!(
        generated.contains("_entity: T") || generated.contains("entity: T"),
        "Parameter type T should be preserved. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_generic_function_with_wrapping_decorator_preserves_type_parameter() {
    // When a generic function has @timeout, @bench, etc., it goes through
    // generate_function_with_wrapping - that path must also emit <T>
    let generated = compile_fixture("generic_with_test").expect("Compilation failed");

    assert!(
        generated.contains("fn identity<T>") || generated.contains("pub fn identity<T>"),
        "Generic function with wrapping decorator should preserve <T>. Generated:\n{}",
        generated
    );
}

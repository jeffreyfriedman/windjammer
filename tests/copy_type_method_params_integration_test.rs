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

// Integration test for Copy type parameters in methods
// Bug: Compiler incorrectly infers &Copy instead of Copy

use std::process::Command;

use tempfile::tempdir;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_copy_type_method_params() {
    // Compile the Windjammer file
    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            "tests/copy_type_method_params_test.wj",
            "--output",
            "tests/generated/copy_type_method_params",
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run windjammer compiler");

    if !output.status.success() {
        panic!(
            "Windjammer compilation failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Read the generated Rust code
    let generated = std::fs::read_to_string(
        "tests/generated/copy_type_method_params/copy_type_method_params_test.rs",
    )
    .expect("Failed to read generated file");

    println!("Generated code:\n{}", generated);

    // Verify Copy type parameters are NOT borrowed in methods
    assert!(
        !generated.contains("color: &Color"),
        "Copy type 'Color' should not be borrowed in method parameters"
    );
    assert!(
        generated.contains("color: Color"),
        "Copy type 'Color' should be passed by value"
    );

    // Verify the methods exist with correct signatures
    // Copy types use self by value (no indirection needed)
    // Note: unused params (x, y, radius) get _ prefix from the compiler's unused-param detection
    assert!(
        generated
            .contains("pub fn draw_circle(self, _x: f32, _y: f32, _radius: f32, color: Color)")
            || generated.contains("pub fn draw_circle(&self, _x: f32, _y: f32, _radius: f32, color: Color)"),
        "draw_circle should be self or &self with unused params prefixed"
    );
    assert!(
        generated
            .contains("pub fn draw_rect(self, _x: f32, _y: f32, w: f32, h: f32, color: Color)")
            || generated.contains("pub fn draw_rect(&self, _x: f32, _y: f32, w: f32, h: f32, color: Color)"),
        "draw_rect should be self or &self with unused params prefixed"
    );

    // Type-check generated Rust in a temp dir (avoid leaving .rlib under tests/generated)
    let rs_path = "tests/generated/copy_type_method_params/copy_type_method_params_test.rs";
    let tmp = tempdir().expect("tempdir");
    let compile_output = Command::new("rustc")
        .args([
            "--crate-type=lib",
            "--emit=metadata",
            "--edition",
            "2021",
            "-o",
        ])
        .arg(tmp.path().join("verify.rmeta"))
        .arg(rs_path)
        .output()
        .expect("Failed to run rustc");

    if !compile_output.status.success() {
        panic!(
            "Generated Rust code failed to compile:\n{}",
            String::from_utf8_lossy(&compile_output.stderr)
        );
    }
}

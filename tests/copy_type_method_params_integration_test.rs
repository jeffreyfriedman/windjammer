// Integration test for Copy type parameters in methods
// Bug: Compiler incorrectly infers &Copy instead of Copy

use std::process::Command;

#[test]
fn test_copy_type_method_params() {
    // Compile the Windjammer file
    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--",
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
    // Methods that don't access self fields should still be &self (borrowed)
    // This is the correct Rust convention - don't consume self unless needed
    assert!(
        generated.contains("pub fn draw_circle(&self, x: f32, y: f32, radius: f32, color: Color)"),
        "draw_circle should be &self (borrowed)"
    );
    assert!(
        generated.contains("pub fn draw_rect(&self, x: f32, y: f32, w: f32, h: f32, color: Color)"),
        "draw_rect should be &self (borrowed)"
    );

    // Try to compile the generated Rust code with rustc
    let compile_output = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            "--edition",
            "2021",
            "tests/generated/copy_type_method_params/copy_type_method_params_test.rs",
            "--out-dir",
            "tests/generated/copy_type_method_params",
        ])
        .output()
        .expect("Failed to run rustc");

    if !compile_output.status.success() {
        panic!(
            "Generated Rust code failed to compile:\n{}",
            String::from_utf8_lossy(&compile_output.stderr)
        );
    }
}

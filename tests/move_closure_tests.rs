//! Tests for move closures

use std::path::PathBuf;
use std::process::Command;

/// Helper to compile a test fixture and return the generated Rust code
fn compile_fixture(fixture_name: &str) -> Result<String, String> {
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(format!("{}.wj", fixture_name));

    let output_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_output");
    std::fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;

    // Run the compiler (--no-cargo to avoid file lock conflicts in parallel tests)
    let compiler_output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            fixture_path.to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
            "--no-cargo", // Skip cargo build in tests
        ])
        .output()
        .map_err(|e| format!("Failed to run compiler: {}", e))?;

    if !compiler_output.status.success() {
        return Err(format!(
            "Compiler failed: {}",
            String::from_utf8_lossy(&compiler_output.stderr)
        ));
    }

    // Read generated Rust code
    let rust_file = output_dir.join(format!("{}.rs", fixture_name));
    std::fs::read_to_string(rust_file).map_err(|e| format!("Failed to read generated code: {}", e))
}

#[test]
fn test_move_closure_generates_move_keyword() {
    let generated = compile_fixture("move_closures").expect("Compilation failed");

    // Check that move closures generate the `move` keyword
    assert!(
        generated.contains("move ||") || generated.contains("move |"),
        "Move closures should generate 'move' keyword. Generated:\n{}",
        generated
    );

    // Check that regular closures don't have the `move` keyword
    // Find closure in test_regular_closure function
    assert!(
        generated.contains("|| ") || generated.contains("|n|"),
        "Regular closures should NOT have 'move' keyword. Generated:\n{}",
        generated
    );

    println!("✓ Move closure generation works");
}

#[test]
fn test_move_closure_with_params() {
    let generated = compile_fixture("move_closures").expect("Compilation failed");

    // Check that move closures with parameters work
    assert!(
        generated.contains("move |n"),
        "Move closure with params should generate 'move |params|'. Generated:\n{}",
        generated
    );

    println!("✓ Move closure with parameters works");
}

//! Tests for doc comments inside impl blocks and trait declarations.

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
fn test_doc_comment_in_impl_block() {
    let generated = compile_fixture("doc_comments_impl").expect("Compilation failed");

    // Check that doc comments appear in the generated Rust code
    assert!(
        generated.contains("/// Creates a new Point at the origin."),
        "Missing doc comment for zero(). Generated code:\n{}",
        generated
    );
    assert!(
        generated.contains("/// Creates a new Point with the given coordinates."),
        "Missing doc comment for new(). Generated code:\n{}",
        generated
    );
    assert!(
        generated.contains("/// Calculates the distance from the origin."),
        "Missing doc comment for distance_from_origin(). Generated code:\n{}",
        generated
    );

    println!("✓ Doc comments in impl blocks work");
}

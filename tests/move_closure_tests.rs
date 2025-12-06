//! Tests for auto-move closures
//!
//! Windjammer Philosophy: The compiler does the work, not the developer.
//! All closures automatically emit `move` in generated Rust - no explicit
//! keyword needed from the user!

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
fn test_closures_auto_generate_move() {
    let generated = compile_fixture("move_closures").expect("Compilation failed");

    // ALL closures should generate `move` automatically
    // This is the Windjammer philosophy - the compiler infers what the developer shouldn't need to write

    // Check that closures generate `move` without user needing to write it
    assert!(
        generated.contains("move ||") || generated.contains("move |"),
        "Closures should auto-generate 'move' keyword. Generated:\n{}",
        generated
    );

    // Verify thread blocks also use move (they already did)
    assert!(
        generated.contains("std::thread::spawn(move ||"),
        "Thread blocks should use 'move'. Generated:\n{}",
        generated
    );

    println!("✓ Windjammer auto-moves closures - no explicit 'move' keyword needed!");
}

#[test]
fn test_no_explicit_move_keyword_needed() {
    // This test verifies the Windjammer philosophy:
    // The developer writes: |x| x + 1
    // We generate: move |x| x + 1
    //
    // The developer writes: thread { ... }
    // We generate: std::thread::spawn(move || { ... })
    //
    // NO explicit 'move' keyword ever needed!

    let generated = compile_fixture("move_closures").expect("Compilation failed");

    // Count how many `move` keywords appear - should be multiple (auto-generated)
    let move_count = generated.matches("move").count();
    assert!(
        move_count >= 2,
        "Expected multiple auto-generated 'move' keywords, found {}. Generated:\n{}",
        move_count,
        generated
    );

    println!("✓ Found {} auto-generated 'move' keywords", move_count);
}

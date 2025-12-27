// Tests for automatic method argument conversion
// Windjammer Philosophy: The compiler does the work, not the developer

use std::path::PathBuf;
use std::process::Command;

/// Helper to compile a test fixture and return the generated Rust code
fn compile_fixture(fixture_name: &str) -> Result<String, String> {
    let compiler_path = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    if !compiler_path.exists() {
        return Err(format!(
            "Compiler binary not found at: {}",
            compiler_path.display()
        ));
    }

    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(format!("{}.wj", fixture_name));

    // Use unique output dir per fixture to avoid race conditions in parallel tests
    let output_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_output")
        .join(fixture_name);
    std::fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;

    // Run the compiler
    let compiler_output = Command::new(&compiler_path)
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
            "Compiler failed:\nSTDOUT: {}\nSTDERR: {}",
            String::from_utf8_lossy(&compiler_output.stdout),
            String::from_utf8_lossy(&compiler_output.stderr)
        ));
    }

    // Read generated Rust code
    let rust_file = output_dir.join(format!("{}.rs", fixture_name));
    std::fs::read_to_string(&rust_file).map_err(|e| {
        format!(
            "Failed to read generated code at {:?}: {}",
            rust_file.display(),
            e
        )
    })
}

// ============================================================================
// contains() method - should auto-add & for the search argument
// ============================================================================

#[test]
fn test_contains_adds_reference() {
    let generated = compile_fixture("method_arg_conversion").expect("Compilation failed");

    // has_item uses contains(), should add & to argument
    assert!(
        generated.contains("contains(&item)"),
        "Should add & for contains() argument: {}",
        generated
    );
}

// ============================================================================
// push() method - should handle ownership correctly
// ============================================================================

#[test]
fn test_push_with_owned_string() {
    let generated = compile_fixture("method_arg_conversion").expect("Compilation failed");

    // add_item uses push() with owned string
    assert!(
        generated.contains("push(item)"),
        "Should handle push() with owned value: {}",
        generated
    );
}

#[test]
fn test_push_string_literal() {
    let generated = compile_fixture("method_arg_conversion").expect("Compilation failed");

    // add_hello uses push("hello"), should convert to String
    assert!(
        generated.contains("push(\"hello\".to_string())"),
        "Should convert string literal for push(): {}",
        generated
    );
}

// ============================================================================
// String methods - starts_with / ends_with
// ============================================================================

#[test]
fn test_starts_with() {
    let generated = compile_fixture("method_arg_conversion").expect("Compilation failed");

    // check_prefix uses starts_with()
    assert!(
        generated.contains("starts_with("),
        "Should handle starts_with(): {}",
        generated
    );
}

#[test]
fn test_ends_with_with_literal() {
    let generated = compile_fixture("method_arg_conversion").expect("Compilation failed");

    // is_rust_file uses ends_with(".rs")
    assert!(
        generated.contains("ends_with(\".rs\")"),
        "Should handle ends_with() with literal: {}",
        generated
    );
}

// ============================================================================
// Combined: verify the entire fixture compiles
// ============================================================================

#[test]
fn test_fixture_compiles_successfully() {
    let generated = compile_fixture("method_arg_conversion").expect("Compilation failed");

    // Debug output to understand CI failures
    if generated.is_empty() {
        eprintln!("WARNING: Generated code is EMPTY!");
    } else {
        eprintln!("Generated code length: {} bytes", generated.len());
    }

    // Basic sanity check - should have the struct
    assert!(
        generated.contains("struct ItemList"),
        "Should generate ItemList struct (length={}): {}",
        generated.len(),
        if generated.len() > 500 {
            &generated[..500]
        } else {
            &generated
        }
    );

    // Should have impl block
    assert!(
        generated.contains("impl ItemList"),
        "Should generate impl block: {}",
        generated
    );
}

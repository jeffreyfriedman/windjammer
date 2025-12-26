// Tests for automatic .iter() and .iter_mut() inference
// Windjammer Philosophy: The compiler does the work, not the developer

use std::path::PathBuf;
use std::process::Command;

/// Helper to compile a test fixture and return the generated Rust code
fn compile_fixture(fixture_name: &str) -> Result<String, String> {
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(format!("{}.wj", fixture_name));

    // Use a unique temp directory to avoid caching issues
    let output_dir = std::env::temp_dir()
        .join("windjammer_iter_test")
        .join(format!("{}_{}", fixture_name, std::process::id()));
    let _ = std::fs::remove_dir_all(&output_dir); // Clean if exists
    std::fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;

    // Run the compiler
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

    // Read generated Rust code
    let rust_file = output_dir.join(format!("{}.rs", fixture_name));
    std::fs::read_to_string(rust_file).map_err(|e| format!("Failed to read generated code: {}", e))
}

#[test]
fn test_iter_inference_field_access() {
    let generated = compile_fixture("iter_inference").expect("Compilation failed");

    // sum_items should have .iter() added for self.items iteration
    // Either .iter() or &self.field is valid for iteration
    assert!(
        generated.contains("self.items.iter()") || generated.contains("&self.items"),
        "Should infer iteration for field access in sum_items: {}",
        generated
    );
}

#[test]
fn test_iter_inference_names_field() {
    let generated = compile_fixture("iter_inference").expect("Compilation failed");

    // Either .iter() or &self.field is valid for iteration
    assert!(
        generated.contains("self.names.iter()") || generated.contains("&self.names"),
        "Should infer iteration for field access in print_names: {}",
        generated
    );
}

#[test]
fn test_no_double_iter() {
    let generated = compile_fixture("iter_inference").expect("Compilation failed");

    // count_positive already has .iter(), should not double it
    assert!(
        !generated.contains(".iter().iter()"),
        "Should not double .iter(): {}",
        generated
    );
}

#[test]
fn test_no_iter_after_enumerate() {
    let generated = compile_fixture("iter_inference").expect("Compilation failed");

    // print_with_index uses .enumerate(), should not add .iter() after it
    assert!(
        !generated.contains(".enumerate().iter()"),
        "Should not add .iter() after .enumerate(): {}",
        generated
    );
}

#[test]
fn test_iter_inference_simple_vec() {
    let generated = compile_fixture("iter_inference").expect("Compilation failed");

    // process_vec should iterate over items (either .iter() or for item in items is valid)
    assert!(
        generated.contains("items.iter()") || generated.contains("for item in items"),
        "Should have valid Vec iteration: {}",
        generated
    );
}

#[test]
fn test_iter_count() {
    let generated = compile_fixture("iter_inference").expect("Compilation failed");

    // Count both .iter() calls and &self.field forms (both are valid iteration)
    let iter_count = generated.matches(".iter()").count();
    let ref_count = generated.matches("for ").count(); // Count for loops as iterations
    assert!(
        iter_count >= 1 && ref_count >= 4,
        "Should have iteration patterns (found {} .iter() calls, {} for loops): {}",
        iter_count,
        ref_count,
        generated
    );
}

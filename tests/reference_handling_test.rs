// Tests for proper handling of & and &mut references

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

    // Run the compiler
    let compiler_output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            fixture_path.to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
        ])
        .output()
        .map_err(|e| format!("Failed to run compiler: {}", e))?;

    if !compiler_output.status.success() {
        return Err(format!(
            "Compiler failed: {}",
            String::from_utf8_lossy(&compiler_output.stderr)
        ));
    }

    // Read the generated Rust code
    let rust_file = output_dir.join(format!("{}.rs", fixture_name));
    std::fs::read_to_string(rust_file).map_err(|e| e.to_string())
}

#[test]
fn test_mut_ref_no_double_borrow() {
    let rust_code = compile_fixture("mut_ref_test").expect("Compilation failed");

    // Should generate `modify(&mut vec)`, NOT `&mut &mut vec`
    assert!(
        rust_code.contains("modify(&mut vec)"),
        "Generated code should have single &mut, got: {}",
        rust_code
    );
    assert!(
        !rust_code.contains("&mut &mut"),
        "Generated code should NOT have double &mut, got: {}",
        rust_code
    );

    // Should also have read(&vec) not &&vec
    assert!(
        rust_code.contains("read(&vec)"),
        "Generated code should have single &, got: {}",
        rust_code
    );
    assert!(
        !rust_code.contains("&&vec") && !rust_code.contains("& &vec"),
        "Generated code should NOT have double &, got: {}",
        rust_code
    );
}

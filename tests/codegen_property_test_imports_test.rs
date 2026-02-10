// TDD Test: Property test should automatically import windjammer_runtime::property functions

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn get_wj_compiler() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_wj"))
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_property_test_imports_property_functions() {
    let source = r#"
@test
@property_test(cases: 10)
fn test_addition_commutative(a: i32) {
    assert_eq(a + a, 2 * a)
}
"#;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&wj_path, source).expect("Failed to write test file");
    fs::create_dir_all(&out_dir).expect("Failed to create output dir");

    let output = Command::new(get_wj_compiler())
        .args([
            "build",
            wj_path.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        panic!("Compilation failed:\n{}", stderr);
    }

    // Read generated Rust
    let generated_rs = out_dir.join("test.rs");
    let rust_code = fs::read_to_string(&generated_rs).expect("Failed to read generated Rust");

    // Should import property_test_with_gen1
    assert!(
        rust_code.contains("use windjammer_runtime::property::property_test_with_gen1;")
            || rust_code.contains("use windjammer_runtime::property::*;"),
        "Generated code should import property_test_with_gen1 or property::*\nGenerated:\n{}",
        rust_code
    );

    // Should generate property_test_with_gen1 call
    assert!(
        rust_code.contains("property_test_with_gen1"),
        "Generated code should call property_test_with_gen1\nGenerated:\n{}",
        rust_code
    );

    // Should use rand::random
    assert!(
        rust_code.contains("rand::random"),
        "Generated code should use rand::random for generation\nGenerated:\n{}",
        rust_code
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_property_test_with_multiple_params_imports_correct_function() {
    let source = r#"
@test
@property_test(cases: 20)
fn test_addition_commutative(a: i32, b: i32) {
    assert_eq(a + b, b + a)
}
"#;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&wj_path, source).expect("Failed to write test file");
    fs::create_dir_all(&out_dir).expect("Failed to create output dir");

    let output = Command::new(get_wj_compiler())
        .args([
            "build",
            wj_path.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        panic!("Compilation failed:\n{}", stderr);
    }

    // Read generated Rust
    let generated_rs = out_dir.join("test.rs");
    let rust_code = fs::read_to_string(&generated_rs).expect("Failed to read generated Rust");

    // Should import property_test_with_gen2
    assert!(
        rust_code.contains("use windjammer_runtime::property::property_test_with_gen2;")
            || rust_code.contains("use windjammer_runtime::property::*;"),
        "Generated code should import property_test_with_gen2 or property::*\nGenerated:\n{}",
        rust_code
    );

    // Should generate property_test_with_gen2 call
    assert!(
        rust_code.contains("property_test_with_gen2"),
        "Generated code should call property_test_with_gen2\nGenerated:\n{}",
        rust_code
    );
}

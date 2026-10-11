#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

//! Integration tests for multi-target code generation
//!
//! Tests that verify Windjammer can correctly compile to Rust, JavaScript, and WebAssembly
//! with consistent behavior across all targets.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Helper function to compile Windjammer code to a specific target
fn compile_to_target(source: &str, target: &str) -> Result<TempDir, String> {
    let temp_dir = TempDir::new().map_err(|e| format!("Failed to create temp dir: {}", e))?;
    let source_file = temp_dir.path().join("test.wj");

    fs::write(&source_file, source).map_err(|e| format!("Failed to write source file: {}", e))?;

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--no-cargo")
        .arg("--target")
        .arg(target)
        .arg(&source_file)
        .arg("--output")
        .arg(temp_dir.path().join("build"))
        .output()
        .map_err(|e| format!("Failed to run wj: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Compilation failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(temp_dir)
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_simple_function_rust() {
    let source = r#"
fn add(a: int, b: int) -> int {
    a + b
}

fn main() {
    let result = add(2, 3)
    println!("{}", result)
}
"#;

    let temp_dir = compile_to_target(source, "rust").expect("Rust compilation failed");

    // Verify generated files exist
    let build_dir = temp_dir.path().join("build");
    assert!(
        build_dir.join("Cargo.toml").exists(),
        "Cargo.toml should exist"
    );

    // Check for generated Rust files (typically test.rs for single file compilation)
    let rust_files: Vec<_> = fs::read_dir(&build_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("rs"))
        .collect();

    assert!(!rust_files.is_empty(), "Should have at least one .rs file");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_simple_function_wasm() {
    let source = r#"
fn add(a: int, b: int) -> int {
    a + b
}

fn main() {
    let result = add(2, 3)
    println!("{}", result)
}
"#;

    let temp_dir = compile_to_target(source, "wasm").expect("WASM compilation failed");

    // Verify generated files exist
    let build_dir = temp_dir.path().join("build");
    assert!(
        build_dir.join("Cargo.toml").exists(),
        "Cargo.toml should exist"
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_package_json_generation() {
    let source = r#"
fn hello() {
    println!("Hello, World!")
}
"#;

    let temp_dir = compile_to_target(source, "javascript").expect("JavaScript compilation failed");

    let build_dir = temp_dir.path().join("build");
    let package_json =
        fs::read_to_string(build_dir.join("package.json")).expect("Failed to read package.json");

    // Verify package.json is valid JSON
    let json: serde_json::Value =
        serde_json::from_str(&package_json).expect("package.json should be valid JSON");

    assert!(json["name"].is_string(), "Should have name field");
    assert!(json["version"].is_string(), "Should have version field");
    assert!(json["type"] == "module", "Should be ES module");
    assert!(json["engines"].is_object(), "Should have engines field");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_rust_output_still_works() {
    // Ensure Rust output hasn't regressed
    let source = r#"
fn fibonacci(n: int) -> int {
    if n <= 1 {
        n
    } else {
        fibonacci(n - 1) + fibonacci(n - 2)
    }
}

fn main() {
    let result = fibonacci(10)
    println!("{}", result)
}
"#;

    let temp_dir = compile_to_target(source, "rust").expect("Rust compilation failed");

    let build_dir = temp_dir.path().join("build");
    let cargo_toml =
        fs::read_to_string(build_dir.join("Cargo.toml")).expect("Failed to read Cargo.toml");

    assert!(
        cargo_toml.contains("[package]"),
        "Should have package section"
    );
    assert!(cargo_toml.contains("name"), "Should have name field");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_all_targets_compile_same_source() {
    // Verify the same source can compile to all three targets without errors
    let source = r#"
fn add(a: int, b: int) -> int {
    a + b
}

fn main() {
    let x = add(2, 3)
    println!("{}", x)
}
"#;

    // Compile to all targets
    compile_to_target(source, "rust").expect("Rust compilation should succeed");
    compile_to_target(source, "javascript").expect("JavaScript compilation should succeed");
    compile_to_target(source, "wasm").expect("WASM compilation should succeed");
}

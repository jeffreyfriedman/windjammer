use anyhow::Result;
/// TDD Test: FFI Dependencies in Test Framework
///
/// PROBLEM: When FFI files use external crates (like wgpu), the test library
/// doesn't have those dependencies, causing compilation errors.
///
/// SOLUTION: When copying FFI files to test library, also copy their dependencies
/// from the project's Cargo.toml to the test library's Cargo.toml.
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn get_wj_compiler() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_wj"))
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_ffi_dependencies_copied_to_test_library() -> Result<()> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("wj_ffi_deps_test_{}", timestamp));
    fs::create_dir_all(&temp_dir)?;

    // Create src_wj with a simple test
    let src_wj_dir = temp_dir.join("src_wj");
    fs::create_dir_all(&src_wj_dir)?;

    let simple_wj = src_wj_dir.join("simple.wj");
    fs::write(
        &simple_wj,
        r#"
fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#,
    )?;

    // Create src/ffi with a file that uses wgpu
    let src_ffi_dir = temp_dir.join("src").join("ffi");
    fs::create_dir_all(&src_ffi_dir)?;

    let ffi_mod_rs = src_ffi_dir.join("mod.rs");
    fs::write(
        &ffi_mod_rs,
        r#"
pub mod gpu;
pub use gpu::*;
"#,
    )?;

    let ffi_gpu_rs = src_ffi_dir.join("gpu.rs");
    fs::write(
        &ffi_gpu_rs,
        r#"
use wgpu;

pub fn gpu_init() -> bool {
    // Uses wgpu
    true
}
"#,
    )?;

    // Create Cargo.toml with wgpu dependency
    let cargo_toml = temp_dir.join("Cargo.toml");
    fs::write(
        &cargo_toml,
        r#"
[package]
name = "ffi-deps-test"
version = "0.1.0"
edition = "2021"

[dependencies]
wgpu = "0.19"
"#,
    )?;

    // Create wj.toml
    let wj_toml = temp_dir.join("wj.toml");
    fs::write(
        &wj_toml,
        r#"
[package]
name = "ffi-deps-test"
version = "0.1.0"

[dependencies]
"#,
    )?;

    // Create tests_wj directory
    let tests_wj_dir = temp_dir.join("tests_wj");
    fs::create_dir_all(&tests_wj_dir)?;

    let test_wj = tests_wj_dir.join("simple_test.wj");
    fs::write(
        &test_wj,
        r#"
@test
fn test_simple() {
    assert!(true);
}
"#,
    )?;

    // Run wj test
    let wj_compiler = get_wj_compiler();
    let output = Command::new(&wj_compiler)
        .arg("test")
        .arg(&test_wj)
        .current_dir(&temp_dir)
        .output()?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);

    // Should NOT have "no external crate `wgpu`" error
    assert!(
        !stderr.contains("no external crate `wgpu`")
            && !stdout.contains("no external crate `wgpu`"),
        "Test library should have wgpu dependency when FFI uses it.\nSTDOUT:\n{}\nSTDERR:\n{}",
        stdout,
        stderr
    );

    // Test should compile (might not run successfully, but should compile)
    // Check that library compilation succeeded (FFI dependency was copied correctly)
    assert!(
        stdout.contains("Library compiled successfully")
            || stdout.contains("test result:")
            || stderr.contains("test result:"),
        "Test library should compile successfully with FFI dependencies.\nSTDOUT:\n{}\nSTDERR:\n{}",
        stdout,
        stderr
    );

    Ok(())
}

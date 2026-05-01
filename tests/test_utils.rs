//! Centralized test utilities for Windjammer compiler tests.
//!
//! Include in your test file with:
//! ```rust
//! #[path = "test_utils.rs"]
//! mod test_utils;
//! use test_utils::*;
//! ```
//!
//! Provides common compilation helpers that properly isolate temp directories,
//! eliminating race conditions in parallel test execution.
#![allow(dead_code)]

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;
use windjammer::compiler::build_project;
use windjammer::CompilationTarget;

/// Compile a single `.wj` source string to Rust and return the generated code.
/// Panics if compilation fails.
pub fn compile_single(source: &str) -> String {
    compile_single_result(source).unwrap_or_else(|e| panic!("Compilation failed:\n{}", e))
}

/// Compile a single `.wj` source string to Rust, returning Result.
pub fn compile_single_result(source: &str) -> Result<String, String> {
    let tmp = TempDir::new().expect("tempdir");
    let wj_file = tmp.path().join("test.wj");
    fs::write(&wj_file, source).unwrap();
    let out_dir = tmp.path().join("build");

    build_project(&wj_file, &out_dir, CompilationTarget::Rust, false).map_err(|e| e.to_string())?;

    fs::read_to_string(out_dir.join("test.rs"))
        .map_err(|e| format!("Failed to read generated file: {}", e))
}

/// Compile a single `.wj` source string and return (generated_rust, success).
/// Does NOT panic on compilation failure — returns empty string with success=false.
pub fn compile_single_check(source: &str) -> (String, bool) {
    let tmp = TempDir::new().expect("tempdir");
    let wj_file = tmp.path().join("test.wj");
    fs::write(&wj_file, source).unwrap();
    let out_dir = tmp.path().join("build");

    let success = build_project(&wj_file, &out_dir, CompilationTarget::Rust, false).is_ok();

    let generated = fs::read_to_string(out_dir.join("test.rs")).unwrap_or_default();
    (generated, success)
}

/// Compile using the `wj` binary (CLI) and return (success, stdout, stderr).
/// Use this when you need to test CLI behavior or need stdout/stderr output.
pub fn compile_via_cli(source: &str) -> (bool, String, String) {
    let tmp = TempDir::new().expect("tempdir");
    let wj_file = tmp.path().join("test.wj");
    fs::write(&wj_file, source).unwrap();
    let out_dir = tmp.path().join("build");

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            wj_file.to_str().unwrap(),
            "--output",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj binary");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (output.status.success(), stdout, stderr)
}

/// Compile using the `wj` binary and return the generated Rust code.
/// Returns (generated_rust, success).
pub fn compile_via_cli_read(source: &str) -> (String, bool) {
    let tmp = TempDir::new().expect("tempdir");
    let wj_file = tmp.path().join("test.wj");
    fs::write(&wj_file, source).unwrap();
    let out_dir = tmp.path().join("build");

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            wj_file.to_str().unwrap(),
            "--output",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj binary");

    let generated = fs::read_to_string(out_dir.join("test.rs")).unwrap_or_default();
    (generated, output.status.success())
}

/// Compile a named `.wj` file (useful when testing specific filename handling).
pub fn compile_named(source: &str, filename: &str) -> String {
    let tmp = TempDir::new().expect("tempdir");
    let wj_file = tmp.path().join(filename);
    fs::write(&wj_file, source).unwrap();
    let out_dir = tmp.path().join("build");

    build_project(&wj_file, &out_dir, CompilationTarget::Rust, false)
        .unwrap_or_else(|e| panic!("Compilation of {} failed:\n{}", filename, e));

    let rs_name = filename.replace(".wj", ".rs");
    fs::read_to_string(out_dir.join(&rs_name))
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", rs_name, e))
}

/// Compile a named `.wj` file and return (generated_rust, success).
pub fn compile_named_check(source: &str, filename: &str) -> (String, bool) {
    let tmp = TempDir::new().expect("tempdir");
    let wj_file = tmp.path().join(filename);
    fs::write(&wj_file, source).unwrap();
    let out_dir = tmp.path().join("build");

    let success = build_project(&wj_file, &out_dir, CompilationTarget::Rust, false).is_ok();

    let rs_name = filename.replace(".wj", ".rs");
    let generated = fs::read_to_string(out_dir.join(&rs_name)).unwrap_or_default();
    (generated, success)
}

/// Create a temporary project directory with source files and return (TempDir, project_path).
/// The TempDir must be kept alive for the duration of the test.
pub fn create_temp_project(files: &[(&str, &str)]) -> (TempDir, PathBuf) {
    let tmp = TempDir::new().expect("tempdir");
    let project = tmp.path().to_path_buf();

    for (name, content) in files {
        let path = project.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, content).unwrap();
    }

    (tmp, project)
}

/// Verify generated Rust code compiles with rustc (type-checking only, no binary output).
pub fn verify_rust_compiles(rust_code: &str) -> Result<(), String> {
    let tmp = TempDir::new().expect("tempdir");
    let rs_file = tmp.path().join("verify.rs");
    fs::write(&rs_file, rust_code).unwrap();

    let output = Command::new("rustc")
        .arg("--edition=2021")
        .arg("--crate-type=lib")
        .arg("--emit=metadata")
        .arg("-o")
        .arg(tmp.path().join("verify.rmeta"))
        .arg(&rs_file)
        .output()
        .map_err(|e| format!("failed to run rustc: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

/// Convert a path to TOML-safe string (forward slashes, no Windows \\?\ prefix).
pub fn path_to_toml_string(path: &std::path::Path) -> String {
    let s = path.display().to_string();
    let s = s.strip_prefix(r"\\?\").unwrap_or(&s);
    s.replace('\\', "/")
}

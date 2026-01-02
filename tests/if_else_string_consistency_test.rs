//! TDD Test: If-else branches should have consistent String types
//!
//! When one branch returns a string literal and another returns a field,
//! both should be String for consistency.
//!
//! NOTE: All tests in this file spawn subprocesses (cargo run) which are very slow
//! under tarpaulin instrumentation, so they're skipped in coverage runs.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_and_verify(code: &str) -> (bool, String, String) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&wj_path, code).expect("Failed to write test file");
    fs::create_dir_all(&out_dir).expect("Failed to create output dir");

    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--",
            "build",
            wj_path.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return (false, String::new(), stderr);
    }

    let generated_path = out_dir.join("test.rs");
    let generated = fs::read_to_string(&generated_path)
        .unwrap_or_else(|_| "Failed to read generated file".to_string());

    let rustc_output = Command::new("rustc")
        .arg("--crate-type=lib")
        .arg(&generated_path)
        .arg("-o")
        .arg(temp_dir.path().join("test.rlib"))
        .output();

    match rustc_output {
        Ok(output) => {
            let rustc_success = output.status.success();
            let rustc_err = String::from_utf8_lossy(&output.stderr).to_string();
            (rustc_success, generated, rustc_err)
        }
        Err(e) => (false, generated, format!("Failed to run rustc: {}", e)),
    }
}

// Skip in coverage - spawns subprocess (very slow under tarpaulin)
#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_if_else_literal_vs_field() {
    // If one branch returns literal, the other returns field
    let code = r#"
pub struct Item {
    name: string,
}

impl Item {
    pub fn display_name(&self) -> string {
        if self.name == "" {
            "Unnamed"
        } else {
            self.name
        }
    }
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    println!("Generated:\n{}", generated);
    if !success {
        println!("Rustc error:\n{}", err);
    }

    assert!(
        success,
        "If-else with literal vs field should compile. Error: {}",
        err
    );
}

// Skip in coverage - spawns subprocess (very slow under tarpaulin)
#[test]
fn test_if_else_both_literals() {
    // Both branches return literals
    let code = r#"
pub fn get_status(active: bool) -> string {
    if active {
        "Active"
    } else {
        "Inactive"
    }
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    println!("Generated:\n{}", generated);
    if !success {
        println!("Rustc error:\n{}", err);
    }

    assert!(
        success,
        "If-else with both literals should compile. Error: {}",
        err
    );
}

// Skip in coverage - spawns subprocess (very slow under tarpaulin)
#[test]
fn test_if_else_field_vs_literal() {
    // Reversed: field first, literal second
    let code = r#"
pub struct Config {
    value: string,
}

impl Config {
    pub fn get_value(&self) -> string {
        if self.value != "" {
            self.value
        } else {
            "default"
        }
    }
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    println!("Generated:\n{}", generated);
    if !success {
        println!("Rustc error:\n{}", err);
    }

    assert!(
        success,
        "If-else with field vs literal should compile. Error: {}",
        err
    );
}

// Skip in coverage - spawns subprocess (very slow under tarpaulin)
#[test]
fn test_nested_if_else_strings() {
    // Nested if-else with strings
    let code = r#"
pub fn classify(code: i32) -> string {
    if code == 0 {
        "Success"
    } else if code < 100 {
        "Minor error"
    } else if code < 500 {
        "Major error"
    } else {
        "Critical"
    }
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    println!("Generated:\n{}", generated);
    if !success {
        println!("Rustc error:\n{}", err);
    }

    assert!(
        success,
        "Nested if-else with strings should compile. Error: {}",
        err
    );
}

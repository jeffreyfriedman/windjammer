//! TDD Test: Auto-reference method arguments when method expects &T but gets T
//!
//! When a method expects &Vec<T> or &Option<T> but receives Vec<T> or Option<T>,
//! the compiler should automatically add &.

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

#[test]
fn test_auto_ref_vec_arg() {
    // Method expects &Vec<T> but we pass Vec<T>
    let code = r#"
pub fn process_items(items: &Vec<i32>) -> i32 {
    let mut sum = 0
    for item in items {
        sum = sum + item
    }
    sum
}

pub fn test() -> i32 {
    let items = vec![1, 2, 3]
    process_items(items)
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    println!("Generated:\n{}", generated);
    if !success {
        println!("Rustc error:\n{}", err);
    }

    // The items should have & added
    assert!(
        generated.contains("process_items(&items)") || generated.contains("process_items(items)"),
        "Should auto-ref Vec arg. Generated:\n{}",
        generated
    );
    assert!(success, "Generated code should compile. Error: {}", err);
}

#[test]
fn test_auto_ref_option_arg() {
    // Method expects &Option<String> but we pass Option<String>
    let code = r#"
pub fn display_optional(value: &Option<string>) -> string {
    match value {
        Some(s) => s.clone(),
        None => "empty".to_string(),
    }
}

pub fn test() -> string {
    let maybe_name = Some("Alice".to_string())
    display_optional(maybe_name)
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    println!("Generated:\n{}", generated);
    if !success {
        println!("Rustc error:\n{}", err);
    }

    assert!(success, "Generated code should compile. Error: {}", err);
}


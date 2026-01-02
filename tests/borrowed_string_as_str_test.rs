//! TDD Test: Borrowed string parameter with .as_str()
//!
//! When a parameter is borrowed (&str), calling .as_str() on it should
//! either be a no-op or not be generated at all.

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

    // Try to compile the generated Rust code
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
#[ignore] // TODO: Handle .as_str() on &str parameters (unstable Rust feature)
fn test_as_str_on_borrowed_string_param() {
    // When a borrowed string parameter is used with .as_str(),
    // the generated code should compile without needing unstable features
    let code = r#"
pub fn log_message(msg: string) {
    println!("{}", msg.as_str())
}

pub fn test_log() {
    log_message("Hello")
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    if !success {
        println!("Generated code:\n{}", generated);
        println!("Rustc error:\n{}", err);
    }

    // The generated code should either:
    // 1. Not have .as_str() at all (since &str doesn't need it)
    // 2. Or handle it correctly
    assert!(success, "Generated code should compile. Error: {}", err);
}

#[test]
#[ignore] // TODO: Handle .as_str() on owned String correctly
fn test_as_str_on_owned_string() {
    // When calling .as_str() on an owned string, it should work
    let code = r#"
pub fn process(text: string) {
    let owned = text.clone()
    println!("{}", owned.as_str())
}

pub fn test_process() {
    process("Test")
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    if !success {
        println!("Generated code:\n{}", generated);
        println!("Rustc error:\n{}", err);
    }

    assert!(success, "Generated code should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_format_with_borrowed_string() {
    // format!() should work with borrowed strings
    let code = r#"
pub fn greet(name: string) -> string {
    format!("Hello, {}!", name)
}

pub fn test_greet() -> string {
    greet("World")
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    if !success {
        println!("Generated code:\n{}", generated);
        println!("Rustc error:\n{}", err);
    }

    assert!(success, "Generated code should compile. Error: {}", err);
}

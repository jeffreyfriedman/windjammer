//! TDD Test: .as_str() is forbidden in Windjammer source
//!
//! Windjammer automatically handles string conversions (String → &str).
//! Using .as_str() is Rust-specific leakage and must be rejected with
//! a helpful error message guiding the user toward idiomatic Windjammer.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_and_verify(code: &str) -> (bool, String, String) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&wj_path, code).expect("Failed to write test file");
    fs::create_dir_all(&out_dir).expect("Failed to create output dir");

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
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
fn test_as_str_rejected_with_helpful_error() {
    // .as_str() is forbidden in Windjammer source — the compiler should
    // reject it with a clear, helpful error message guiding the user
    let code = r#"
pub fn log_message(msg: string) {
    println!("{}", msg.as_str())
}

pub fn test_log() {
    log_message("Hello")
}
"#;

    let (success, _generated, err) = compile_and_verify(code);

    assert!(!success, ".as_str() should be rejected by the compiler");
    assert!(
        err.contains(".as_str()") && err.contains("forbidden"),
        "Error message should explain that .as_str() is forbidden. Got: {}",
        err
    );
}

#[test]
fn test_as_str_on_owned_string_rejected() {
    // Even on owned strings, .as_str() is forbidden — the compiler handles
    // String → &str conversion automatically
    let code = r#"
pub fn process(text: string) {
    let owned = text.clone()
    println!("{}", owned.as_str())
}

pub fn test_process() {
    process("Test")
}
"#;

    let (success, _generated, err) = compile_and_verify(code);

    assert!(!success, ".as_str() on owned String should be rejected");
    assert!(
        err.contains(".as_str()") && err.contains("forbidden"),
        "Error message should explain that .as_str() is forbidden. Got: {}",
        err
    );
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

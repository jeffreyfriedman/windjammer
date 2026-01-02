//! TDD Test: Return statements with string literals need .to_string()
//!
//! When returning a string literal from a function that returns String,
//! the literal should be converted automatically.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_and_verify(code: &str) -> (bool, String, String) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&wj_path, code).expect("Failed to write test file");
    fs::create_dir_all(&out_dir).expect("Failed to create output dir");

    // Use the pre-built wj binary directly (much faster than cargo run, especially under tarpaulin)
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
#[cfg_attr(tarpaulin, ignore)]
fn test_return_empty_string() {
    // Return empty string literal
    let code = r#"
pub fn get_default() -> string {
    return ""
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    println!("Generated:\n{}", generated);
    if !success {
        println!("Rustc error:\n{}", err);
    }

    assert!(
        generated.contains(r#""".to_string()"#),
        "Empty return should convert to String. Generated:\n{}",
        generated
    );
    assert!(success, "Generated code should compile. Error: {}", err);
}

#[test]
fn test_return_string_literal() {
    // Return non-empty string literal
    let code = r#"
pub fn get_message() -> string {
    return "Hello"
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    println!("Generated:\n{}", generated);
    if !success {
        println!("Rustc error:\n{}", err);
    }

    assert!(
        generated.contains(r#""Hello".to_string()"#),
        "Return should convert to String. Generated:\n{}",
        generated
    );
    assert!(success, "Generated code should compile. Error: {}", err);
}

#[test]
fn test_early_return_string_literal() {
    // Early return with string literal
    let code = r#"
pub struct Widget {
    visible: bool,
}

impl Widget {
    pub fn render(&self) -> string {
        if !self.visible {
            return ""
        }
        return "<div>Visible</div>"
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
        "Early return with string literal should compile. Error: {}",
        err
    );
}

#[test]
fn test_conditional_return_strings() {
    // Multiple conditional returns
    let code = r#"
pub fn status_message(code: i32) -> string {
    if code == 0 {
        return "Success"
    }
    if code == 1 {
        return "Error"
    }
    return "Unknown"
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    println!("Generated:\n{}", generated);
    if !success {
        println!("Rustc error:\n{}", err);
    }

    assert!(
        success,
        "Conditional returns should compile. Error: {}",
        err
    );
}

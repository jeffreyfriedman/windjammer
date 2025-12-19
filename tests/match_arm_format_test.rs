//! TDD Test: Match arms with format! should convert string literals
//!
//! When one match arm uses format!() and another uses a string literal,
//! the string literal should be converted to String for type consistency.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_and_get_generated(code: &str) -> (bool, String, String) {
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

    (true, generated, String::new())
}

#[test]
fn test_match_with_format_and_literal() {
    let code = r#"
fn render_label(value: Option<f32>) -> string {
    let label = match value {
        Some(v) => format!("{:.2}", v),
        None => "N/A",
    }
    label
}
"#;

    let (success, generated, err) = compile_and_get_generated(code);

    println!("Generated:\n{}", generated);
    if !success {
        println!("Compile error:\n{}", err);
    }

    assert!(success, "Compilation should succeed");

    // The "N/A" literal should be converted to "N/A".to_string()
    assert!(
        generated.contains("\"N/A\".to_string()"),
        "String literal in match arm should be converted to String. Generated:\n{}",
        generated
    );
}


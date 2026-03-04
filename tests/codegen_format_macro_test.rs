use std::fs;
/// TDD: Test that format! macro is passed through correctly to Rust
///
/// The format! macro is a Rust macro that should be passed through as-is.
/// Windjammer doesn't need to parse or understand it, just pass it to rustc.
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn get_wj_compiler() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_wj"))
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_format_macro_simple() {
    let source = r#"
fn test_format() -> String {
    let x = 42
    format!("The answer is {}", x)
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("test_format.wj");
    let output_dir = temp_dir.path().join("build");
    fs::write(&input_path, source).unwrap();

    let output = Command::new(get_wj_compiler())
        .args([
            "build",
            input_path.to_str().unwrap(),
            "--no-cargo",
            "--output",
            output_dir.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute wj");

    assert!(
        output.status.success(),
        "Compilation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let generated = fs::read_to_string(output_dir.join("test_format.rs"))
        .expect("Failed to read generated file");

    // Should contain format! macro as-is
    assert!(
        generated.contains("format!"),
        "Generated code should contain format! macro"
    );
    assert!(
        generated.contains(r#""The answer is {}""#),
        "Format string should be preserved"
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_format_macro_in_function_call() {
    let source = r#"
fn log_message(msg: String) {
    // Just a test function
}

fn test_format_in_call() {
    let score = 100
    log_message(format!("Score: {}", score))
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("test_format_call.wj");
    let output_dir = temp_dir.path().join("build");
    fs::write(&input_path, source).unwrap();

    let output = Command::new(get_wj_compiler())
        .args([
            "build",
            input_path.to_str().unwrap(),
            "--no-cargo",
            "--output",
            output_dir.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute wj");

    assert!(
        output.status.success(),
        "Compilation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let generated = fs::read_to_string(output_dir.join("test_format_call.rs"))
        .expect("Failed to read generated file");

    println!("Generated code:\n{}", generated);

    // format!() should be extracted to a temp variable for safety (handles &str/String coercion)
    assert!(
        generated.contains("format!(") && generated.contains("log_message("),
        "format! should be used and log_message should be called"
    );
}

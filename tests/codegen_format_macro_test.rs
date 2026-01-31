use std::fs;
/// TDD: Test that format! macro is passed through correctly to Rust
///
/// The format! macro is a Rust macro that should be passed through as-is.
/// Windjammer doesn't need to parse or understand it, just pass it to rustc.
use std::process::Command;

#[test]
fn test_format_macro_simple() {
    let source = r#"
fn test_format() -> String {
    let x = 42
    format!("The answer is {}", x)
}
"#;

    fs::write("test_format.wj", source).unwrap();

    let output = Command::new("./target/release/wj")
        .args(["build", "test_format.wj", "--no-cargo"])
        .output()
        .expect("Failed to execute wj");

    assert!(
        output.status.success(),
        "Compilation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let generated =
        fs::read_to_string("./build/test_format.rs").expect("Failed to read generated file");

    // Should contain format! macro as-is
    assert!(
        generated.contains("format!"),
        "Generated code should contain format! macro"
    );
    assert!(
        generated.contains(r#""The answer is {}""#),
        "Format string should be preserved"
    );

    fs::remove_file("test_format.wj").ok();
}

#[test]
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

    fs::write("test_format_call.wj", source).unwrap();

    let output = Command::new("./target/release/wj")
        .args(["build", "test_format_call.wj", "--no-cargo"])
        .output()
        .expect("Failed to execute wj");

    assert!(
        output.status.success(),
        "Compilation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let generated =
        fs::read_to_string("./build/test_format_call.rs").expect("Failed to read generated file");

    println!("Generated code:\n{}", generated);

    // Should contain format! macro in function call
    assert!(
        generated.contains("log_message(format!"),
        "format! macro should be inside function call"
    );

    fs::remove_file("test_format_call.wj").ok();
}

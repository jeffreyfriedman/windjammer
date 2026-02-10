//! TDD Test: println without ! macro syntax
//! WINDJAMMER PHILOSOPHY: Reduce ceremony - println should work like a regular function
//!
//! Rules:
//! 1. println("text") -> generates println!("text")
//! 2. println("format {}", var) -> generates println!("format {}", var)
//! 3. Works with string interpolation
//! 4. Other macros keep ! syntax (for now)

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_code(code: &str) -> Result<String, String> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_dir = temp_dir.path();
    let input_file = test_dir.join("test.wj");
    fs::write(&input_file, code).expect("Failed to write source file");

    // Use the pre-built wj binary directly (much faster than cargo run, especially under tarpaulin)
    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            input_file.to_str().unwrap(),
            "--output",
            test_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let generated_file = test_dir.join("test.rs");
    let generated = fs::read_to_string(&generated_file).expect("Failed to read generated file");

    Ok(generated)
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_println_simple_string() {
    // TDD: println("text") should generate println!("text")
    let code = r#"
    pub fn greet() {
        println("Hello, World!")
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // Should generate println! with macro syntax
    assert!(
        generated.contains("println!(\"Hello, World!\")"),
        "Should generate println! macro. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_println_with_format() {
    // TDD: println("format {}", var) should generate println!("format {}", var)
    let code = r#"
    pub fn log_value(x: int) {
        println("Value: {}", x)
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // Should generate println! with format arguments
    assert!(
        generated.contains("println!(\"Value: {}\", x)"),
        "Should generate println! with format args. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_println_multiple_args() {
    // TDD: println with multiple format arguments
    let code = r#"
    pub fn log_pair(a: int, b: int) {
        println("Values: {} and {}", a, b)
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // Should generate println! with multiple arguments
    assert!(
        generated.contains("println!(\"Values: {} and {}\", a, b)"),
        "Should generate println! with multiple args. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_println_with_string_variable() {
    // TDD: println with String variable
    let code = r#"
    pub fn display(message: string) {
        println("Message: {}", message)
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // Should generate println! with string variable
    assert!(
        generated.contains("println!(\"Message: {}\", message)"),
        "Should generate println! with string var. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_println_in_if_statement() {
    // TDD: println inside conditional
    let code = r#"
    pub fn check(x: int) {
        if x > 0 {
            println("Positive: {}", x)
        } else {
            println("Non-positive")
        }
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // Both println calls should be converted
    assert!(
        generated.contains("println!(\"Positive: {}\", x)"),
        "Should generate println! in if block. Generated:\n{}",
        generated
    );

    assert!(
        generated.contains("println!(\"Non-positive\")"),
        "Should generate println! in else block. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_println_with_expression() {
    // TDD: println with expression as argument
    let code = r#"
    pub fn calc(x: int, y: int) {
        println("Sum: {}", x + y)
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // Should generate println! with expression
    assert!(
        generated.contains("println!(\"Sum: {}\", x + y)"),
        "Should generate println! with expression. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_println_with_method_call() {
    // TDD: println with method call result
    let code = r#"
    pub fn show_length(text: string) {
        println("Length: {}", text.len())
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // Should generate println! with method call
    assert!(
        generated.contains("println!(\"Length: {}\", text.len())"),
        "Should generate println! with method call. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_println_keeps_existing_macro_syntax() {
    // TDD: println! with ! should stay as-is (backward compatibility)
    let code = r#"
    pub fn test() {
        println!("With macro!")
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // Should keep the macro syntax
    assert!(
        generated.contains("println!(\"With macro!\")"),
        "Should preserve existing println! syntax. Generated:\n{}",
        generated
    );
}

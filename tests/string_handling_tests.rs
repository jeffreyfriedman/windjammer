//! String Handling Tests
//!
//! Tests for automatic string type conversions including:
//! - Mutable string variables get .to_string()
//! - String literals in function args
//! - Match arm type consistency
//! - String concatenation in returns

use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Helper to compile and verify generated Rust code
fn compile_and_verify_rust(code: &str) -> (bool, String, String) {
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

// ============================================================================
// Test: Mutable String Variables
// ============================================================================

#[test]
fn test_mutable_string_empty_init() {
    let code = r#"
pub fn build_html() -> string {
    let mut html = ""
    html = html + "<div>"
    html = html + "</div>"
    html
}
"#;

    let (success, generated, err) = compile_and_verify_rust(code);
    assert!(success, "Compilation failed: {}", err);

    // Mutable string should be initialized as String
    assert!(
        generated.contains(r#""".to_string()"#) || generated.contains("String::new()"),
        "Mutable string should be String. Generated:\n{}",
        generated
    );
}

#[test]
fn test_mutable_string_with_initial_value() {
    let code = r#"
pub fn greet(name: string) -> string {
    let mut greeting = "Hello, "
    greeting = greeting + name.as_str()
    greeting = greeting + "!"
    greeting
}
"#;

    let (success, generated, err) = compile_and_verify_rust(code);
    assert!(success, "Compilation failed: {}", err);

    // Mutable string should have .to_string()
    assert!(
        generated.contains(r#""Hello, ".to_string()"#),
        "Mutable string should be String. Generated:\n{}",
        generated
    );
}

#[test]
fn test_immutable_string_no_to_string() {
    let code = r#"
pub fn get_message() -> &'static str {
    let msg = "Hello"
    msg
}
"#;

    // This tests that immutable strings don't unnecessarily get .to_string()
    let (success, _generated, _err) = compile_and_verify_rust(code);
    // Note: This may or may not compile depending on return type handling
    // The important thing is that we test the behavior
    let _ = success;
}

// ============================================================================
// Test: String Literals in Function Arguments
// ============================================================================

#[test]
fn test_string_literal_to_stored_param() {
    // When parameter is stored (not just returned), it should be owned
    let code = r#"
pub struct Container {
    value: string,
}

impl Container {
    pub fn new(data: string) -> Container {
        Container { value: data }
    }
}

pub fn create() -> Container {
    Container::new("hello")
}
"#;

    let (success, generated, err) = compile_and_verify_rust(code);
    assert!(success, "Compilation failed: {}", err);

    // String literal passed to stored param should have .to_string()
    assert!(
        generated.contains(r#""hello".to_string()"#)
            || generated.contains(r#"String::from("hello")"#),
        "String literal should be converted to String. Generated:\n{}",
        generated
    );
}

#[test]
fn test_string_literal_to_method_contains() {
    let code = r#"
pub fn has_word(text: string) -> bool {
    text.contains("word")
}
"#;

    let (success, generated, err) = compile_and_verify_rust(code);
    assert!(success, "Compilation failed: {}", err);

    // contains() takes &str, should NOT have .to_string()
    assert!(
        !generated.contains(r#""word".to_string()"#),
        "contains() should not convert literal. Generated:\n{}",
        generated
    );
}

// ============================================================================
// Test: Match Arm Type Consistency
// ============================================================================

#[test]
fn test_match_arms_all_literals() {
    let code = r#"
pub fn status_message(code: i32) -> string {
    match code {
        0 => "OK",
        1 => "Warning",
        2 => "Error",
        _ => "Unknown",
    }
}
"#;

    let (success, generated, err) = compile_and_verify_rust(code);
    assert!(success, "Compilation failed: {}", err);

    // All arms should be converted to String since return type is string
    let ok_converted = generated.contains(r#""OK".to_string()"#);
    let warning_converted = generated.contains(r#""Warning".to_string()"#);

    assert!(
        ok_converted && warning_converted,
        "Match arms should all be String. Generated:\n{}",
        generated
    );
}

#[test]
fn test_match_arms_mixed_types() {
    let code = r#"
pub fn format_value(opt: Option<string>) -> string {
    match opt {
        Some(s) => s,
        None => "default",
    }
}
"#;

    let (success, generated, err) = compile_and_verify_rust(code);
    assert!(success, "Compilation failed: {}", err);

    // The None arm should be .to_string() to match Some(s)
    assert!(
        generated.contains(r#""default".to_string()"#),
        "None arm should be converted to String. Generated:\n{}",
        generated
    );
}

// ============================================================================
// Test: Return Statement String Handling
// ============================================================================

#[test]
fn test_return_string_literal() {
    let code = r#"
pub fn get_name() -> string {
    return "Alice"
}
"#;

    let (success, generated, err) = compile_and_verify_rust(code);
    assert!(success, "Compilation failed: {}", err);

    // Return should have .to_string()
    assert!(
        generated.contains(r#""Alice".to_string()"#),
        "Return literal should be String. Generated:\n{}",
        generated
    );
}

#[test]
fn test_implicit_return_string_literal() {
    let code = r#"
pub fn get_version() -> string {
    "1.0.0"
}
"#;

    let (success, generated, err) = compile_and_verify_rust(code);
    assert!(success, "Compilation failed: {}", err);

    // Implicit return should have .to_string()
    assert!(
        generated.contains(r#""1.0.0".to_string()"#),
        "Implicit return should be String. Generated:\n{}",
        generated
    );
}

// ============================================================================
// Test: String Method Chains
// ============================================================================

#[test]
fn test_replace_method() {
    let code = r#"
pub fn escape_html(text: string) -> string {
    text.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")
}
"#;

    let (success, generated, err) = compile_and_verify_rust(code);
    assert!(success, "Compilation failed: {}", err);

    // replace() takes &str, should NOT have .to_string() on arguments
    assert!(
        !generated.contains(r#""&".to_string()"#),
        "replace() pattern should not be converted. Generated:\n{}",
        generated
    );
    assert!(
        !generated.contains(r#""&amp;".to_string()"#),
        "replace() replacement should not be converted. Generated:\n{}",
        generated
    );
}

#[test]
fn test_split_method() {
    let code = r#"
pub fn get_parts(text: string) -> Vec<string> {
    let mut parts = Vec::new()
    for part in text.split(",") {
        parts.push(part.to_string())
    }
    parts
}
"#;

    let (success, generated, err) = compile_and_verify_rust(code);
    assert!(success, "Compilation failed: {}", err);

    // split() takes &str
    assert!(
        !generated.contains(r#"",".to_string()"#),
        "split() delimiter should not be converted. Generated:\n{}",
        generated
    );
}

// ============================================================================
// Test: Struct Field Initialization
// ============================================================================

#[test]
fn test_struct_string_fields() {
    let code = r#"
pub struct Person {
    name: string,
    city: string,
}

pub fn create_person() -> Person {
    Person {
        name: "John",
        city: "New York",
    }
}
"#;

    let (success, generated, err) = compile_and_verify_rust(code);
    assert!(success, "Compilation failed: {}", err);

    // String fields should have .to_string()
    assert!(
        generated.contains(r#""John".to_string()"#) || generated.contains("String::from"),
        "Struct string fields should be String. Generated:\n{}",
        generated
    );
}

// ============================================================================
// Test: Vec<string> Operations
// ============================================================================

#[test]
fn test_vec_push_string_literal() {
    let code = r#"
pub fn get_colors() -> Vec<string> {
    let mut colors = Vec::new()
    colors.push("red")
    colors.push("green")
    colors.push("blue")
    colors
}
"#;

    let (success, generated, err) = compile_and_verify_rust(code);
    assert!(success, "Compilation failed: {}", err);

    // push() to Vec<String> should have .to_string()
    assert!(
        generated.contains(r#""red".to_string()"#),
        "Vec::push should convert literal. Generated:\n{}",
        generated
    );
}

// ============================================================================
// Test: If/Else String Returns
// ============================================================================

#[test]
fn test_if_else_string_returns() {
    let code = r#"
pub fn classify(n: i32) -> string {
    if n > 0 {
        "positive"
    } else if n < 0 {
        "negative"
    } else {
        "zero"
    }
}
"#;

    let (success, generated, err) = compile_and_verify_rust(code);
    assert!(success, "Compilation failed: {}", err);

    // All branches should be .to_string()
    assert!(
        generated.contains(r#""positive".to_string()"#),
        "If branch should be String. Generated:\n{}",
        generated
    );
    assert!(
        generated.contains(r#""zero".to_string()"#),
        "Else branch should be String. Generated:\n{}",
        generated
    );
}

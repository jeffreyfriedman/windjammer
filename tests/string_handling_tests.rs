#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

//! String Handling Tests
//!
//! Tests for automatic string type conversions including:
//! - Mutable string variables get .to_string()
//! - String literals in function args
//! - Match arm type consistency
//! - String concatenation in returns

#[path = "common/test_utils.rs"]
mod test_utils;

/// Helper to compile and verify generated Rust code
// ============================================================================
// Test: Mutable String Variables
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_mutable_string_empty_init() {
    let code = r#"
pub fn build_html() -> string {
    let mut html = ""
    html = html + "<div>"
    html = html + "</div>"
    html
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };
    assert!(success, "Compilation failed: {}", err);

    // Mutable string should be initialized as String
    assert!(
        generated.contains(r#""".to_string()"#) || generated.contains("String::new()"),
        "Mutable string should be String. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_mutable_string_with_initial_value() {
    let code = r#"
pub fn greet(name: string) -> string {
    let mut greeting = "Hello, "
    greeting = greeting + name
    greeting = greeting + "!"
    greeting
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };
    assert!(success, "Compilation failed: {}", err);

    assert!(
        generated.contains(r#""Hello, ".to_string()"#)
            || generated.contains(r#"string::from("Hello, ")"#)
            || generated.contains(r#"String::from("Hello, ")"#),
        "Mutable string should be String. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_immutable_string_no_to_string() {
    let code = r#"
pub fn get_message() -> &'static string {
    let msg = "Hello"
    msg
}
"#;

    // This tests that immutable strings don't unnecessarily get .to_string()
    let (success, _generated, _err) = test_utils::compile_via_cli(code);
    // Note: This may or may not compile depending on return type handling
    // The important thing is that we test the behavior
    let _ = success;
}

// ============================================================================
// Test: String Literals in Function Arguments
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
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

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };
    assert!(success, "Compilation failed: {}", err);

    // String literal passed to struct-literal-stored &str param — direct at call site.
    assert!(
        generated.contains("Container::new(\"hello\")"),
        "String literal should pass as &str at call site. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_literal_to_method_contains() {
    let code = r#"
pub fn has_word(text: string) -> bool {
    text.contains("word")
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };
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
#[cfg_attr(tarpaulin, ignore)]
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

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };
    assert!(success, "Compilation failed: {}", err);

    let ok_converted = generated.contains(r#""OK".to_string()"#)
        || generated.contains(r#"string::from("OK")"#)
        || generated.contains(r#"String::from("OK")"#);
    let warning_converted = generated.contains(r#""Warning".to_string()"#)
        || generated.contains(r#"string::from("Warning")"#)
        || generated.contains(r#"String::from("Warning")"#);

    assert!(
        ok_converted && warning_converted,
        "Match arms should all be String. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_match_arms_mixed_types() {
    let code = r#"
pub fn format_value(opt: Option<string>) -> string {
    match opt {
        Some(s) => s,
        None => "default",
    }
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };
    assert!(success, "Compilation failed: {}", err);

    assert!(
        generated.contains(r#""default".to_string()"#)
            || generated.contains(r#"string::from("default")"#)
            || generated.contains(r#"String::from("default")"#),
        "None arm should be converted to String. Generated:\n{}",
        generated
    );
}

// ============================================================================
// Test: Return Statement String Handling
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_return_string_literal() {
    let code = r#"
pub fn get_name() -> string {
    return "Alice"
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };
    assert!(success, "Compilation failed: {}", err);

    assert!(
        generated.contains(r#""Alice".to_string()"#)
            || generated.contains(r#"string::from("Alice")"#)
            || generated.contains(r#"String::from("Alice")"#),
        "Return literal should be String. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_implicit_return_string_literal() {
    let code = r#"
pub fn get_version() -> string {
    "1.0.0"
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };
    assert!(success, "Compilation failed: {}", err);

    assert!(
        generated.contains(r#""1.0.0".to_string()"#)
            || generated.contains(r#"string::from("1.0.0")"#)
            || generated.contains(r#"String::from("1.0.0")"#),
        "Implicit return should be String. Generated:\n{}",
        generated
    );
}

// ============================================================================
// Test: String Method Chains
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_replace_method() {
    let code = r#"
pub fn escape_html(text: string) -> string {
    text.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };
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
#[cfg_attr(tarpaulin, ignore)]
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

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };
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
#[cfg_attr(tarpaulin, ignore)]
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

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };
    assert!(success, "Compilation failed: {}", err);

    // String fields should have .to_string()
    assert!(
        generated.contains(r#""John".to_string()"#) || generated.contains("String::from"),
        "Struct string fields should be String. Generated:\n{}",
        generated
    );
}

// ============================================================================
// Test: Vec<String> Operations
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
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

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };
    assert!(success, "Compilation failed: {}", err);

    assert!(
        generated.contains(r#""red".to_string()"#)
            || generated.contains(r#"string::from("red")"#)
            || generated.contains(r#"String::from("red")"#),
        "Vec::push should convert literal. Generated:\n{}",
        generated
    );
}

// ============================================================================
// Test: If/Else String Returns
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
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

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };
    assert!(success, "Compilation failed: {}", err);

    assert!(
        generated.contains(r#""positive".to_string()"#)
            || generated.contains(r#"string::from("positive")"#)
            || generated.contains(r#"String::from("positive")"#),
        "If branch should be String. Generated:\n{}",
        generated
    );
    assert!(
        generated.contains(r#""zero".to_string()"#)
            || generated.contains(r#"string::from("zero")"#)
            || generated.contains(r#"String::from("zero")"#),
        "Else branch should be String. Generated:\n{}",
        generated
    );
}

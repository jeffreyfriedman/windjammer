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

//! TDD Test: .as_str() is forbidden in Windjammer source
//!
//! Windjammer automatically handles string conversions (String → &str).
//! Using .as_str() is Rust-specific leakage and must be rejected with
//! a helpful error message guiding the user toward idiomatic Windjammer.

#[path = "../common/test_utils.rs"]
mod test_utils;

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

    let (success, _generated, err) = test_utils::compile_via_cli(code);

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

    let (success, _generated, err) = test_utils::compile_via_cli(code);

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

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };

    if !success {
        println!("Generated code:\n{}", generated);
        println!("Rustc error:\n{}", err);
    }

    assert!(success, "Generated code should compile. Error: {}", err);
}

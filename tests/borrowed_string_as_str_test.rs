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

//! TDD Test: .as_str() is redundant in Windjammer source
//!
//! Windjammer automatically handles string conversions (String → &str).
//! `.as_str()` is redundant: it compiles (identity) and is reported as the W0012 lint.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_as_str_is_accepted() {
    // .as_str() is redundant — the compiler accepts it as identity and lints it
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

    assert!(success, "`.as_str()` is identity (W0012 lint), not an error. Got: {}", err);
}

#[test]
fn test_as_str_on_owned_string_accepted() {
    // Even on owned strings, .as_str() is identity — the compiler handles
    // String → &str conversion automatically
    let code = r#"
pub fn process(text: string) {
    let owned = text
    println!("{}", owned.as_str())
}

pub fn test_process() {
    process("Test")
}
"#;

    let (success, _generated, err) = test_utils::compile_via_cli(code);

    assert!(success, "`.as_str()` is identity (W0012 lint), not an error. Got: {}", err);
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

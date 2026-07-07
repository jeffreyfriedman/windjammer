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

//! Tests for forbidden @derive patterns.
//!
//! Windjammer auto-infers derivable traits from struct field types.
//! Explicit @derive(Copy), @derive(Serialize), @derive(Deserialize) are
//! FORBIDDEN — the compiler must reject them with a helpful error.
use windjammer::analyzer::Analyzer;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;

fn analyze(source: &str) -> Result<(), String> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Parse should succeed");
    let mut analyzer = Analyzer::new();
    analyzer.analyze_program(&program).map(|_| ())
}

#[test]
fn derive_copy_is_rejected() {
    let result = analyze("@derive(Copy)\nstruct Point {\n    x: f32,\n    y: f32,\n}\n");
    assert!(result.is_err(), "Should reject @derive(Copy)");
    let err = result.unwrap_err();
    assert!(
        err.contains("forbidden") && err.contains("Copy"),
        "Error should mention forbidden Copy: {err}"
    );
}

#[test]
fn derive_serialize_is_rejected() {
    let result = analyze(
        r#"
@derive(Serialize)
struct Config {
    name: string,
    value: i32,
}
"#,
    );
    assert!(result.is_err(), "Should reject @derive(Serialize)");
    let err = result.unwrap_err();
    assert!(
        err.contains("forbidden") && err.contains("Serialize"),
        "Error should mention forbidden Serialize: {err}"
    );
}

#[test]
fn derive_deserialize_is_rejected() {
    let result = analyze(
        r#"
@derive(Deserialize)
struct Payload {
    body: string,
}
"#,
    );
    assert!(result.is_err(), "Should reject @derive(Deserialize)");
    let err = result.unwrap_err();
    assert!(
        err.contains("forbidden") && err.contains("Deserialize"),
        "Error should mention forbidden Deserialize: {err}"
    );
}

#[test]
fn derive_copy_serialize_together_rejected() {
    let result = analyze(
        "@derive(Copy, Serialize)\nstruct Pair {\n    a: i32,\n    b: i32,\n}\n",
    );
    assert!(
        result.is_err(),
        "Should reject @derive with forbidden traits"
    );
}

#[test]
fn no_derive_decorator_is_fine() {
    let result = analyze(
        r#"
struct Point {
    x: f32,
    y: f32,
}
"#,
    );
    assert!(
        result.is_ok(),
        "Should allow structs without @derive\nError: {:?}",
        result.err()
    );
}

#[test]
fn auto_derive_without_decorator_produces_copy_for_all_copy_fields() {
    let result = analyze(
        r#"
struct Point {
    x: f32,
    y: f32,
}
"#,
    );
    assert!(
        result.is_ok(),
        "Auto-derive should work without decorators\nError: {:?}",
        result.err()
    );
}

#[test]
fn derive_custom_non_standard_trait_is_allowed() {
    let result = analyze(
        r#"
@derive(MyCustomTrait)
struct Widget {
    name: string,
}
"#,
    );
    assert!(
        result.is_ok(),
        "Should allow @derive for non-standard custom traits\nError: {:?}",
        result.err()
    );
}

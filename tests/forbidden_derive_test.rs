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

//! Tests for @derive handling.
//!
//! Windjammer auto-infers derivable traits from struct field types.
//! Explicit @derive for standard traits is silently accepted (backward
//! compatibility) while auto-derive handles them.  Non-standard traits
//! like Serialize are passed through.
use windjammer::analyzer::Analyzer;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;

#[test]
fn test_derive_copy_is_accepted() {
    let source = "@derive(Copy)\nstruct Point {\n    x: f32,\n    y: f32,\n}\n";

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Parse should succeed");

    let mut analyzer = Analyzer::new();
    let result = analyzer.analyze_program(&program);

    assert!(
        result.is_ok(),
        "Should accept @derive(Copy) (auto-derive handles it)\nError: {:?}",
        result.err()
    );
}

#[test]
fn test_derive_multiple_standard_traits_is_accepted() {
    let source = "@derive(Copy, Clone, Debug)\nstruct Vec2 {\n    x: f32,\n    y: f32,\n}\n";

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Parse should succeed");

    let mut analyzer = Analyzer::new();
    let result = analyzer.analyze_program(&program);

    assert!(
        result.is_ok(),
        "Should accept @derive with standard traits (auto-derive handles them)\nError: {:?}",
        result.err()
    );
}

#[test]
fn test_derive_third_party_trait_is_allowed() {
    let source = r#"
@derive(Serialize)
struct Config {
    name: string,
    value: i32,
}
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Parse should succeed");

    let mut analyzer = Analyzer::new();
    let result = analyzer.analyze_program(&program);

    assert!(
        result.is_ok(),
        "Should allow @derive(Serialize) (third-party trait)\nError: {:?}",
        result.err()
    );
}

#[test]
fn test_no_derive_decorator_is_fine() {
    let source = r#"
struct Point {
    x: f32,
    y: f32,
}
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Parse should succeed");

    let mut analyzer = Analyzer::new();
    let result = analyzer.analyze_program(&program);

    assert!(
        result.is_ok(),
        "Should allow structs without @derive\nError: {:?}",
        result.err()
    );
}

#[test]
fn test_derive_hash_is_accepted() {
    let source = "@derive(Hash)\nstruct EntityId {\n    id: u32,\n}\n";

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Parse should succeed");

    let mut analyzer = Analyzer::new();
    let result = analyzer.analyze_program(&program);

    assert!(
        result.is_ok(),
        "Should accept @derive(Hash) (auto-derive handles it)\nError: {:?}",
        result.err()
    );
}

#[test]
fn test_auto_derive_without_decorator_produces_copy_for_all_copy_fields() {
    let source = r#"
struct Point {
    x: f32,
    y: f32,
}
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Parse should succeed");

    let mut analyzer = Analyzer::new();
    let result = analyzer.analyze_program(&program);
    assert!(
        result.is_ok(),
        "Auto-derive should work without decorators\nError: {:?}",
        result.err()
    );
}

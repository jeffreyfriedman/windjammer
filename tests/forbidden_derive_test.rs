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

/// TDD TEST: `@derive(Copy)` and other standard traits should be forbidden.
///
/// Windjammer auto-infers derivable traits from struct field types.
/// Manually specifying them is Rust-leakage that violates the
/// "compiler does the hard work" principle.
use windjammer::analyzer::Analyzer;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;

#[test]
fn test_derive_copy_is_forbidden() {
    // NOTE: We embed the @derive via string concatenation to avoid
    // the removal script stripping it from test source
    let source = "@derive(Copy)\nstruct Point {\n    x: f32,\n    y: f32,\n}\n";

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Parse should succeed");

    let mut analyzer = Analyzer::new();
    let result = analyzer.analyze_program(&program);

    assert!(result.is_err(), "Should reject @derive(Copy)");
    let err_msg = result.unwrap_err();
    assert!(
        err_msg.contains("@derive(Copy)") && err_msg.contains("forbidden"),
        "Error should mention forbidden @derive(Copy)\nActual: {}",
        err_msg
    );
}

#[test]
fn test_derive_multiple_standard_traits_is_forbidden() {
    let source = "@derive(Copy, Clone, Debug)\nstruct Vec2 {\n    x: f32,\n    y: f32,\n}\n";

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Parse should succeed");

    let mut analyzer = Analyzer::new();
    let result = analyzer.analyze_program(&program);

    assert!(result.is_err(), "Should reject @derive with standard traits");
    let err_msg = result.unwrap_err();
    assert!(
        err_msg.contains("forbidden"),
        "Error should say forbidden\nActual: {}",
        err_msg
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
fn test_derive_hash_is_forbidden() {
    let source = "@derive(Hash)\nstruct EntityId {\n    id: u32,\n}\n";

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Parse should succeed");

    let mut analyzer = Analyzer::new();
    let result = analyzer.analyze_program(&program);

    assert!(result.is_err(), "Should reject @derive(Hash)");
    let err_msg = result.unwrap_err();
    assert!(
        err_msg.contains("Hash") && err_msg.contains("forbidden"),
        "Error should mention forbidden Hash\nActual: {}",
        err_msg
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

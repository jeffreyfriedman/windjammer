/// TDD TEST: `.as_str()` should be forbidden in Windjammer source
///
/// LANGUAGE DESIGN: Windjammer automatically handles string conversions.
/// Users shouldn't need Rust-specific `.as_str()` boilerplate.
///
/// This enforces cross-backend consistency (Go/JS don't have .as_str()).

use windjammer::analyzer::Analyzer;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;

#[test]
fn test_as_str_is_forbidden() {
    let source = r#"
enum BuildType {
    Warrior,
    Rogue,
}

impl BuildType {
    pub fn from_name(name: string) -> BuildType {
        match name.as_str() {
            "warrior" => BuildType::Warrior,
            _ => BuildType::Rogue,
        }
    }
}
"#;

    // Lex
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();

    // Parse
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Parse should succeed");

    // Analyze - should FAIL because of .as_str()
    let mut analyzer = Analyzer::new();
    let result = analyzer.analyze_program(&program);

    assert!(result.is_err(), "Should reject .as_str()");
    let err_msg = result.unwrap_err();
    assert!(
        err_msg.contains("`.as_str()` is forbidden"),
        "Error should explain that .as_str() is forbidden\nActual error: {}",
        err_msg
    );
    assert!(
        err_msg.contains("backend-agnostic"),
        "Error should explain backend-agnostic reasoning\nActual error: {}",
        err_msg
    );
}

#[test]
fn test_string_match_without_as_str_is_allowed() {
    let source = r#"
enum BuildType {
    Warrior,
    Rogue,
}

impl BuildType {
    pub fn from_name(name: string) -> BuildType {
        match name {
            "warrior" => BuildType::Warrior,
            _ => BuildType::Rogue,
        }
    }
}
"#;

    // Lex
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();

    // Parse
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Parse should succeed");

    // Analyze - should SUCCEED (no .as_str())
    let mut analyzer = Analyzer::new();
    let result = analyzer.analyze_program(&program);

    assert!(result.is_ok(), "Should allow match without .as_str()");
}

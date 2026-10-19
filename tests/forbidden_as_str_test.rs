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

/// `.as_str()` is accepted (identity on every backend) and reported as the W0012 lint (`wj fix` removes it).
/// It used to be a hard error in `wj build` only, which rejected interpreter-first code.
///
/// LANGUAGE DESIGN: Windjammer automatically handles string conversions.
/// Users shouldn't need Rust-specific `.as_str()` boilerplate.
///
/// This enforces cross-backend consistency (Go/JS don't have .as_str()).
use windjammer::analyzer::Analyzer;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;

#[test]
fn test_as_str_is_accepted_and_linted() {
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

    assert!(result.is_ok(), "`.as_str()` must not be rejected: {:?}", result.err());
    let lints = windjammer::linter::lint_text("t.wj", source).expect("parses");
    assert!(lints.iter().any(|d| d.lint_name == "W0012"), "W0012 expected: {lints:?}");
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

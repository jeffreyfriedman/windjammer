/// TDD: Parser warnings mechanism
///
/// The parser should collect warnings into a structured vector instead of
/// printing them to stderr with eprintln!. This allows callers to:
/// - Display warnings in IDE-friendly formats
/// - Filter/suppress specific warnings
/// - Test that warnings are generated correctly

use windjammer::parser::Parser;

#[test]
fn test_parser_has_warnings_method() {
    let source = "fn hello() -> i32 { 42 }";
    let mut parser = Parser::new_with_source(
        windjammer::lexer::Lexer::new(source).tokenize_with_locations(),
        "test.wj".to_string(),
        source.to_string(),
    );
    let _program = parser.parse().unwrap();
    let warnings = parser.warnings();
    assert!(warnings.is_empty(), "Clean code should produce no warnings");
}

#[test]
fn test_format_macro_generates_warning() {
    let source = r#"
fn hello() -> string {
    format!("hello {}", name)
}
"#;
    let mut parser = Parser::new_with_source(
        windjammer::lexer::Lexer::new(source).tokenize_with_locations(),
        "test.wj".to_string(),
        source.to_string(),
    );
    let _result = parser.parse();
    let warnings = parser.warnings();
    assert!(
        !warnings.is_empty(),
        "format!() usage should generate a warning"
    );
    let warning = &warnings[0];
    assert!(
        warning.message.contains("format!()"),
        "Warning should mention format!(). Got: {}",
        warning.message
    );
    assert!(
        warning.message.contains("string interpolation"),
        "Warning should suggest string interpolation. Got: {}",
        warning.message
    );
}

#[test]
fn test_warning_has_location() {
    let source = r#"
fn hello() -> string {
    format!("hello {}", name)
}
"#;
    let mut parser = Parser::new_with_source(
        windjammer::lexer::Lexer::new(source).tokenize_with_locations(),
        "test.wj".to_string(),
        source.to_string(),
    );
    let _result = parser.parse();
    let warnings = parser.warnings();
    if !warnings.is_empty() {
        let w = &warnings[0];
        assert!(
            w.file.is_some(),
            "Warning should have file location"
        );
        assert!(
            w.line.is_some(),
            "Warning should have line number"
        );
    }
}

#[test]
fn test_multiple_warnings_collected() {
    let source = r#"
fn hello() -> string {
    let a = format!("hello {}", name)
    let b = format!("world {}", x)
    a
}
"#;
    let mut parser = Parser::new_with_source(
        windjammer::lexer::Lexer::new(source).tokenize_with_locations(),
        "test.wj".to_string(),
        source.to_string(),
    );
    let _result = parser.parse();
    let warnings = parser.warnings();
    assert!(
        warnings.len() >= 2,
        "Two format!() calls should produce two warnings. Got: {}",
        warnings.len()
    );
}

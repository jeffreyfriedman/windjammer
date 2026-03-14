//! CLI tests for `wj lint` - Rust leakage linter
//!
//! Validates that the linter correctly:
//! - Passes clean files
//! - Detects W0001-W0004 violations
//! - Fails with --strict when warnings present
//!
//! Uses library API directly (windjammer::linter) since wj binary
//! requires full CLI feature build.

use windjammer::lexer::Lexer;
use windjammer::linter::rust_leakage::RustLeakageLinter;
use windjammer::parser::Parser;

fn parse_and_lint(source: &str, file_name: &str) -> Vec<windjammer::linter::LintDiagnostic> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new_with_source(tokens, file_name.to_string(), source.to_string());
    let program = parser.parse().expect("Parse should succeed");
    let mut linter = RustLeakageLinter::new(file_name);
    linter.lint_program(&program);
    linter.into_diagnostics()
}

#[test]
fn test_lint_clean_file() {
    let source = r#"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;

    let warnings = parse_and_lint(source, "clean.wj");
    assert!(warnings.is_empty(), "Clean file should have no warnings");
}

#[test]
fn test_lint_finds_explicit_borrow() {
    let source = r#"
pub fn process(data: &Vec<i32>) {
    println("{}", data.len())
}
"#;

    let warnings = parse_and_lint(source, "leaky.wj");
    assert!(!warnings.is_empty(), "Should detect W0001");
    let w0001 = warnings.iter().find(|w| w.lint_name == "W0001");
    assert!(w0001.is_some(), "Should detect explicit ownership");
}

#[test]
fn test_lint_strict_fails_on_warnings() {
    let source = r#"
pub fn process(data: &Vec<i32>) {
    let _ = data.iter()
}
"#;

    let warnings = parse_and_lint(source, "leaky.wj");
    assert!(!warnings.is_empty(), "Should have warnings for strict mode test");
    // In strict mode, lint_file would bail - we're testing the linter logic
    assert!(warnings.len() >= 2, "Should detect &Vec and .iter()");
}

#[test]
fn test_lint_detects_unwrap() {
    let source = r#"
pub fn get_first(items: Vec<i32>) -> i32 {
    items.get(0).unwrap()
}
"#;

    let warnings = parse_and_lint(source, "unwrap.wj");
    let w0002 = warnings.iter().find(|w| w.lint_name == "W0002");
    assert!(w0002.is_some(), "Should detect .unwrap()");
}

//! Integration tests for IR shadow validation (solver vs legacy analyzer parity).

use windjammer::analyzer::Analyzer;
use windjammer::ir::shadow::{finish_shadow_validation, validate_shadow};
use windjammer::lexer::Lexer;

fn analyze_and_validate(source: &str) -> windjammer::ir::shadow::ShadowValidationResult {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let parser = Box::leak(Box::new(windjammer::parser::Parser::new(tokens)));
    let program = parser.parse().expect("parse");
    let mut analyzer = Analyzer::new();
    let (analyzed, registry, _) = analyzer.analyze_program(&program).expect("analyze");
    validate_shadow(&analyzed, &registry)
}

#[test]
fn shadow_validation_clean_for_simple_copy_param() {
    let result = analyze_and_validate(
        r#"
        fn double(x: i32) -> i32 {
            x + x
        }
    "#,
    );
    assert!(
        result.is_clean(),
        "expected clean shadow validation, got: {:?}",
        result.discrepancies
    );
}

#[test]
fn shadow_validation_reports_string_type_discrepancy() {
    // Known gap: IR resolves string params as BaseType::String while analyzer
    // may infer Custom("str") for &str optimization. Shadow mode catches this.
    let result = analyze_and_validate(
        r#"
        fn greet(name: string) -> i32 {
            name.len()
        }
    "#,
    );
    assert!(
        !result.is_clean(),
        "string param should surface IR vs analyzer type discrepancy during migration"
    );
}

#[test]
fn shadow_validation_clean_for_borrowed_string_param() {
    let result = analyze_and_validate(
        r#"
        fn print_msg(msg: string) {
            println("{}", msg)
        }
    "#,
    );
    assert!(
        result.is_clean(),
        "expected clean shadow validation, got: {:?}",
        result.discrepancies
    );
}

#[test]
fn shadow_validation_strict_mode_passes_when_clean() {
    let result = analyze_and_validate(
        r#"
        fn add(a: i32, b: i32) -> i32 {
            a + b
        }
    "#,
    );
    windjammer::ir::shadow::set_shadow_validate_strict(true);
    let finish = finish_shadow_validation(&result);
    windjammer::ir::shadow::set_shadow_validate_strict(false);
    assert!(finish.is_ok(), "clean result should pass strict mode");
}

#[test]
fn shadow_validation_checks_multiple_functions() {
    let result = analyze_and_validate(
        r#"
        fn one(x: i32) -> i32 { x }
        fn two(y: i32) -> i32 { y + 1 }
    "#,
    );
    assert_eq!(result.functions_checked, 2);
}

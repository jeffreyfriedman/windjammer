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

//! FAILING REPRO — `"".to_string()` is forbidden Windjammer (Rust leakage).
//!
//! Empty string is already `string`. Prefer `""` (codegen → `String::new()`).
//! `"".to_string()` is a Rustism used as an ownership workaround and must lint.
//!
//! Language-only; no product/repo names.

use windjammer::lexer::Lexer;
use windjammer::linter::rust_leakage::RustLeakageLinter;
use windjammer::parser::Parser;

fn lint_source(source: &str) -> Vec<String> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let parser = Box::leak(Box::new(Parser::new(tokens)));
    let program = parser.parse().unwrap();
    let mut linter = RustLeakageLinter::new("test.wj");
    linter.lint_program(&program);
    linter
        .diagnostics()
        .iter()
        .map(|d| format!("{}: {}", d.lint_name, d.message))
        .collect()
}

#[test]
fn empty_string_to_string_is_forbidden() {
    let source = r#"
pub fn empty_owned() -> string {
    "".to_string()
}
"#;

    let diagnostics = lint_source(source);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| {
            d.contains("W0006")
                || d.contains("\".to_string()\"")
                || d.contains("empty string")
                || d.contains("\".\".to_string()")
                || d.to_lowercase().contains("forbidden")
                || (d.contains("to_string") && d.contains("empty"))
        })
        .collect();

    assert!(
        !hits.is_empty(),
        "Expected rust-leakage lint for `\"\".to_string()` (prefer `\"\"`). Got: {:?}",
        diagnostics
    );
}

#[test]
fn bare_empty_string_literal_is_allowed() {
    let source = r#"
pub fn empty_label() -> string {
    ""
}
"#;

    let diagnostics = lint_source(source);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.contains("W0006") || (d.contains("to_string") && d.contains("empty")))
        .collect();

    assert!(
        hits.is_empty(),
        "Bare `\"\"` must not trigger empty-to_string lint. Got: {:?}",
        hits
    );
}

#[test]
fn empty_string_literal_in_option_match_arm_must_be_owned() {
    // Companion codegen expectation: bare `""` opposite `Some(s) => s` must not be `&str`.
    // Until tip promotes match-arm `""` → String::new(), helpers like empty_string() are needed.
    // This test documents the forbidden workaround remains `"".to_string()`.
    let source = r#"
pub fn or_empty(opt: Option<string>) -> string {
    match opt {
        Some(s) => s,
        None => "".to_string(),
    }
}
"#;
    let diagnostics = lint_source(source);
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.contains("W0006") || (d.contains("to_string") && d.contains("empty")))
        .collect();
    assert!(
        !hits.is_empty(),
        "match-arm `\"\".to_string()` must still be forbidden; tip should accept bare `\"\"` as owned. Got: {:?}",
        diagnostics
    );
}


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

//! String-literal `.to_string()` is forbidden Windjammer (Rust leakage).
//!
//! String literals are already `string`. Prefer bare `"text"` / `""`
//! (codegen → `String::from("text")` / `String::new()`).
//! `"…".to_string()` is a Rustism and must lint for **any** literal, not only empty.

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

fn hits_literal_to_string(diagnostics: &[String]) -> Vec<&String> {
    diagnostics
        .iter()
        .filter(|d| {
            d.contains("W0006")
                || d.contains("literal") && d.contains("to_string")
                || d.to_lowercase().contains("string literal") && d.contains("to_string")
                || d.contains("\".to_string()\"")
                || (d.contains("to_string")
                    && (d.contains("forbidden")
                        || d.contains("Rust leakage")
                        || d.contains("prefer bare")))
        })
        .collect()
}

#[test]
fn empty_string_literal_to_string_is_forbidden() {
    let diagnostics = lint_source(
        r#"
pub fn empty_owned() -> string {
    "".to_string()
}
"#,
    );
    assert!(
        !hits_literal_to_string(&diagnostics).is_empty(),
        "Expected lint for `\"\".to_string()`. Got: {:?}",
        diagnostics
    );
}

#[test]
fn non_empty_string_literal_to_string_is_forbidden() {
    let diagnostics = lint_source(
        r#"
pub fn label() -> string {
    "Hello".to_string()
}
"#,
    );
    assert!(
        !hits_literal_to_string(&diagnostics).is_empty(),
        "Expected lint for `\"Hello\".to_string()` (prefer bare `\"Hello\"`). Got: {:?}",
        diagnostics
    );
}

#[test]
fn string_literal_to_string_in_call_arg_is_forbidden() {
    let diagnostics = lint_source(
        r#"
pub fn take(s: string) -> string { s }

pub fn call() -> string {
    take("role".to_string())
}
"#,
    );
    assert!(
        !hits_literal_to_string(&diagnostics).is_empty(),
        "Expected lint for call-arg `\"role\".to_string()`. Got: {:?}",
        diagnostics
    );
}

#[test]
fn string_literal_to_string_in_concat_lhs_is_forbidden() {
    let diagnostics = lint_source(
        r#"
pub fn needle(key: string) -> string {
    "\"".to_string() + key + "\""
}
"#,
    );
    assert!(
        !hits_literal_to_string(&diagnostics).is_empty(),
        "Expected lint for concat LHS `\"\\\"\".to_string()`. Got: {:?}",
        diagnostics
    );
}

#[test]
fn bare_string_literals_are_allowed() {
    let diagnostics = lint_source(
        r#"
pub fn labels() -> string {
    let a = ""
    let b = "Hello"
    let c = "role"
    "${a}${b}${c}"
}
"#,
    );
    assert!(
        hits_literal_to_string(&diagnostics).is_empty(),
        "Bare string literals must not trigger literal-to_string lint. Got: {:?}",
        diagnostics
    );
}

#[test]
fn identifier_to_string_is_not_this_lint() {
    // Non-literal `.to_string()` may be a separate concern; this lint is literal-only.
    let diagnostics = lint_source(
        r#"
pub fn from_int(n: int) -> string {
    n.to_string()
}
"#,
    );
    let literal_hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| hits_literal_to_string(&[(*d).clone()]).is_empty() == false)
        .filter(|d| d.contains("Hello") || d.contains("\"\"") || d.contains("literal"))
        .collect();
    assert!(
        literal_hits.is_empty(),
        "int.to_string() must not be flagged as string-literal leakage. Got: {:?}",
        diagnostics
    );
}

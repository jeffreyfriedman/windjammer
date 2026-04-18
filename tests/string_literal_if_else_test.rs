//! TDD: if/else branches must unify String vs string literals (E0308 in Rust).
//!
//! When one branch is owned `String` (e.g. `.clone()`) and the other is a literal,
//! the literal arm must become `String::from(...)` / `.to_string()`.

use windjammer::analyzer::Analyzer;
use windjammer::codegen::rust::CodeGenerator;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;
use windjammer::CompilationTarget;

fn compile_to_rust(source: &str) -> String {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let parser = Box::leak(Box::new(Parser::new(tokens)));
    let program = parser.parse().unwrap();
    let mut analyzer = Analyzer::new();
    let (analyzed_fns, registry, _) = analyzer.analyze_program(&program).unwrap();
    let mut codegen = CodeGenerator::new_for_module(registry, CompilationTarget::Rust);
    codegen.generate_program(&program, &analyzed_fns)
}

#[test]
fn test_if_else_string_clone_vs_literal_tail_return() {
    let source = r#"
pub fn pick(parts: Vec<string>, cond: bool) -> string {
    if cond {
        parts[0].clone()
    } else {
        "0"
    }
}
"#;
    let rust = compile_to_rust(source);
    let ok_from = rust.contains("String::from(\"0\")");
    let ok_to_string = rust.contains("\"0\".to_string()");
    assert!(
        ok_from || ok_to_string,
        "else branch literal must be owned String to match .clone() arm. Got:\n{}",
        rust
    );
}

#[test]
fn test_if_else_both_literal_returns_str_no_owned_coercion() {
    // -> str maps to Rust `&str`; both arms should stay `&'static str`, no `.to_string()`.
    let source = r#"
pub fn two_literals(cond: bool) -> str {
    if cond {
        "hello"
    } else {
        "world"
    }
}
"#;
    let rust = compile_to_rust(source);
    assert!(
        !rust.contains("\"hello\".to_string()") && !rust.contains("\"world\".to_string()"),
        "both branches are literals returning &str; must not force .to_string(). Got:\n{}",
        rust
    );
    assert!(
        rust.contains("\"hello\"") && rust.contains("\"world\""),
        "expected both string literals preserved. Got:\n{}",
        rust
    );
}

#[test]
fn test_if_else_clone_vs_literal_not_last_statement_still_unifies() {
    let source = r#"
pub fn pick(parts: Vec<string>, cond: bool) -> string {
    let _dummy = 1
    if cond {
        parts[0].clone()
    } else {
        "0"
    }
    "done"
}
"#;
    let rust = compile_to_rust(source);
    assert!(
        rust.contains("\"0\".to_string()") || rust.contains("String::from(\"0\")"),
        "when if/else is not the last stmt, else literal must still be String if then is String. Got:\n{}",
        rust
    );
}

#[test]
fn test_if_else_call_returning_string_vs_literal_tail() {
    let source = r#"
fn make_s() -> string {
    "hi"
}

pub fn pick(cond: bool) -> string {
    if cond {
        make_s()
    } else {
        "0"
    }
}
"#;
    let rust = compile_to_rust(source);
    assert!(
        rust.contains("\"0\".to_string()") || rust.contains("String::from(\"0\")"),
        "else literal must be String when other arm is a call returning String. Got:\n{}",
        rust
    );
}

#[test]
fn test_if_else_clone_vs_literal_untyped_let_then_return() {
    let source = r#"
pub fn pick(parts: Vec<string>, cond: bool) -> string {
    let x = if cond {
        parts[0].clone()
    } else {
        "0"
    }
    x
}
"#;
    let rust = compile_to_rust(source);
    assert!(
        rust.contains("\"0\".to_string()") || rust.contains("String::from(\"0\")"),
        "untyped let RHS if/else must coerce literal when other arm is owned String. Got:\n{}",
        rust
    );
}

#[test]
fn test_if_else_clone_vs_literal_in_non_string_function() {
    let source = r#"
fn parse_int(s: string) -> i32 {
    let parts = split(s, ".")
    let int_part = if parts.len() > 0 { parts[0].clone() } else { "0" }
    0
}

fn split(s: string, delim: string) -> Vec<string> {
    Vec::new()
}
"#;
    let rust = compile_to_rust(source);
    assert!(
        rust.contains("\"0\".to_string()") || rust.contains("String::from(\"0\")"),
        "let binding if/else in non-string function must coerce literal when other arm is String. Got:\n{}",
        rust
    );
}

#[test]
fn test_if_else_string_var_vs_literal_in_typed_let() {
    let source = r#"
pub fn f(cond: bool, s: string) -> string {
    let x: string = if cond {
        s
    } else {
        "default"
    }
    x
}
"#;
    let rust = compile_to_rust(source);
    let ok = rust.contains("\"default\".to_string()") || rust.contains("String::from(\"default\")");
    assert!(
        ok,
        "else literal must be owned when other arm is String (typed let). Got:\n{}",
        rust
    );
}


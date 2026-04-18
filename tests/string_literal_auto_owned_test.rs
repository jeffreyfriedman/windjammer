//! TDD: String literals in String-typed contexts must generate `"".to_string()` in Rust, not bare `&str` literals.
//!
//! After removing `.to_string()` from Windjammer source, codegen must still emit owned `String` where Rust expects it.

use windjammer::*;

fn compile_and_get_rust(source: &str) -> String {
    let mut lexer = lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse");

    let mut int_inference = type_inference::IntInference::new();
    int_inference.infer_program(&program);
    assert!(
        int_inference.errors.is_empty(),
        "Int inference errors: {:?}",
        int_inference.errors
    );

    let mut float_inference = type_inference::FloatInference::new();
    float_inference.infer_program(&program);
    assert!(
        float_inference.errors.is_empty(),
        "Float inference errors: {:?}",
        float_inference.errors
    );

    let mut analyzer = analyzer::Analyzer::new();
    let (analyzed, signatures, _trait_methods) = analyzer
        .analyze_program(&program)
        .expect("Failed to analyze");

    let mut generator = codegen::CodeGenerator::new(signatures, CompilationTarget::Rust);
    generator.set_int_inference(int_inference);
    generator.set_float_inference(float_inference);
    generator.generate_program(&program, &analyzed)
}

#[test]
fn test_empty_string_in_match_arm() {
    let source = r#"
pub fn pick_name(m: Option<string>) -> string {
    match m {
        Some(name) => name,
        None => ""
    }
}
"#;
    let rust = compile_and_get_rust(source);
    assert!(
        rust.contains("None => \"\".to_string()"),
        "None arm with empty literal should emit .to_string() for String. Got:\n{}",
        rust
    );
}

#[test]
fn test_empty_string_in_if_else() {
    let source = r#"
pub fn branch(use_default: bool, s: string) -> string {
    if use_default {
        ""
    } else {
        s
    }
}
"#;
    let rust = compile_and_get_rust(source);
    assert!(
        rust.contains("\"\".to_string()"),
        "if branch returning empty string should emit .to_string(). Got:\n{}",
        rust
    );
}

#[test]
fn test_string_literal_return() {
    let source = r#"
pub fn empty_str() -> string {
    return ""
}
"#;
    let rust = compile_and_get_rust(source);
    // Compiler may elide `return` and emit implicit tail expression
    let ok_tail = rust.contains("\"\".to_string()") && rust.contains("empty_str");
    let ok_explicit = rust.contains("return \"\".to_string()");
    assert!(
        ok_tail || ok_explicit,
        "Function returning string literal should emit .to_string() (tail or return). Got:\n{}",
        rust
    );
}

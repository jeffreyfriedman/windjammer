//! TDD tests for automatic borrowing in method calls.
//!
//! Bug: Windjammer .wj files must write `push_str(&x)` and
//!      `extend_from_slice(&bytes)` which leaks Rust's borrow syntax.
//! Root Cause: Codegen doesn't auto-borrow for methods that take &T/&[T].
//! Fix: Auto-borrow list for common methods that take references.

use windjammer::analyzer::Analyzer;
use windjammer::codegen::rust::CodeGenerator;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;
use windjammer::CompilationTarget;

fn parse_and_generate(code: &str) -> String {
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs, _analyzed_trait_methods) =
        analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    generator.generate_program(&program, &analyzed_functions)
}

#[test]
fn test_push_str_auto_borrows_variable() {
    let source = r#"
    fn build_html() -> String {
        let mut html = String::new()
        let title = "Hello"
        html.push_str(title)
        html
    }
    "#;
    let output = parse_and_generate(source);
    assert!(
        output.contains("push_str(&title)") || output.contains("push_str(title)"),
        "push_str should auto-borrow or pass through. Got:\n{}",
        output
    );
}

#[test]
fn test_push_str_auto_borrows_field_access() {
    let source = r#"
    struct Page {
        title: String,
    }

    impl Page {
        fn render(self) -> String {
            let mut html = String::new()
            html.push_str(self.title)
            html
        }
    }
    "#;
    let output = parse_and_generate(source);
    assert!(
        output.contains("push_str(&self.title)"),
        "push_str should auto-borrow field access WITHOUT clone (borrow makes clone redundant). Got:\n{}",
        output
    );
}

#[test]
fn test_extend_from_slice_auto_borrows() {
    let source = r#"
    fn serialize() -> Vec<u8> {
        let mut bytes = Vec::new()
        let data = Vec::new()
        bytes.extend_from_slice(data)
        bytes
    }
    "#;
    let output = parse_and_generate(source);
    assert!(
        output.contains("extend_from_slice(&data)"),
        "extend_from_slice should auto-borrow. Got:\n{}",
        output
    );
}

#[test]
fn test_push_str_literal_no_double_borrow() {
    let source = r#"
    fn build() -> String {
        let mut s = String::new()
        s.push_str("hello")
        s
    }
    "#;
    let output = parse_and_generate(source);
    assert!(
        !output.contains("push_str(&&"),
        "String literal should NOT get double-borrowed. Got:\n{}",
        output
    );
}

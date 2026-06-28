#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

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
fn subprocess_spawn_borrows_vec_args() {
    let source = r#"
    use std::subprocess

    fn test_echo() {
        let args = vec!["hello".to_string()]
        subprocess::spawn("echo", args)
    }
    "#;

    let output = parse_and_generate(source);
    eprintln!("Generated Rust:\n{output}");
    assert!(
        output.contains("subprocess::spawn(\"echo\", &args)"),
        "expected &args for runtime subprocess::spawn, got:\n{output}"
    );
}

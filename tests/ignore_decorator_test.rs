// TDD: Test @ignore decorator generates #[ignore]

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
fn test_ignore_decorator() {
    let source = r#"
@test
@ignore
fn expensive_test() {
    // This test is skipped
}
"#;

    let rust_code = parse_and_generate(source);

    println!("Generated Rust:\n{}", rust_code);

    // Should have both #[test] and #[ignore]
    assert!(rust_code.contains("#[test]"), "Missing #[test]");
    assert!(rust_code.contains("#[ignore]"), "Missing #[ignore]");
}

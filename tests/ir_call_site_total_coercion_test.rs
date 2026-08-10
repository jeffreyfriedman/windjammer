//! Gates: IR call-site coercion is total; missing boundary signatures fail closed.

use windjammer::analyzer::Analyzer;
use windjammer::codegen::rust::CodeGenerator;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;
use windjammer::CompilationTarget;

fn parse_program(source: &str) -> windjammer::parser::Program<'static> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let parser = Box::leak(Box::new(Parser::new(tokens)));
    parser.parse().expect("parse")
}

#[test]
fn generate_program_fails_closed_on_missing_boundary_signature() {
    let program = parse_program(
        r#"
fn main() {
    unknown_crate::missing_api("x")
}
"#,
    );
    let mut analyzer = Analyzer::new();
    let (analyzed, registry, _) = analyzer.analyze_program(&program).expect("analyze");
    assert!(!analyzed.is_empty(), "expected analyzed main");
    let mut gen = CodeGenerator::new(registry, CompilationTarget::Rust);
    assert!(
        gen.ir_cutover_call_sites_enabled(),
        "production cutover must enable call_sites"
    );
    let output = gen.generate_program(&program, &analyzed);
    eprintln!("BOUNDARY OUTPUT:\n{output}");
    assert!(
        output.contains("compile_error!") && output.contains("missing boundary signature"),
        "expected hard error for missing boundary signature, got:\n{output}"
    );
}

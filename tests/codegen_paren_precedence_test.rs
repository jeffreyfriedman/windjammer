//! TDD: Same-precedence `*`/`/` on the RHS of `/` must keep parentheses in Rust output.
//!
//! Without parens, `x / (2.0 * y)` becomes `x / 2.0 * y` which parses as `(x / 2.0) * y`.

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
fn test_div_rhs_mul_same_precedence_keeps_parens() {
    let source = r#"
pub fn scaled_div(x: f32, y: f32) -> f32 {
    let result = x / (2.0 * y)
    result
}
"#;
    let rust = compile_to_rust(source);
    assert!(
        rust.contains("(2.0_f32 * y)"),
        "RHS of `/` must stay grouped so Rust does not parse `(x / 2.0) * y`. Generated:\n{}",
        rust
    );
    assert!(
        !rust.contains("x / 2.0_f32 * y") && !rust.contains("x / 2.0 * y"),
        "must not emit flat division-then-multiply without grouping. Generated:\n{}",
        rust
    );
}

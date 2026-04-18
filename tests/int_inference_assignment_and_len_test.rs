//! TDD: E0308-style integer literal suffixes from assignment targets and `.len()` comparisons.
//!
//! - `self.field = 0` where `field: i64` must emit `0_i64`, not `0_i32`.
//! - `vec.len() > 0` must emit `0_usize` for the literal.

use windjammer::*;

fn compile_with_int_inference(source: &str) -> String {
    let mut lexer = lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse().expect("parse");

    let mut int_inference = type_inference::IntInference::new();
    int_inference.infer_program(&program);
    assert!(
        int_inference.errors.is_empty(),
        "int inference errors: {:?}",
        int_inference.errors
    );

    let mut analyzer = analyzer::Analyzer::new();
    let (analyzed, _, _) = analyzer.analyze_program(&program).expect("analyze");

    let registry = analyzer::SignatureRegistry::new();
    let mut generator = codegen::CodeGenerator::new(registry, CompilationTarget::Rust);
    generator.set_int_inference(int_inference);
    generator.generate_program(&program, &analyzed)
}

#[test]
fn test_assign_i64_field_literal_suffix() {
    let source = r#"
pub struct Pipeline {
    pub total: i64,
}

impl Pipeline {
    pub fn zero(mut self) {
        self.total = 0
    }
}
"#;
    let rust = compile_with_int_inference(source);
    assert!(
        rust.contains("0_i64") || rust.contains("0i64"),
        "expected 0_i64 for assignment to i64 field; got:\n{}",
        rust
    );
}

#[test]
fn test_len_comparison_literal_usize_suffix() {
    let source = r#"
pub fn non_empty(ids: Vec<string>) -> bool {
    ids.len() > 0
}
"#;
    let rust = compile_with_int_inference(source);
    assert!(
        rust.contains("0_usize") || rust.contains("0usize"),
        "expected 0_usize when comparing to .len(); got:\n{}",
        rust
    );
}

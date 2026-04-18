//! E0308: `expected VoxelScene, found &VoxelScene` when fluent methods use `let mut result = self`
//! and the analyzer inferred `&self` for an explicit consuming `self` parameter.
//!
//! Rust receiver must be `mut self` so `result` is owned and matches the declared return type.

use windjammer::*;

fn compile_and_get_rust(source: &str) -> String {
    let mut lexer = lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse");

    let mut analyzer = analyzer::Analyzer::new();
    let (analyzed, _signatures, _trait_methods) = analyzer
        .analyze_program(&program)
        .expect("Failed to analyze");

    let registry = analyzer::SignatureRegistry::new();
    let mut generator = codegen::CodeGenerator::new(registry, CompilationTarget::Rust);
    generator.generate_program(&program, &analyzed)
}

#[test]
fn test_fluent_method_owned_self_emits_mut_self_when_returning_impl_struct() {
    let source = r#"
pub struct Widget {
    count: i32,
}

impl Widget {
    pub fn with_count(self, n: i32) -> Widget {
        let mut result = self
        result.count = n
        result
    }
}
"#;

    let rust = compile_and_get_rust(source);
    assert!(
        rust.contains("fn with_count(mut self"),
        "expected mut self for builder-style return; got:\n{rust}"
    );
    assert!(
        !rust.contains("fn with_count(&self"),
        "must not emit &self when returning owned struct via result binding; got:\n{rust}"
    );
}

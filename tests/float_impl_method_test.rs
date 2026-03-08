// TDD Test: Float literals in impl method returns should match return type
// Bug: Methods in impl blocks generate f64 instead of f32

use windjammer::*;

#[test]
fn test_float_literal_in_impl_method_if_else() {
    let source = r#"
pub struct Achievement {
    pub requirement: u32,
    pub progress: u32,
}

impl Achievement {
    pub fn progress_percentage(self) -> f32 {
        if self.requirement == 0 {
            1.0
        } else {
            (self.progress as f32) / (self.requirement as f32)
        }
    }
}
"#;

    let mut lexer = lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse");

    let mut analyzer = analyzer::Analyzer::new();
    let (analyzed, _signatures, _trait_methods) = analyzer
        .analyze_program(&program)
        .expect("Failed to analyze");

    // TDD: Run float inference (like ModuleCompiler does)
    let mut float_inference = type_inference::FloatInference::new();
    float_inference.infer_program(&program);
    
    if !float_inference.errors.is_empty() {
        panic!("Float inference errors: {:?}", float_inference.errors);
    }

    let registry = analyzer::SignatureRegistry::new();
    let mut generator = codegen::CodeGenerator::new(registry, CompilationTarget::Rust);
    let rust_code = generator.generate_program(&program, &analyzed);

    // DEBUG: Print generated code
    eprintln!("Generated for impl method:\n{}", rust_code);

    // When return type is f32, literals in if-else branches should be f32, NOT f64
    assert!(
        !rust_code.contains("1.0_f64"),
        "Should not generate f64 when return type is f32.\nGenerated:\n{}",
        rust_code
    );
    
    // Should contain f32
    assert!(
        rust_code.contains("1.0_f32") || rust_code.contains("1.0f32"),
        "Float literals in impl method should match return type (f32).\nGenerated:\n{}",
        rust_code
    );
}

// TDD Test: Match arms must have matching float types
// Bug: `match opt { Some(v) => v, None => 999999.0 }` generates incompatible f32/f64 types

use windjammer::*;

#[test]
fn test_match_arms_must_match_float_type() {
    let source = r#"
use std::collections::HashMap

pub fn get_score_or_default(scores: HashMap<i32, f32>, key: i32) -> f32 {
    match scores.get(key) {
        Some(v) => *v,
        None => 999999.0,
    }
}
"#;

    let mut lexer = lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse");

    let mut float_inference = type_inference::FloatInference::new();
    float_inference.infer_program(&program);
    
    if !float_inference.errors.is_empty() {
        panic!("Float inference errors: {:?}", float_inference.errors);
    }

    let mut analyzer = analyzer::Analyzer::new();
    let (analyzed, _signatures, _trait_methods) = analyzer
        .analyze_program(&program)
        .expect("Failed to analyze");

    let registry = analyzer::SignatureRegistry::new();
    let mut generator = codegen::CodeGenerator::new(registry, CompilationTarget::Rust);
    generator.set_float_inference(float_inference);
    let rust_code = generator.generate_program(&program, &analyzed);

    eprintln!("Generated match:\n{}", rust_code);

    // Match arms must have same float type (f32 from Some arm)
    assert!(
        rust_code.contains("999999.0_f32") || rust_code.contains("999999.0f32"),
        "Match None arm should be f32 to match Some arm.\nGenerated:\n{}",
        rust_code
    );
    
    // Should NOT mix types
    assert!(
        !rust_code.contains("999999.0_f64"),
        "Match arms must have compatible types.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_binary_comparison_operands_must_match() {
    let source = r#"
pub fn is_ready(current_wait: f32) -> bool {
    current_wait > 0.0
}
"#;

    let mut lexer = lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse");

    let mut float_inference = type_inference::FloatInference::new();
    float_inference.infer_program(&program);
    
    if !float_inference.errors.is_empty() {
        panic!("Float inference errors: {:?}", float_inference.errors);
    }

    let mut analyzer = analyzer::Analyzer::new();
    let (analyzed, _signatures, _trait_methods) = analyzer
        .analyze_program(&program)
        .expect("Failed to analyze");

    let registry = analyzer::SignatureRegistry::new();
    let mut generator = codegen::CodeGenerator::new(registry, CompilationTarget::Rust);
    generator.set_float_inference(float_inference);
    let rust_code = generator.generate_program(&program, &analyzed);

    eprintln!("Generated comparison:\n{}", rust_code);

    // Comparison operands must match (f32 from param type)
    assert!(
        rust_code.contains("0.0_f32") || rust_code.contains("0.0f32"),
        "Comparison operand should be f32 to match left side.\nGenerated:\n{}",
        rust_code
    );
    
    // Should NOT compare f32 > f64
    assert!(
        !rust_code.contains("0.0_f64"),
        "Comparison operands must have compatible types.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
#[ignore] // TODO: Requires return type → variable assignment propagation for nested generics
fn test_tuple_element_type_inference() {
    let source = r#"
pub fn get_neighbors() -> Vec<(i32, i32, f32)> {
    let mut result = Vec::new()
    result.push((1, 2, 1.414))
    result
}
"#;

    let mut lexer = lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse");

    let mut float_inference = type_inference::FloatInference::new();
    float_inference.infer_program(&program);
    
    if !float_inference.errors.is_empty() {
        panic!("Float inference errors: {:?}", float_inference.errors);
    }

    let mut analyzer = analyzer::Analyzer::new();
    let (analyzed, _signatures, _trait_methods) = analyzer
        .analyze_program(&program)
        .expect("Failed to analyze");

    let registry = analyzer::SignatureRegistry::new();
    let mut generator = codegen::CodeGenerator::new(registry, CompilationTarget::Rust);
    generator.set_float_inference(float_inference);
    let rust_code = generator.generate_program(&program, &analyzed);

    eprintln!("Generated tuple:\n{}", rust_code);

    // Tuple element should match return type (f32)
    assert!(
        rust_code.contains("1.414_f32") || rust_code.contains("1.414f32"),
        "Tuple element should match tuple type (f32).\nGenerated:\n{}",
        rust_code
    );
}

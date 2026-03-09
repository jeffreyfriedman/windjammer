// TDD Test: Parameter types must propagate to identifiers in expressions
// Bug: `pub fn foo(x: f32) { x > 0.0 }` generates `0.0_f64` instead of `0.0_f32`

use windjammer::*;

#[test]
fn test_parameter_type_propagates_to_binary_comparison() {
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

    eprintln!("Generated:\n{}", rust_code);

    // When comparing f32 param to literal, literal should be f32
    assert!(
        rust_code.contains("0.0_f32") || rust_code.contains("0.0f32"),
        "Comparison literal should match parameter type (f32).\nGenerated:\n{}",
        rust_code
    );
    
    assert!(
        !rust_code.contains("0.0_f64"),
        "Should not generate f64 when comparing with f32.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_parameter_type_propagates_in_arithmetic() {
    let source = r#"
pub fn scale(value: f32, factor: f32) -> f32 {
    value * factor * 0.5
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

    eprintln!("Generated:\n{}", rust_code);

    // When multiplying f32 params by literal, literal should be f32
    assert!(
        rust_code.contains("0.5_f32") || rust_code.contains("0.5f32"),
        "Arithmetic literal should match parameter types (f32).\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_parameter_f64_propagates_correctly() {
    let source = r#"
pub fn compute(x: f64) -> f64 {
    x * 2.0
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

    eprintln!("Generated:\n{}", rust_code);

    // When param is f64, literals should be f64
    assert!(
        rust_code.contains("2.0_f64") || rust_code.contains("2.0f64"),
        "Arithmetic literal should match parameter type (f64).\nGenerated:\n{}",
        rust_code
    );
}

// TDD Test: Float literals in struct field initialization should match field types
// Bug: `AStarCell { cost: 1.0 }` generates `cost: 1.0_f64` when field is f32

use windjammer::*;

#[test]
fn test_float_literal_in_struct_field_initialization() {
    let source = r#"
pub struct AStarCell {
    pub walkable: bool,
    pub cost: f32,
}

pub fn create_cells() -> Vec<AStarCell> {
    let mut cells = Vec::new()
    cells.push(AStarCell { walkable: true, cost: 1.0 })
    cells
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

    // When struct field is f32, literal should be f32
    assert!(
        rust_code.contains("cost: 1.0_f32") || rust_code.contains("cost: 1.0f32"),
        "Struct field literal should match field type (f32).\nGenerated:\n{}",
        rust_code
    );
    
    // Should NOT contain f64
    assert!(
        !rust_code.contains("cost: 1.0_f64"),
        "Should not generate f64 when field is f32.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_float_literal_in_binary_op_with_f32() {
    let source = r#"
pub fn calculate_diagonal_cost(cost: f32) -> f32 {
    cost * 1.414
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

    eprintln!("Generated binary op:\n{}", rust_code);

    // When param and return are f32, binary op literals should be f32
    assert!(
        rust_code.contains("1.414_f32") || rust_code.contains("1.414f32"),
        "Binary op literal should match operand type (f32).\nGenerated:\n{}",
        rust_code
    );
    
    // Should NOT mix f32 * f64
    assert!(
        !rust_code.contains("1.414_f64"),
        "Should not generate f64 when operating on f32.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_hashmap_insert_float_value_type() {
    let source = r#"
use std::collections::HashMap

pub fn initialize_scores() -> HashMap<(i32, i32), f32> {
    let mut scores = HashMap::new()
    scores.insert((0, 0), 0.0)
    scores
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

    eprintln!("Generated HashMap insert:\n{}", rust_code);

    // When HashMap value type is f32, inserted literals should be f32
    assert!(
        rust_code.contains("0.0_f32") || rust_code.contains("0.0f32"),
        "HashMap insert value should match value type (f32).\nGenerated:\n{}",
        rust_code
    );
}

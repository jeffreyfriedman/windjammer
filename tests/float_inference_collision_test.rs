// TDD Test: Float inference must handle expressions without locations
// Bug: Expressions without unique locations all map to ExprId { line: 0, col: 0 }
// Solution: Sequential ID assignment during constraint collection

use windjammer::*;

#[test]
fn test_multiple_float_literals_without_locations_get_unique_ids() {
    // This test simulates the multi-file scenario where expressions
    // might not have unique line/col locations
    let source = r#"
pub fn func1() -> f32 {
    1.0
}

pub fn func2() -> f32 {
    2.0
}

pub fn func3() -> f64 {
    3.0
}
"#;

    let mut lexer = lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse");

    // Run float inference
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

    eprintln!("Generated code:\n{}", rust_code);

    // func1 and func2 should generate f32 (return type is f32)
    let func1_has_f32 = rust_code.contains("pub fn func1() -> f32") 
        && (rust_code.contains("1.0_f32") || rust_code.contains("1.0f32"));
    let func2_has_f32 = rust_code.contains("pub fn func2() -> f32") 
        && (rust_code.contains("2.0_f32") || rust_code.contains("2.0f32"));
    
    // func3 should generate f64 (return type is f64)
    let func3_has_f64 = rust_code.contains("pub fn func3() -> f64") 
        && (rust_code.contains("3.0_f64") || rust_code.contains("3.0f64"));

    assert!(func1_has_f32, "func1 should use f32 literals.\nGenerated:\n{}", rust_code);
    assert!(func2_has_f32, "func2 should use f32 literals.\nGenerated:\n{}", rust_code);
    assert!(func3_has_f64, "func3 should use f64 literals.\nGenerated:\n{}", rust_code);
    
    // Should NOT mix types
    assert!(!rust_code.contains("1.0_f64"), "func1 should not generate f64");
    assert!(!rust_code.contains("2.0_f64"), "func2 should not generate f64");
}

#[test]
fn test_if_else_branches_distinct_float_types() {
    // Test that if-else branches with different float types work correctly
    let source = r#"
pub fn get_value(flag: bool) -> f32 {
    if flag {
        1.5
    } else {
        2.5
    }
}

pub fn get_other(flag: bool) -> f64 {
    if flag {
        3.5
    } else {
        4.5
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

    eprintln!("Generated if-else code:\n{}", rust_code);

    // get_value should use f32 in both branches
    assert!(rust_code.contains("1.5_f32") || rust_code.contains("1.5f32"), 
        "get_value if branch should use f32.\nGenerated:\n{}", rust_code);
    assert!(rust_code.contains("2.5_f32") || rust_code.contains("2.5f32"), 
        "get_value else branch should use f32.\nGenerated:\n{}", rust_code);
    
    // get_other should use f64 in both branches
    assert!(rust_code.contains("3.5_f64") || rust_code.contains("3.5f64"), 
        "get_other if branch should use f64.\nGenerated:\n{}", rust_code);
    assert!(rust_code.contains("4.5_f64") || rust_code.contains("4.5f64"), 
        "get_other else branch should use f64.\nGenerated:\n{}", rust_code);
}

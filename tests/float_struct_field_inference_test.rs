// TDD Test: Float literals in struct construction should match field types
// Bug: cost: 1.0 generates cost: 1.0_f64 even when field is f32

use windjammer::*;

#[test]
fn test_float_literal_matches_f32_field() {
    let source = r#"
pub struct Cell {
    pub walkable: bool,
    pub cost: f32,
}

pub fn create_cell() -> Cell {
    Cell { walkable: true, cost: 1.0 }
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

    let registry = analyzer::SignatureRegistry::new();
    let mut generator = codegen::CodeGenerator::new(registry, CompilationTarget::Rust);
    let rust_code = generator.generate_program(&program, &analyzed);

    // When struct field is f32, literal 1.0 should generate 1.0_f32, not 1.0_f64
    assert!(
        rust_code.contains("cost: 1.0_f32") || rust_code.contains("cost: 1.0f32"),
        "Float literal should match struct field type (f32).\nGenerated:\n{}",
        rust_code
    );
    
    // Should NOT contain f64 suffix
    assert!(
        !rust_code.contains("1.0_f64"),
        "Should not generate f64 when field is f32.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_float_literal_in_arithmetic_with_f32_return() {
    let source = r#"
pub fn calculate() -> f32 {
    1.414 * 2.0
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

    let registry = analyzer::SignatureRegistry::new();
    let mut generator = codegen::CodeGenerator::new(registry, CompilationTarget::Rust);
    let rust_code = generator.generate_program(&program, &analyzed);

    // DEBUG: Print generated code
    eprintln!("Generated for f32 return:\n{}", rust_code);

    // When return type is f32, literals should be f32 (may be constant-folded)
    // Should contain _f32 suffix, not _f64
    assert!(
        rust_code.contains("_f32") || rust_code.contains("f32"),
        "Float expression should use f32 type.\nGenerated:\n{}",
        rust_code
    );
    
    // Should NOT contain f64
    assert!(
        !rust_code.contains("_f64") && !rust_code.contains("f64"),
        "Should not generate f64 when return type is f32.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_float_literal_in_if_else_with_f32_return() {
    let source = r#"
pub fn progress_percentage(requirement: u32) -> f32 {
    if requirement == 0 {
        1.0
    } else {
        0.5
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

    let registry = analyzer::SignatureRegistry::new();
    let mut generator = codegen::CodeGenerator::new(registry, CompilationTarget::Rust);
    let rust_code = generator.generate_program(&program, &analyzed);

    // DEBUG: Print generated code
    eprintln!("Generated for if-else f32 return:\n{}", rust_code);

    // When return type is f32, literals in if-else branches should be f32
    assert!(
        rust_code.contains("1.0_f32") || (rust_code.contains("1.0") && !rust_code.contains("1.0_f64")),
        "Float literals in if-else should match return type (f32).\nGenerated:\n{}",
        rust_code
    );
    
    // Should NOT contain f64 suffix
    assert!(
        !rust_code.contains("_f64"),
        "Should not generate f64 when return type is f32.\nGenerated:\n{}",
        rust_code
    );
}

// TDD Test: Float literal inference for local variables with type annotations
// Bug: `let x: f32 = 1.0` generates `1.0_f64`, causing E0308 errors

use windjammer::*;

#[test]
fn test_let_with_f32_annotation() {
    let source = r#"
pub fn foo() {
    let x: f32 = 1.0
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

    assert!(
        rust_code.contains("1.0_f32") || rust_code.contains("1.0f32"),
        "let x: f32 = 1.0 should generate 1.0_f32.\nGenerated:\n{}",
        rust_code
    );
    assert!(
        !rust_code.contains("1.0_f64"),
        "Should not generate f64 when annotation is f32.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_let_with_f64_annotation() {
    let source = r#"
pub fn foo() {
    let x: f64 = 2.5
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

    assert!(
        rust_code.contains("2.5_f64") || rust_code.contains("2.5f64"),
        "let x: f64 = 2.5 should generate 2.5_f64.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_const_with_f32_annotation() {
    let source = r#"
const PI: f32 = 3.14159

pub fn foo() -> f32 {
    PI
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

    assert!(
        rust_code.contains("3.14159_f32") || rust_code.contains("3.14159f32"),
        "const PI: f32 = 3.14159 should generate 3.14159_f32.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_static_with_f32_annotation() {
    let source = r#"
static GRAVITY: f32 = 9.8

pub fn foo() -> f32 {
    GRAVITY
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

    assert!(
        rust_code.contains("9.8_f32") || rust_code.contains("9.8f32"),
        "static GRAVITY: f32 = 9.8 should generate 9.8_f32.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_shadowing_different_float_types() {
    let source = r#"
pub fn foo() {
    let x: f64 = 1.0
    let x: f32 = 2.0
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

    // First literal (1.0) should be f64, second (2.0) should be f32
    assert!(
        rust_code.contains("1.0_f64") || rust_code.contains("1.0f64"),
        "First let x: f64 = 1.0 should generate 1.0_f64.\nGenerated:\n{}",
        rust_code
    );
    assert!(
        rust_code.contains("2.0_f32") || rust_code.contains("2.0f32"),
        "Second let x: f32 = 2.0 should generate 2.0_f32.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_unannotated_let_defaults_to_f64() {
    let source = r#"
pub fn foo() {
    let x = 1.0
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

    // Unannotated let x = 1.0 should default to f64
    assert!(
        rust_code.contains("1.0_f64") || rust_code.contains("1.0f64"),
        "let x = 1.0 (no annotation) should default to f64.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_let_f32_with_binary_op() {
    let source = r#"
pub fn foo() {
    let x: f32 = 1.0
    let y = x * 0.5
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

    // Both 1.0 and 0.5 should be f32 (0.5 from MustMatch with x * 0.5)
    assert!(
        rust_code.contains("1.0_f32") || rust_code.contains("1.0f32"),
        "let x: f32 = 1.0 should generate 1.0_f32.\nGenerated:\n{}",
        rust_code
    );
    assert!(
        rust_code.contains("0.5_f32") || rust_code.contains("0.5f32"),
        "x * 0.5 should propagate f32 to 0.5.\nGenerated:\n{}",
        rust_code
    );
}

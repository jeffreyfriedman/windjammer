/// TDD Test: Float literal codegen produces correct f32/f64 suffixes
///
/// Bug: Compiler generates f64 literals by default, causing type mismatches
/// when variables/params/fields expect f32 (e.g., Breach Protocol game engine).
///
/// Root cause: Windjammer AST stores float literals without type suffix,
/// Rust codegen emits bare 2.0 (defaults to f64 in Rust).

use windjammer::*;

fn compile_to_rust(source: &str) -> String {
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
    generator.generate_program(&program, &analyzed)
}

#[test]
fn test_f32_literal_in_variable_init() {
    let source = r#"
fn calculate() -> f32 {
    let x: f32 = 2.0
    let y: f32 = x + 1.0
    return y
}
"#;
    let rust_output = compile_to_rust(source);

    assert!(
        rust_output.contains("2.0_f32") || rust_output.contains("2_f32"),
        "let x: f32 = 2.0 should generate f32 literal. Got:\n{}",
        rust_output
    );
    assert!(
        rust_output.contains("1.0_f32") || rust_output.contains("1_f32"),
        "x + 1.0 should generate f32 literal. Got:\n{}",
        rust_output
    );
}

#[test]
fn test_f32_literal_in_binary_op() {
    let source = r#"
fn add_offset(pos: f32) -> f32 {
    pos + 10.0
}
"#;
    let rust_output = compile_to_rust(source);

    assert!(
        rust_output.contains("10.0_f32") || rust_output.contains("10_f32"),
        "pos + 10.0 should generate f32 literal. Got:\n{}",
        rust_output
    );
}

#[test]
fn test_f32_literal_in_function_call() {
    let source = r#"
fn scale(x: f32) -> f32 { x * 2.0 }
fn main() {
    let result = scale(1.5)
}
"#;
    let rust_output = compile_to_rust(source);

    assert!(
        rust_output.contains("2.0_f32") || rust_output.contains("2_f32"),
        "x * 2.0 should generate f32 literal. Got:\n{}",
        rust_output
    );
    assert!(
        rust_output.contains("1.5_f32") || rust_output.contains("1.5f32"),
        "scale(1.5) should generate f32 literal. Got:\n{}",
        rust_output
    );
}

#[test]
fn test_f64_literal_explicit() {
    let source = r#"
fn calculate() -> f64 {
    let x: f64 = 2.0
    return x + 1.0
}
"#;
    let rust_output = compile_to_rust(source);

    assert!(
        rust_output.contains("2.0_f64") || rust_output.contains("2_f64"),
        "let x: f64 = 2.0 should generate f64 literal. Got:\n{}",
        rust_output
    );
    assert!(
        rust_output.contains("1.0_f64") || rust_output.contains("1_f64"),
        "x + 1.0 in f64 context should generate f64 literal. Got:\n{}",
        rust_output
    );
}

#[test]
fn test_unconstrained_defaults_to_f32() {
    // Game engine standard: unconstrained float literals default to f32
    let source = r#"
fn test() {
    let x = 1.0
}
"#;
    let rust_output = compile_to_rust(source);

    assert!(
        rust_output.contains("_f32") || rust_output.contains("f32"),
        "Unconstrained 1.0 should default to f32 (game engine standard). Got:\n{}",
        rust_output
    );
}

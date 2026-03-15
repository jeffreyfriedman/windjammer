/// TDD: f32/f64 Codegen Fix for E0308 Errors
///
/// Bug: Windjammer generates f64 literals in f32 contexts, causing ~1,360 E0308
/// "expected f32, found f64" errors in windjammer-game-core.
///
/// Root cause: Numeric literals defaulting to f64, not propagating context type.
///
/// Tests reproduce the exact patterns that cause E0308 in game code.
/// Uses internal API (like float_inference_struct_fields_test) - no wj binary needed.

use windjammer::*;

fn compile_and_get_rust(source: &str) -> String {
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

/// 3.14 in f32 context should be f32
#[test]
fn test_f32_literal_in_f32_context() {
    let source = r#"
fn test() {
    let x: f32 = 3.14
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("3.14_f32") || output.contains("3.14f32"),
        "3.14 in let x: f32 = 3.14 should generate _f32, got:\n{}",
        output
    );
    assert!(
        !output.contains("3.14_f64"),
        "Should not generate f64 in f32 context, got:\n{}",
        output
    );
}

/// 3.14 in f64 context should be f64
#[test]
fn test_f64_literal_in_f64_context() {
    let source = r#"
fn test() {
    let x: f64 = 3.14
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("3.14_f64") || output.contains("3.14f64"),
        "3.14 in let x: f64 = 3.14 should generate _f64, got:\n{}",
        output
    );
}

/// f32 + literal should infer literal as f32 (mixed math)
#[test]
fn test_mixed_math_f32() {
    let source = r#"
fn test() {
    let x: f32 = 1.0
    let result = x + 2.5
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("2.5_f32") || output.contains("2.5f32"),
        "2.5 in x + 2.5 (where x is f32) should infer f32, got:\n{}",
        output
    );
    assert!(
        !output.contains("2.5_f64"),
        "Should not generate f64 when other operand is f32, got:\n{}",
        output
    );
}

/// Vec3::new(1.0, 2.0, 3.0) - constructor args should be f32 when Vec3 uses f32
#[test]
fn test_vec3_constructor_f32() {
    let source = r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x, y, z }
    }
}

fn test() {
    let v = Vec3::new(1.0, 2.0, 3.0)
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("1.0_f32") && output.contains("2.0_f32") && output.contains("3.0_f32"),
        "Vec3::new(1.0, 2.0, 3.0) with f32 params should generate _f32, got:\n{}",
        output
    );
    assert!(
        !output.contains("1.0_f64") && !output.contains("2.0_f64") && !output.contains("3.0_f64"),
        "Should not generate f64 for Vec3 constructor args, got:\n{}",
        output
    );
}

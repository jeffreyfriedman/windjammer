/// TDD: expected_float_type propagation for E0308 reduction
///
/// When FloatInference returns Unknown (e.g. cross-module, location mismatch),
/// the codegen falls back to expected_float_type from function/method parameter.
///
/// This is defense-in-depth: param_float_type() + expected_float_type ensures
/// foo(0.5) where fn foo(x: f32) generates 0.5_f32 even if inference fails.

use windjammer::*;

fn compile_and_assert(source: &str, assertions: impl Fn(&str)) {
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

    assertions(&rust_code);
}

/// Type::Float param should propagate f32 to literal (game/graphics convention)
#[test]
fn test_float_param_propagates_f32() {
    let source = r#"
fn process(value: float) { }
fn main() {
    process(0.5)
}
"#;
    compile_and_assert(source, |rust_code| {
        assert!(
            rust_code.contains("0.5_f32") || rust_code.contains("0.5f32"),
            "process(0.5) with float param should generate _f32 (game convention), got:\n{}",
            rust_code
        );
    });
}

/// Method with f32 param should propagate to literal
#[test]
fn test_method_f32_param_propagates() {
    let source = r#"
struct Thing { x: f32 }
impl Thing {
    fn scale(self, factor: f32) { }
}
fn main() {
    let t = Thing { x: 1.0 }
    t.scale(2.0)
}
"#;
    compile_and_assert(source, |rust_code| {
        assert!(
            rust_code.contains("2.0_f32") || rust_code.contains("2_f32"),
            "t.scale(2.0) should generate _f32 when param is f32, got:\n{}",
            rust_code
        );
    });
}

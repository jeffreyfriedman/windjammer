/// TDD: Test that constant folding doesn't break f32 inference
use windjammer::*;

fn compile_and_get_rust(source: &str) -> String {
    let mut lexer = lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse");

    let mut float_inference = type_inference::FloatInference::new();
    float_inference.infer_program(&program);

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
fn test_acos_times_constant_expression() {
    let source = r#"
pub fn angle_deg(dot: f32) -> f32 {
    let angle = dot.acos() * (180.0 / 3.14159)
    angle
}
"#;
    let output = compile_and_get_rust(source);
    println!("Generated:\n{}", output);
    
    assert!(
        !output.contains(".acos() as f64") && !output.contains("acos() as f64"),
        "must not cast acos() to f64; got:\n{}",
        output
    );
    
    assert!(
        !output.contains(" as f64"),
        "must not insert `as f64` in f32 arithmetic with constant folding; got:\n{}",
        output
    );
}

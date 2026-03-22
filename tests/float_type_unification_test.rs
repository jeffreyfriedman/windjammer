/// TDD: f32/f64 unification when locals are inferred from float casts (dogfooding: squad_tactics.wj).
///
/// Bug: `let survival_rate = (alive as f32) / (total as f32); survival_rate < 0.3` emitted `0.3_f64`.
/// Root cause: `infer_type_from_expression` had no `Cast` arm, so `var_types` never stored the local as
/// f32 and float comparison did not constrain the literal.
///
/// Fix: Infer `Type::Custom("f32"|"f64")` from `expr as f32` / `as f64` for `let` variable type tracking.
///
/// Uses the in-process codegen path (same as f32_f64_codegen_e0308_test) so `cargo test` always matches
/// the library under test — not a stale `target/release/wj` binary.

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

#[test]
fn test_cast_div_f32_local_comparison_literal_is_f32() {
    let source = r#"
pub fn check(alive: i32, total: i32) -> bool {
    let survival_rate = (alive as f32) / (total as f32)
    survival_rate < 0.3
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("0.3_f32"),
        "expected 0.3_f32 (squad_tactics pattern), got:\n{}",
        output
    );
    assert!(
        !output.contains("0.3_f64"),
        "must not emit 0.3_f64 against f32 survival_rate, got:\n{}",
        output
    );
}

#[test]
fn test_cast_f32_local_compare_to_literal_after_let() {
    let source = r#"
pub fn almost_one(n: i32) -> bool {
    let x = n as f32
    x < 1.0
}
"#;
    let output = compile_and_get_rust(source);
    assert!(output.contains("1.0_f32"), "expected 1.0_f32, got:\n{}", output);
    assert!(
        !output.contains("1.0_f64"),
        "must not use 1.0_f64, got:\n{}",
        output
    );
}

#[test]
fn test_cast_f64_local_still_emits_f64_literal() {
    let source = r#"
pub fn big(n: i32) -> bool {
    let x = n as f64
    x > 0.5
}
"#;
    let output = compile_and_get_rust(source);
    assert!(output.contains("0.5_f64"), "expected 0.5_f64, got:\n{}", output);
}

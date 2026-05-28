#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

//! TDD: Float literals on assignment RHS and match default arms infer f32 from context.
//! Regression: Rust E0308 (`expected f32, found f64`) from `_f64` suffixes.

use windjammer::*;

fn build_rust(src: &str, _lint: bool) -> Result<String, String> {
    let mut lexer = lexer::Lexer::new(src);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = parser::Parser::new(tokens);
    let program = parser
        .parse()
        .map_err(|e| format!("parse failed: {:?}", e))?;

    let mut float_inference = type_inference::FloatInference::new();
    float_inference.infer_program(&program);
    if !float_inference.errors.is_empty() {
        return Err(format!(
            "float inference errors: {:?}",
            float_inference.errors
        ));
    }

    let mut analyzer = analyzer::Analyzer::new();
    let (analyzed, _signatures, _trait_methods) = analyzer
        .analyze_program(&program)
        .map_err(|e| format!("analyze failed: {:?}", e))?;

    let registry = analyzer::SignatureRegistry::new();
    let mut generator = codegen::CodeGenerator::new(registry, CompilationTarget::Rust);
    generator.set_float_inference(float_inference);
    Ok(generator.generate_program(&program, &analyzed))
}

#[test]
fn test_float_field_assignment_infers_f32() {
    let src = r#"
struct Timer {
    elapsed: f32,
}

fn reset(mut t: Timer) {
    t.elapsed = 0.0
}
"#;

    let rust = build_rust(src, false).expect("Should compile");

    assert!(
        rust.contains("0.0_f32"),
        "Expected '0.0_f32' from field assignment. Generated:\n{}",
        rust
    );
    assert!(
        !rust.contains("0.0_f64"),
        "Should not default to f64 when field is f32.\n{}",
        rust
    );
}

#[test]
fn test_float_match_arm_unifies_with_some_type() {
    // HashMap and Windjammer Map<K,V> share the same float inference path for .get → Option<V>.
    let src = r#"
use std::collections::HashMap

fn get_score(scores: HashMap<String, f32>, key: String) -> f32 {
    match scores.get(key) {
        Some(v) => v,
        None => 0.0
    }
}
"#;

    let rust = build_rust(src, false).expect("Should compile");

    assert!(
        rust.contains("0.0_f32"),
        "Expected '0.0_f32' in None arm. Generated:\n{}",
        rust
    );
}

#[test]
fn test_float_match_large_default() {
    let src = r#"
use std::collections::HashMap

fn get_g_score(g_score: HashMap<(i32, i32), f32>, pos: (i32, i32)) -> f32 {
    match g_score.get(pos) {
        Some(v) => v,
        None => 999999.0
    }
}
"#;

    let rust = build_rust(src, false).expect("Should compile");

    assert!(
        rust.contains("999999.0_f32"),
        "Expected '999999.0_f32' in None arm. Generated:\n{}",
        rust
    );
}

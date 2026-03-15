//! E0308 Systematic Pattern Tests - Phase 11
//!
//! TDD tests for high-frequency E0308 patterns to verify compiler fixes.
//! Goal: Reduce E0308 from 188 to <160 through pattern-based fixes.

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

/// Pattern A: Tuple fields in struct literal - Keyframe { rotation: (0.0, 0.0, 0.0, 1.0) }
#[test]
fn test_pattern_a_tuple_fields_f32() {
    let source = r#"
pub struct Keyframe {
    pub rotation: (f32, f32, f32, f32),
}

pub fn make() -> Keyframe {
    Keyframe { rotation: (0.0, 0.0, 0.0, 1.0) }
}
"#;
    let rust = compile_and_get_rust(source);
    assert!(!rust.contains("_f64"), "Tuple fields should be f32. Got:\n{}", rust);
    assert!(rust.contains("0.0_f32") || rust.contains("0.0f32"), "Expected f32. Got:\n{}", rust);
}

/// Pattern B: Function argument f32 - process(0.5) where fn process(value: f32)
#[test]
fn test_pattern_b_function_arg_f32() {
    let source = r#"
pub fn process(value: f32) -> f32 { value }
pub fn call_it() -> f32 {
    process(0.5)
}
"#;
    let rust = compile_and_get_rust(source);
    assert!(rust.contains("0.5_f32") || rust.contains("0.5f32"), "Arg should be f32. Got:\n{}", rust);
}

/// Pattern C: Return literal f32 - fn get_value() -> f32 { 0.0 }
#[test]
fn test_pattern_c_return_f32_literal() {
    let source = r#"
pub fn get_value() -> f32 {
    0.0
}
"#;
    let rust = compile_and_get_rust(source);
    assert!(rust.contains("0.0_f32") || rust.contains("0.0f32"), "Return should be f32. Got:\n{}", rust);
}

/// Pattern D: Match arm None => 999999.0 with Some(v) => *v (f32)
#[test]
fn test_pattern_d_match_arm_none_literal() {
    let source = r#"
use std::collections::HashMap

pub fn get_score(scores: HashMap<i32, f32>, key: i32) -> f32 {
    match scores.get(key) {
        Some(v) => *v,
        None => 999999.0,
    }
}
"#;
    let rust = compile_and_get_rust(source);
    assert!(rust.contains("999999.0_f32") || rust.contains("999999.0f32"),
        "None arm should be f32 to match Some arm. Got:\n{}", rust);
}

/// Pattern E: Vec push with f32 - items.push(0.0) where Vec<f32>
#[test]
fn test_pattern_e_vec_push_f32() {
    let source = r#"
pub fn make_floats() -> Vec<f32> {
    let mut items: Vec<f32> = Vec::new()
    items.push(0.0)
    items.push(1.5)
    items
}
"#;
    let rust = compile_and_get_rust(source);
    assert!(rust.contains("0.0_f32") || rust.contains("0.0f32"), "push(0.0) should be f32. Got:\n{}", rust);
    assert!(rust.contains("1.5_f32") || rust.contains("1.5f32"), "push(1.5) should be f32. Got:\n{}", rust);
}

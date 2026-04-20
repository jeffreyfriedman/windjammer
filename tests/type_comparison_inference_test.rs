/// TDD: Mixed integer comparisons and arithmetic (Rust E0277 / E0308).
///
/// Windjammer allows safe implicit mixing in the analyzer; codegen must emit `as <T>` so rustc accepts it.
///
/// Uses the same in-process pipeline as other inference tests (no subprocess `wj` binary).
use windjammer::*;

fn compile_and_get_rust(source: &str) -> String {
    let mut lexer = lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse().expect("parse");

    let mut int_inference = type_inference::IntInference::new();
    int_inference.infer_program(&program);
    assert!(
        int_inference.errors.is_empty(),
        "Int inference errors: {:?}",
        int_inference.errors
    );

    let mut float_inference = type_inference::FloatInference::new();
    float_inference.infer_program(&program);
    assert!(
        float_inference.errors.is_empty(),
        "Float inference errors: {:?}",
        float_inference.errors
    );

    let mut analyzer = analyzer::Analyzer::new();
    let (analyzed, registry, trait_methods) = analyzer.analyze_program(&program).expect("analyze");

    let mut generator = codegen::CodeGenerator::new(registry, CompilationTarget::Rust);
    generator.set_analyzed_trait_methods(trait_methods);
    generator.set_int_inference(int_inference);
    generator.set_float_inference(float_inference);
    generator.generate_program(&program, &analyzed)
}

#[test]
fn test_u32_compared_to_i32_inserts_cast() {
    let source = r#"
pub fn cmp_u32_i32(a: u32, b: i32) -> bool {
    a > b
}
"#;

    let out = compile_and_get_rust(source);
    assert!(
        out.contains("b as u32") || out.contains("(b as u32)"),
        "expected `b as u32` (promote to u32), got:\n{}",
        out
    );
}

#[test]
fn test_i32_compared_to_literal_uses_i32_suffix() {
    let source = r#"
pub fn cmp_i32_lit(x: i32) -> bool {
    x > 0
}
"#;

    let out = compile_and_get_rust(source);
    assert!(
        out.contains("0_i32") || out.contains("0i32"),
        "expected int literal suffix for i32 context, got:\n{}",
        out
    );
}

#[test]
fn test_u32_plus_i32_inserts_cast() {
    let source = r#"
pub fn add_u32_i32(a: u32, b: i32) -> u32 {
    a + b
}
"#;

    let out = compile_and_get_rust(source);
    assert!(
        out.contains("b as u32") || out.contains("(b as u32)"),
        "expected `b as u32` for u32 + i32, got:\n{}",
        out
    );
}

#[test]
fn test_len_compared_to_i32_variable() {
    let source = r#"
pub fn f(items: Vec<i32>, i: i32) -> bool {
    items.len() > i
}
"#;

    let out = compile_and_get_rust(source);
    assert!(
        out.contains(".len() as i64") || out.contains(".len()) as i64"),
        "expected .len() cast to i64 for safe comparison with signed int, got:\n{}",
        out
    );
}

/// Regression: do not force `N_usize` on literals that must stay in u32/i32 context.
#[test]
fn test_u32_comparison_literal_not_usize_suffixed() {
    let source = r#"
pub fn f(x: u32) -> bool {
    x > 0
}
"#;

    let out = compile_and_get_rust(source);
    assert!(
        !out.contains("0_usize"),
        "expected no 0_usize in u32 compare (E0308 regression), got:\n{}",
        out
    );
    assert!(
        out.contains("0_u32") || out.contains("x > 0"),
        "expected 0_u32 or contextual bare 0 with u32 lhs, got:\n{}",
        out
    );
}

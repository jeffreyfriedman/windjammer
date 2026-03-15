//! TDD Test: If/Else branch float type unification
//!
//! Bug: `if cond { 1.0 } else { f32_var }` generates incompatible f32/f64 types (E0308).
//! Root cause: Float literals in if/else branches don't unify with known branch types.
//!
//! Goal: Unify float literal types across if/else branches.

use windjammer::*;

fn compile_and_assert(source: &str, expect_f32: &[&str], expect_no_f64: bool) -> String {
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

    for pattern in expect_f32 {
        assert!(
            rust_code.contains(&format!("{}_f32", pattern)) || rust_code.contains(&format!("{}f32", pattern)),
            "Expected '{}_f32' in output.\nGenerated:\n{}",
            pattern,
            rust_code
        );
    }

    if expect_no_f64 {
        assert!(
            !rust_code.contains("_f64"),
            "Should not contain '_f64'.\nGenerated:\n{}",
            rust_code
        );
    }

    rust_code
}

#[test]
fn test_if_literal_else_f32_var() {
    // if cond { 1.0 } else { f32_var } → 1.0_f32
    let source = r#"
pub fn choose(cond: bool, f32_var: f32) -> f32 {
    if cond {
        1.0
    } else {
        f32_var
    }
}
"#;
    let rust = compile_and_assert(source, &["1.0"], true);
    assert!(rust.contains("1.0_f32") || rust.contains("1.0f32"),
        "Literal in if branch should be f32 to match else branch.\n{}", rust);
}

#[test]
fn test_if_f32_var_else_literal() {
    // if cond { f32_var } else { 2.0 } → 2.0_f32
    let source = r#"
pub fn choose(cond: bool, f32_var: f32) -> f32 {
    if cond {
        f32_var
    } else {
        2.0
    }
}
"#;
    let rust = compile_and_assert(source, &["2.0"], true);
    assert!(rust.contains("2.0_f32") || rust.contains("2.0f32"),
        "Literal in else branch should be f32 to match if branch.\n{}", rust);
}

#[test]
fn test_if_f64_var_else_literal() {
    // if cond { f64_var } else { 2.0 } → 2.0_f64
    let source = r#"
pub fn choose(cond: bool, f64_var: f64) -> f64 {
    if cond {
        f64_var
    } else {
        2.0
    }
}
"#;
    let rust = compile_and_assert(source, &["2.0"], false);
    assert!(rust.contains("2.0_f64") || rust.contains("2.0f64"),
        "Literal in else branch should be f64 to match if branch.\n{}", rust);
}

#[test]
fn test_let_f32_annotation_if_both_literals() {
    // let x: f32 = if cond { 1.0 } else { 2.0 } → both _f32
    let source = r#"
pub fn choose(cond: bool) -> f32 {
    let x: f32 = if cond {
        1.0
    } else {
        2.0
    }
    x
}
"#;
    compile_and_assert(source, &["1.0", "2.0"], true);
}

#[test]
fn test_if_else_both_literals_return_f32() {
    // Function returns f32, both branches are literals
    let source = r#"
pub fn clamp_zero_one(x: f32) -> f32 {
    if x < 0.0 {
        0.0
    } else if x > 1.0 {
        1.0
    } else {
        x
    }
}
"#;
    compile_and_assert(source, &["0.0", "1.0"], true);
}

#[test]
fn test_safe_divide_expr_vs_literal() {
    // if branch: a/b (f32), else branch: 0.0
    let source = r#"
pub fn safe_divide(a: f32, b: f32) -> f32 {
    if b != 0.0 {
        a / b
    } else {
        0.0
    }
}
"#;
    compile_and_assert(source, &["0.0"], true);
}

#[test]
fn test_nested_if_else_literals() {
    // Nested if/else with literals in multiple branches
    let source = r#"
pub fn compute(x: f32) -> f32 {
    if x < 0.0 {
        if x < -10.0 {
            -10.0
        } else {
            x
        }
    } else {
        0.0
    }
}
"#;
    compile_and_assert(source, &["-10.0", "0.0"], true);
}

#[test]
fn test_normalize_len_division() {
    // from type_inference_if_else_arms_test.rs
    let source = r#"
pub fn normalize(len: f32) -> f32 {
    if len > 0.0 {
        1.0 / len
    } else {
        0.0
    }
}
"#;
    compile_and_assert(source, &["0.0", "1.0"], true);
}

#[test]
fn test_match_arms_mixed_float_types() {
    // Match arms with mixed float types - literal should match known type
    let source = r#"
pub fn get_or_default(has_value: bool, val: f32) -> f32 {
    match has_value {
        true => val,
        false => 42.0,
    }
}
"#;
    compile_and_assert(source, &["42.0"], true);
}

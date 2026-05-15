//! TDD: Match / branch float literals must unify with f32 context (Option, Result, enums).
//! Regression: literals defaulting to f64 in arms while other arms are f32 (Rust E0308).

use windjammer::*;

fn compile_and_check(source: &str, must_have_f32: &[&str], reject_f64_literals: bool) -> String {
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

    for lit in must_have_f32 {
        let ok = rust_code.contains(&format!("{}_f32", lit))
            || rust_code.contains(&format!("{}f32", lit));
        assert!(
            ok,
            "Expected float literal '{}' with f32 suffix in generated Rust.\n{}",
            lit, rust_code
        );
    }

    if reject_f64_literals {
        assert!(
            !rust_code.contains("999.0_f64")
                && !rust_code.contains("0.0_f64")
                && !rust_code.contains("0.1_f64")
                && !rust_code.contains("0.9_f64")
                && !rust_code.contains("1.0_f64"),
            "Unexpected f64 suffix on floats that should be f32.\n{}",
            rust_code
        );
    }

    rust_code
}

#[test]
fn test_option_f32_match_some_v_none_literal() {
    let source = r#"
struct Node {
    cost: Option<f32>,
}

fn get_cost(node: Node) -> f32 {
    match node.cost {
        Some(v) => v,
        None => 999.0,
    }
}
"#;
    compile_and_check(source, &["999.0"], true);
}

#[test]
fn test_result_f32_match_err_arm_literal() {
    let source = r#"
fn parse_or_default(result: Result<f32, String>) -> f32 {
    match result {
        Ok(v) => v,
        Err(_) => 0.0,
    }
}
"#;
    compile_and_check(source, &["0.0"], true);
}

#[test]
fn test_enum_match_all_literal_arms_f32() {
    let source = r#"
enum Value {
    Low,
    High,
}

fn to_float(v: Value) -> f32 {
    match v {
        Value::Low => 0.1,
        Value::High => 0.9,
    }
}
"#;
    let rust = compile_and_check(source, &["0.1", "0.9"], true);
    assert!(
        !rust.contains("0.1_f64") && !rust.contains("0.9_f64"),
        "Enum match literals should not be f64.\n{}",
        rust
    );
}

#[test]
fn test_if_else_clamp_f32_literals() {
    let source = r#"
fn clamp(x: f32) -> f32 {
    if x < 0.0 {
        0.0
    } else if x > 1.0 {
        1.0
    } else {
        x
    }
}
"#;
    compile_and_check(source, &["0.0", "1.0"], true);
}

/// Match in `let` with void function: no `-> f32` to drive arm literals; must use scrutinee (Option<f32>).
#[test]
fn test_let_match_option_f32_void_fn_no_annotation() {
    let source = r#"
fn consume(opt: Option<f32>) {
    let score = match opt {
        Some(v) => *v,
        None => 999.0,
    };
    let _ = score;
}
"#;
    compile_and_check(source, &["999.0"], true);
}

/// Explicit `let x: f32 = match` inside void function must constrain literals in arms.
#[test]
fn test_let_annotated_f32_match_void_fn() {
    let source = r#"
fn consume(opt: Option<f32>) {
    let score: f32 = match opt {
        Some(v) => *v,
        None => 999.0,
    };
    let _ = score;
}
"#;
    compile_and_check(source, &["999.0"], true);
}

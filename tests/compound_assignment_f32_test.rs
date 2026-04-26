/// TDD: Compound assignment (`+=`, `-=`, `*=`, `/=`) must not insert spurious `as f64` on f32 RHS
/// when both sides are f32 (E0277: cannot multiply-assign f32 by f64).
use std::process::Command;
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

fn run_rustc(rs_code: &str) -> (bool, String) {
    let test_dir = tempfile::tempdir().expect("failed to create temp dir");

    let rs_file = test_dir.path().join("test.rs");
    std::fs::write(&rs_file, rs_code).unwrap();

    let out_file = test_dir.path().join("output.rlib");
    let output = Command::new("rustc")
        .arg(&rs_file)
        .arg("--crate-type")
        .arg("lib")
        .arg("--edition")
        .arg("2021")
        .arg("-o")
        .arg(&out_file)
        .output()
        .expect("Failed to run rustc");

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (output.status.success(), stderr)
}

#[test]
fn test_compound_mul_assign_f32_fields_no_rhs_f64_cast() {
    let source = r#"
pub struct State {
    visibility: f32,
    crouch_modifier: f32,
}

pub fn apply(mut s: State) -> f32 {
    s.visibility *= s.crouch_modifier
    s.visibility
}
"#;

    let output = compile_and_get_rust(source);
    assert!(
        !output.contains("crouch_modifier as f64") && !output.contains(" as f64"),
        "f32 *= f32 must not cast RHS to f64; got:\n{}",
        output
    );

    let (ok, stderr) = run_rustc(&output);
    assert!(
        ok,
        "rustc failed:\nstderr: {}\n\nGenerated:\n{}",
        stderr, output
    );
}

#[test]
fn test_compound_add_sub_div_assign_f32_fields_no_rhs_f64_cast() {
    let source = r#"
pub struct T {
    value: f32,
    adjustment: f32,
    scale: f32,
}

pub fn run(mut t: T) -> f32 {
    t.value += t.adjustment
    t.value -= t.adjustment
    t.value *= t.scale
    t.value /= t.scale
    t.value
}
"#;

    let output = compile_and_get_rust(source);
    assert!(
        !output.contains(" as f64"),
        "f32 compound assigns must not use as f64 on f32 fields; got:\n{}",
        output
    );

    let (ok, stderr) = run_rustc(&output);
    assert!(
        ok,
        "rustc failed:\nstderr: {}\n\nGenerated:\n{}",
        stderr, output
    );
}

/// Folded `x = x * y` must match explicit `x *= y` (no spurious promotion on RHS).
#[test]
fn test_folded_assign_mul_f32_fields_no_rhs_f64_cast() {
    let source = r#"
pub struct State {
    visibility: f32,
    crouch_modifier: f32,
}

pub fn apply(mut s: State) -> f32 {
    s.visibility = s.visibility * s.crouch_modifier
    s.visibility
}
"#;

    let output = compile_and_get_rust(source);
    assert!(
        !output.contains(" as f64"),
        "f32 * f32 folded to compound must not insert as f64; got:\n{}",
        output
    );

    let (ok, stderr) = run_rustc(&output);
    assert!(
        ok,
        "rustc failed:\nstderr: {}\n\nGenerated:\n{}",
        stderr, output
    );
}

/// f32 field * local float: assignment RHS must stay in f32 (no `field as f64` from mixed promotion).
#[test]
fn test_f32_field_assign_mul_local_float_no_lhs_f64_cast() {
    let source = r#"
pub struct Demo { v: f32 }
pub fn tick(mut d: Demo) -> f32 {
    let pi = 3.14159265
    d.v = d.v * pi
    d.v
}
"#;

    let output = compile_and_get_rust(source);
    assert!(
        !output.contains(" as f64"),
        "f32 field assignment must not introduce f64 promotion; got:\n{}",
        output
    );

    let (ok, stderr) = run_rustc(&output);
    assert!(
        ok,
        "rustc failed:\nstderr: {}\n\nGenerated:\n{}",
        stderr, output
    );
}

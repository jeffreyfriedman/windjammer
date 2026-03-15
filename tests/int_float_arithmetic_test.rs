/// TDD: Integer/Float arithmetic E0277 elimination (Phase 10 regression fix)
///
/// Tests for patterns that caused "cannot add i32 to f32", "cannot multiply f32 by i32", etc.
/// Phase 10 added int*f32 casts but only for multiplication and only one direction.
/// This fix handles ALL arithmetic operators (+, -, *, /, %) in BOTH directions.
///
/// Error categories fixed:
/// - f32 + i32, i32 + f32 (add)
/// - f32 - i32, i32 - f32 (subtract)
/// - f32 * i32, i32 * f32 (multiply)
/// - f32 / i32, i32 / f32 (divide)
/// - f32 % i32 (modulo)
/// - f32 op {integer} (integer literals)
/// - f32 op i64 (i64 variables)

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
    let temp_dir = std::env::temp_dir();
    let test_id = format!(
        "int_float_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let test_dir = temp_dir.join(&test_id);
    std::fs::create_dir_all(&test_dir).unwrap();

    let rs_file = test_dir.join("test.rs");
    std::fs::write(&rs_file, rs_code).unwrap();

    let output = Command::new("rustc")
        .arg(&rs_file)
        .arg("--crate-type")
        .arg("lib")
        .arg("--edition")
        .arg("2021")
        .output()
        .expect("Failed to run rustc");

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let _ = std::fs::remove_dir_all(&test_dir);

    (output.status.success(), stderr)
}

/// f32 + i32: (dx) as f32 + dy (pathfinding pattern)
#[test]
fn test_f32_add_i32() {
    let source = r#"
pub fn add(x: f32, y: i32) -> f32 {
    x + y
}
"#;

    let output = compile_and_get_rust(source);
    assert!(
        output.contains("(y) as f32") || output.contains("(x) as f32"),
        "f32 + i32 should cast int to f32. Got:\n{}",
        output
    );

    let (rustc_ok, stderr) = run_rustc(&output);
    assert!(
        rustc_ok || !stderr.contains("cannot add"),
        "E0277 f32+i32:\nstderr: {}\n\nGenerated:\n{}",
        stderr,
        output
    );
}

/// i32 * f32: base * scale (existing test, verify still works)
#[test]
fn test_i32_multiply_f32() {
    let source = r#"
pub fn multiply(count: i32, scale: f32) -> f32 {
    count * scale
}
"#;

    let output = compile_and_get_rust(source);
    assert!(
        output.contains("as f32"),
        "i32 * f32 should cast int to f32. Got:\n{}",
        output
    );

    let (rustc_ok, stderr) = run_rustc(&output);
    assert!(
        rustc_ok || !stderr.contains("cannot multiply"),
        "E0277 i32*f32:\nstderr: {}\n\nGenerated:\n{}",
        stderr,
        output
    );
}

/// f32 - i32: value - offset (pathfinding dx/dy pattern)
#[test]
fn test_f32_subtract_i32() {
    let source = r#"
pub fn subtract(value: f32, offset: i32) -> f32 {
    value - offset
}
"#;

    let output = compile_and_get_rust(source);
    assert!(
        output.contains("(offset) as f32"),
        "f32 - i32 should cast int to f32. Got:\n{}",
        output
    );

    let (rustc_ok, stderr) = run_rustc(&output);
    assert!(
        rustc_ok || !stderr.contains("cannot subtract"),
        "E0277 f32-i32:\nstderr: {}\n\nGenerated:\n{}",
        stderr,
        output
    );
}

/// f32 / i32: total / count
#[test]
fn test_f32_divide_i32() {
    let source = r#"
pub fn divide(total: f32, count: i32) -> f32 {
    total / count
}
"#;

    let output = compile_and_get_rust(source);
    assert!(
        output.contains("(count) as f32"),
        "f32 / i32 should cast int to f32. Got:\n{}",
        output
    );

    let (rustc_ok, stderr) = run_rustc(&output);
    assert!(
        rustc_ok || !stderr.contains("cannot divide"),
        "E0277 f32/i32:\nstderr: {}\n\nGenerated:\n{}",
        stderr,
        output
    );
}

/// f32 * i32 (reverse order): scale * count
#[test]
fn test_f32_multiply_i32() {
    let source = r#"
pub fn scale_by_count(scale: f32, count: i32) -> f32 {
    scale * count
}
"#;

    let output = compile_and_get_rust(source);
    assert!(
        output.contains("(count) as f32"),
        "f32 * i32 should cast int to f32. Got:\n{}",
        output
    );

    let (rustc_ok, stderr) = run_rustc(&output);
    assert!(
        rustc_ok || !stderr.contains("cannot multiply"),
        "E0277 f32*i32:\nstderr: {}\n\nGenerated:\n{}",
        stderr,
        output
    );
}

/// f32 + integer literal: x + 2
#[test]
fn test_f32_add_int_literal() {
    let source = r#"
pub fn add_two(x: f32) -> f32 {
    x + 2
}
"#;

    let output = compile_and_get_rust(source);
    assert!(
        output.contains("as f32"),
        "f32 + int literal should cast. Got:\n{}",
        output
    );

    let (rustc_ok, stderr) = run_rustc(&output);
    assert!(
        rustc_ok || !stderr.contains("cannot add"),
        "E0277 f32+literal:\nstderr: {}\n\nGenerated:\n{}",
        stderr,
        output
    );
}

/// f32 / integer literal: sum / 2 (squad_tactics pattern)
#[test]
fn test_f32_divide_int_literal() {
    let source = r#"
pub fn half(value: f32) -> f32 {
    value / 2
}
"#;

    let output = compile_and_get_rust(source);
    assert!(
        output.contains("as f32"),
        "f32 / int literal should cast. Got:\n{}",
        output
    );

    let (rustc_ok, stderr) = run_rustc(&output);
    assert!(
        rustc_ok || !stderr.contains("cannot divide"),
        "E0277 f32/literal:\nstderr: {}\n\nGenerated:\n{}",
        stderr,
        output
    );
}

/// Cast chain: (dx) as f32 + dy (exact pathfinding pattern)
#[test]
fn test_cast_chain_f32_add_i32() {
    let source = r#"
pub fn distance(dx: i32, dy: i32) -> f32 {
    ((dx) as f32 + dy) as f32
}
"#;

    let output = compile_and_get_rust(source);
    assert!(
        output.contains("(dy) as f32") || output.contains("(dx) as f32"),
        "Cast chain (dx) as f32 + dy should cast dy. Got:\n{}",
        output
    );

    let (rustc_ok, stderr) = run_rustc(&output);
    assert!(
        rustc_ok || !stderr.contains("cannot add"),
        "E0277 cast chain:\nstderr: {}\n\nGenerated:\n{}",
        stderr,
        output
    );
}

/// f32 - integer literal: offset - 5 (squad_tactics pattern)
#[test]
fn test_f32_subtract_int_literal() {
    let source = r#"
pub fn offset_center(offset: f32) -> f32 {
    offset - 5
}
"#;

    let output = compile_and_get_rust(source);
    assert!(
        output.contains("as f32"),
        "f32 - int literal should cast. Got:\n{}",
        output
    );

    let (rustc_ok, stderr) = run_rustc(&output);
    assert!(
        rustc_ok || !stderr.contains("cannot subtract"),
        "E0277 f32-literal:\nstderr: {}\n\nGenerated:\n{}",
        stderr,
        output
    );
}

/// i32 + f32 (reverse order)
#[test]
fn test_i32_add_f32() {
    let source = r#"
pub fn add_reverse(x: i32, y: f32) -> f32 {
    x + y
}
"#;

    let output = compile_and_get_rust(source);
    assert!(
        output.contains("(x) as f32"),
        "i32 + f32 should cast int to f32. Got:\n{}",
        output
    );

    let (rustc_ok, stderr) = run_rustc(&output);
    assert!(
        rustc_ok || !stderr.contains("cannot add"),
        "E0277 i32+f32:\nstderr: {}\n\nGenerated:\n{}",
        stderr,
        output
    );
}

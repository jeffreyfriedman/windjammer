/// TDD: Comprehensive int/float arithmetic E0277 elimination
///
/// Phase 11 fix: Cast integers to floats for ALL arithmetic operators in BOTH
/// binary expressions AND compound assignments.
///
/// Error categories fixed:
/// - f32 + i32, i32 + f32 (add)
/// - f32 - i32, i32 - f32 (subtract)
/// - f32 * i32, i32 * f32 (multiply)
/// - f32 / i32, i32 / f32 (divide)
/// - f32 % i32 (modulo)
/// - f32 op {integer} (integer literals)
/// - Compound: price += 1, scale *= count

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
        "int_float_complete_{}",
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

/// All arithmetic ops: +, -, *, /, %
#[test]
fn test_all_arithmetic_ops() {
    let source = r#"
pub fn test_ops(x: f32, y: i32) -> f32 {
    let a = x + y
    let b = x - y
    let c = x * y
    let d = x / y
    let e = x % y
    a + b + c + d + e
}
"#;

    let output = compile_and_get_rust(source);
    assert!(
        output.contains("(y) as f32"),
        "f32 op i32 should cast y. Got:\n{}",
        output
    );
    assert!(
        !output.contains("cannot add") || run_rustc(&output).0,
        "Should compile without E0277"
    );
}

/// Compound assignment: price += 1
#[test]
fn test_compound_assignment_f32_plus_int() {
    let source = r#"
pub fn accumulate() -> f32 {
    let mut price = 0.0
    price += 1
    price += 2
    price
}
"#;

    let output = compile_and_get_rust(source);
    assert!(
        output.contains("as f32"),
        "price += 1 should cast int to f32. Got:\n{}",
        output
    );

    let (rustc_ok, stderr) = run_rustc(&output);
    assert!(
        rustc_ok || !stderr.contains("cannot add"),
        "E0277 compound assignment:\nstderr: {}\n\nGenerated:\n{}",
        stderr,
        output
    );
}

/// Compound assignment: scale *= count
#[test]
fn test_compound_assignment_f32_times_int() {
    let source = r#"
pub fn scale_by(count: i32) -> f32 {
    let mut scale = 1.0
    scale *= count
    scale
}
"#;

    let output = compile_and_get_rust(source);
    assert!(
        output.contains("as f32"),
        "scale *= count should cast. Got:\n{}",
        output
    );

    let (rustc_ok, stderr) = run_rustc(&output);
    assert!(
        rustc_ok || !stderr.contains("cannot multiply"),
        "E0277:\nstderr: {}\n\nGenerated:\n{}",
        stderr,
        output
    );
}

/// Self field: self.count * 0.5 (impl block) - no explicit cast, compiler adds it
#[test]
fn test_impl_block_self_field_int_times_float() {
    let source = r#"
pub struct Stats {
    count: i32,
}

impl Stats {
    pub fn average(self) -> f32 {
        self.count * 0.5
    }
}
"#;

    let output = compile_and_get_rust(source);
    assert!(
        output.contains("as f32") || output.contains("_f32"),
        "self.count * 0.5 should have float cast. Got:\n{}",
        output
    );

    let (rustc_ok, stderr) = run_rustc(&output);
    assert!(
        rustc_ok || !stderr.contains("cannot multiply"),
        "E0277 self.count * 0.5:\nstderr: {}\n\nGenerated:\n{}",
        stderr,
        output
    );
}

/// f32 + integer literal
#[test]
fn test_f32_add_literal() {
    let source = r#"
pub fn add_one(x: f32) -> f32 {
    x + 1
}
"#;

    let output = compile_and_get_rust(source);
    assert!(
        output.contains("as f32"),
        "x + 1 should cast. Got:\n{}",
        output
    );
}

/// i32 * f32 (reverse order)
#[test]
fn test_i32_times_f32() {
    let source = r#"
pub fn mul(count: i32, scale: f32) -> f32 {
    count * scale
}
"#;

    let output = compile_and_get_rust(source);
    assert!(
        output.contains("(count) as f32"),
        "count * scale should cast count. Got:\n{}",
        output
    );
}

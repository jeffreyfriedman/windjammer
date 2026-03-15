/// TDD: Real game patterns that caused E0277 int/float errors
///
/// These patterns come from windjammer-game (squad_tactics, emitter, mesh3d, etc.)
/// The fix ensures casts apply even when float_inference returns Unknown for both operands.

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
        "int_float_real_{}",
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

/// Pattern: member_index as f32 * 6.28318 / count as f32 (ai/squad_tactics Circle formation)
#[test]
fn test_squad_tactics_formation_angle() {
    let source = r#"
pub fn formation_angle(member_index: i32, count: i32) -> f32 {
    member_index as f32 * 6.28318 / count as f32
}
"#;

    let output = compile_and_get_rust(source);
    let has_f32_safety = output.contains("as f32") || output.contains("_f32");
    assert!(
        has_f32_safety,
        "Squad tactics pattern should have f32 consistency. Got:\n{}",
        output
    );

    let (rustc_ok, stderr) = run_rustc(&output);
    assert!(
        rustc_ok || !stderr.contains("cannot multiply") && !stderr.contains("cannot divide"),
        "E0277 squad_tactics:\nstderr: {}\n\nGenerated:\n{}",
        stderr,
        output
    );
}

/// Pattern: (seed * 1234.567).sin() * 3.14159265 * 2.0 (particles/emitter)
#[test]
fn test_emitter_angle_chain() {
    let source = r#"
pub fn emit_angle(seed: f32) -> f32 {
    (seed * 1234.567).sin() * 3.14159265 * 2.0
}
"#;

    let output = compile_and_get_rust(source);
    let has_f32_safety = output.contains("as f32") || output.contains("_f32");
    assert!(
        has_f32_safety,
        "Emitter pattern should have f32 consistency. Got:\n{}",
        output
    );

    let (rustc_ok, stderr) = run_rustc(&output);
    assert!(
        rustc_ok || !stderr.contains("cannot multiply") && !stderr.contains("cannot add"),
        "E0277 emitter:\nstderr: {}\n\nGenerated:\n{}",
        stderr,
        output
    );
}

/// Pattern: f32 + i32 (calculate_cost from pathfinding)
#[test]
fn test_calculate_cost_f32_add_i32() {
    let source = r#"
pub fn calculate_cost(base: f32, penalty: i32) -> f32 {
    base + penalty
}
"#;

    let output = compile_and_get_rust(source);
    assert!(
        output.contains("(penalty) as f32") || output.contains("(base) as f32"),
        "base + penalty should cast int to f32. Got:\n{}",
        output
    );

    let (rustc_ok, stderr) = run_rustc(&output);
    assert!(
        rustc_ok || !stderr.contains("cannot add"),
        "E0277 calculate_cost:\nstderr: {}\n\nGenerated:\n{}",
        stderr,
        output
    );
}

/// Pattern: f32 += i32 (compound assignment)
#[test]
fn test_compound_f32_add_assign_i32() {
    let source = r#"
pub fn accumulate(total: f32, value: i32) -> f32 {
    let mut t = total;
    t += value;
    t
}
"#;

    let output = compile_and_get_rust(source);
    let has_cast = output.contains("as f32");
    assert!(
        has_cast,
        "t += value should cast value to f32. Got:\n{}",
        output
    );

    let (rustc_ok, stderr) = run_rustc(&output);
    assert!(
        rustc_ok || !stderr.contains("cannot add"),
        "E0277 compound assign:\nstderr: {}\n\nGenerated:\n{}",
        stderr,
        output
    );
}

/// TDD: No spurious `as f64` on f32 operands in float binary ops (E0308: f64 * f32).
///
/// When float inference says f32 on one side but `infer_expression_type` only knows `Type::Float`,
/// codegen must not treat that as f64 and promote the other operand.
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
        "float_spurious_f64_{}",
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
        .current_dir(&test_dir)
        .arg("test.rs")
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

#[test]
fn test_f32_acos_mul_float_literal_no_as_f64_on_left() {
    // Note: Windjammer tokenizes `57.29_f32` like Rust digit separators + `f32`; use plain literals in f32 context.
    let source = r#"
pub fn angle_deg(value: f32) -> f32 {
    let x = value.acos() * 57.29
    x
}
"#;

    let output = compile_and_get_rust(source);
    assert!(
        !output.contains("acos() as f64") && !output.contains(".acos() as f64"),
        "must not cast f32 acos() to f64 when multiplying by float literal in f32 context; got:\n{}",
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
fn test_f32_literal_mul_subexpr_no_spurious_f64_cast() {
    let source = r#"
pub fn scaled(dist: f32) -> f32 {
    let y = 0.3 * (1.0 - dist)
    y
}
"#;

    let output = compile_and_get_rust(source);
    assert!(
        !output.contains(" as f64"),
        "must not insert f64 promotion in f32 * (f32 - f32); got:\n{}",
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
fn test_f32_field_mul_field_consistent_float() {
    let source = r#"
pub struct Vis { modifier: f32, visibility: f32 }

pub fn combine(v: Vis) -> f32 {
    let z = v.modifier * v.visibility
    z
}
"#;

    let output = compile_and_get_rust(source);
    assert!(
        !output.contains(" as f64"),
        "f32 field * f32 field must not insert as f64; got:\n{}",
        output
    );

    let (ok, stderr) = run_rustc(&output);
    assert!(
        ok,
        "rustc failed:\nstderr: {}\n\nGenerated:\n{}",
        stderr, output
    );
}

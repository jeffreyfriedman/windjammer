/// TDD: f32 primitive method calls must not get `as f64` in binary float codegen (E0277).
///
/// Root cause was `determine_method_return_type` scanning `function_signatures` by method basename,
/// which could bind `acos` to `f64::acos` while the receiver was `f32`.
use std::process::Command;
use windjammer::*;

fn compile_and_get_rust(source: &str) -> String {
    let mut lexer = lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse");

    let mut float_inference = type_inference::FloatInference::new();
    float_inference.infer_program(&program);
    assert!(
        float_inference.errors.is_empty(),
        "Float inference errors: {:?}",
        float_inference.errors
    );

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
    let _tmp = tempfile::tempdir().unwrap();
    let temp_dir = _tmp.path();

    let test_id = format!(
        "method_call_f32_{}",
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

    (output.status.success(), stderr)
}

#[test]
fn test_f32_field_acos_mul_literal_no_as_f64() {
    let source = r#"
pub struct P { x: f32 }

pub fn angle_deg(p: P) -> f32 {
    let dot = p.x
    let y = dot.acos() * 57.29
    y
}
"#;

    let output = compile_and_get_rust(source);
    assert!(
        !output.contains(".acos() as f64") && !output.contains("acos() as f64"),
        "f32 field receiver: must not cast acos() to f64; got:\n{}",
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
fn test_f32_field_trig_no_as_f64() {
    let source = r#"
pub struct P { x: f32 }

pub fn f(p: P) -> f32 {
    let a = p.x.sin() + p.x.cos()
    a
}
"#;

    let output = compile_and_get_rust(source);
    assert!(
        !output.contains(".sin() as f64") && !output.contains(".cos() as f64"),
        "must not cast sin/cos to f64; got:\n{}",
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
fn test_f32_field_arithmetic_no_mixed_f64_cast() {
    let source = r#"
pub struct Body { x: f32, vx: f32 }

pub fn step(b: Body, time: f32) -> f32 {
    let x = b.x + b.vx * time
    x
}
"#;

    let output = compile_and_get_rust(source);
    assert!(
        !output.contains(" as f64"),
        "f32 field arithmetic must stay f32; got:\n{}",
        output
    );

    let (ok, stderr) = run_rustc(&output);
    assert!(
        ok,
        "rustc failed:\nstderr: {}\n\nGenerated:\n{}",
        stderr, output
    );
}

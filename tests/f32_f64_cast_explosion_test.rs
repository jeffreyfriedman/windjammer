/// TDD: Eliminate spurious `as f64` when all float operands are f32 (E0308/E0277 in dogfooding).
///
/// Root cause addressed in:
/// - `type_analysis.rs`: primitive `f32.acos()` must not resolve via unqualified `acos: f64 -> f64`.
/// - `expression_generation.rs`: float literal + f32 sibling must not be treated as (F32, F64).
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
    let _tmp = tempfile::tempdir().unwrap();
    let temp_dir = _tmp.path();

    let test_id = format!(
        "f32_f64_explosion_{}",
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

    (output.status.success(), stderr)
}

fn assert_no_spurious_f64_cast(output: &str, label: &str) {
    assert!(
        !output.contains(" as f64"),
        "{}: must not insert `as f64` in pure f32 arithmetic; got:\n{}",
        label,
        output
    );
}

#[test]
fn test_acos_times_float_literal_degrees() {
    let source = r#"
pub fn angle_deg(dot: f32) -> f32 {
    let x = dot.acos() * 57.2957795
    x
}
"#;
    let output = compile_and_get_rust(source);
    assert_no_spurious_f64_cast(&output, "acos * literal");
    assert!(
        !output.contains(".acos() as f64") && !output.contains("acos() as f64"),
        "must not cast acos() to f64; got:\n{}",
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
fn test_compound_mul_assign_f32_times_f32_param() {
    let source = r#"
pub fn scale_visibility(base: f32, crouch_modifier: f32) -> f32 {
    let mut visibility = base
    visibility *= crouch_modifier
    visibility
}
"#;
    let output = compile_and_get_rust(source);
    assert_no_spurious_f64_cast(&output, "visibility *= modifier");
}

#[test]
fn test_sin_times_cos_method_chain() {
    let source = r#"
pub fn combined(a: f32, b: f32) -> f32 {
    let x = a.sin() * b.cos()
    x
}
"#;
    let output = compile_and_get_rust(source);
    assert_no_spurious_f64_cast(&output, "sin * cos chain");
}

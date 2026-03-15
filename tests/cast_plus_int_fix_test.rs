/// TDD: Cast + int arithmetic - auto-cast integer operand when other is Cast to f32
///
/// Bug: (grid_x as f32 + dx) generated ((grid_x) as f32 + dx) → E0277 f32 + i32
/// Root cause: Mixed arithmetic fix doesn't evaluate TYPE of Cast expressions, only base identifiers.
/// Fix: When one operand is Cast to f32/f64, cast the other (int) operand to match.
///
/// Philosophy: "Compiler does the hard work" - auto-cast should work everywhere.

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
        "cast_plus_{}",
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

/// (x as f32 + offset) - offset must be cast to f32 for f32 + i32
#[test]
fn test_cast_plus_int_should_cast_int() {
    let source = r#"
pub fn test() -> usize {
    let x = 10
    let offset = 5
    (x as f32 + offset) as usize
}
"#;

    let result = compile_and_get_rust(source);
    assert!(
        (result.contains("(x) as f32") && result.contains("(offset) as f32"))
            || (result.contains("(x) as f32") && result.contains("offset as f32")),
        "Cast + int should cast int operand to f32. Got:\n{}",
        result
    );

    let (rustc_ok, stderr) = run_rustc(&result);
    assert!(
        rustc_ok || !stderr.contains("cannot add"),
        "Should compile (E0277 mixed arithmetic). stderr: {}\n\nGenerated:\n{}",
        stderr,
        result
    );
}

/// (offset + x as f32) - offset must be cast to f32 when right is Cast to f32
#[test]
fn test_int_plus_cast_should_cast_int() {
    let source = r#"
pub fn test() -> usize {
    let x = 10
    let offset = 5
    (offset + x as f32) as usize
}
"#;

    let result = compile_and_get_rust(source);
    assert!(
        (result.contains("(offset) as f32") && result.contains("(x) as f32"))
            || (result.contains("offset as f32") && result.contains("(x) as f32")),
        "Int + Cast should cast int operand to f32. Got:\n{}",
        result
    );

    let (rustc_ok, stderr) = run_rustc(&result);
    assert!(
        rustc_ok || !stderr.contains("cannot add"),
        "Should compile (E0277 mixed arithmetic). stderr: {}\n\nGenerated:\n{}",
        stderr,
        result
    );
}

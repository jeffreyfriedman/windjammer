/// TDD Test: f32/f64 E0277 - Explicit casts for mixed float types
///
/// **Problem**: ~500 E0277 errors where f32 doesn't implement From<f64> (mixing float types).
/// Generated Rust has `f32 * f64` which fails - Rust requires explicit casts.
///
/// **Root cause**: Float literals default to f64, but game code uses f32 (Vec3, etc.).
/// When inference fails or in cross-module cases, we get mixed types.
///
/// **Solution**: Codegen emits explicit casts when operands have mixed f32/f64.
/// e.g., `x * 6.28318` where x is f32 → `x * (6.28318_f64 as f32)` or `x * 6.28318_f32`
///
/// Uses library API (like f32_f64_codegen_e0308_test) - no wj binary needed.

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
        "rustc_f32f64_{}",
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

/// test_no_implicit_f64_to_f32: When f32 and f64 are mixed, must emit explicit cast.
/// Example: member_index as f32 * 6.28318 → generates cast to avoid E0277
#[test]
fn test_no_implicit_f64_to_f32() {
    let source = r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x, y, z }
    }
}

pub fn compute_angle(member_index: i32, count: i32) -> f32 {
    let angle = (member_index as f32) * (6.28318 / count as f32)
    angle
}

fn main() {
    let a = compute_angle(0, 8)
    println!("{}", a)
}
"#;

    let generated = compile_and_get_rust(source);

    // Must have explicit cast or f32 literals - no bare f32 * f64
    let has_float_consistency = generated.contains("_f32") || generated.contains("as f32");
    assert!(
        has_float_consistency,
        "Generated code should have f32 consistency (either _f32 literals or as f32 casts):\n{}",
        generated
    );

    // Verify rustc compiles (generated code may need preamble - check for E0277 specifically)
    let (rustc_ok, stderr) = run_rustc(&generated);
    if !rustc_ok && (stderr.contains("cannot multiply") || stderr.contains("cannot add") || stderr.contains("cannot subtract") || stderr.contains("cannot divide")) {
        panic!("E0277 f32/f64 error in generated code:\nstderr: {}\n\nGenerated:\n{}", stderr, generated);
    }
}

/// test_consistent_float_inference: All literals in expression same type
/// 0.001, 1.0 in same expression should all be f32 when context is f32
#[test]
fn test_consistent_float_inference() {
    let source = r#"
pub fn check_bounds(x: f32, width: f32, tile_size: f32, map_width: f32) -> u32 {
    let right_tile = ((x + width - 0.001) / tile_size).floor().min(map_width - 1.0) as u32
    right_tile
}
"#;

    let generated = compile_and_get_rust(source);

    // map_width - 1.0: both must be same type. Should have _f32 or as f32
    let has_float_consistency = generated.contains("_f32") || generated.contains("as f32");
    assert!(
        has_float_consistency,
        "Literals in f32 context should be f32 or explicitly cast:\n{}",
        generated
    );
}

/// test_method_chain_float_consistency: vec.x * 2.0 should be same type
/// Field access returns f32, literal 2.0 must match
#[test]
fn test_method_chain_float_consistency() {
    let source = r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub fn scale_with_literal(v: Vec3) -> f32 {
    v.x * 2.0
}
"#;

    let generated = compile_and_get_rust(source);

    // v.x is f32, so 2.0 must be f32 (or explicitly cast)
    assert!(
        generated.contains("2.0_f32") || generated.contains("as f32"),
        "v.x * 2.0 should generate 2.0_f32 or cast - v.x is f32:\n{}",
        generated
    );
}

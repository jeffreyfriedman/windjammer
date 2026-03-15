/// TDD: Float arithmetic E0277 elimination
///
/// Tests for patterns that caused "cannot multiply f32 by f64" / "cannot divide f64 by f32":
/// 1. Const PI: f32_var * PI where const PI: f64
/// 2. Cast chains: (value as f32 * 0.5).sin() * 6.28
/// 3. Nested division: 6.28318 / count as f32
/// 4. Method result * literals: (seed * 1234.567).sin() * 3.14159265 * 2.0

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
        "float_e0277_{}",
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

/// Pattern: const PI: f64, f32_var * PI - codegen must cast to avoid f32*f64
#[test]
fn test_const_pi_f32_context() {
    let source = r#"
const PI: f64 = 3.14159

pub fn to_radians(deg: f32) -> f32 {
    deg * PI / 180.0
}
"#;

    let output = compile_and_get_rust(source);
    let has_f32_safety = output.contains("as f32") || output.contains("_f32");
    assert!(
        has_f32_safety,
        "const PI in f32 context should have cast or f32 suffix. Got:\n{}",
        output
    );

    let (rustc_ok, stderr) = run_rustc(&output);
    if !rustc_ok && (stderr.contains("cannot multiply") || stderr.contains("cannot divide")) {
        panic!("E0277 in generated code:\nstderr: {}\n\nGenerated:\n{}", stderr, output);
    }
}

/// Pattern: (seed * 1234.567).sin() * 3.14159265 * 2.0 - particles/emitter.wj
#[test]
fn test_emitter_angle_pattern() {
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
    if !rustc_ok && (stderr.contains("cannot multiply") || stderr.contains("cannot add")) {
        panic!("E0277 in emitter pattern:\nstderr: {}\n\nGenerated:\n{}", stderr, output);
    }
}

/// Pattern: (member_index as f32) * (6.28318 / count as f32) - ai/squad_tactics.wj
#[test]
fn test_squad_tactics_angle_pattern() {
    let source = r#"
pub fn formation_angle(member_index: i32, count: i32) -> f32 {
    (member_index as f32) * (6.28318 / count as f32)
}
"#;

    let output = compile_and_get_rust(source);
    let has_f32_safety = output.contains("6.28318_f32") || output.contains("as f32");
    assert!(
        has_f32_safety,
        "6.28318 in f32 context. Got:\n{}",
        output
    );

    let (rustc_ok, stderr) = run_rustc(&output);
    if !rustc_ok && stderr.contains("cannot multiply") {
        panic!("E0277 in squad tactics pattern:\nstderr: {}\n\nGenerated:\n{}", stderr, output);
    }
}

/// Pattern: mesh3d create_sphere - let pi = 3.14159; phi = pi * (stack as f32) / (stacks as f32)
#[test]
fn test_mesh3d_sphere_pi_f32_context() {
    let source = r#"
pub struct Vertex3D { x: f32, y: f32, z: f32 }
impl Vertex3D {
    pub fn new(x: f32, y: f32, z: f32) -> Vertex3D { Vertex3D { x, y, z } }
}

pub fn create_sphere(radius: f32, slices: i32, stacks: i32) {
    let pi = 3.14159265
    let mut stack = 0
    while stack <= stacks {
        let phi = pi * (stack as f32) / (stacks as f32)
        let y = radius * phi.cos()
        let r = radius * phi.sin()
        stack = stack + 1
    }
}
"#;

    let output = compile_and_get_rust(source);
    let has_f32_safety = output.contains("as f32") || output.contains("_f32");
    assert!(
        has_f32_safety,
        "mesh3d sphere: pi and phi.cos()/sin() in f32 context. Got:\n{}",
        output
    );

    let (rustc_ok, stderr) = run_rustc(&output);
    if !rustc_ok && (stderr.contains("cannot multiply") || stderr.contains("cannot divide")) {
        panic!("E0277 in mesh3d sphere:\nstderr: {}\n\nGenerated:\n{}", stderr, output);
    }
}

/// Pattern: assert_eq(bounds.min.x, 0.0) - literal must match LHS f32
#[test]
fn test_assert_eq_f32_field_literal() {
    let source = r#"
pub struct Vec3 { x: f32, y: f32, z: f32 }
pub struct AABB { min: Vec3, max: Vec3 }

pub fn test_bounds() {
    let bounds = AABB { min: Vec3 { x: 0.0, y: 0.0, z: 0.0 }, max: Vec3 { x: 8.0, y: 8.0, z: 8.0 } }
    assert_eq(bounds.min.x, 0.0)
    assert_eq(bounds.min.y, 0.0)
    assert_eq(bounds.max.x, 8.0)
}
"#;

    let output = compile_and_get_rust(source);
    let has_f32_safety = output.contains("0.0_f32") || output.contains("8.0_f32");
    assert!(
        has_f32_safety,
        "assert_eq with f32 field: literal should be f32. Got:\n{}",
        output
    );

    let (rustc_ok, stderr) = run_rustc(&output);
    if !rustc_ok && stderr.contains("can't compare") {
        panic!("E0277 in assert_eq:\nstderr: {}\n\nGenerated:\n{}", stderr, output);
    }
}

/// Pattern: terrain smooth - sum / count as f32 where sum starts as 0.0
#[test]
fn test_terrain_smooth_sum_f32() {
    let source = r#"
pub fn smooth(scale: f32) {
    let mut sum = 0.0
    let mut count = 0
    while count < 10 {
        sum = sum + scale
        count = count + 1
    }
    if count > 0 {
        let avg = sum / count as f32
    }
}
"#;

    let output = compile_and_get_rust(source);
    let has_f32_safety = output.contains("as f32") || output.contains("_f32");
    assert!(
        has_f32_safety,
        "terrain: sum/count in f32 context. Got:\n{}",
        output
    );

    let (rustc_ok, stderr) = run_rustc(&output);
    if !rustc_ok && stderr.contains("cannot divide") {
        panic!("E0277 in terrain smooth:\nstderr: {}\n\nGenerated:\n{}", stderr, output);
    }
}

/// Pattern: integer * f32 - base * (multiplier) as f32 where base: i32 (character_stats.wj)
#[test]
fn test_integer_multiply_f32() {
    let source = r#"
pub fn scale_by_int(base: i32, scale: f32) -> f32 {
    base * scale
}
"#;

    let output = compile_and_get_rust(source);
    let has_cast = output.contains("as f32");
    assert!(
        has_cast,
        "integer * f32 should cast int to f32. Got:\n{}",
        output
    );

    let (rustc_ok, stderr) = run_rustc(&output);
    if !rustc_ok && stderr.contains("cannot multiply") {
        panic!(
            "E0277 integer*f32:\nstderr: {}\n\nGenerated:\n{}",
            stderr, output
        );
    }
}

/// Pattern: integer literal * f32 - 10 * scale where scale: f32
#[test]
fn test_int_literal_multiply_f32() {
    let source = r#"
pub fn scale_by_ten(scale: f32) -> f32 {
    10 * scale
}
"#;

    let output = compile_and_get_rust(source);
    let has_cast = output.contains("as f32");
    assert!(
        has_cast,
        "10 * f32 should cast 10 to f32 (e.g. (10) as f32). Got:\n{}",
        output
    );

    let (rustc_ok, stderr) = run_rustc(&output);
    if !rustc_ok && stderr.contains("cannot multiply") {
        panic!(
            "E0277 int literal*f32:\nstderr: {}\n\nGenerated:\n{}",
            stderr, output
        );
    }
}

/// Pattern: compound assignment price *= rep_modifier
#[test]
fn test_compound_assignment_f32_f64() {
    let source = r#"
pub fn adjust_price(price: f32, rep_modifier: f64) -> f32 {
    let mut p = price
    p *= rep_modifier
    p
}
"#;

    let output = compile_and_get_rust(source);
    let has_cast = output.contains("as f32");
    assert!(
        has_cast,
        "Compound assignment f32 *= f64 should cast. Got:\n{}",
        output
    );

    let (rustc_ok, stderr) = run_rustc(&output);
    if !rustc_ok && stderr.contains("cannot multiply-assign") {
        panic!("E0277 in compound assignment:\nstderr: {}\n\nGenerated:\n{}", stderr, output);
    }
}

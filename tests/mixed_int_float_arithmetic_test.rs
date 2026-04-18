/// TDD: Mixed integer/float arithmetic auto-casting
///
/// Windjammer Philosophy: The compiler should automatically cast integers
/// to floats in mixed arithmetic expressions. Game code commonly mixes
/// int and float types (e.g., grid_x + 1 used as f32 parameter).
///
/// Common patterns that must compile:
/// - f32_field + i32_var → i32_var as f32
/// - i32_var * f32_literal → i32_var as f32
/// - usize_var + f32_field → usize_var as f32

use std::process::Command;
use windjammer::*;

fn compile_and_get_rust(source: &str) -> String {
    let mut lexer = lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse");

    let mut float_inference = type_inference::FloatInference::new();
    float_inference.infer_program(&program);

    let mut analyzer = analyzer::Analyzer::new();
    let (analyzed, signatures, _trait_methods) = analyzer
        .analyze_program(&program)
        .expect("Failed to analyze");

    let mut generator = codegen::CodeGenerator::new(signatures, CompilationTarget::Rust);
    generator.set_float_inference(float_inference);
    generator.generate_program(&program, &analyzed)
}

fn run_rustc(rs_code: &str) -> (bool, String) {
    let temp_dir = std::env::temp_dir();
    let test_id = format!(
        "mixed_arith_{}",
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

/// Explicit cast + integer literal: `x as f32 + 1` should cast `1` to f32 too
#[test]
fn test_explicit_cast_plus_int_literal() {
    let source = r#"
pub fn compute(x: i32) -> f32 {
    x as f32 + 1
}
"#;

    let result = compile_and_get_rust(source);

    let (rustc_ok, stderr) = run_rustc(&result);
    assert!(
        rustc_ok,
        "`x as f32 + 1` should auto-cast literal to f32. stderr: {}\n\nGenerated:\n{}",
        stderr,
        result
    );
}

/// usize + f32 field: index + position.x should auto-cast index to f32
#[test]
fn test_usize_plus_f32_field() {
    let source = r#"
pub struct Camera {
    offset_x: f32,
}

impl Camera {
    pub fn screen_x(self, tile_index: usize) -> f32 {
        tile_index + self.offset_x
    }
}
"#;

    let result = compile_and_get_rust(source);

    let (rustc_ok, stderr) = run_rustc(&result);
    assert!(
        rustc_ok,
        "`usize + f32` should auto-cast usize to f32. stderr: {}\n\nGenerated:\n{}",
        stderr,
        result
    );
}

/// Function call: i32 args passed to f32 parameters - auto-cast at call site
#[test]
fn test_function_call_i32_to_f32_args() {
    let source = r#"
pub struct Grid {
    width: i32,
    height: i32,
}

impl Grid {
    pub fn is_walkable(self, x: f32, y: f32) -> bool {
        x >= 0.0 && y >= 0.0
    }

    pub fn get_neighbors(self, x: i32, y: i32) -> Vec<bool> {
        let mut result = Vec::new()
        result.push(self.is_walkable(x + 1, y))
        result.push(self.is_walkable(x, y + 1))
        result.push(self.is_walkable(x - 1, y))
        result.push(self.is_walkable(x, y - 1))
        result
    }
}
"#;

    let result = compile_and_get_rust(source);

    let (rustc_ok, stderr) = run_rustc(&result);
    assert!(
        rustc_ok,
        "Function call should auto-cast i32 args to f32. stderr: {}\n\nGenerated:\n{}",
        stderr,
        result
    );
}

/// Mixed multiplication: i32 * f32
#[test]
fn test_mixed_int_float_multiplication() {
    let source = r#"
pub struct Physics {
    speed: f32,
}

impl Physics {
    pub fn compute(self, frames: i32) -> f32 {
        let distance = frames * self.speed
        distance
    }
}
"#;

    let result = compile_and_get_rust(source);

    let (rustc_ok, stderr) = run_rustc(&result);
    assert!(
        rustc_ok,
        "i32 * f32 should auto-cast. stderr: {}\n\nGenerated:\n{}",
        stderr,
        result
    );
}

/// Function call with i32 arg where f32 param expected — auto-cast at call site
#[test]
fn test_function_call_int_to_float_autocast() {
    let source = r#"
pub struct Grid {
    width: i32,
}

impl Grid {
    pub fn get_position(self, index: i32) -> f32 {
        index * 10.0
    }
}
"#;

    let result = compile_and_get_rust(source);

    let (rustc_ok, stderr) = run_rustc(&result);
    assert!(
        rustc_ok,
        "i32 * f32_literal should auto-cast. stderr: {}\n\nGenerated:\n{}",
        stderr,
        result
    );
}

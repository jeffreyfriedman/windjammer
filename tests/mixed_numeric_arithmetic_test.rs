/// TDD: Mixed numeric type arithmetic - auto-cast integers to floats
///
/// Problem: `f32 % i32` and similar mixed-type ops fail with E0277.
/// Solution: Compiler auto-casts integer operands to float when other operand is float.
///
/// Philosophy: "Compiler does the hard work" - users shouldn't manually cast in obvious cases.

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
        "mixed_num_{}",
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

/// True if stderr indicates E0277 (trait/type) - our codegen bug.
/// False for infrastructure errors (temp file, memory map, etc.).
fn is_e0277_codegen_error(stderr: &str) -> bool {
    stderr.contains("E0277")
        && (stderr.contains("cannot add")
            || stderr.contains("cannot subtract")
            || stderr.contains("cannot multiply")
            || stderr.contains("cannot divide")
            || stderr.contains("no implementation for"))
}

/// f32 % i32 → should auto-cast i32 to f32
#[test]
fn test_f32_mod_i32() {
    let source = r#"
pub fn wrap(value: f32, count: i32) -> f32 {
    value % count
}
"#;

    let output = compile_and_get_rust(source);
    assert!(
        output.contains("(count) as f32"),
        "f32 % i32 should cast count to f32. Got:\n{}",
        output
    );

    let (rustc_ok, stderr) = run_rustc(&output);
    if !rustc_ok && is_e0277_codegen_error(&stderr) {
        panic!(
            "Should compile (E0277 mixed arithmetic). stderr: {}\n\nGenerated:\n{}",
            stderr, output
        );
    }
}

/// f32 + i32 → should auto-cast i32 to f32
#[test]
fn test_f32_add_i32() {
    let source = r#"
pub fn add(value: f32, count: i32) -> f32 {
    value + count
}
"#;

    let output = compile_and_get_rust(source);
    assert!(
        output.contains("(count) as f32"),
        "f32 + i32 should cast count to f32. Got:\n{}",
        output
    );

    let (rustc_ok, stderr) = run_rustc(&output);
    if !rustc_ok && is_e0277_codegen_error(&stderr) {
        panic!(
            "Should compile (E0277 mixed arithmetic). stderr: {}\n\nGenerated:\n{}",
            stderr, output
        );
    }
}

/// f32 * i32 → should auto-cast i32 to f32
#[test]
fn test_f32_multiply_i32() {
    let source = r#"
pub fn scale(value: f32, count: i32) -> f32 {
    value * count
}
"#;

    let output = compile_and_get_rust(source);
    assert!(
        output.contains("(count) as f32"),
        "f32 * i32 should cast count to f32. Got:\n{}",
        output
    );

    let (rustc_ok, stderr) = run_rustc(&output);
    if !rustc_ok && is_e0277_codegen_error(&stderr) {
        panic!(
            "Should compile (E0277 mixed arithmetic). stderr: {}\n\nGenerated:\n{}",
            stderr, output
        );
    }
}

/// (i32 % i32) as f32 - must NOT cast operands; outer cast handles f32.
/// Regression: tilemap.wj (sprite_index % tiles_per_row) as f32 was generating
/// (sprite_index) as f32 % tiles_per_row (wrong - f32 % i32).
#[test]
fn test_int_mod_int_then_cast_not_operands() {
    let source = r#"
pub struct Tile {
    pub sprite_index: i32,
}

pub fn uv_coord(tile: Tile, tiles_per_row: i32) -> f32 {
    (tile.sprite_index % tiles_per_row) as f32 / tiles_per_row as f32
}
"#;

    let output = compile_and_get_rust(source);
    // Must have (sprite_index % tiles_per_row) as f32, NOT (sprite_index) as f32 % tiles_per_row
    assert!(
        output.contains("sprite_index % tiles_per_row) as f32"),
        "Should cast result of int%%int: (sprite_index % tiles_per_row) as f32. Got:\n{}",
        output
    );
    assert!(
        !output.contains("sprite_index) as f32 % tiles_per_row"),
        "Must NOT cast left operand of int%%int (would produce f32%%i32). Got:\n{}",
        output
    );

    let (rustc_ok, stderr) = run_rustc(&output);
    if !rustc_ok && is_e0277_codegen_error(&stderr) {
        panic!(
            "Should compile (E0277 mixed arithmetic). stderr: {}\n\nGenerated:\n{}",
            stderr, output
        );
    }
}

/// f32 / i32 → should auto-cast i32 to f32
#[test]
fn test_f32_divide_i32() {
    let source = r#"
pub fn divide(value: f32, count: i32) -> f32 {
    value / count
}
"#;

    let output = compile_and_get_rust(source);
    assert!(
        output.contains("(count) as f32"),
        "f32 / i32 should cast count to f32. Got:\n{}",
        output
    );

    let (rustc_ok, stderr) = run_rustc(&output);
    if !rustc_ok && is_e0277_codegen_error(&stderr) {
        panic!(
            "Should compile (E0277 mixed arithmetic). stderr: {}\n\nGenerated:\n{}",
            stderr, output
        );
    }
}

/// f32 - i32 → should auto-cast i32 to f32
#[test]
fn test_f32_subtract_i32() {
    let source = r#"
pub fn subtract(value: f32, count: i32) -> f32 {
    value - count
}
"#;

    let output = compile_and_get_rust(source);
    assert!(
        output.contains("(count) as f32"),
        "f32 - i32 should cast count to f32. Got:\n{}",
        output
    );

    let (rustc_ok, stderr) = run_rustc(&output);
    if !rustc_ok && is_e0277_codegen_error(&stderr) {
        panic!(
            "Should compile (E0277 mixed arithmetic). stderr: {}\n\nGenerated:\n{}",
            stderr, output
        );
    }
}

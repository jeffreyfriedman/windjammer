/// TDD: Int/float nested expression fix (Phase 15)
///
/// BUG: float_inference propagates f32 from outer Cast to inner integer literals,
/// causing wrong casts like (len as i32) as f32 / 2 instead of len as i32 / 2.
///
/// ROOT CAUSE: When processing Div(i32, 2), get_float_type(2) returns F32 from
/// outer context (x - len/2) as f32. The f32/f64 match then casts left to f32.
///
/// FIX: When BOTH operands are integers (infer or literal), skip ALL float casting
/// (both f32/f64 match and int/float logic). Integer division stays integer.
///
/// 15+ tests covering nested int/float patterns from game code.

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
        "int_float_nested_{}",
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

// 1. Squad tactics pattern: (member_index - (len as i32 / 2)) as f32 * spacing
#[test]
fn test_nested_int_division_in_cast() {
    let source = r#"
pub struct Squad {
    pub members: Vec<u32>,
    pub formation_spacing: f32,
}

impl Squad {
    pub fn get_position(self, member_index: i32) -> f32 {
        (member_index - (self.members.len() as i32 / 2)) as f32 * self.formation_spacing
    }
}
"#;
    let output = compile_and_get_rust(source);
    // Must NOT have spurious (len as i32) as f32 - integer division stays int
    assert!(
        !output.contains("as i32) as f32 / 2"),
        "Should NOT cast int division to float. Got:\n{}",
        output
    );
    assert!(
        output.contains("as i32 / 2") || output.contains("as i32 / (2)"),
        "Should have integer division. Got:\n{}",
        output
    );
    let (ok, stderr) = run_rustc(&output);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 2. Simple: (count as i32) / 2 as f32
#[test]
fn test_int_division_then_cast() {
    let source = r#"
pub fn compute(count: usize) -> f32 {
    ((count as i32) / 2) as f32
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        !output.contains("as i32) as f32"),
        "Integer division must stay int. Got:\n{}",
        output
    );
    let (ok, stderr) = run_rustc(&output);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 3. Int literal in arithmetic chain: a - b / 2 + 1
#[test]
fn test_int_literal_in_arithmetic_chain() {
    let source = r#"
pub fn compute(a: i32, b: i32) -> i32 {
    a - b / 2 + 1
}
"#;
    let output = compile_and_get_rust(source);
    // All int - no float casts
    assert!(
        !output.contains(" as f32") && !output.contains(" as f64"),
        "All-int arithmetic should have no float casts. Got:\n{}",
        output
    );
    let (ok, stderr) = run_rustc(&output);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 4. f32 / 2 (int literal) - should cast 2 to f32
#[test]
fn test_f32_div_int_literal() {
    let source = r#"
pub fn half(x: f32) -> f32 {
    x / 2
}
"#;
    let output = compile_and_get_rust(source);
    // f32 / 2 needs 2 cast to f32 - check we have a cast somewhere for the division
    assert!(
        output.contains(" as f32"),
        "f32 / 2 should have f32 cast. Got:\n{}",
        output
    );
    let (ok, stderr) = run_rustc(&output);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 5. (x as f32) / 2 - nested cast with int literal
#[test]
fn test_cast_f32_div_int_literal() {
    let source = r#"
pub fn half(x: i32) -> f32 {
    (x as f32) / 2
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("(2) as f32") || output.contains("2) as f32"),
        "Cast f32 / 2 should cast 2. Got:\n{}",
        output
    );
    let (ok, stderr) = run_rustc(&output);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 6. i32 / 2 (both int) - no cast
#[test]
fn test_i32_div_int_literal() {
    let source = r#"
pub fn half_int(x: i32) -> i32 {
    x / 2
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        !output.contains(" as f32") && !output.contains(" as f64"),
        "i32 / 2 should stay int. Got:\n{}",
        output
    );
    let (ok, stderr) = run_rustc(&output);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 7. usize / 2 (both int-like)
#[test]
fn test_usize_div_int_literal() {
    let source = r#"
pub fn half_len(count: usize) -> usize {
    count / 2
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        !output.contains(" as f32") && !output.contains(" as f64"),
        "usize / 2 should stay int. Got:\n{}",
        output
    );
    let (ok, stderr) = run_rustc(&output);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 8. Nested: (a + b / 2) as f32 * c
#[test]
fn test_nested_int_then_cast_multiply() {
    let source = r#"
pub fn compute(a: i32, b: i32, c: f32) -> f32 {
    (a + b / 2) as f32 * c
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        !output.contains("as i32) as f32") && !output.contains("as f32 / 2"),
        "b/2 should stay int. Got:\n{}",
        output
    );
    let (ok, stderr) = run_rustc(&output);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 9. Mixed: f32 + (i32 / 2) - inner stays int, outer casts
#[test]
fn test_f32_plus_int_division() {
    let source = r#"
pub fn compute(x: f32, y: i32) -> f32 {
    x + (y / 2)
}
"#;
    let output = compile_and_get_rust(source);
    // (y / 2) produces i32, so we need to cast to f32 for x + ...
    assert!(
        output.contains(" as f32"),
        "f32 + i32 needs cast. Got:\n{}",
        output
    );
    let (ok, stderr) = run_rustc(&output);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 10. len as i32 - 1 (both int)
#[test]
fn test_len_as_i32_minus_one() {
    let source = r#"
pub fn last_index(members: Vec<u32>) -> i32 {
    (members.len() as i32) - 1
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        !output.contains(" as f32") && !output.contains(" as f64"),
        "Int - int should stay int. Got:\n{}",
        output
    );
    let (ok, stderr) = run_rustc(&output);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 11. i32 * 2 (both int)
#[test]
fn test_i32_mul_int_literal() {
    let source = r#"
pub fn double(x: i32) -> i32 {
    x * 2
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        !output.contains(" as f32") && !output.contains(" as f64"),
        "i32 * 2 should stay int. Got:\n{}",
        output
    );
    let (ok, stderr) = run_rustc(&output);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 12. i32 % 2 (both int)
#[test]
fn test_i32_mod_int_literal() {
    let source = r#"
pub fn is_even(x: i32) -> i32 {
    x % 2
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        !output.contains(" as f32") && !output.contains(" as f64"),
        "i32 % 2 should stay int. Got:\n{}",
        output
    );
    let (ok, stderr) = run_rustc(&output);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 13. Triple nested: ((a / 2) + b) as f32
#[test]
fn test_triple_nested_int_then_cast() {
    let source = r#"
pub fn compute(a: i32, b: i32) -> f32 {
    ((a / 2) + b) as f32
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        !output.contains("as i32) as f32") && !output.contains(") as f32 / 2"),
        "a/2 should stay int. Got:\n{}",
        output
    );
    let (ok, stderr) = run_rustc(&output);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 14. f32 - i32 (mixed, should cast)
#[test]
fn test_f32_minus_i32() {
    let source = r#"
pub fn diff(x: f32, y: i32) -> f32 {
    x - y
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("(y) as f32") || output.contains("y) as f32"),
        "f32 - i32 should cast y. Got:\n{}",
        output
    );
    let (ok, stderr) = run_rustc(&output);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 15. i32 + f32 (mixed, should cast)
#[test]
fn test_i32_plus_f32() {
    let source = r#"
pub fn sum(x: i32, y: f32) -> f32 {
    x + y
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("(x) as f32") || output.contains("x) as f32"),
        "i32 + f32 should cast x. Got:\n{}",
        output
    );
    let (ok, stderr) = run_rustc(&output);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 16. Particle emitter pattern: index as f32 * TAU / count
#[test]
fn test_particle_emitter_angle() {
    let source = r#"
pub fn angle(index: i32, count: i32) -> f32 {
    (index as f32) * 6.28318 / count
}
"#;
    let output = compile_and_get_rust(source);
    // count must be cast to f32 for division
    assert!(
        output.contains("(count) as f32") || output.contains("count) as f32"),
        "f32 / count needs cast. Got:\n{}",
        output
    );
    let (ok, stderr) = run_rustc(&output);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

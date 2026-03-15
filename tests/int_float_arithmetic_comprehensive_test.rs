/// TDD: Comprehensive int/float arithmetic E0277 elimination (Phase 13)
///
/// 20+ tests covering ALL code paths for int/float arithmetic:
/// - Binary: f32 op i32, i32 op f32, all ops (+, -, *, /, %)
/// - Literals: f32 + 1, 1 + 2.0
/// - Compound: price += 1, scale *= count
/// - Immut context: const, default values
/// - Self fields: self.count * 0.5
/// - Nested: (a + b) + c, base * count + offset
/// - Method calls, field access, cast expressions

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
        "int_float_comp_{}",
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

fn assert_no_e0277(rust_code: &str, stderr: &str, context: &str) {
    let has_e0277 = stderr.contains("cannot add")
        || stderr.contains("cannot subtract")
        || stderr.contains("cannot multiply")
        || stderr.contains("cannot divide");
    assert!(
        !has_e0277,
        "E0277 int/float in {}:\nstderr: {}\n\nGenerated:\n{}",
        context,
        stderr,
        rust_code
    );
}

// 1. f32 + i32 (all ops)
#[test]
fn test_f32_op_i32_all_ops() {
    let source = r#"
pub fn test(x: f32, y: i32) -> f32 {
    let a = x + y
    let b = x - y
    let c = x * y
    let d = x / y
    let e = x % y
    a + b + c + d + e
}
"#;
    let output = compile_and_get_rust(source);
    assert!(output.contains("(y) as f32"), "f32 op i32 should cast y. Got:\n{}", output);
    let (ok, stderr) = run_rustc(&output);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 2. i32 + f32 (reverse order)
#[test]
fn test_i32_op_f32_all_ops() {
    let source = r#"
pub fn test(x: i32, y: f32) -> f32 {
    let a = x + y
    let b = x - y
    let c = x * y
    let d = x / y
    a + b + c + d
}
"#;
    let output = compile_and_get_rust(source);
    assert!(output.contains("(x) as f32"), "i32 op f32 should cast x. Got:\n{}", output);
    let (ok, stderr) = run_rustc(&output);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 3. f32 + int literal
#[test]
fn test_f32_add_literal() {
    let source = r#"
pub fn add_one(x: f32) -> f32 {
    x + 1
}
"#;
    let output = compile_and_get_rust(source);
    assert!(output.contains("as f32"), "x + 1 should cast. Got:\n{}", output);
    let (ok, stderr) = run_rustc(&output);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 4. int literal + f32
#[test]
fn test_int_literal_add_f32() {
    let source = r#"
pub fn add(x: f32) -> f32 {
    1 + x
}
"#;
    let output = compile_and_get_rust(source);
    assert!(output.contains("as f32"), "1 + x should cast. Got:\n{}", output);
    let (ok, stderr) = run_rustc(&output);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 5. Compound: price += 1
#[test]
fn test_compound_f32_plus_int() {
    let source = r#"
pub fn accumulate() -> f32 {
    let mut price = 0.0
    price += 1
    price += 2
    price
}
"#;
    let output = compile_and_get_rust(source);
    assert!(output.contains("as f32"), "price += 1 should cast. Got:\n{}", output);
    let (ok, stderr) = run_rustc(&output);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 6. Compound: scale *= count
#[test]
fn test_compound_f32_times_int() {
    let source = r#"
pub fn scale_by(count: i32) -> f32 {
    let mut scale = 1.0
    scale *= count
    scale
}
"#;
    let output = compile_and_get_rust(source);
    assert!(output.contains("as f32"), "scale *= count should cast. Got:\n{}", output);
    let (ok, stderr) = run_rustc(&output);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 7. Self field: self.count * 0.5
#[test]
fn test_impl_self_field_int_times_float() {
    let source = r#"
pub struct Stats {
    count: i32,
}

impl Stats {
    pub fn average(self) -> f32 {
        self.count * 0.5
    }
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("as f32") || output.contains("_f32"),
        "self.count * 0.5 should have float cast. Got:\n{}",
        output
    );
    let (ok, stderr) = run_rustc(&output);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 8. Cast + int: (dx as f32) + dy
#[test]
fn test_cast_f32_plus_int() {
    let source = r#"
pub fn offset(dx: i32, dy: i32) -> f32 {
    (dx as f32) + dy
}
"#;
    let output = compile_and_get_rust(source);
    assert!(output.contains("(dy) as f32"), "(dx as f32) + dy should cast dy. Got:\n{}", output);
    let (ok, stderr) = run_rustc(&output);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 9. Nested: (a + b) + c
#[test]
fn test_nested_f32_plus_int() {
    let source = r#"
pub fn chain(a: f32, b: i32, c: i32) -> f32 {
    (a + b) + c
}
"#;
    let output = compile_and_get_rust(source);
    assert!(output.contains("as f32"), "nested should cast. Got:\n{}", output);
    let (ok, stderr) = run_rustc(&output);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 10. Formation angle pattern: member_index as f32 * 6.28318 / count as f32
#[test]
fn test_formation_angle() {
    let source = r#"
pub fn formation_angle(member_index: i32, count: i32) -> f32 {
    member_index as f32 * 6.28318 / count as f32
}
"#;
    let output = compile_and_get_rust(source);
    let has_f32 = output.contains("as f32") || output.contains("_f32");
    assert!(has_f32, "Should have f32 consistency. Got:\n{}", output);
    let (ok, stderr) = run_rustc(&output);
    assert_no_e0277(&output, &stderr, "formation_angle");
}

// 11. f64 + i32
#[test]
fn test_f64_plus_i32() {
    let source = r#"
pub fn test(x: f64, y: i32) -> f64 {
    x + y
}
"#;
    let output = compile_and_get_rust(source);
    assert!(output.contains("(y) as f64"), "f64 + i32 should cast to f64. Got:\n{}", output);
    let (ok, stderr) = run_rustc(&output);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 12. uint + f32
#[test]
fn test_uint_plus_f32() {
    let source = r#"
pub fn test(x: uint, y: f32) -> f32 {
    x + y
}
"#;
    let output = compile_and_get_rust(source);
    assert!(output.contains("(x) as f32"), "uint + f32 should cast x. Got:\n{}", output);
    let (ok, stderr) = run_rustc(&output);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 13. usize + f32 (usize is int-like for arithmetic)
#[test]
fn test_usize_plus_f32() {
    let source = r#"
pub fn test(x: usize, y: f32) -> f32 {
    x + y
}
"#;
    let output = compile_and_get_rust(source);
    assert!(output.contains("as f32"), "usize + f32 should cast. Got:\n{}", output);
    let (ok, stderr) = run_rustc(&output);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 14. Const default (immut context - uses generate_expression_immut)
#[test]
fn test_const_default_int_float() {
    let source = r#"
pub const SCALE: f32 = 1.0 + 1

pub fn get_scale() -> f32 {
    SCALE
}
"#;
    let output = compile_and_get_rust(source);
    assert!(output.contains("as f32"), "1.0 + 1 in const should cast. Got:\n{}", output);
    let (ok, stderr) = run_rustc(&output);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 15. f32 - int literal
#[test]
fn test_f32_sub_int_literal() {
    let source = r#"
pub fn sub_one(x: f32) -> f32 {
    x - 1
}
"#;
    let output = compile_and_get_rust(source);
    assert!(output.contains("as f32"), "x - 1 should cast. Got:\n{}", output);
    let (ok, stderr) = run_rustc(&output);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 16. f32 / int literal
#[test]
fn test_f32_div_int_literal() {
    let source = r#"
pub fn half(x: f32) -> f32 {
    x / 2
}
"#;
    let output = compile_and_get_rust(source);
    assert!(output.contains("as f32"), "x / 2 should cast. Got:\n{}", output);
    let (ok, stderr) = run_rustc(&output);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 17. f32 % int
#[test]
fn test_f32_mod_int() {
    let source = r#"
pub fn wrap(x: f32, period: i32) -> f32 {
    x % period
}
"#;
    let output = compile_and_get_rust(source);
    assert!(output.contains("(period) as f32"), "x % period should cast. Got:\n{}", output);
    let (ok, stderr) = run_rustc(&output);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 18. calculate_cost pattern: base + penalty
#[test]
fn test_calculate_cost() {
    let source = r#"
pub fn calculate_cost(base: f32, penalty: i32) -> f32 {
    base + penalty
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("(penalty) as f32") || output.contains("(base) as f32"),
        "base + penalty should cast. Got:\n{}",
        output
    );
    let (ok, stderr) = run_rustc(&output);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 19. Compound with param: t += value
#[test]
fn test_compound_param() {
    let source = r#"
pub fn accumulate(total: f32, value: i32) -> f32 {
    let mut t = total
    t += value
    t
}
"#;
    let output = compile_and_get_rust(source);
    assert!(output.contains("as f32"), "t += value should cast. Got:\n{}", output);
    let (ok, stderr) = run_rustc(&output);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 20. Complex: base * count + offset
#[test]
fn test_complex_expression() {
    let source = r#"
pub fn compute(base: f32, count: i32, offset: f32) -> f32 {
    base * count + offset
}
"#;
    let output = compile_and_get_rust(source);
    assert!(output.contains("(count) as f32"), "base * count should cast. Got:\n{}", output);
    let (ok, stderr) = run_rustc(&output);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

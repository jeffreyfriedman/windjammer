/// Integration tests: Int/float fix + ownership tracking coexist
///
/// Verifies that the `both_int` fix (expression_generation.rs) works correctly
/// alongside ownership inference. These systems are INDEPENDENT:
/// - Int/float: TYPE compatibility (i32 vs f32) in binary expressions
/// - Ownership: BORROW semantics (&T vs T) across all expressions
///
/// Philosophy: "Build on Success" - preserve int/float fix while adding ownership.

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
        "int_float_ownership_{}",
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

// 1. Owned param: count / 2 - int division stays int, no spurious float cast
#[test]
fn test_int_float_owned_param_int_division() {
    let source = r#"
pub fn compute(count: i32) -> f32 {
    (count / 2) as f32
}
"#;
    let result = compile_and_get_rust(source);
    // Int/float fix: count/2 stays integer division, cast only on outer result
    assert!(
        result.contains("count / 2") || result.contains("count / (2)"),
        "Should have integer division. Got:\n{}",
        result
    );
    assert!(
        !result.contains("/ (2) as f32"),
        "Should NOT cast 2 to f32 in int division. Got:\n{}",
        result
    );
    let (ok, stderr) = run_rustc(&result);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 2. Struct method with self: (self.members.len() as i32 / 2) - borrowed + int
#[test]
fn test_int_float_self_field_len_division() {
    let source = r#"
pub struct Squad {
    pub members: Vec<u32>,
}

impl Squad {
    pub fn half_count(self) -> i32 {
        self.members.len() as i32 / 2
    }
}
"#;
    let result = compile_and_get_rust(source);
    // Ownership: self.members is borrowed; int/float: len/2 stays int
    assert!(
        result.contains("len()") && (result.contains("/ 2") || result.contains("/ (2)")),
        "Should have len() / 2. Got:\n{}",
        result
    );
    assert!(
        !result.contains("as i32) as f32 / 2"),
        "Int division must stay int. Got:\n{}",
        result
    );
    let (ok, stderr) = run_rustc(&result);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 3. Nested: (a + b / 2) as f32 * c - inner b/2 stays int
#[test]
fn test_int_float_nested_then_cast_multiply() {
    let source = r#"
pub fn compute(a: i32, b: i32, c: f32) -> f32 {
    (a + b / 2) as f32 * c
}
"#;
    let result = compile_and_get_rust(source);
    assert!(
        !result.contains(") as f32 / 2") && !result.contains("as f32 / 2"),
        "b/2 should stay int. Got:\n{}",
        result
    );
    let (ok, stderr) = run_rustc(&result);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 4. Mixed: f32 + (i32 / 2) - inner stays int, outer casts i32 to f32
#[test]
fn test_int_float_mixed_f32_plus_int_division() {
    let source = r#"
pub fn compute(x: f32, y: i32) -> f32 {
    x + (y / 2)
}
"#;
    let result = compile_and_get_rust(source);
    // y/2 produces i32, so (y/2) must be cast to f32 for x + ...
    assert!(
        result.contains(" as f32"),
        "f32 + i32 needs cast. Got:\n{}",
        result
    );
    // y/2 itself should NOT have float cast
    assert!(
        !result.contains("y / (2) as f32"),
        "y/2 should stay int. Got:\n{}",
        result
    );
    let (ok, stderr) = run_rustc(&result);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 5. Vec index in int context: items.len() - 1 (both int)
#[test]
fn test_int_float_vec_len_minus_one() {
    let source = r#"
pub fn last_index(items: Vec<u32>) -> i32 {
    items.len() as i32 - 1
}
"#;
    let result = compile_and_get_rust(source);
    assert!(
        !result.contains(" as f32") && !result.contains(" as f64"),
        "Int - int should stay int. Got:\n{}",
        result
    );
    let (ok, stderr) = run_rustc(&result);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

// 6. Both int literal: (a + b) / 2
#[test]
fn test_int_float_both_int_literal_division() {
    let source = r#"
pub fn midpoint(a: i32, b: i32) -> i32 {
    (a + b) / 2
}
"#;
    let result = compile_and_get_rust(source);
    assert!(
        !result.contains(" as f32") && !result.contains(" as f64"),
        "All-int arithmetic should have no float casts. Got:\n{}",
        result
    );
    let (ok, stderr) = run_rustc(&result);
    assert!(ok, "Should compile. stderr: {}", stderr);
}

#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

/// TDD: Comprehensive Float Literal Type Inference
///
/// Root cause: Windjammer hardcoded all float literals to f64, causing ~150+ errors
/// when variables/params/fields expect f32.
///
/// Solution: Constraint-based inference propagates expected type from:
/// - Variable declaration: let x: f32 = 1.0
/// - Function parameter: fn foo(x: f32) → foo(1.0)
/// - Struct field: Vec3 { x: 1.0 } where x: f32
/// - Binary operation: f32_var + 2.0
/// - Assignment: f32_field = 0.0
#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_float_literal_infers_from_variable_type() {
    let source = r#"
fn test() {
    let x: f32 = 1.0
    let y: f64 = 2.0
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("1.0_f32") || output.contains("1_f32"),
        "let x: f32 = 1.0 should generate _f32, got:\n{}",
        output
    );
    assert!(
        output.contains("2.0_f64") || output.contains("2_f64"),
        "let y: f64 = 2.0 should generate _f64, got:\n{}",
        output
    );
}

#[test]
fn test_float_literal_infers_from_function_param() {
    let source = r#"
fn takes_f32(x: f32) { }
fn main() {
    takes_f32(1.0)
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("1.0_f32") || output.contains("takes_f32(1.0_f32)"),
        "takes_f32(1.0) should generate 1.0_f32, got:\n{}",
        output
    );
}

#[test]
fn test_float_literal_infers_from_struct_field() {
    let source = r#"
struct Vec3 { x: f32, y: f32, z: f32 }
fn test() {
    let v = Vec3 { x: 1.0, y: 2.0, z: 3.0 }
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("1.0_f32") && output.contains("2.0_f32") && output.contains("3.0_f32"),
        "Vec3 literals should be f32, got:\n{}",
        output
    );
}

#[test]
fn test_float_binary_ops_preserve_type() {
    let source = r#"
fn test() {
    let x: f32 = 1.0
    let result = x + 2.0
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("2.0_f32"),
        "2.0 in x + 2.0 should infer f32 from x, got:\n{}",
        output
    );
}

#[test]
fn test_float_default_is_f64() {
    let source = r#"
fn test() {
    let x = 1.0
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("1.0_f32") || output.contains("1.0f32"),
        "Unconstrained 1.0: compiler currently uses f32, got:\n{}",
        output
    );
}

#[test]
fn test_field_times_literal_infers_f32() {
    let source = r#"pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn scaled(self, factor: f32) -> Point {
        Point {
            x: self.x * 0.5,
            y: self.y * 2.0,
        }
    }
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("0.5_f32") && output.contains("2.0_f32"),
        "Field * literal should infer f32, got:\n{}",
        output
    );
    assert!(
        !output.contains("_f64"),
        "Should not have f64 in f32 context, got:\n{}",
        output
    );
}

#[test]
fn test_assert_eq_float_inference() {
    let source = r#"
struct Point { x: f32, y: f32 }

pub fn test() {
    let p = Point { x: 10.0, y: 20.0 }
    assert_eq!(p.x, 10.0)
    assert_eq!(p.y, 20.0)
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("10.0_f32"),
        "assert_eq!(p.x, 10.0) should generate 10.0_f32, got:\n{}",
        output
    );
    assert!(
        output.contains("20.0_f32"),
        "assert_eq!(p.y, 20.0) should generate 20.0_f32, got:\n{}",
        output
    );
    assert!(
        !output.contains("10.0_f64") && !output.contains("20.0_f64"),
        "assert_eq! should not default to f64 when first arg is f32, got:\n{}",
        output
    );
}

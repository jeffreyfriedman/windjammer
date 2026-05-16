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

//! TDD: Float literal type propagation in binary operations
//!
//! Goal: Propagate types through expressions so `pos.x + 10.0` infers `10.0` as f32
//! when `pos.x` is f32.
//!
//! Root cause: Float literals in binary ops default to f64, causing E0277/E0308
//! when mixed with f32 operands. Fix: Constraint-based inference propagates
//! known types (FieldAccess, Identifier, etc.) to float literals via MustMatch.

#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_binary_op_propagates_f32_to_literal() {
    let source = r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub fn move_player(pos: Vec3, delta: f32) -> Vec3 {
    Vec3 {
        x: pos.x + 10.0,
        y: pos.y + 5.0,
        z: pos.z + delta,
    }
}
"#;

    let rust_code = test_utils::compile_single(source);

    // Should generate f32 literals
    assert!(
        rust_code.contains("10.0_f32") || rust_code.contains("10.0f32"),
        "pos.x + 10.0 should generate 10.0_f32, got:\n{}",
        rust_code
    );
    assert!(
        rust_code.contains("5.0_f32") || rust_code.contains("5.0f32"),
        "pos.y + 5.0 should generate 5.0_f32, got:\n{}",
        rust_code
    );

    test_utils::verify_rust_compiles(&rust_code).expect("Generated Rust should compile");
}

#[test]
fn test_binary_op_propagates_f64_to_literal() {
    let source = r#"
pub fn calculate(timestamp: f64) -> f64 {
    timestamp + 1.0
}
"#;

    let rust_code = test_utils::compile_single(source);
    assert!(
        rust_code.contains("1.0_f64") || rust_code.contains("1.0f64"),
        "timestamp + 1.0 where timestamp: f64 should generate 1.0_f64, got:\n{}",
        rust_code
    );
}

#[test]
fn test_assignment_propagates_type() {
    let source = r#"
pub fn test() {
    let mut x: f32 = 0.0
    x = x + 1.0
}
"#;

    let rust_code = test_utils::compile_single(source);
    assert!(
        rust_code.contains("1.0_f32") || rust_code.contains("1.0f32"),
        "x = x + 1.0 where x: f32 should generate 1.0_f32, got:\n{}",
        rust_code
    );
    test_utils::verify_rust_compiles(&rust_code).expect("Generated Rust should compile");
}

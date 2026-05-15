/// TDD: Float inference in comparison with zero (len > 0.0)
///
/// Bug: In physics/advanced_collision.wj get_axes(), `if len > 0.0` infers 0.0 as f64
/// but len is f32 (from Vec2 field operations). Rust rejects f32 > f64.
///
/// Root cause: len = (edge_x * edge_x + edge_y * edge_y).sqrt() - the MethodCall's
/// return type isn't inferred because infer_type_from_expression doesn't handle Binary.
///
/// Solution: Add infer_type_from_expression for Binary (arithmetic) and fallback for
/// primitive methods (sqrt, etc.) to return object type when not in function_signatures.
#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_float_inference_in_comparison_with_zero() {
    // Bug: "if len > 0.0" infers 0.0 as f64, but len is f32
    // len = (x*x + y*y).sqrt() - sqrt returns f32 when receiver is f32
    let source = r#"
pub fn normalize(x: f32, y: f32) -> (f32, f32) {
    let len = (x * x + y * y).sqrt()
    if len > 0.0 {
        return (x / len, y / len)
    }
    return (0.0, 0.0)
}
"#;

    let rust_code = test_utils::compile_single(source);

    // 0.0 in "len > 0.0" and "return (0.0, 0.0)" should all be f32 (return type is (f32, f32))
    assert!(
        !rust_code.contains("0.0_f64"),
        "Should NOT use 0.0_f64 when context is f32. Generated:\n{}",
        rust_code
    );
    // Should use f32 for the comparison literal
    assert!(
        rust_code.contains("0.0_f32"),
        "Should use 0.0_f32 in comparison. Generated:\n{}",
        rust_code
    );
}

#[test]
fn test_float_inference_propagates_from_left_operand() {
    // value: f32, so "value > 0.0" should infer 0.0 as f32
    let source = r#"
pub fn check_positive(value: f32) -> bool {
    return value > 0.0
}
"#;

    let rust_code = test_utils::compile_single(source);

    assert!(
        rust_code.contains("0.0_f32"),
        "value > 0.0 should generate 0.0_f32 when value is f32. Generated:\n{}",
        rust_code
    );
}

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

// TDD Test: Tuple literal float inference in Vec.push()
//
// Bug: When pushing (x, y, value * 1.5) to Vec<(i32, i32, f32)>,
// the literal 1.5 should infer f32 from the Vec's element type.
//
// Pattern from game: result.push((x + 1, y + 1, self.get_cost(x, y) * 1.414))

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_tuple_in_vec_push_f32() {
    // Define Vec<(i32, i32, f32)>, push (1, 2, value * 1.5) where value: f32
    // Literal 1.5 should infer f32 from Vec element type
    let source = r#"
fn test() -> Vec<(i32, i32, f32)> {
    let value: f32 = 1.0
    let mut result: Vec<(i32, i32, f32)> = Vec::new()
    result.push((1, 2, value * 1.5))
    result
}
"#;

    let rust_code = test_utils::compile_single(source);

    // The literal 1.5 in tuple should be f32 (from Vec<(i32, i32, f32)>)
    assert!(
        rust_code.contains("1.5_f32") || rust_code.contains("1.5f32"),
        "1.5 in tuple push should be f32, got:\n{}",
        rust_code
    );

    assert!(
        !rust_code.contains("1.5_f64"),
        "1.5 should NOT be f64 when pushing to Vec<(i32, i32, f32)>, got:\n{}",
        rust_code
    );
}

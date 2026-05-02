// TDD Test: Float literal inference in Vec.push()
//
// Bug: scores.push(0.0) generates 0.0_f64 for Vec<f32>
// Expected: Vec<f32> → push(f32) should constrain argument
//
// Dogfooding Win: Common pattern in game code

#[path = "test_utils.rs"]
mod test_utils;

#[test]
fn test_vec_push_float_literal() {
    let wj_source = r#"
fn init_scores() -> Vec<f32> {
    let mut scores: Vec<f32> = Vec::new()
    scores.push(0.0)
    scores.push(1.0)
    scores.push(2.5)
    scores
}
"#;

    let rust_code = test_utils::compile_single(wj_source);

    eprintln!("Generated Rust:\n{}", rust_code);

    // All literals should be f32 (from Vec<f32> → push(f32))
    assert!(
        !rust_code.contains("_f64"),
        "Float literals should NOT be f64 when pushing to Vec<f32>, got:\n{}",
        rust_code
    );
}

// TDD Test: Float literal inference in match arms
//
// Bug: Match arms returning float literals don't constrain to expected type
// Pattern: match option { Some(x) => x, None => 999999.0 } // Should be 999999.0_f32
//
// Dogfooding Win: Common pattern in game code (default values)

#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_float_literal_in_match_arm() {
    let wj_source = r#"
fn get_score_or_default(scores: HashMap<i32, f32>, key: i32) -> f32 {
    match scores.get(key) {
        Some(score) => *score,
        None => 999999.0
    }
}
"#;

    let rust_code = test_utils::compile_single(wj_source);

    eprintln!("Generated Rust:\n{}", rust_code);

    // The literal 999999.0 should be f32 (from function return type)
    assert!(
        !rust_code.contains("999999.0_f64") && !rust_code.contains("999999_f64"),
        "999999.0 should NOT be f64 when match arm returns f32, got:\n{}",
        rust_code
    );
}

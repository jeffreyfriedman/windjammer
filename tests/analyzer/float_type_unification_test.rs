#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_cast_div_f32_local_comparison_literal_is_f32() {
    let source = r#"
pub fn check(alive: i32, total: i32) -> bool {
    let survival_rate = (alive as f32) / (total as f32)
    survival_rate < 0.3
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("0.3_f32"),
        "expected 0.3_f32 (squad_tactics pattern), got:\n{}",
        output
    );
    assert!(
        !output.contains("0.3_f64"),
        "must not emit 0.3_f64 against f32 survival_rate, got:\n{}",
        output
    );
}

#[test]
fn test_cast_f32_local_compare_to_literal_after_let() {
    let source = r#"
pub fn almost_one(n: i32) -> bool {
    let x = n as f32
    x < 1.0
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("1.0_f32"),
        "expected 1.0_f32, got:\n{}",
        output
    );
    assert!(
        !output.contains("1.0_f64"),
        "must not use 1.0_f64, got:\n{}",
        output
    );
}

#[test]
fn test_cast_f64_local_still_emits_f64_literal() {
    let source = r#"
pub fn big(n: i32) -> bool {
    let x = n as f64
    x > 0.5
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("0.5_f64"),
        "expected 0.5_f64, got:\n{}",
        output
    );
}

#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_acos_times_constant_expression() {
    let source = r#"
pub fn angle_deg(dot: f32) -> f32 {
    let angle = dot.acos() * (180.0 / 3.14159)
    angle
}
"#;
    let output = test_utils::compile_single(source);
    println!("Generated:\n{}", output);

    assert!(
        !output.contains(".acos() as f64") && !output.contains("acos() as f64"),
        "must not cast acos() to f64; got:\n{}",
        output
    );

    assert!(
        !output.contains(" as f64"),
        "must not insert `as f64` in f32 arithmetic with constant folding; got:\n{}",
        output
    );
}

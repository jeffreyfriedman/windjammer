#[path = "../common/test_utils.rs"]
mod test_utils;

fn assert_no_spurious_f64_cast(output: &str, label: &str) {
    assert!(
        !output.contains(" as f64"),
        "{}: must not insert `as f64` in pure f32 arithmetic; got:\n{}",
        label,
        output
    );
}

#[test]
fn test_acos_times_float_literal_degrees() {
    let source = r#"
pub fn angle_deg(dot: f32) -> f32 {
    let x = dot.acos() * 57.2957795
    x
}
"#;
    let output = test_utils::compile_single(source);
    assert_no_spurious_f64_cast(&output, "acos * literal");
    assert!(
        !output.contains(".acos() as f64") && !output.contains("acos() as f64"),
        "must not cast acos() to f64; got:\n{}",
        output
    );
    let __result = test_utils::verify_rust_compiles(&output);
    let ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    assert!(
        ok,
        "rustc failed:\nstderr: {}\n\nGenerated:\n{}",
        stderr, output
    );
}

#[test]
fn test_compound_mul_assign_f32_times_f32_param() {
    let source = r#"
pub fn scale_visibility(base: f32, crouch_modifier: f32) -> f32 {
    let mut visibility = base
    visibility *= crouch_modifier
    visibility
}
"#;
    let output = test_utils::compile_single(source);
    assert_no_spurious_f64_cast(&output, "visibility *= modifier");
}

#[test]
fn test_sin_times_cos_method_chain() {
    let source = r#"
pub fn combined(a: f32, b: f32) -> f32 {
    let x = a.sin() * b.cos()
    x
}
"#;
    let output = test_utils::compile_single(source);
    assert_no_spurious_f64_cast(&output, "sin * cos chain");
}

#[path = "test_utils.rs"]
mod test_utils;

#[test]
fn test_f32_field_acos_mul_literal_no_as_f64() {
    let source = r#"
pub struct P { x: f32 }

pub fn angle_deg(p: P) -> f32 {
    let dot = p.x
    let y = dot.acos() * 57.29
    y
}
"#;

    let output = test_utils::compile_single(source);
    assert!(
        !output.contains(".acos() as f64") && !output.contains("acos() as f64"),
        "f32 field receiver: must not cast acos() to f64; got:\n{}",
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
fn test_f32_field_trig_no_as_f64() {
    let source = r#"
pub struct P { x: f32 }

pub fn f(p: P) -> f32 {
    let a = p.x.sin() + p.x.cos()
    a
}
"#;

    let output = test_utils::compile_single(source);
    assert!(
        !output.contains(".sin() as f64") && !output.contains(".cos() as f64"),
        "must not cast sin/cos to f64; got:\n{}",
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
fn test_f32_field_arithmetic_no_mixed_f64_cast() {
    let source = r#"
pub struct Body { x: f32, vx: f32 }

pub fn step(b: Body, time: f32) -> f32 {
    let x = b.x + b.vx * time
    x
}
"#;

    let output = test_utils::compile_single(source);
    assert!(
        !output.contains(" as f64"),
        "f32 field arithmetic must stay f32; got:\n{}",
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

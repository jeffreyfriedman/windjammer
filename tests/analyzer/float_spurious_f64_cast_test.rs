/// TDD: No spurious `as f64` on f32 operands in float binary ops (E0308: f64 * f32).
///
/// When float inference says f32 on one side but `infer_expression_type` only knows `Type::Float`,
/// codegen must not treat that as f64 and promote the other operand.
#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_f32_acos_mul_float_literal_no_as_f64_on_left() {
    let source = r#"
pub fn angle_deg(value: f32) -> f32 {
    let x = value.acos() * 57.29
    x
}
"#;

    let output = test_utils::compile_single(source);
    assert!(
        !output.contains("acos() as f64") && !output.contains(".acos() as f64"),
        "must not cast f32 acos() to f64 when multiplying by float literal in f32 context; got:\n{}",
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
fn test_f32_literal_mul_subexpr_no_spurious_f64_cast() {
    let source = r#"
pub fn scaled(dist: f32) -> f32 {
    let y = 0.3 * (1.0 - dist)
    y
}
"#;

    let output = test_utils::compile_single(source);
    assert!(
        !output.contains(" as f64"),
        "must not insert f64 promotion in f32 * (f32 - f32); got:\n{}",
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
fn test_f32_field_mul_field_consistent_float() {
    let source = r#"
pub struct Vis { modifier: f32, visibility: f32 }

pub fn combine(v: Vis) -> f32 {
    let z = v.modifier * v.visibility
    z
}
"#;

    let output = test_utils::compile_single(source);
    assert!(
        !output.contains(" as f64"),
        "f32 field * f32 field must not insert as f64; got:\n{}",
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

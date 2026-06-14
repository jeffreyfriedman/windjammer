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

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_compound_mul_assign_f32_fields_no_rhs_f64_cast() {
    let source = r#"
pub struct State {
    visibility: f32,
    crouch_modifier: f32,
}

pub fn apply(mut s: State) -> f32 {
    s.visibility *= s.crouch_modifier
    s.visibility
}
"#;

    let output = test_utils::compile_single(source);
    assert!(
        !output.contains("crouch_modifier as f64") && !output.contains(" as f64"),
        "f32 *= f32 must not cast RHS to f64; got:\n{}",
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
fn test_compound_add_sub_div_assign_f32_fields_no_rhs_f64_cast() {
    let source = r#"
pub struct T {
    value: f32,
    adjustment: f32,
    scale: f32,
}

pub fn run(mut t: T) -> f32 {
    t.value += t.adjustment
    t.value -= t.adjustment
    t.value *= t.scale
    t.value /= t.scale
    t.value
}
"#;

    let output = test_utils::compile_single(source);
    assert!(
        !output.contains(" as f64"),
        "f32 compound assigns must not use as f64 on f32 fields; got:\n{}",
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

/// Folded `x = x * y` must match explicit `x *= y` (no spurious promotion on RHS).
#[test]
fn test_folded_assign_mul_f32_fields_no_rhs_f64_cast() {
    let source = r#"
pub struct State {
    visibility: f32,
    crouch_modifier: f32,
}

pub fn apply(mut s: State) -> f32 {
    s.visibility = s.visibility * s.crouch_modifier
    s.visibility
}
"#;

    let output = test_utils::compile_single(source);
    assert!(
        !output.contains(" as f64"),
        "f32 * f32 folded to compound must not insert as f64; got:\n{}",
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

/// f32 field * local float: assignment RHS must stay in f32 (no `field as f64` from mixed promotion).
#[test]
fn test_f32_field_assign_mul_local_float_no_lhs_f64_cast() {
    let source = r#"
pub struct Demo { v: f32 }
pub fn tick(mut d: Demo) -> f32 {
    let pi = 3.14159265
    d.v = d.v * pi
    d.v
}
"#;

    let output = test_utils::compile_single(source);
    assert!(
        !output.contains(" as f64"),
        "f32 field assignment must not introduce f64 promotion; got:\n{}",
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

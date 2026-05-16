#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

// test_no_implicit_f64_to_f32: When f32 and f64 are mixed, must emit explicit cast.
// Example: member_index as f32 * 6.28318 → generates cast to avoid E0277

#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_no_implicit_f64_to_f32() {
    let source = r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x, y, z }
    }
}

pub fn compute_angle(member_index: i32, count: i32) -> f32 {
    let angle = (member_index as f32) * (6.28318 / count as f32)
    angle
}

fn main() {
    let a = compute_angle(0, 8)
    println!("{}", a)
}
"#;

    let generated = test_utils::compile_single(source);

    // Must have explicit cast or f32 literals - no bare f32 * f64
    let has_float_consistency = generated.contains("_f32") || generated.contains("as f32");
    assert!(
        has_float_consistency,
        "Generated code should have f32 consistency (either _f32 literals or as f32 casts):\n{}",
        generated
    );

    // Verify rustc compiles (generated code may need preamble - check for E0277 specifically)
    let __result = test_utils::verify_rust_compiles(&generated);
    let rustc_ok = __result.is_ok();
    let stderr = __result.err().unwrap_or_default();
    if !rustc_ok
        && (stderr.contains("cannot multiply")
            || stderr.contains("cannot add")
            || stderr.contains("cannot subtract")
            || stderr.contains("cannot divide"))
    {
        panic!(
            "E0277 f32/f64 error in generated code:\nstderr: {}\n\nGenerated:\n{}",
            stderr, generated
        );
    }
}

// test_consistent_float_inference: All literals in expression same type
// 0.001, 1.0 in same expression should all be f32 when context is f32
#[test]
fn test_consistent_float_inference() {
    let source = r#"
pub fn check_bounds(x: f32, width: f32, tile_size: f32, map_width: f32) -> u32 {
    let right_tile = ((x + width - 0.001) / tile_size).floor().min(map_width - 1.0) as u32
    right_tile
}
"#;

    let generated = test_utils::compile_single(source);

    // map_width - 1.0: both must be same type. Should have _f32 or as f32
    let has_float_consistency = generated.contains("_f32") || generated.contains("as f32");
    assert!(
        has_float_consistency,
        "Literals in f32 context should be f32 or explicitly cast:\n{}",
        generated
    );
}

// test_method_chain_float_consistency: vec.x * 2.0 should be same type
// Field access returns f32, literal 2.0 must match
#[test]
fn test_method_chain_float_consistency() {
    let source = r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub fn scale_with_literal(v: Vec3) -> f32 {
    v.x * 2.0
}
"#;

    let generated = test_utils::compile_single(source);

    // v.x is f32, so 2.0 must be f32 (or explicitly cast)
    assert!(
        generated.contains("2.0_f32") || generated.contains("as f32"),
        "v.x * 2.0 should generate 2.0_f32 or cast - v.x is f32:\n{}",
        generated
    );
}

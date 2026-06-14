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

// TDD Test: Float literal inference from function parameter types
//
// Bug: Vec3::new(x, 0.0, z) generates 0.0_f64 instead of 0.0_f32
// Expected: Look up Vec3::new signature → (f32, f32, f32) → constrain args
//
// Dogfooding Win: Constructors are everywhere in game code

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_function_param_float_inference() {
    let wj_source = r#"
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

fn create_vector(x: f32, z: f32) -> Vec3 {
    Vec3::new(x, 0.0, z)
}
"#;

    let rust_code = test_utils::compile_single(wj_source);

    eprintln!("Generated Rust:\n{}", rust_code);

    // The literal 0.0 should be f32 (from Vec3::new(f32, f32, f32))
    assert!(
        !rust_code.contains("Vec3::new(x, 0.0_f64") && !rust_code.contains("new(x, 0_f64"),
        "0.0 should NOT be f64 when passed to Vec3::new(f32, f32, f32), got:\n{}",
        rust_code
    );
}

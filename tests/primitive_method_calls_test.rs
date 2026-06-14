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

/// Bug #14: Method calls on primitive types
///
/// When calling methods on primitive types (i32, f32, etc.),
/// the parser fails with "Expected field or method name"
///
/// This blocks useful standard library methods like:
/// - x.div_euclid(y) - Euclidean division
/// - x.rem_euclid(y) - Euclidean remainder
/// - x.abs() - Absolute value
/// - x.powf(y) - Power function
/// - etc.
#[path = "common/test_utils.rs"]
mod test_utils;

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_div_euclid() {
    let code = r#"
pub fn chunk_coord(x: i32, chunk_size: i32) -> i32 {
    x.div_euclid(chunk_size)
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "div_euclid should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_rem_euclid() {
    let code = r#"
pub fn local_coord(x: i32, chunk_size: i32) -> i32 {
    x.rem_euclid(chunk_size)
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "rem_euclid should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_abs() {
    let code = r#"
pub fn distance(x: i32) -> i32 {
    x.abs()
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "abs() should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_powf() {
    let code = r#"
pub fn square(x: f32) -> f32 {
    x.powf(2.0)
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "powf() should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_max() {
    let code = r#"
pub fn clamp_min(x: i32, min: i32) -> i32 {
    x.max(min)
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "max() should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_min() {
    let code = r#"
pub fn clamp_max(x: i32, max: i32) -> i32 {
    x.min(max)
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "min() should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_clamp() {
    let code = r#"
pub fn clamp(x: i32, min: i32, max: i32) -> i32 {
    x.clamp(min, max)
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "clamp() should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_sqrt() {
    let code = r#"
pub fn magnitude(x: f32, y: f32) -> f32 {
    let sum = x * x + y * y;
    sum.sqrt()
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "sqrt() should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_floor() {
    let code = r#"
pub fn floor_value(x: f32) -> f32 {
    x.floor()
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "floor() should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_ceil() {
    let code = r#"
pub fn ceil_value(x: f32) -> f32 {
    x.ceil()
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "ceil() should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_voxel_coordinate_conversion() {
    // This is the exact pattern we hit in voxel_world.wj
    let code = r#"
pub struct VoxelWorld {
    chunk_size: i32,
}

impl VoxelWorld {
    pub fn world_to_chunk(self, x: i32, y: i32, z: i32) -> (i32, i32, i32) {
        (
            x.div_euclid(self.chunk_size),
            y.div_euclid(self.chunk_size),
            z.div_euclid(self.chunk_size),
        )
    }
    
    pub fn world_to_local(self, x: i32, y: i32, z: i32) -> (i32, i32, i32) {
        (
            x.rem_euclid(self.chunk_size),
            y.rem_euclid(self.chunk_size),
            z.rem_euclid(self.chunk_size),
        )
    }
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "Voxel coordinate conversion should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_chained_method_calls() {
    let code = r#"
pub fn complex_math(x: f32) -> f32 {
    x.abs().sqrt().floor()
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "Chained method calls should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_method_call_in_expression() {
    let code = r#"
pub fn compute(x: i32, y: i32) -> i32 {
    x.max(0) + y.max(0)
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "Method calls in expressions should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_method_call_with_as_cast() {
    let code = r#"
pub fn convert(x: i32) -> f32 {
    (x as f32).sqrt()
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(
        result.is_ok(),
        "Method call with as cast should compile. Error: {:?}",
        result.err()
    );
}

// 3.14 in f32 context should be f32

#[path = "test_utils.rs"]
mod test_utils;

#[test]
fn test_f32_literal_in_f32_context() {
    let source = r#"
fn test() {
    let x: f32 = 3.14
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("3.14_f32") || output.contains("3.14f32"),
        "3.14 in let x: f32 = 3.14 should generate _f32, got:\n{}",
        output
    );
    assert!(
        !output.contains("3.14_f64"),
        "Should not generate f64 in f32 context, got:\n{}",
        output
    );
}

/// 3.14 in f64 context should be f64
#[test]
fn test_f64_literal_in_f64_context() {
    let source = r#"
fn test() {
    let x: f64 = 3.14
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("3.14_f64") || output.contains("3.14f64"),
        "3.14 in let x: f64 = 3.14 should generate _f64, got:\n{}",
        output
    );
}

/// f32 + literal should infer literal as f32 (mixed math)
#[test]
fn test_mixed_math_f32() {
    let source = r#"
fn test() {
    let x: f32 = 1.0
    let result = x + 2.5
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("2.5_f32") || output.contains("2.5f32"),
        "2.5 in x + 2.5 (where x is f32) should infer f32, got:\n{}",
        output
    );
    assert!(
        !output.contains("2.5_f64"),
        "Should not generate f64 when other operand is f32, got:\n{}",
        output
    );
}

/// Vec3::new(1.0, 2.0, 3.0) - constructor args should be f32 when Vec3 uses f32
#[test]
fn test_vec3_constructor_f32() {
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

fn test() {
    let v = Vec3::new(1.0, 2.0, 3.0)
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("1.0_f32") && output.contains("2.0_f32") && output.contains("3.0_f32"),
        "Vec3::new(1.0, 2.0, 3.0) with f32 params should generate _f32, got:\n{}",
        output
    );
    assert!(
        !output.contains("1.0_f64") && !output.contains("2.0_f64") && !output.contains("3.0_f64"),
        "Should not generate f64 for Vec3 constructor args, got:\n{}",
        output
    );
}

/// Match on `HashMap::get` + non-float fn return: default float literal must be `f32` (library codegen).
#[test]
fn test_hashmap_get_match_default_arm_f32_non_float_return() {
    let source = r#"
use std::collections::HashMap

fn demo() -> i32 {
    let m: HashMap<(i32, i32), f32> = HashMap::new()
    let g = match m.get(&(0, 0)) {
        Some(v) => *v,
        None => 999999.0,
    }
    if g > 0.0 {
        1
    } else {
        0
    }
}

fn main() {
    let _ = demo()
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("999999.0_f32"),
        "expected f32 default arm, got:\n{}",
        output
    );
    assert!(
        !output.contains("999999.0_f64"),
        "should not use f64 for f32 map value match, got:\n{}",
        output
    );
}

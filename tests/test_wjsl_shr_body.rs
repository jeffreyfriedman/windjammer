/// TDD: WJSL body parser improvements
///
/// Bug 1: `let val: vec4<f32> = ...` fails with "Expected Assign, found Colon"
///         The body parser's `let` handling doesn't support type annotations.
///
/// Bug 2: `skip_optional_angle_bracket` doesn't handle `Shr` token for nested
///         generics like `vec4<f32>` inside angle brackets (>> lexed as Shr).
///
/// Fix: Add type annotation support to `let` handling (like `var` already has).
///       Handle `Shr` in `skip_optional_angle_bracket` and `parse_type_annotation`.

use windjammer::wjsl::transpile_wjsl;

#[test]
fn test_shift_right_in_expression() {
    let source = r#"
fn test_shift(a: u32) -> u32 {
    return a >> 2u;
}
"#;
    let result = transpile_wjsl(source);
    assert!(
        result.is_ok(),
        "shift-right should work. Error: {:?}",
        result.err()
    );
    let wgsl = result.unwrap();
    assert!(wgsl.contains(">>"), "WGSL should contain >>. Got:\n{}", wgsl);
}

#[test]
fn test_let_with_type_annotation() {
    let source = r#"
@group(0) @binding(0) storage read data: array<f32>;

fn test_body() -> f32 {
    let val: f32 = data[0u];
    return val;
}
"#;
    let result = transpile_wjsl(source);
    assert!(
        result.is_ok(),
        "let with type annotation should work. Error: {:?}",
        result.err()
    );
}

#[test]
fn test_let_with_vec_type_annotation() {
    let source = r#"
@group(0) @binding(0) storage read data: array<vec4<f32>>;

fn test_body() -> vec4<f32> {
    let val: vec4<f32> = data[0u];
    return val;
}
"#;
    let result = transpile_wjsl(source);
    assert!(
        result.is_ok(),
        "let with vec type annotation should work. Error: {:?}",
        result.err()
    );
}

#[test]
fn test_let_without_type_annotation() {
    let source = r#"
fn test_body() -> f32 {
    let val = 1.0;
    return val;
}
"#;
    let result = transpile_wjsl(source);
    assert!(
        result.is_ok(),
        "let without type annotation should still work. Error: {:?}",
        result.err()
    );
}

#[test]
fn test_shift_right_alongside_generics() {
    let source = r#"
@group(0) @binding(0) storage read data: array<vec4<f32>>;

fn test_combined(idx: u32) -> vec4<f32> {
    let shifted = idx >> 2u;
    let val = data[shifted];
    return val;
}
"#;
    let result = transpile_wjsl(source);
    assert!(
        result.is_ok(),
        ">> with generics should work. Error: {:?}",
        result.err()
    );
    let wgsl = result.unwrap();
    assert!(wgsl.contains(">>"), "WGSL should contain >>. Got:\n{}", wgsl);
}

#[test]
fn test_shift_left_works() {
    let source = r#"
fn test_shl(a: u32) -> u32 {
    return a << 2u;
}
"#;
    let result = transpile_wjsl(source);
    assert!(
        result.is_ok(),
        "shift-left should work. Error: {:?}",
        result.err()
    );
    let wgsl = result.unwrap();
    assert!(wgsl.contains("<<"), "WGSL should contain <<. Got:\n{}", wgsl);
}

#[test]
fn test_shift_right_complex_expression() {
    let source = r#"
fn hash(input: u32) -> u32 {
    var h = input;
    h = h ^ (h >> 16u);
    h = h * 0x45d9f3bu;
    h = h ^ (h >> 16u);
    return h;
}
"#;
    let result = transpile_wjsl(source);
    assert!(
        result.is_ok(),
        "complex shift-right should work. Error: {:?}",
        result.err()
    );
    let wgsl = result.unwrap();
    assert!(wgsl.contains(">>"), "WGSL should contain >>. Got:\n{}", wgsl);
}

#[test]
fn test_let_with_array_nested_generic_type() {
    let source = r#"
@group(0) @binding(0) storage read data: array<vec4<f32>>;

fn test_body() {
    let val: array<vec4<f32>> = data;
}
"#;
    let result = transpile_wjsl(source);
    assert!(
        result.is_ok(),
        "let with array<vec4<f32>> type should work. Error: {:?}",
        result.err()
    );
}

/// TDD test: WJSL parser handles nested generics like array<vec4<f32>>
///
/// Bug: The lexer combines `>>` into a single `Shr` token. When parsing
/// `array<vec4<f32>>`, the parser expects two separate `RAngle` tokens
/// but gets `Shr` instead, causing "Expected RAngle, found Shr" error.
///
/// Fix: In Parser::expect(), when expecting RAngle and finding Shr,
/// split it: consume Shr as one RAngle and set current to RAngle
/// for the outer generic's closing bracket.

#[test]
fn test_array_of_vec4_f32() {
    let source = r#"
@group(0) @binding(0) storage read data: array<vec4<f32>>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let val = data[id.x];
}
"#;
    let result = windjammer::wjsl::transpile_wjsl(source);
    assert!(
        result.is_ok(),
        "array<vec4<f32>> should parse successfully. Error: {:?}",
        result.err()
    );
    let wgsl = result.unwrap();
    assert!(
        wgsl.contains("array<vec4<f32>>"),
        "Generated WGSL should contain array<vec4<f32>>. Got:\n{}",
        wgsl
    );
}

#[test]
fn test_array_of_vec3_f32() {
    let source = r#"
@group(0) @binding(0) storage read normals: array<vec3<f32>>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let n = normals[id.x];
}
"#;
    let result = windjammer::wjsl::transpile_wjsl(source);
    assert!(
        result.is_ok(),
        "array<vec3<f32>> should parse successfully. Error: {:?}",
        result.err()
    );
}

#[test]
fn test_array_of_vec2_u32() {
    let source = r#"
@group(0) @binding(0) storage read indices: array<vec2<u32>>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = indices[id.x];
}
"#;
    let result = windjammer::wjsl::transpile_wjsl(source);
    assert!(
        result.is_ok(),
        "array<vec2<u32>> should parse successfully. Error: {:?}",
        result.err()
    );
}

#[test]
fn test_array_of_mat4x4_f32() {
    let source = r#"
@group(0) @binding(0) storage read transforms: array<mat4x4<f32>>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let t = transforms[id.x];
}
"#;
    let result = windjammer::wjsl::transpile_wjsl(source);
    assert!(
        result.is_ok(),
        "array<mat4x4<f32>> should parse successfully. Error: {:?}",
        result.err()
    );
}

#[test]
fn test_single_rangle_still_works() {
    let source = r#"
@group(0) @binding(0) storage read data: array<f32>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let val = data[id.x];
}
"#;
    let result = windjammer::wjsl::transpile_wjsl(source);
    assert!(
        result.is_ok(),
        "array<f32> should still parse correctly. Error: {:?}",
        result.err()
    );
}

#[test]
fn test_multiple_nested_generics_in_struct() {
    let source = r#"
struct MyData {
    positions: array<vec4<f32>>,
    normals: array<vec3<f32>>,
    indices: array<vec2<u32>>,
}

@group(0) @binding(0) storage read data: MyData;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let pos = data.positions[id.x];
}
"#;
    let result = windjammer::wjsl::transpile_wjsl(source);
    assert!(
        result.is_ok(),
        "Struct with multiple nested generics should parse. Error: {:?}",
        result.err()
    );
}

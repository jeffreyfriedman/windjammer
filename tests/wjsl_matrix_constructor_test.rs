/// TDD: mat3x3/mat4x4/mat2x2 multi-argument constructor support in WJSL type checker
///
/// Bug: The WJSL BodyParser only parses ONE argument for matrix constructors
/// (mat3x3, mat4x4, mat2x2), then expects ')'. But WGSL matrix constructors
/// take multiple column vectors: mat3x3(col0, col1, col2). The parser sees
/// the comma after the first argument and errors with "Expected RParen, found Comma".

fn transpile(source: &str) -> Result<String, String> {
    windjammer::wjsl::transpile_wjsl(source).map_err(|e| e.to_string())
}

#[test]
fn test_mat3x3_three_vec3_args() {
    let source = r#"
fn identity3() -> mat3x3<f32> {
    return mat3x3(vec3(1.0, 0.0, 0.0), vec3(0.0, 1.0, 0.0), vec3(0.0, 0.0, 1.0));
}

@compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let m = identity3();
}
"#;
    let result = transpile(source);
    assert!(
        result.is_ok(),
        "mat3x3 constructor with 3 vec3 arguments must parse: {}",
        result.unwrap_err()
    );
}

#[test]
fn test_mat4x4_four_vec4_args() {
    let source = r#"
fn identity4() -> mat4x4<f32> {
    return mat4x4(vec4(1.0, 0.0, 0.0, 0.0), vec4(0.0, 1.0, 0.0, 0.0), vec4(0.0, 0.0, 1.0, 0.0), vec4(0.0, 0.0, 0.0, 1.0));
}

@compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let m = identity4();
}
"#;
    let result = transpile(source);
    assert!(
        result.is_ok(),
        "mat4x4 constructor with 4 vec4 arguments must parse: {}",
        result.unwrap_err()
    );
}

#[test]
fn test_mat2x2_two_vec2_args() {
    let source = r#"
fn identity2() -> mat2x2<f32> {
    return mat2x2(vec2(1.0, 0.0), vec2(0.0, 1.0));
}

@compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let m = identity2();
}
"#;
    let result = transpile(source);
    assert!(
        result.is_ok(),
        "mat2x2 constructor with 2 vec2 arguments must parse: {}",
        result.unwrap_err()
    );
}

#[test]
fn test_mat3x3_in_variable_binding() {
    let source = r#"
fn make_rotation() -> mat3x3<f32> {
    let col0 = vec3(1.0, 0.0, 0.0);
    let col1 = vec3(0.0, 1.0, 0.0);
    let col2 = vec3(0.0, 0.0, 1.0);
    return mat3x3(col0, col1, col2);
}

@compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let m = make_rotation();
}
"#;
    let result = transpile(source);
    assert!(
        result.is_ok(),
        "mat3x3 constructor with variable arguments must parse: {}",
        result.unwrap_err()
    );
}

#[test]
fn test_mat3x3_used_in_multiplication() {
    let source = r#"
@compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let m = mat3x3(vec3(1.0, 0.0, 0.0), vec3(0.0, 1.0, 0.0), vec3(0.0, 0.0, 1.0));
    let v = vec3(1.0, 2.0, 3.0);
    let result = m * v;
}
"#;
    let result = transpile(source);
    assert!(
        result.is_ok(),
        "mat3x3 constructor used in multiplication must work: {}",
        result.unwrap_err()
    );
}

/// TDD: Test real shader files that are failing transpilation
///
/// Bug: Multiple shaders fail with WJSL syntax/type errors:
/// - voxel_raymarch.wjsl: "Unknown identifier 'voxel'"
/// - voxel_lighting.wjsl: "Expected semicolon, found FloatLiteral(25.0)"
/// - point_light/area_light.wjsl: "Invalid operands for *: mat4x4 and mat4x4"

#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "parser_tests",
))]

fn transpile(source: &str) -> Result<String, String> {
    windjammer::wjsl::transpile_wjsl(source).map_err(|e| e.to_string())
}

#[test]
fn test_voxel_raymarch_let_mut_pattern() {
    // Minimal repro of voxel_raymarch.wjsl line 206
    let source = r#"
@fragment
fn main() {
    let pos = vec3(0.0);
    let inv_vs = 1.0;
    let vs = 1.0;
    
    let mut voxel = floor(pos * inv_vs) * vs;
    voxel = voxel + vec3(1.0);
}
"#;
    let result = transpile(source);
    assert!(result.is_ok(), 
        "voxel_raymarch pattern should transpile: {:?}", result.err());
    let wgsl = result.unwrap();
    assert!(wgsl.contains("var voxel"), 
        "Should convert 'let mut voxel' to 'var voxel': {}", wgsl);
}

#[test]
fn test_voxel_lighting_semicolon_issue() {
    // Minimal repro of voxel_lighting.wjsl line 43
    // "Expected semicolon, found FloatLiteral(25.0)"
    let source = r#"
@fragment
fn main() {
    let radius = 25.0;
    let x = radius * 2.0;
}
"#;
    let result = transpile(source);
    assert!(result.is_ok(), 
        "simple float literal assignment should transpile: {:?}", result.err());
}

#[test]
fn test_mat4_multiply() {
    // point_light.wjsl and area_light.wjsl: "Invalid operands for *: mat4x4 and mat4x4"
    let source = r#"
@fragment
fn main() {
    let view = mat4x4<f32>(
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0
    );
    let proj = mat4x4<f32>(
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0
    );
    let vp = proj * view;
}
"#;
    let result = transpile(source);
    assert!(result.is_ok(), 
        "mat4x4 * mat4x4 should transpile: {:?}", result.err());
}

#[test]
fn test_ssgi_rparen_issue() {
    // ssgi.wjsl and ssr.wjsl: "Unexpected token in expression: RParen"
    // This usually means function call with trailing comma or empty parens
    let source = r#"
@fragment
fn main() {
    let x = some_func();
    let y = other_func(1.0, 2.0);
}

fn some_func() -> f32 {
    return 1.0;
}

fn other_func(a: f32, b: f32) -> f32 {
    return a + b;
}
"#;
    let result = transpile(source);
    assert!(result.is_ok(), 
        "function calls with empty/normal args should transpile: {:?}", result.err());
}

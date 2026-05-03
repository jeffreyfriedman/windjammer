//! TDD tests for WJSL → WGSL codegen
//!
//! Tests the transpiler that converts WJSL AST to WGSL source code.

use windjammer::wjsl::transpile_wjsl;

#[test]
fn test_transpile_vertex_shader() {
    let source = r#"
@vertex
fn main(@builtin(vertex_index) idx: u32) -> @builtin(position) vec4 {
    return vec4(0.0, 0.0, 0.0, 1.0);
}
"#;
    let wgsl = transpile_wjsl(source).unwrap();
    assert!(wgsl.contains("@vertex"));
    assert!(wgsl.contains("@builtin(vertex_index)"));
    assert!(wgsl.contains("@builtin(position)"));
    assert!(wgsl.contains("vec4<f32>"));
    assert!(wgsl.contains("return vec4(0.0, 0.0, 0.0, 1.0)"));
}

#[test]
fn test_transpile_uniforms() {
    let source = r#"
struct CameraUniforms {
    view_matrix: mat4x4,
    proj_matrix: mat4x4,
}

@group(0) @binding(0) uniform camera: CameraUniforms;
@group(0) @binding(1) uniform params: vec2<f32>;

@vertex
fn main(@builtin(vertex_index) idx: u32) -> @builtin(position) vec4 {
    return vec4(0.0, 0.0, 0.0, 1.0);
}
"#;
    let wgsl = transpile_wjsl(source).unwrap();
    assert!(wgsl.contains("struct CameraUniforms"));
    assert!(wgsl.contains("view_matrix"));
    assert!(wgsl.contains("mat4x4<f32>"));
    assert!(wgsl.contains("@group(0) @binding(0) var<uniform> camera"));
    assert!(wgsl.contains("@group(0) @binding(1) var<uniform> params"));
    assert!(wgsl.contains("vec2<f32>"));
}

#[test]
fn test_transpile_compute_shader() {
    let source = r#"
@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= 128u || id.y >= 128u) { return; }
}
"#;
    let wgsl = transpile_wjsl(source).unwrap();
    assert!(wgsl.contains("@compute"));
    assert!(wgsl.contains("@workgroup_size(8, 8, 1)"));
    assert!(wgsl.contains("@builtin(global_invocation_id)"));
    assert!(wgsl.contains("vec3<u32>"));
}

#[test]
fn test_transpile_storage_bindings() {
    let source = r#"
@group(0) @binding(0) storage read svo_nodes: array<u32>;
@group(0) @binding(1) storage read_write gbuffer: array<vec4>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    return;
}
"#;
    let wgsl = transpile_wjsl(source).unwrap();
    assert!(wgsl.contains("var<storage, read> svo_nodes"));
    assert!(wgsl.contains("var<storage, read_write> gbuffer"));
    assert!(wgsl.contains("array<u32>"));
    assert!(wgsl.contains("array<vec4<f32>>"));
}

#[test]
fn test_transpile_texture_and_sampler() {
    let source = r#"
@group(0) @binding(0) texture_2d albedo_map: texture_2d<f32>;
@group(0) @binding(1) sampler tex_sampler: sampler;

@fragment
fn main(@location(0) uv: vec2) -> @location(0) vec4 {
    return vec4(0.0, 0.0, 0.0, 1.0);
}
"#;
    let wgsl = transpile_wjsl(source).unwrap();
    assert!(wgsl.contains("var albedo_map: texture_2d<f32>"));
    assert!(wgsl.contains("var tex_sampler: sampler"));
    assert!(wgsl.contains("@fragment"));
    assert!(wgsl.contains("@location(0)"));
}

#[test]
fn test_transpile_helper_functions() {
    let source = r#"
fn tonemap_aces(color: vec3) -> vec3 {
    let a = 2.51;
    return clamp(color * a, vec3(0.0), vec3(1.0));
}

@fragment
fn main(@location(0) color: vec4) -> @location(0) vec4 {
    let lit = tonemap_aces(color.rgb);
    return vec4(lit, 1.0);
}
"#;
    let wgsl = transpile_wjsl(source).unwrap();
    assert!(wgsl.contains("fn tonemap_aces"));
    assert!(wgsl.contains("color: vec3<f32>"));
    assert!(wgsl.contains("-> vec3<f32>"));
    assert!(wgsl.contains("tonemap_aces(color.rgb)"));
}

//! TDD tests for WJSL (Windjammer Shader Language) parser
//!
//! Tests the RFC syntax: @vertex, @fragment, @compute, @group, @binding, etc.

use windjammer::wjsl::{parse_wjsl, BindingKind, ShaderStage, StorageAccess};


#[test]
fn test_parse_vertex_shader() {
    let source = r#"
@vertex
fn main(@builtin(vertex_index) idx: u32) -> @builtin(position) vec4 {
    let x = f32((idx << 1u) & 2u) - 1.0;
    let y = f32(idx & 2u) - 1.0;
    return vec4(x, y, 0.0, 1.0);
}
"#;
    let ast = parse_wjsl(source).unwrap();
    assert!(!ast.entry_points.is_empty(), "Expected at least one entry point");
    assert!(
        matches!(ast.entry_points[0].stage, ShaderStage::Vertex),
        "Expected vertex stage, got {:?}",
        ast.entry_points[0].stage
    );
    assert_eq!(ast.entry_points[0].name, "main");
}

#[test]
fn test_parse_fragment_shader() {
    let source = r#"
@fragment
fn main(@location(0) color: vec4) -> @location(0) vec4 {
    return color;
}
"#;
    let ast = parse_wjsl(source).unwrap();
    assert!(!ast.entry_points.is_empty());
    assert!(
        matches!(ast.entry_points[0].stage, ShaderStage::Fragment),
        "Expected fragment stage"
    );
}

#[test]
fn test_parse_compute_shader() {
    let source = r#"
@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= 128u || id.y >= 128u) { return; }
    let idx = id.y * 128u + id.x;
    output[idx] = vec4(1.0, 0.0, 0.0, 1.0);
}
"#;
    let ast = parse_wjsl(source).unwrap();
    assert!(!ast.entry_points.is_empty());
    assert!(
        matches!(ast.entry_points[0].stage, ShaderStage::Compute),
        "Expected compute stage"
    );
    let workgroup = ast.entry_points[0].workgroup_size.unwrap();
    assert_eq!(workgroup, (8, 8, 1));
}

#[test]
fn test_parse_uniforms() {
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
    let ast = parse_wjsl(source).unwrap();
    assert_eq!(ast.bindings.len(), 2, "Expected 2 bindings");

    let camera_binding = &ast.bindings[0];
    assert_eq!(camera_binding.name, "camera");
    assert_eq!(camera_binding.group, 0);
    assert_eq!(camera_binding.binding, 0);
    assert!(matches!(camera_binding.kind, windjammer::wjsl::BindingKind::Uniform(_)));

    let params_binding = &ast.bindings[1];
    assert_eq!(params_binding.name, "params");
    assert_eq!(params_binding.group, 0);
    assert_eq!(params_binding.binding, 1);
}

#[test]
fn test_parse_storage_bindings() {
    let source = r#"
@group(0) @binding(0) storage read svo_nodes: array<u32>;
@group(0) @binding(1) storage read_write gbuffer: array<vec4>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    return;
}
"#;
    let ast = parse_wjsl(source).unwrap();
    assert_eq!(ast.bindings.len(), 2);

    let read_binding = &ast.bindings[0];
    assert_eq!(read_binding.name, "svo_nodes");
    assert!(matches!(
        read_binding.kind,
        windjammer::wjsl::BindingKind::Storage { access_mode, .. } if access_mode == windjammer::wjsl::StorageAccess::Read
    ));

    let rw_binding = &ast.bindings[1];
    assert_eq!(rw_binding.name, "gbuffer");
    assert!(matches!(
        rw_binding.kind,
        windjammer::wjsl::BindingKind::Storage { access_mode, .. } if access_mode == windjammer::wjsl::StorageAccess::ReadWrite
    ));
}

#[test]
fn test_parse_struct_declarations() {
    let source = r#"
struct CameraUniforms {
    view_matrix: mat4x4,
    proj_matrix: mat4x4,
    position: vec3,
    screen_size: vec2,
}

struct Material {
    base_color: vec4,
    metallic: f32,
    roughness: f32,
}

@vertex
fn main(@builtin(vertex_index) idx: u32) -> @builtin(position) vec4 {
    return vec4(0.0, 0.0, 0.0, 1.0);
}
"#;
    let ast = parse_wjsl(source).unwrap();
    assert_eq!(ast.structs.len(), 2, "Expected 2 structs");

    assert_eq!(ast.structs[0].name, "CameraUniforms");
    assert_eq!(ast.structs[0].fields.len(), 4);
    assert_eq!(ast.structs[0].fields[0].name, "view_matrix");
    assert_eq!(ast.structs[0].fields[2].name, "position");

    assert_eq!(ast.structs[1].name, "Material");
    assert_eq!(ast.structs[1].fields.len(), 3);
    assert_eq!(ast.structs[1].fields[1].name, "metallic");
}

#[test]
fn test_parse_helper_functions() {
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
    let ast = parse_wjsl(source).unwrap();
    assert_eq!(ast.functions.len(), 1, "Expected 1 helper function");
    assert_eq!(ast.functions[0].name, "tonemap_aces");
    assert_eq!(ast.functions[0].params.len(), 1);
    assert_eq!(ast.functions[0].params[0].name, "color");

    assert_eq!(ast.entry_points.len(), 1);
    assert_eq!(ast.entry_points[0].name, "main");
}

#[test]
fn test_parse_texture_and_sampler_bindings() {
    let source = r#"
@group(0) @binding(0) texture_2d albedo_map: texture_2d<f32>;
@group(0) @binding(1) sampler tex_sampler: sampler;

@fragment
fn main(@location(0) uv: vec2) -> @location(0) vec4 {
    return vec4(0.0, 0.0, 0.0, 1.0);
}
"#;
    let ast = parse_wjsl(source).unwrap();
    assert_eq!(ast.bindings.len(), 2);

    let tex_binding = &ast.bindings[0];
    assert_eq!(tex_binding.name, "albedo_map");
    assert!(matches!(
        tex_binding.kind,
        windjammer::wjsl::BindingKind::Texture { .. }
    ));

    let sampler_binding = &ast.bindings[1];
    assert_eq!(sampler_binding.name, "tex_sampler");
    assert!(matches!(
        sampler_binding.kind,
        windjammer::wjsl::BindingKind::Sampler
    ));
}

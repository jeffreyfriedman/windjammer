//! G-Buffer Shader for Deferred Rendering and SSGI
//!
//! This shader renders scene geometry to multiple render targets (G-buffer):
//! - Position (world space)
//! - Normal (world space)
//! - Albedo (base color)
//! - Material properties (roughness, metallic, etc.)
//!
//! The G-buffer is used for:
//! - Deferred shading
//! - Screen-space global illumination (SSGI)
//! - Screen-space ambient occlusion (SSAO)
//! - Post-processing effects

struct CameraUniform {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
    view_projection: mat4x4<f32>,
    position: vec3<f32>,
    _padding: f32,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coords: vec2<f32>,
    @location(3) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) tex_coords: vec2<f32>,
    @location(3) color: vec4<f32>,
    @location(4) view_position: vec3<f32>,
}

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    // Transform position to clip space
    let world_pos = vec4<f32>(vertex.position, 1.0);
    let view_pos = camera.view * world_pos;
    out.clip_position = camera.projection * view_pos;
    
    // Pass world space position and normal
    out.world_position = vertex.position;
    out.world_normal = normalize(vertex.normal);
    out.tex_coords = vertex.tex_coords;
    out.color = vertex.color;
    out.view_position = view_pos.xyz;
    
    return out;
}

// G-Buffer output structure
struct GBufferOutput {
    @location(0) position: vec4<f32>,  // RGB: position, A: depth
    @location(1) normal: vec4<f32>,    // RGB: normal, A: unused
    @location(2) albedo: vec4<f32>,    // RGB: albedo, A: alpha
    @location(3) material: vec4<f32>,  // R: roughness, G: metallic, B: ao, A: unused
}

@fragment
fn fs_main(in: VertexOutput) -> GBufferOutput {
    var output: GBufferOutput;
    
    // Position (world space) + depth
    let depth = length(in.view_position);
    output.position = vec4<f32>(in.world_position, depth);
    
    // Normal (world space)
    output.normal = vec4<f32>(normalize(in.world_normal), 1.0);
    
    // Albedo (base color)
    output.albedo = in.color;
    
    // Material properties (default values for now)
    // R: roughness (0.5 = medium roughness)
    // G: metallic (0.0 = non-metallic)
    // B: ambient occlusion (1.0 = no occlusion)
    output.material = vec4<f32>(0.5, 0.0, 1.0, 1.0);
    
    return output;
}

// Simplified version for objects that don't need full G-buffer
@fragment
fn fs_simple(in: VertexOutput) -> @location(0) vec4<f32> {
    // Simple directional lighting
    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.3));
    let ambient = 0.3;
    let diffuse = max(dot(in.world_normal, light_dir), 0.0) * 0.7;
    
    let lighting = ambient + diffuse;
    let final_color = in.color.rgb * lighting;
    
    return vec4<f32>(final_color, in.color.a);
}


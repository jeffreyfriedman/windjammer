//! Simple 3D Shader for Greybox Games
//!
//! Uses vertex colors and simple lighting for fast, clean rendering.

struct CameraUniform {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
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
    @location(2) color: vec4<f32>,
}

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    // Transform position to clip space
    let view_pos = camera.view * vec4<f32>(vertex.position, 1.0);
    out.clip_position = camera.projection * view_pos;
    
    // Pass world space position and normal
    out.world_position = vertex.position;
    out.world_normal = normalize(vertex.normal);
    out.color = vertex.color;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Simple directional lighting
    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.3));
    let ambient = 0.3;
    let diffuse = max(dot(in.world_normal, light_dir), 0.0) * 0.7;
    
    let lighting = ambient + diffuse;
    let final_color = in.color.rgb * lighting;
    
    return vec4<f32>(final_color, in.color.a);
}


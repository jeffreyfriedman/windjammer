// Composite Shader
// Combines direct lighting with SSGI for final output

struct CameraUniform {
    view_proj: mat4x4<f32>,
    inverse_view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _padding: f32,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(0) @binding(1)
var gbuffer_albedo: texture_2d<f32>;
@group(0) @binding(2)
var gbuffer_sampler: sampler;

@group(0) @binding(3)
var ssgi_texture: texture_2d<f32>;
@group(0) @binding(4)
var ssgi_sampler: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(vertex.position, 0.0, 1.0);
    out.tex_coords = vertex.tex_coords;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample albedo (base color)
    let albedo = textureSample(gbuffer_albedo, gbuffer_sampler, in.tex_coords).rgb;
    
    // Sample SSGI (indirect lighting)
    let gi = textureSample(ssgi_texture, ssgi_sampler, in.tex_coords).rgb;
    
    // Simple directional light (direct lighting)
    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.3));
    let ambient = 0.3;
    let direct_light = ambient + 0.7; // Simplified for now
    
    // Combine direct and indirect lighting
    let final_color = albedo * (direct_light + gi);
    
    return vec4<f32>(final_color, 1.0);
}


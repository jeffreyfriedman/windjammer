// Shadow Map Generation Shader
// Renders depth from light's perspective for shadow mapping

struct Camera {
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> light_view_proj: Camera;

struct VertexInput {
    @location(0) position: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
}

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    // Transform position to light clip space
    out.clip_position = light_view_proj.view_proj * vec4<f32>(vertex.position, 1.0);
    
    return out;
}

// No fragment shader needed - depth is written automatically
// But we need a dummy fragment shader for wgpu
@fragment
fn fs_main(in: VertexOutput) {
    // Depth is written automatically to the depth buffer
    // This fragment shader is just a placeholder
}



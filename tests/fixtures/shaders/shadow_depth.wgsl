// Shadow Depth Pass - Vertex-only shader for shadow map generation
//
// Renders geometry into a depth-only buffer from the light's perspective.
// Used by each cascade in the cascaded shadow map system.

struct ShadowUniforms {
    light_view_proj: mat4x4<f32>,
}

struct ModelUniforms {
    model_matrix: mat4x4<f32>,
    normal_matrix: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> shadow: ShadowUniforms;
@group(0) @binding(1) var<uniform> model: ModelUniforms;

@vertex
fn vs_main(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    let world_pos = model.model_matrix * vec4<f32>(position, 1.0);
    return shadow.light_view_proj * world_pos;
}

@fragment
fn fs_main() {}

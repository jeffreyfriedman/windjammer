struct Uniforms {
    screen_size: vec2<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read_write> output: array<vec4<f32>>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let width = u32(uniforms.screen_size.x);
    let height = u32(uniforms.screen_size.y);
    if (id.x >= width || id.y >= height) {
        return;
    }
    // TODO: Add shader body from .wjsl functions
}

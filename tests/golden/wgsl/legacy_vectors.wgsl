struct Uniforms {
    a: vec2<f32>,
    b: vec3<u32>,
    c: vec4<i32>,
    m: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    // TODO: Add shader body from .wjsl functions
}

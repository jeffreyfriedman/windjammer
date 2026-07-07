// Debug Normals Shader - World-space normal visualization
//
// Reads GBuffer normals, outputs RGB (normal * 0.5 + 0.5)
// Workgroup: 8x8 threads (64 total)

struct GBufferPixel {
    position: vec3<f32>,
    _pad1: f32,
    normal: vec3<f32>,
    material_id: f32,
    depth: f32,
    geometry_source: f32,
    _pad2: vec2<f32>,
}

@group(0) @binding(0) var<storage, read> gbuffer: array<GBufferPixel>;
@group(0) @binding(1) var<storage, read_write> color_output: array<vec4<f32>>;
@group(0) @binding(2) var<uniform> screen_size: vec2<u32>;

// World-space normal -> RGB (map -1..1 to 0..1)
fn normal_to_rgb(n: vec3<f32>) -> vec3<f32> {
    return n * 0.5 + 0.5;
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let width = screen_size.x;
    let height = screen_size.y;
    if (id.x >= width || id.y >= height) { return; }

    let pixel_idx = id.y * width + id.x;
    let gb = gbuffer[pixel_idx];
    let n = normalize(gb.normal);
    let rgb = normal_to_rgb(n);
    color_output[pixel_idx] = vec4<f32>(rgb, 1.0);
}

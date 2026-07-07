// Debug Depth Shader - Linear depth visualization
//
// Reads GBuffer depth, outputs grayscale (white=near, black=far)
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

struct DepthParams {
    near: f32,
    far: f32,
    _pad: vec2<f32>,
}

@group(0) @binding(0) var<storage, read> gbuffer: array<GBufferPixel>;
@group(0) @binding(1) var<storage, read_write> color_output: array<vec4<f32>>;
@group(0) @binding(2) var<uniform> params: DepthParams;
@group(0) @binding(3) var<uniform> screen_size: vec2<u32>;

// Linear depth -> grayscale (0=near/white, 1=far/black)
fn depth_to_grayscale(depth: f32, near: f32, far: f32) -> f32 {
    let t = (depth - near) / (far - near);
    return 1.0 - clamp(t, 0.0, 1.0);
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let width = screen_size.x;
    let height = screen_size.y;
    if (id.x >= width || id.y >= height) { return; }

    let pixel_idx = id.y * width + id.x;
    let depth = gbuffer[pixel_idx].depth;
    let gray = depth_to_grayscale(depth, params.near, params.far);
    color_output[pixel_idx] = vec4<f32>(gray, gray, gray, 1.0);
}

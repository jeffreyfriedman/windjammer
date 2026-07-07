// Debug UVs Shader - Texture coordinate visualization
//
// Reads GBuffer UVs (if available) or uses screen UV. For GBuffer without UVs,
// we use a placeholder - in full pipeline this would read from a separate UV buffer.
// Workgroup: 8x8 threads (64 total)
//
// Note: Standard GBuffer may not have UVs. This shader uses screen-space UV
// as fallback for debug visualization when UV buffer is not bound.

struct DebugUVParams {
    use_screen_uv: f32,  // 1 = use screen UV, 0 = use gbuffer (if available)
    _pad: vec3<f32>,
}

@group(0) @binding(0) var<storage, read> uv_input: array<vec4<f32>>;  // vec4(uv.x, uv.y, 0, 0) per pixel
@group(0) @binding(1) var<storage, read_write> color_output: array<vec4<f32>>;
@group(0) @binding(2) var<uniform> screen_size: vec2<u32>;

// UV -> RGB (u=R, v=G, 0=B)
fn uv_to_rgb(uv: vec2<f32>) -> vec3<f32> {
    return vec3<f32>(uv.x, uv.y, 0.0);
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let width = screen_size.x;
    let height = screen_size.y;
    if (id.x >= width || id.y >= height) { return; }

    let pixel_idx = id.y * width + id.x;
    let uv = uv_input[pixel_idx].xy;
    let rgb = uv_to_rgb(uv);
    color_output[pixel_idx] = vec4<f32>(rgb, 1.0);
}

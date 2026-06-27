// Debug Wireframe Shader - Edge detection overlay
//
// Detects edges via depth/normal discontinuities, draws dark lines.
// Reads GBuffer, outputs wireframe overlay (additive over base color).
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

struct WireframeParams {
    edge_threshold: f32,   // Depth difference threshold
    normal_threshold: f32,  // Normal dot product threshold
    line_color: vec3<f32>,
    _pad: f32,
}

@group(0) @binding(0) var<storage, read> gbuffer: array<GBufferPixel>;
@group(0) @binding(1) var<storage, read> base_color: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read_write> color_output: array<vec4<f32>>;
@group(0) @binding(3) var<uniform> params: WireframeParams;
@group(0) @binding(4) var<uniform> screen_size: vec2<u32>;

fn get_pixel_safe(gb: array<GBufferPixel>, idx: u32, max_idx: u32) -> GBufferPixel {
    if (idx >= max_idx) {
        return GBufferPixel(vec3<f32>(0.0), 0.0, vec3<f32>(0.0, 1.0, 0.0), 0.0, 0.0, 0.0, vec2<f32>(0.0));
    }
    return gbuffer[idx];
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let width = screen_size.x;
    let height = screen_size.y;
    if (id.x >= width || id.y >= height) { return; }

    let pixel_idx = id.y * width + id.x;
    let max_idx = width * height;

    let center = gbuffer[pixel_idx];
    let depth_diff_thresh = params.edge_threshold;
    let normal_thresh = params.normal_threshold;

    // Sample neighbors (left, right, up, down) - clamp to avoid OOB
    let left_idx = select(pixel_idx, pixel_idx - 1u, id.x != 0u);
    let right_idx = select(pixel_idx, pixel_idx + 1u, id.x < width - 1u);
    let up_idx = select(pixel_idx, pixel_idx - width, id.y != 0u);
    let down_idx = select(pixel_idx, pixel_idx + width, id.y < height - 1u);

    let left = get_pixel_safe(gbuffer, left_idx, max_idx);
    let right = get_pixel_safe(gbuffer, right_idx, max_idx);
    let up = get_pixel_safe(gbuffer, up_idx, max_idx);
    let down = get_pixel_safe(gbuffer, down_idx, max_idx);

    // Edge = depth discontinuity OR normal discontinuity
    let depth_edge_lr = abs(center.depth - left.depth) > depth_diff_thresh || abs(center.depth - right.depth) > depth_diff_thresh;
    let depth_edge_ud = abs(center.depth - up.depth) > depth_diff_thresh || abs(center.depth - down.depth) > depth_diff_thresh;

    let n = normalize(center.normal);
    let nl = dot(n, normalize(left.normal));
    let nr = dot(n, normalize(right.normal));
    let nu = dot(n, normalize(up.normal));
    let nd = dot(n, normalize(down.normal));

    let normal_edge = nl < normal_thresh || nr < normal_thresh || nu < normal_thresh || nd < normal_thresh;

    let is_edge = depth_edge_lr || depth_edge_ud || normal_edge;

    let base = base_color[pixel_idx].rgb;
    let wire_color = select(params.line_color, base, !is_edge);  // select(f,t,c): t if c else f
    color_output[pixel_idx] = vec4<f32>(wire_color, 1.0);
}

// Bilateral Filter for Voxel Denoising
//
// High-quality spatial denoising that preserves edges.
// Uses Gaussian spatial + range weights for edge-aware smoothing.
// Workgroup: 8x8 threads
//
// Parameter tuning (validated by denoise_quality_test.rs):
// - radius: 2=5x5 (~10-15ms 1080p), 3=7x7 (~20ms). Larger = more smoothing, slower.
// - spatial_sigma: 1.5-2.0. Controls spatial falloff (Gaussian width).
// - range_sigma: 0.3=strong edge preservation, 0.6=more smoothing for noise.
//   For 50% variance reduction on ±0.1 noise: use 0.6 + two-pass.

struct BilateralParams {
    radius: u32,
    spatial_sigma: f32,
    range_sigma: f32,
    width: u32,
    height: u32,
}

@group(0) @binding(0) var<storage, read> input_buffer: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read_write> output_buffer: array<vec4<f32>>;
@group(0) @binding(2) var<uniform> params: BilateralParams;

fn bilateral_filter(pos: vec2<u32>) -> vec4<f32> {
    let center_color = input_buffer[pos.y * params.width + pos.x];
    var sum = vec4<f32>(0.0);
    var weight_sum = 0.0;

    let radius_i = i32(params.radius);
    for (var dy = -radius_i; dy <= radius_i; dy++) {
        for (var dx = -radius_i; dx <= radius_i; dx++) {
            let sx = i32(pos.x) + dx;
            let sy = i32(pos.y) + dy;

            if (sx < 0 || sy < 0 || sx >= i32(params.width) || sy >= i32(params.height)) {
                continue;
            }

            let sample_idx = u32(sy) * params.width + u32(sx);
            let sample_color = input_buffer[sample_idx];

            // Spatial weight (Gaussian based on distance)
            let spatial_dist = length(vec2<f32>(f32(dx), f32(dy)));
            let spatial_weight = exp(-spatial_dist * spatial_dist / (2.0 * params.spatial_sigma * params.spatial_sigma));

            // Range weight (Gaussian based on color difference)
            let color_diff = length(sample_color.rgb - center_color.rgb);
            let range_weight = exp(-color_diff * color_diff / (2.0 * params.range_sigma * params.range_sigma + 0.0001));

            let weight = spatial_weight * range_weight;
            sum += sample_color * weight;
            weight_sum += weight;
        }
    }

    if (weight_sum > 0.0001) {
        return sum / weight_sum;
    }
    return center_color;
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= params.width || id.y >= params.height) {
        return;
    }

    let denoised = bilateral_filter(id.xy);
    let idx = id.y * params.width + id.x;
    output_buffer[idx] = denoised;
}

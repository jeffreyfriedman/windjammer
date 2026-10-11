struct DenoiseParams {
    alpha: f32,
    sigma_color: f32,
    sigma_normal: f32,
    frame_count: u32,
}

struct GBufferPixel {
    position: vec4<f32>,
    normal: vec4<f32>,
    depth: f32,
    geometry_source: f32,
    _pad: vec2<f32>,
}

@group(0) @binding(0) var<uniform> params: DenoiseParams;
@group(0) @binding(1) var<storage, read> current_color: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read_write> history_color: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read_write> output_color: array<vec4<f32>>;
@group(0) @binding(4) var<storage, read> gbuffer: array<GBufferPixel>;
@group(0) @binding(5) var<uniform> screen_size: vec2<u32>;
fn gbuffer_base(pixel_idx: u32) -> u32 {
    return pixel_idx * 12u;
}

fn gbuf_position(g: GBufferPixel) -> vec3<f32> {
    return g.position.xyz;
}

fn gbuf_normal(g: GBufferPixel) -> vec3<f32> {
    return g.normal.xyz;
}

fn gbuf_material_id(g: GBufferPixel) -> u32 {
    return u32(g.normal.w);
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    // 5x5 a-trous wavelet kernel weights (local var for dynamic indexing)
        var kernel: array<f32, 25>;
        kernel[0]  = 1.0/256.0; kernel[1]  = 4.0/256.0; kernel[2]  = 6.0/256.0; kernel[3]  = 4.0/256.0; kernel[4]  = 1.0/256.0;
        kernel[5]  = 4.0/256.0; kernel[6]  = 16.0/256.0; kernel[7] = 24.0/256.0; kernel[8]  = 16.0/256.0; kernel[9]  = 4.0/256.0;
        kernel[10] = 6.0/256.0; kernel[11] = 24.0/256.0; kernel[12]= 36.0/256.0; kernel[13] = 24.0/256.0; kernel[14] = 6.0/256.0;
        kernel[15] = 4.0/256.0; kernel[16] = 16.0/256.0; kernel[17]= 24.0/256.0; kernel[18] = 16.0/256.0; kernel[19] = 4.0/256.0;
        kernel[20] = 1.0/256.0; kernel[21] = 4.0/256.0; kernel[22]= 6.0/256.0; kernel[23] = 4.0/256.0; kernel[24] = 1.0/256.0;
        let width = screen_size.x;
        let height = screen_size.y;
        if (id.x >= width || id.y >= height) { return; }
    
        let pixel_idx = id.y * width + id.x;
        let current = current_color[pixel_idx];
        let center_gbuf = gbuffer[pixel_idx];
        let center_normal = center_gbuf.normal.xyz;
        let center_depth = center_gbuf.depth;
    
        // Edge-aware spatial filter (a-trous wavelet)
        var filtered = vec3(0.0);
        var weight_sum = 0.0;
    
        for (var dy = -2i; dy <= 2i; dy++) {
            for (var dx = -2i; dx <= 2i; dx++) {
                let sx = i32(id.x) + dx;
                let sy = i32(id.y) + dy;
    
                if (sx < 0 || sx >= i32(width) || sy < 0 || sy >= i32(height)) {
                    continue;
                }
    
                let sample_idx = u32(sy) * width + u32(sx);
                let sample_color = current_color[sample_idx];
                let sample_gbuf = gbuffer[sample_idx];
    
                // Kernel weight
                let ki = (dy + 2) * 5 + (dx + 2);
                var w = kernel[ki];
    
                // Color edge weight
                let color_diff = length(current.rgb - sample_color.rgb);
                w *= exp(-color_diff * color_diff / (params.sigma_color * params.sigma_color + 0.0001));
    
                // Normal edge weight
                let normal_dot = max(dot(center_normal, sample_gbuf.normal.xyz), 0.0);
                w *= pow(normal_dot, params.sigma_normal);
    
                // Depth edge weight
                let depth_diff = abs(center_depth - sample_gbuf.depth);
                w *= exp(-depth_diff * 0.1);
    
                filtered += sample_color.rgb * w;
                weight_sum += w;
            }
        }
    
        if (weight_sum > 0.0) {
            filtered = filtered / weight_sum;
        } else {
            filtered = current.rgb;
        }
    
        // Spatial-only output: no temporal blending.
        // Without motion vectors, temporal accumulation causes ghosting and instability
        // (walls appear to shift/disappear as camera moves). The spatial a-trous wavelet
        // filter above already smooths GI noise adequately.
        output_color[pixel_idx] = vec4(filtered, 1.0);
        history_color[pixel_idx] = vec4(filtered, center_depth);
}


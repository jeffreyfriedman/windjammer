struct GBufferPixel {
    position: vec4<f32>,
    normal: vec4<f32>,
    depth: f32,
    geometry_source: f32,
    _pad: vec2<f32>,
}

struct DOFUniforms {
    focus_distance: f32,
    focus_range: f32,
    max_blur: f32,
    aperture: f32,
}

@group(0) @binding(0) var<uniform> params: DOFUniforms;
@group(0) @binding(1) var<storage, read> color_input: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read> gbuffer: array<f32>;
@group(0) @binding(3) var<storage, read_write> dof_output: array<vec4<f32>>;
@group(0) @binding(4) var<uniform> screen_size: vec2<u32>;
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

fn circle_of_confusion(depth: f32) -> f32 {
    let dist_from_focus = abs(depth - params.focus_distance);
        let coc = clamp(dist_from_focus / max(params.focus_range, 0.001), 0.0, 1.0);
        return coc * params.max_blur * params.aperture;
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let width = screen_size.x;
        let height = screen_size.y;
        if (id.x >= width || id.y >= height) { return; }
    
        let pixel_idx = id.y * width + id.x;
    
        if (params.max_blur <= 0.0 || params.aperture <= 0.0) {
            dof_output[pixel_idx] = color_input[pixel_idx];
            return;
        }
    
        let base = gbuffer_base(pixel_idx);
        let depth = gbuffer[base + 8u];
    
        if (depth <= 0.0) {
            dof_output[pixel_idx] = color_input[pixel_idx];
            return;
        }
    
        let blur_radius = circle_of_confusion(depth);
    
        if (blur_radius < 0.5) {
            dof_output[pixel_idx] = color_input[pixel_idx];
            return;
        }
    
        let radius_i = i32(ceil(blur_radius));
        let radius_sq = blur_radius * blur_radius;
    
        var color_sum = vec3(0.0);
        var weight_sum = 0.0;
    
        for (var dy = -radius_i; dy <= radius_i; dy = dy + 1) {
            for (var dx = -radius_i; dx <= radius_i; dx = dx + 1) {
                let dist_sq = f32(dx * dx + dy * dy);
                if (dist_sq > radius_sq) { continue; }
    
                let sx = i32(id.x) + dx;
                let sy = i32(id.y) + dy;
                if (sx < 0 || sx >= i32(width) || sy < 0 || sy >= i32(height)) { continue; }
    
                let sample_idx = u32(sy) * width + u32(sx);
                let sample_color = color_input[sample_idx].rgb;
    
                let w = 1.0 - sqrt(dist_sq) / blur_radius;
                color_sum = color_sum + sample_color * w;
                weight_sum = weight_sum + w;
            }
        }
    
        let result = select(color_input[pixel_idx].rgb, color_sum / weight_sum, weight_sum > 0.0);
        dof_output[pixel_idx] = vec4(result, 1.0);
}


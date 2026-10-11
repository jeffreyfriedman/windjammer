struct GBufferPixel {
    position: vec4<f32>,
    normal: vec4<f32>,
    depth: f32,
    geometry_source: f32,
    _pad: vec2<f32>,
}

@group(0) @binding(0) var<storage, read> input_color: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read_write> output_color: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read> gbuffer: array<GBufferPixel>;
@group(0) @binding(3) var<uniform> screen_size: vec2<u32>;
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

fn luminance(c: vec3<f32>) -> f32 {
    return dot(c, vec3(0.2126, 0.7152, 0.0722));
}

fn sample_color(x: i32, y: i32, w: u32, h: u32) -> vec3<f32> {
    let cx = clamp(x, 0, i32(w) - 1);
        let cy = clamp(y, 0, i32(h) - 1);
        return input_color[u32(cy) * w + u32(cx)].rgb;
}

fn sample_normal(x: i32, y: i32, w: u32, h: u32) -> vec3<f32> {
    let cx = clamp(x, 0, i32(w) - 1);
        let cy = clamp(y, 0, i32(h) - 1);
        return gbuffer[u32(cy) * w + u32(cx)].normal.xyz;
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let width = screen_size.x;
        let height = screen_size.y;
        if (id.x >= width || id.y >= height) { return; }
    
        let pixel_idx = id.y * width + id.x;
        let ix = i32(id.x);
        let iy = i32(id.y);
    
        let c = sample_color(ix, iy, width, height);
        let n = sample_color(ix, iy - 1, width, height);
        let s = sample_color(ix, iy + 1, width, height);
        let e = sample_color(ix + 1, iy, width, height);
        let w_col = sample_color(ix - 1, iy, width, height);
    
        let lum_c = luminance(c);
        let lum_n = luminance(n);
        let lum_s = luminance(s);
        let lum_e = luminance(e);
        let lum_w = luminance(w_col);
    
        let lum_min = min(lum_c, min(min(lum_n, lum_s), min(lum_e, lum_w)));
        let lum_max = max(lum_c, max(max(lum_n, lum_s), max(lum_e, lum_w)));
        let lum_range = lum_max - lum_min;
    
        let edge_threshold = 0.05;
        let edge_threshold_min = 0.01;
        if (lum_range < max(edge_threshold_min, lum_max * edge_threshold)) {
            output_color[pixel_idx] = vec4(c, 1.0);
            return;
        }
    
        let cn = sample_normal(ix, iy, width, height);
        let nn = sample_normal(ix, iy - 1, width, height);
        let sn = sample_normal(ix, iy + 1, width, height);
        let en = sample_normal(ix + 1, iy, width, height);
        let wn = sample_normal(ix - 1, iy, width, height);
    
        let normal_edge = 4.0 - dot(cn, nn) - dot(cn, sn) - dot(cn, en) - dot(cn, wn);
        let is_geometry_edge = normal_edge > 0.5;
    
        let edge_h = abs(lum_n + lum_s - 2.0 * lum_c);
        let edge_v = abs(lum_e + lum_w - 2.0 * lum_c);
        let is_horizontal = edge_h >= edge_v;
    
        let blend_factor = clamp(lum_range / lum_max, 0.0, 0.5);
        var result = c;
    
        if (is_geometry_edge) {
            if (is_horizontal) {
                result = mix(c, (n + s) * 0.5, blend_factor);
            } else {
                result = mix(c, (e + w_col) * 0.5, blend_factor);
            }
        }
    
        output_color[pixel_idx] = vec4(result, 1.0);
}


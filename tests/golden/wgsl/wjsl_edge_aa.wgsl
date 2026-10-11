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
@group(0) @binding(4) var<uniform> frame_parity: u32;
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

fn sample_at(x: i32, y: i32, w: u32, h: u32) -> vec3<f32> {
    let cx = clamp(x, 0, i32(w) - 1);
        let cy = clamp(y, 0, i32(h) - 1);
        return input_color[u32(cy) * w + u32(cx)].rgb;
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let width = screen_size.x;
        let height = screen_size.y;
        if (id.x >= width || id.y >= height) { return; }
    
        let idx = id.y * width + id.x;
        let ix = i32(id.x);
        let iy = i32(id.y);
    
        var offset_x = 0;
        var offset_y = 0;
        if (frame_parity == 0u) {
            offset_x = 1;
        } else {
            offset_y = 1;
        }
    
        let c = sample_at(ix, iy, width, height);
        let edge = sample_at(ix + offset_x, iy + offset_y, width, height);
        let lum_diff = abs(luminance(c) - luminance(edge));
        if (lum_diff > 0.08) {
            output_color[idx] = vec4(mix(c, edge, 0.5), 1.0);
        } else {
            output_color[idx] = vec4(c, 1.0);
        }
}


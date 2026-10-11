struct GBufferPixel {
    position: vec4<f32>,
    normal: vec4<f32>,
    depth: f32,
    geometry_source: f32,
    _pad: vec2<f32>,
}

struct HiZParams {
    src_width: u32,
    src_height: u32,
    dst_width: u32,
    dst_height: u32,
}

@group(0) @binding(0) var<uniform> hiz_params: HiZParams;
@group(0) @binding(1) var<storage, read> gbuffer_in: array<GBufferPixel>;
@group(0) @binding(2) var<storage, read_write> hiz_output: array<f32>;
var<workgroup> tile_depth: array<f32, 64>;

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
fn main(@builtin(global_invocation_id) id: vec3<u32>, @builtin(local_invocation_id) lid: vec3<u32>) {
    let tid = lid.y * 8u + lid.x;
    
        // Each thread loads one depth value from G-buffer
        var d = 0.0;
        if (id.x < hiz_params.src_width && id.y < hiz_params.src_height) {
            let pidx = id.y * hiz_params.src_width + id.x;
            d = gbuffer_in[pidx].depth;
        }
        tile_depth[tid] = d;
        workgroupBarrier();
    
        // Every 4th thread in x and y computes max of its 4x4 sub-block
        if (lid.x % 4u != 0u || lid.y % 4u != 0u) { return; }
    
        var max_d = 0.0;
        for (var dy = 0u; dy < 4u; dy++) {
            for (var dx = 0u; dx < 4u; dx++) {
                let lx = lid.x + dx;
                let ly = lid.y + dy;
                if (lx < 8u && ly < 8u) {
                    let v = tile_depth[ly * 8u + lx];
                    if (v > max_d) { max_d = v; }
                }
            }
        }
    
        let out_x = id.x / 4u;
        let out_y = id.y / 4u;
        if (out_x < hiz_params.dst_width && out_y < hiz_params.dst_height) {
            hiz_output[out_y * hiz_params.dst_width + out_x] = max_d;
        }
}


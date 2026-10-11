struct BloomCombineParams {
    bloom_strength: f32,
    _pad1: f32,
    _pad2: f32,
    _pad3: f32,
}

@group(0) @binding(0) var<uniform> params: BloomCombineParams;
@group(0) @binding(1) var<storage, read> hdr_input: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read> bloom_tex: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read_write> combined_output: array<vec4<f32>>;
@group(0) @binding(4) var<uniform> screen_size: vec2<u32>;
@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let width = screen_size.x;
        let height = screen_size.y;
        if (id.x >= width || id.y >= height) { return; }
    
        let idx = id.y * width + id.x;
        let hdr = hdr_input[idx].rgb;
        let bloom = bloom_tex[idx].rgb;
    
        let combined = min(hdr + bloom * params.bloom_strength, vec3(5.0));
        combined_output[idx] = vec4(combined, 1.0);
}


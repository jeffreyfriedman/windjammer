// Instanced sprite batch shader - single draw call for many sprites
// Vertex buffer 0: quad vertices (6 vertices, step_mode: Vertex)
// Vertex buffer 1: instance data (step_mode: Instance)

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
}

// Instance data from vertex buffer 1 - one per sprite
struct InstanceInput {
    @location(4) position: vec2<f32>,
    @location(5) size: vec2<f32>,
    @location(6) uv_rect: vec4<f32>,  // x, y, width, height
    @location(7) color: vec4<f32>,
}

// Viewport uniform: (width, height) for pixel to NDC
@group(0) @binding(0) var<uniform> viewport: vec2<f32>;
// Texture bindings for future texture sampling (currently use vertex color)
@group(0) @binding(1) var sprite_sampler: sampler;
@group(0) @binding(2) var sprite_texture: texture_2d<f32>;

const QUAD_POS: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
    vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0),
);

@vertex
fn vs_main(
    @builtin(vertex_index) vid: u32,
    instance: InstanceInput,
) -> VertexOutput {
    let quad_pos = QUAD_POS[vid];
    let quad_uv = vec2<f32>(
        instance.uv_rect.x + quad_pos.x * instance.uv_rect.z,
        instance.uv_rect.y + quad_pos.y * instance.uv_rect.w
    );

    // Transform: quad_pos (0..1) * size + position -> pixel space
    // Pixel to NDC: (x / width * 2 - 1, 1 - y / height * 2)
    let pixel_pos = (quad_pos * instance.size) + instance.position;
    let ndc_x = (pixel_pos.x / viewport.x) * 2.0 - 1.0;
    let ndc_y = 1.0 - (pixel_pos.y / viewport.y) * 2.0;  // Y flip for screen

    var out: VertexOutput;
    out.clip_position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.uv = quad_uv;
    out.color = instance.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}

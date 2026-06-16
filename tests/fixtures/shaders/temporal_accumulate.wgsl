// Temporal Accumulation Compute Shader
//
// Accumulates samples across frames for improved denoising quality.
// Exponential moving average: output = history * blend_factor + current * (1 - blend_factor)
// When camera moves, blend_factor = 0 (use only current frame, invalidate history)
// Workgroup: 8x8 threads

struct TemporalParams {
    blend_factor: f32,  // 0.0 = no history, 1.0 = full history
    width: u32,
    height: u32,
    _pad: u32,
}

@group(0) @binding(0) var<storage, read> current_frame: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read> history_buffer: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read_write> output: array<vec4<f32>>;
@group(0) @binding(3) var<uniform> params: TemporalParams;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= params.width || id.y >= params.height) {
        return;
    }

    let idx = id.y * params.width + id.x;

    let current = current_frame[idx];
    let history = history_buffer[idx];

    // Exponential moving average
    let blended = history * params.blend_factor + current * (1.0 - params.blend_factor);

    output[idx] = blended;
}

// 2D Sprite Shader
// Simple, efficient rendering for 2D games

// Vertex shader input
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) color: vec4<f32>,
}

// Vertex shader output / Fragment shader input
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
}

// Vertex shader
@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    
    // Simple orthographic projection for 2D
    // Assuming screen space coordinates (0,0) to (width, height)
    // Convert to clip space (-1, -1) to (1, 1)
    let screen_size = vec2<f32>(800.0, 600.0); // TODO: Pass as uniform
    let normalized = input.position / screen_size;
    let clip_pos = vec2<f32>(
        normalized.x * 2.0 - 1.0,
        1.0 - normalized.y * 2.0
    );
    
    output.clip_position = vec4<f32>(clip_pos, 0.0, 1.0);
    output.tex_coords = input.tex_coords;
    output.color = input.color;
    
    return output;
}

// Fragment shader
@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // For now, just return the vertex color
    // TODO: Add texture sampling
    return input.color;
}


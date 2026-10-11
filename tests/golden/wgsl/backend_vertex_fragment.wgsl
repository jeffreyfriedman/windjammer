struct VertexOutput {
    position: vec4<f32>,
    color: vec4<f32>,
}

@vertex
fn vs_main(@location(0) pos: vec2<f32>) -> VertexOutput {
    return VertexOutput(vec4(pos.x, pos.y, 0, 1), vec4(1, 1, 1, 1));
}

@fragment
fn fs_main(@location(0) color: vec4<f32>) -> vec4<f32> {
    return color;
}


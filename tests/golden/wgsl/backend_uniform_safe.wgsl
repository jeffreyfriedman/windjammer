@binding(3)
var<uniform> screen_size: /* auto-converted from vec2<u32> */ vec2<f32>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let w = u32(screen_size.x);
}


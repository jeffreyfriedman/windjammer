// Hybrid voxel + mesh compositing: depth-tested merge with equal-depth blending and voxel transparency.

struct Params {
    width: u32,
    height: u32,
    blend_threshold: f32,
    _pad: f32,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> mesh_color: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read> mesh_depth: array<f32>;
@group(0) @binding(3) var<storage, read> voxel_color: array<vec4<f32>>;
@group(0) @binding(4) var<storage, read> voxel_depth: array<f32>;
@group(0) @binding(5) var<storage, read_write> output_packed: array<u32>;

fn pack_rgba8_unorm(c: vec4<f32>) -> u32 {
    let r = u32(clamp(c.r, 0.0, 1.0) * 255.0);
    let g = u32(clamp(c.g, 0.0, 1.0) * 255.0);
    let b = u32(clamp(c.b, 0.0, 1.0) * 255.0);
    let a = u32(clamp(c.a, 0.0, 1.0) * 255.0);
    return r | (g << 8u) | (b << 16u) | (a << 24u);
}

fn voxel_over_mesh(Cv: vec4<f32>, Cm: vec4<f32>) -> vec4<f32> {
    let a = clamp(Cv.a, 0.0, 1.0);
    return vec4<f32>(Cv.rgb * a + Cm.rgb * (1.0 - a), 1.0);
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x >= params.width || gid.y >= params.height) {
        return;
    }

    let idx = gid.y * params.width + gid.x;
    let Cm = mesh_color[idx];
    let dm = mesh_depth[idx];
    let Cv = voxel_color[idx];
    let dv = voxel_depth[idx];

    let diff = abs(dm - dv);
    let bt = max(params.blend_threshold, 1e-8);

    var out: vec4<f32>;

    if (diff < bt) {
        var hard: vec4<f32>;
        if (dv < dm) {
            hard = voxel_over_mesh(Cv, Cm);
        } else if (dm < dv) {
            hard = vec4<f32>(Cm.rgb, 1.0);
        } else {
            hard = voxel_over_mesh(Cv, Cm);
        }

        let equal_mix = vec4<f32>(mix(Cm.rgb, Cv.rgb, 0.5), 1.0);
        let w = smoothstep(0.0, bt, diff);
        out = mix(equal_mix, hard, w);
    } else if (dv < dm) {
        if (Cv.a < 1.0) {
            out = voxel_over_mesh(Cv, Cm);
        } else {
            out = vec4<f32>(Cv.rgb, 1.0);
        }
    } else {
        out = vec4<f32>(Cm.rgb, 1.0);
    }

    output_packed[idx] = pack_rgba8_unorm(clamp(out, vec4<f32>(0.0), vec4<f32>(1.0)));
}

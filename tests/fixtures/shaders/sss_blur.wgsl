// Separable subsurface scattering blur (Jimenez 2015-style screen-space SSS)
//
// Per-channel Gaussian kernels: wider red, medium green, narrow blue (different
// diffusion lengths). Two passes: horizontal (lit -> temp) then vertical (temp -> packed u32).
// Only pixels with (gbuffer_material_flags & sss_material_mask) != 0 are blurred; others copy through.
//
// Workgroup: 8x8 threads

struct SssParams {
    width: u32,
    height: u32,
    // 1 = horizontal pass (read lit_color, write temp), 0 = vertical (read temp, write output_u32)
    pass_horizontal: u32,
    sss_material_mask: u32,
    kernel_len_r: u32,
    kernel_len_g: u32,
    kernel_len_b: u32,
    _pad_u32: u32,
    // Kernel standard deviations in pixels (matches CPU pre-compute; wider R, medium G, narrow B)
    radius_r: f32,
    radius_g: f32,
    radius_b: f32,
    sss_strength: f32,
}

@group(0) @binding(0) var<uniform> params: SssParams;
@group(0) @binding(1) var<storage, read> lit_color: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read> gbuffer_material_flags: array<u32>;
@group(0) @binding(3) var<storage, read_write> temp_rgba: array<vec4<f32>>;
@group(0) @binding(4) var<storage, read_write> output_u32: array<u32>;
@group(0) @binding(5) var<storage, read> kernel_r: array<f32>;
@group(0) @binding(6) var<storage, read> kernel_g: array<f32>;
@group(0) @binding(7) var<storage, read> kernel_b: array<f32>;

fn pack_rgba8(c: vec4<f32>) -> u32 {
    let r = u32(clamp(c.r, 0.0, 1.0) * 255.0);
    let g = u32(clamp(c.g, 0.0, 1.0) * 255.0);
    let b = u32(clamp(c.b, 0.0, 1.0) * 255.0);
    let a = u32(clamp(c.a, 0.0, 1.0) * 255.0);
    return r | (g << 8u) | (b << 16u) | (a << 24u);
}

fn is_sss_pixel(idx: u32) -> bool {
    return (gbuffer_material_flags[idx] & params.sss_material_mask) != 0u;
}

fn blur_horizontal_sss(pos: vec2<u32>) -> vec4<f32> {
    let idx_center = pos.y * params.width + pos.x;
    let kr = params.kernel_len_r;
    let kg = params.kernel_len_g;
    let kb = params.kernel_len_b;
    let half_r = (kr - 1u) / 2u;
    let half_g = (kg - 1u) / 2u;
    let half_b = (kb - 1u) / 2u;

    var acc_r = 0.0;
    var acc_g = 0.0;
    var acc_b = 0.0;
    var acc_a = 0.0;

    for (var i = 0u; i < kr; i++) {
        let off = i32(i) - i32(half_r);
        let sx = i32(pos.x) + off;
        if (sx < 0 || sx >= i32(params.width)) {
            continue;
        }
        let sidx = pos.y * params.width + u32(sx);
        let s = lit_color[sidx];
        acc_r += s.r * kernel_r[i];
    }
    for (var j = 0u; j < kg; j++) {
        let off = i32(j) - i32(half_g);
        let sx = i32(pos.x) + off;
        if (sx < 0 || sx >= i32(params.width)) {
            continue;
        }
        let sidx = pos.y * params.width + u32(sx);
        let s = lit_color[sidx];
        acc_g += s.g * kernel_g[j];
    }
    for (var k = 0u; k < kb; k++) {
        let off = i32(k) - i32(half_b);
        let sx = i32(pos.x) + off;
        if (sx < 0 || sx >= i32(params.width)) {
            continue;
        }
        let sidx = pos.y * params.width + u32(sx);
        let s = lit_color[sidx];
        acc_b += s.b * kernel_b[k];
    }
    // Alpha: same footprint as green (medium) for stability
    for (var j = 0u; j < kg; j++) {
        let off = i32(j) - i32(half_g);
        let sx = i32(pos.x) + off;
        if (sx < 0 || sx >= i32(params.width)) {
            continue;
        }
        let sidx = pos.y * params.width + u32(sx);
        acc_a += lit_color[sidx].a * kernel_g[j];
    }

    let c = lit_color[idx_center];
    let blurred = vec4<f32>(acc_r, acc_g, acc_b, acc_a);
    return mix(c, blurred, params.sss_strength);
}

fn blur_vertical_sss(pos: vec2<u32>) -> vec4<f32> {
    let idx_center = pos.y * params.width + pos.x;
    let kr = params.kernel_len_r;
    let kg = params.kernel_len_g;
    let kb = params.kernel_len_b;
    let half_r = (kr - 1u) / 2u;
    let half_g = (kg - 1u) / 2u;
    let half_b = (kb - 1u) / 2u;

    var acc_r = 0.0;
    var acc_g = 0.0;
    var acc_b = 0.0;
    var acc_a = 0.0;

    for (var i = 0u; i < kr; i++) {
        let off = i32(i) - i32(half_r);
        let sy = i32(pos.y) + off;
        if (sy < 0 || sy >= i32(params.height)) {
            continue;
        }
        let sidx = u32(sy) * params.width + pos.x;
        let s = temp_rgba[sidx];
        acc_r += s.r * kernel_r[i];
    }
    for (var j = 0u; j < kg; j++) {
        let off = i32(j) - i32(half_g);
        let sy = i32(pos.y) + off;
        if (sy < 0 || sy >= i32(params.height)) {
            continue;
        }
        let sidx = u32(sy) * params.width + pos.x;
        let s = temp_rgba[sidx];
        acc_g += s.g * kernel_g[j];
    }
    for (var k = 0u; k < kb; k++) {
        let off = i32(k) - i32(half_b);
        let sy = i32(pos.y) + off;
        if (sy < 0 || sy >= i32(params.height)) {
            continue;
        }
        let sidx = u32(sy) * params.width + pos.x;
        let s = temp_rgba[sidx];
        acc_b += s.b * kernel_b[k];
    }
    for (var j = 0u; j < kg; j++) {
        let off = i32(j) - i32(half_g);
        let sy = i32(pos.y) + off;
        if (sy < 0 || sy >= i32(params.height)) {
            continue;
        }
        let sidx = u32(sy) * params.width + pos.x;
        acc_a += temp_rgba[sidx].a * kernel_g[j];
    }

    let c = temp_rgba[idx_center];
    let blurred = vec4<f32>(acc_r, acc_g, acc_b, acc_a);
    return mix(c, blurred, params.sss_strength);
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= params.width || id.y >= params.height) {
        return;
    }

    let idx = id.y * params.width + id.x;

    if (params.pass_horizontal != 0u) {
        // Horizontal pass
        if (!is_sss_pixel(idx)) {
            temp_rgba[idx] = lit_color[idx];
        } else {
            temp_rgba[idx] = blur_horizontal_sss(id.xy);
        }
    } else {
        // Vertical pass -> packed output
        if (!is_sss_pixel(idx)) {
            output_u32[idx] = pack_rgba8(temp_rgba[idx]);
        } else {
            let v = blur_vertical_sss(id.xy);
            output_u32[idx] = pack_rgba8(v);
        }
    }
}

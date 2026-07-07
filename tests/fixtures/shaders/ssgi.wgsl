// Screen-space global illumination (SSGI) — half-resolution compute pass
//
// Short screen-space rays in random cosine-weighted hemisphere directions (Hammersley),
// depth ray-march against the G-buffer (Hi-Z optional acceleration), gather lit color,
// cosine weight, temporal EMA into RGBA16F storage.
//
// Workgroup: 8×8 (each invocation = one half-res pixel)

struct GiUniforms {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    screen_size: vec2<f32>,
    sample_count: u32,
    max_distance: f32,
    intensity: f32,
    temporal_blend: f32,
    frame_index: u32,
    _pad: f32,
}

// ─── G-buffer + Hi-Z (group 0) ───
@group(0) @binding(0) var<uniform> gi: GiUniforms;
@group(0) @binding(1) var gbuf_normal_roughness: texture_2d<f32>;
@group(0) @binding(2) var gbuf_depth: texture_depth_2d;
@group(0) @binding(3) var hiz_pyramid: texture_2d<f32>;

// ─── Lit color + temporal (group 1) ───
@group(1) @binding(0) var lit_scene: texture_2d<f32>;
@group(1) @binding(1) var lit_sampler: sampler;
@group(1) @binding(2) var prev_indirect: texture_2d<f32>;
@group(1) @binding(3) var out_indirect: texture_storage_2d<rgba16float, write>;

const LINEAR_STEPS: u32 = 20u;
const HIZ_MAX_MIP_FALLBACK: u32 = 7u;

fn octahedral_decode(e: vec2<f32>) -> vec3<f32> {
    let p = e * 2.0 - 1.0;
    let z = 1.0 - abs(p.x) - abs(p.y);
    var n: vec3<f32>;
    if (z >= 0.0) {
        n = vec3<f32>(p.x, p.y, z);
    } else {
        let signs = vec2<f32>(
            select(-1.0, 1.0, p.x >= 0.0),
            select(-1.0, 1.0, p.y >= 0.0)
        );
        n = vec3<f32>((1.0 - abs(p.y)) * signs.x, (1.0 - abs(p.x)) * signs.y, z);
    }
    return normalize(n);
}

fn world_pos_from_depth(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    let ndc = vec4<f32>(uv * 2.0 - 1.0, depth, 1.0);
    let ndc_flipped = vec4<f32>(ndc.x, -ndc.y, ndc.z, 1.0);
    var world_pos = gi.inv_view_proj * ndc_flipped;
    world_pos = world_pos / world_pos.w;
    return world_pos.xyz;
}

fn project_view_to_uv_ndc_z(view_p: vec3<f32>) -> vec3<f32> {
    let clip = gi.proj * vec4<f32>(view_p, 1.0);
    let inv_w = 1.0 / clip.w;
    let ndc_xy = clip.xy * inv_w;
    let ndc_z = clip.z * inv_w;
    let uv = vec2<f32>(ndc_xy.x * 0.5 + 0.5, (-ndc_xy.y) * 0.5 + 0.5);
    return vec3<f32>(uv, ndc_z);
}

fn radical_inverse_vdc(bits_in: u32) -> f32 {
    var bits = bits_in;
    bits = (bits << 16u) | (bits >> 16u);
    bits = ((bits & 0x55555555u) << 1u) | ((bits & 0xAAAAAAAAu) >> 1u);
    bits = ((bits & 0x33333333u) << 2u) | ((bits & 0xCCCCCCCCu) >> 2u);
    bits = ((bits & 0x0F0F0F0Fu) << 4u) | ((bits & 0xF0F0F0F0u) >> 4u);
    bits = ((bits & 0x00FF00FFu) << 8u) | ((bits & 0xFF00FF00u) >> 8u);
    return f32(bits) * 2.3283064365386963e-10;
}

fn hammersley(i: u32, n: u32) -> vec2<f32> {
    return vec2<f32>(f32(i) / f32(max(n, 1u)), radical_inverse_vdc(i));
}

fn build_tbn(n: vec3<f32>) -> mat3x3<f32> {
    let up = select(vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(1.0, 0.0, 0.0), abs(n.y) > 0.99);
    let t = normalize(cross(up, n));
    let b = cross(n, t);
    return mat3x3<f32>(t, b, n);
}

fn sample_cosine_hemisphere(u: vec2<f32>) -> vec3<f32> {
    let phi = 6.28318530718 * u.x;
    let cos_sq = 1.0 - u.y;
    let sin_theta = sqrt(u.y);
    let cos_theta = sqrt(cos_sq);
    return vec3<f32>(cos(phi) * sin_theta, sin(phi) * sin_theta, cos_theta);
}

fn rotate_hammersley(base: vec2<f32>, pix: vec2<u32>, frame: u32) -> vec2<f32> {
    let f = f32(frame % 1024u);
    let a = (f * 0.0245436926 + f32(pix.x & 127u) * 0.01 + f32(pix.y & 127u) * 0.013) * 6.28318530718;
    let c = cos(a);
    let s = sin(a);
    return vec2<f32>(base.x * c - base.y * s, base.x * s + base.y * c);
}

fn sample_hiz_max(uv: vec2<f32>, mip: i32) -> f32 {
    let level = u32(clamp(mip, 0, i32(HIZ_MAX_MIP_FALLBACK)));
    let dims = textureDimensions(hiz_pyramid, i32(level));
    let max_x = i32(max(dims.x, 1u)) - 1;
    let max_y = i32(max(dims.y, 1u)) - 1;
    let fc = uv * vec2<f32>(dims) - vec2<f32>(0.5);
    let px = clamp(i32(floor(fc.x)), 0, max_x);
    let py = clamp(i32(floor(fc.y)), 0, max_y);
    return textureLoad(hiz_pyramid, vec2<i32>(px, py), i32(level)).r;
}

fn trace_diffuse_gi(
    uv: vec2<f32>,
    world_pos: vec3<f32>,
    world_n: vec3<f32>,
    dir_world: vec3<f32>,
    width: u32,
    height: u32,
) -> vec3<f32> {
    let view_start = (gi.view * vec4<f32>(world_pos, 1.0)).xyz;
    let dir_view = normalize((gi.view * vec4<f32>(dir_world, 0.0)).xyz);
    let step_len = gi.max_distance / f32(LINEAR_STEPS);

    for (var s = 1u; s <= LINEAR_STEPS; s++) {
        let t = f32(s) * step_len;
        let p_v = view_start + dir_view * t;
        let p_uvz = project_view_to_uv_ndc_z(p_v);
        let p_uv = p_uvz.xy;
        let ray_z = p_uvz.z;

        if (p_uv.x <= 0.0 || p_uv.x >= 1.0 || p_uv.y <= 0.0 || p_uv.y >= 1.0) {
            break;
        }

        let mip_bias = i32(floor(f32(s - 1u) / 4.0));
        let mip = min(mip_bias, i32(HIZ_MAX_MIP_FALLBACK));
        _ = sample_hiz_max(p_uv, mip);

        let pix = vec2<i32>(
            i32(clamp(p_uv.x * f32(width), 0.0, f32(width - 1u))),
            i32(clamp(p_uv.y * f32(height), 0.0, f32(height - 1u)))
        );
        let scene_d = textureLoad(gbuf_depth, pix, 0);

        // Same depth test as SSR: ray passes behind (or through) stored surface.
        if (ray_z >= scene_d) {
            let radiance = textureSampleLevel(lit_scene, lit_sampler, p_uv, 0.0).rgb;
            let cos_theta = max(dot(world_n, dir_world), 0.0);
            return radiance * cos_theta;
        }
    }
    return vec3<f32>(0.0, 0.0, 0.0);
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let full_w = max(u32(gi.screen_size.x), 1u);
    let full_h = max(u32(gi.screen_size.y), 1u);
    let half_w = max(full_w / 2u, 1u);
    let half_h = max(full_h / 2u, 1u);

    if (id.x >= half_w || id.y >= half_h) {
        return;
    }

    let full_px = vec2<i32>(
        min(i32(id.x * 2u + 1u), i32(full_w) - 1),
        min(i32(id.y * 2u + 1u), i32(full_h) - 1)
    );

    let uv = vec2<f32>(
        (f32(full_px.x) + 0.5) / f32(full_w),
        (f32(full_px.y) + 0.5) / f32(full_h)
    );

    let normal_r = textureLoad(gbuf_normal_roughness, full_px, 0);
    let depth = textureLoad(gbuf_depth, full_px, 0);

    let out_px = vec2<i32>(i32(id.x), i32(id.y));
    let uv_half = vec2<f32>(
        (f32(id.x) + 0.5) / f32(half_w),
        (f32(id.y) + 0.5) / f32(half_h)
    );

    if (depth >= 0.99999 || depth <= 0.0) {
        textureStore(out_indirect, out_px, vec4<f32>(0.0, 0.0, 0.0, 1.0));
        return;
    }

    let N = octahedral_decode(normal_r.xy);
    let world_pos = world_pos_from_depth(uv, depth);
    let tbn = build_tbn(N);

    let n_samples = min(gi.sample_count, 32u);
    var accum = vec3<f32>(0.0, 0.0, 0.0);

    for (var i = 0u; i < 32u; i++) {
        if (i >= n_samples) {
            break;
        }
        let h0 = hammersley(i, n_samples);
        let h = rotate_hammersley(h0, vec2<u32>(id.x, id.y), gi.frame_index);
        let local = sample_cosine_hemisphere(h);
        let dir_w = normalize(tbn * local);
        accum += trace_diffuse_gi(uv, world_pos, N, dir_w, full_w, full_h);
    }

    let gi_rgb = (accum / f32(max(n_samples, 1u))) * gi.intensity;

    let prev = textureSampleLevel(prev_indirect, lit_sampler, uv_half, 0.0).rgb;
    let a = clamp(gi.temporal_blend, 0.0, 1.0);
    let blended = prev * (1.0 - a) + gi_rgb * a;

    textureStore(out_indirect, out_px, vec4<f32>(blended, 1.0));
}

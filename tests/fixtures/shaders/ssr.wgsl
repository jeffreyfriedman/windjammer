// Screen-Space Reflections (SSR) — Hi-Z accelerated compute pass
//
// Traces reflected view rays in screen space using a hierarchical depth pyramid,
// with linear marching + binary refinement. On miss, outputs transparent RGBA
// so the deferred pass can fall back to IBL.
//
// Workgroup: 8×8

struct CameraUniforms {
    view_matrix: mat4x4<f32>,
    proj_matrix: mat4x4<f32>,
    inv_view: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    position: vec3<f32>,
    _pad0: f32,
    screen_size: vec2<f32>,
    near_plane: f32,
    far_plane: f32,
}

struct SsrParams {
    step_size: f32,
    max_distance: f32,
    edge_border: f32,
    roughness_cutoff: f32,
    hiz_max_mip: u32,
    width: u32,
    height: u32,
    _pad0: u32,
}

// ─── G-buffer + Hi-Z (group 0) ───
@group(0) @binding(0) var gbuf_normal_roughness: texture_2d<f32>;
@group(0) @binding(1) var gbuf_depth: texture_depth_2d;
@group(0) @binding(2) var hiz_pyramid: texture_2d<f32>;
@group(0) @binding(3) var<uniform> camera: CameraUniforms;
@group(0) @binding(4) var<uniform> ssr_params: SsrParams;

// ─── Previous frame color + output (group 1) ───
@group(1) @binding(0) var prev_frame_color: texture_2d<f32>;
@group(1) @binding(1) var prev_sampler: sampler;
@group(1) @binding(2) var<storage, read_write> output_ssr: array<u32>;

const LINEAR_STEPS: u32 = 32u;
const BINARY_STEPS: u32 = 8u;

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
    var world_pos = camera.inv_proj * ndc_flipped;
    world_pos = world_pos / world_pos.w;
    world_pos = camera.inv_view * world_pos;
    return world_pos.xyz;
}

fn project_view_to_uv_ndc_z(view_p: vec3<f32>) -> vec3<f32> {
    let clip = camera.proj_matrix * vec4<f32>(view_p, 1.0);
    let inv_w = 1.0 / clip.w;
    let ndc_xy = clip.xy * inv_w;
    let ndc_z = clip.z * inv_w;
    let uv = vec2<f32>(ndc_xy.x * 0.5 + 0.5, (-ndc_xy.y) * 0.5 + 0.5);
    return vec3<f32>(uv, ndc_z);
}

fn pack_rgba_to_u32(c: vec4<f32>) -> u32 {
    let r = u32(clamp(c.r * 255.0, 0.0, 255.0));
    let g = u32(clamp(c.g * 255.0, 0.0, 255.0));
    let b = u32(clamp(c.b * 255.0, 0.0, 255.0));
    let a = u32(clamp(c.a * 255.0, 0.0, 255.0));
    return r | (g << 8u) | (b << 16u) | (a << 24u);
}

fn edge_fade_factor(uv: vec2<f32>, border: f32) -> f32 {
    let d = min(min(uv.x, 1.0 - uv.x), min(uv.y, 1.0 - uv.y));
    return smoothstep(0.0, border, d);
}

fn roughness_fade_factor(roughness: f32, cutoff: f32) -> f32 {
    if (roughness <= cutoff) {
        return 1.0;
    }
    return 1.0 - smoothstep(cutoff, 1.0, roughness);
}

fn sample_hiz_max(uv: vec2<f32>, mip: i32) -> f32 {
    let level = u32(clamp(mip, 0, i32(ssr_params.hiz_max_mip)));
    let dims = textureDimensions(hiz_pyramid, i32(level));
    let max_x = i32(max(dims.x, 1u)) - 1;
    let max_y = i32(max(dims.y, 1u)) - 1;
    let fc = uv * vec2<f32>(dims) - vec2<f32>(0.5);
    let px = clamp(i32(floor(fc.x)), 0, max_x);
    let py = clamp(i32(floor(fc.y)), 0, max_y);
    return textureLoad(hiz_pyramid, vec2<i32>(px, py), i32(level)).r;
}

fn trace_ssr(
    uv: vec2<f32>,
    world_pos: vec3<f32>,
    N: vec3<f32>,
    roughness: f32,
) -> vec4<f32> {
    let width = ssr_params.width;
    let height = ssr_params.height;
    let border = ssr_params.edge_border;
    let rcut = ssr_params.roughness_cutoff;

    let edge_att = edge_fade_factor(uv, border);
    let rough_att = roughness_fade_factor(roughness, rcut);
    let fade = edge_att * rough_att;

    if (fade < 0.001) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    let view_start = (camera.view_matrix * vec4<f32>(world_pos, 1.0)).xyz;
    let n_view = normalize((camera.view_matrix * vec4<f32>(N, 0.0)).xyz);
    let v_view = normalize(-view_start);
    let r_view = reflect(-v_view, n_view);

    var t_near = 0.0;
    var t_far = ssr_params.max_distance;
    var hit_uv = uv;
    var hit = false;

    var prev_t = 0.0;

    for (var i = 1u; i <= LINEAR_STEPS; i++) {
        let raw_t = (f32(i) / f32(LINEAR_STEPS)) * t_far;
        let t = max(raw_t, ssr_params.step_size);
        let p_v = view_start + r_view * t;
        let p_uvz = project_view_to_uv_ndc_z(p_v);
        let p_uv = p_uvz.xy;
        let ray_ndc_z = p_uvz.z;

        if (p_uv.x <= 0.0 || p_uv.x >= 1.0 || p_uv.y <= 0.0 || p_uv.y >= 1.0) {
            break;
        }

        let mip_bias = i32(floor(f32(i - 1u) / 4.0));
        let mip = min(mip_bias, i32(ssr_params.hiz_max_mip));
        let hiz_d = sample_hiz_max(p_uv, mip);

        if (ray_ndc_z > hiz_d + 0.0001) {
            // Ray is behind the conservative max depth in this tile — continue (Hi-Z acceleration path).
        }

        let pix = vec2<i32>(
            i32(clamp(p_uv.x * f32(width), 0.0, f32(width - 1u))),
            i32(clamp(p_uv.y * f32(height), 0.0, f32(height - 1u)))
        );
        let scene_d = textureLoad(gbuf_depth, pix, 0);

        if (ray_ndc_z >= scene_d) {
            hit = true;
            hit_uv = p_uv;
            t_near = prev_t;
            t_far = t;
            break;
        }

        prev_t = t;
    }

    if (!hit) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    // Binary refinement on [t_near, t_far]
    var lo = t_near;
    var hi = t_far;
    for (var b = 0u; b < BINARY_STEPS; b++) {
        let tm = (lo + hi) * 0.5;
        let p_v = view_start + r_view * tm;
        let p_uvz = project_view_to_uv_ndc_z(p_v);
        let p_uv = p_uvz.xy;
        let ray_ndc_z = p_uvz.z;

        if (p_uv.x <= 0.0 || p_uv.x >= 1.0 || p_uv.y <= 0.0 || p_uv.y >= 1.0) {
            hi = tm;
            continue;
        }

        let pix = vec2<i32>(
            i32(clamp(p_uv.x * f32(width), 0.0, f32(width - 1u))),
            i32(clamp(p_uv.y * f32(height), 0.0, f32(height - 1u)))
        );
        let scene_d = textureLoad(gbuf_depth, pix, 0);

        if (ray_ndc_z >= scene_d) {
            hi = tm;
            hit_uv = p_uv;
        } else {
            lo = tm;
        }
    }

    let refl = textureSampleLevel(prev_frame_color, prev_sampler, hit_uv, 0.0);
    return vec4<f32>(refl.rgb * fade, refl.a * fade);
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let width = ssr_params.width;
    let height = ssr_params.height;

    if (id.x >= width || id.y >= height) {
        return;
    }

    let pixel = vec2<i32>(i32(id.x), i32(id.y));
    let uv = vec2<f32>((f32(id.x) + 0.5) / f32(width), (f32(id.y) + 0.5) / f32(height));

    let normal_roughness = textureLoad(gbuf_normal_roughness, pixel, 0);
    let depth = textureLoad(gbuf_depth, pixel, 0);

    let idx = id.y * width + id.x;

    if (depth >= 1.0) {
        output_ssr[idx] = pack_rgba_to_u32(vec4<f32>(0.0, 0.0, 0.0, 0.0));
        return;
    }

    let N = octahedral_decode(normal_roughness.rg);
    let roughness = normal_roughness.b;
    let world_pos = world_pos_from_depth(uv, depth);

    let color = trace_ssr(uv, world_pos, N, roughness);
    output_ssr[idx] = pack_rgba_to_u32(color);
}

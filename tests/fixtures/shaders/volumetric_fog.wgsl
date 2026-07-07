// Raymarched volumetric fog — half-resolution compute pass with temporal reprojection.
// Group 0: uniforms + depth + linear sampler; Group 1: storage output + previous fog.
// Workgroup: 8×8, up to 64 ray-march steps.

struct FogUniforms {
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    fog_color: vec4<f32>,
    fog_density: f32,
    fog_height_falloff: f32,
    fog_start: f32,
    fog_end: f32,
    light_dir: vec4<f32>,
    light_color: vec4<f32>,
    time: f32,
    hg_g: f32,
    temporal_blend: f32,
    _pad: f32,
    screen_size: vec2<f32>,
    _pad2: vec2<f32>,
}

@group(0) @binding(0) var<uniform> u: FogUniforms;
@group(0) @binding(1) var depth_tex: texture_depth_2d;
@group(0) @binding(2) var linear_sampler: sampler;

@group(1) @binding(0) var fog_out: texture_storage_2d<rgba16float, write>;
@group(1) @binding(1) var fog_prev: texture_2d<f32>;

const MAX_STEPS: u32 = 64u;

fn hash3(p: vec3<f32>) -> f32 {
    let q = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
    let dot = q.x + q.y * 37.0 + q.z * 13.0;
    return fract(sin(dot) * 43758.5453);
}

fn fbm3(p: vec3<f32>) -> f32 {
    var v = 0.0;
    var a = 0.5;
    var pp = p;
    for (var i = 0u; i < 4u; i++) {
        v = v + a * hash3(pp);
        pp = pp * 2.07 + vec3<f32>(17.0, 13.0, 11.0);
        a = a * 0.5;
    }
    return v;
}

fn henyey_greenstein(cos_theta: f32, g: f32) -> f32 {
    let g2 = g * g;
    let denom = pow(max(1.0 + g2 - 2.0 * g * cos_theta, 1e-6), 1.5);
    return (1.0 - g2) / (4.0 * 3.14159265 * denom);
}

fn world_from_uv_depth(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    let ndc = vec4<f32>(uv * 2.0 - 1.0, depth, 1.0);
    let ndc_flipped = vec4<f32>(ndc.x, -ndc.y, ndc.z, 1.0);
    var wp = u.inv_view_proj * ndc_flipped;
    wp = wp / wp.w;
    return wp.xyz;
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = textureDimensions(fog_out);
    if (gid.x >= dim.x || gid.y >= dim.y) {
        return;
    }

    let uv = (vec2<f32>(gid.xy) + vec2<f32>(0.5, 0.5)) / max(u.screen_size, vec2<f32>(1.0, 1.0));
    let full_sz = max(u._pad2, vec2<f32>(1.0, 1.0));

    let fc = uv * full_sz - vec2<f32>(0.5, 0.5);
    let max_x = max(i32(full_sz.x) - 1, 0);
    let max_y = max(i32(full_sz.y) - 1, 0);
    let di = vec2<i32>(
        clamp(i32(floor(fc.x)), 0, max_x),
        clamp(i32(floor(fc.y)), 0, max_y)
    );
    // textureLoad for depth (compute-safe); linear_sampler kept for API/temporal path evolution
    let depth = textureLoad(depth_tex, di, 0);

    let world_hit = world_from_uv_depth(uv, depth);
    let cam = u.camera_pos.xyz;
    var ray_dir = world_hit - cam;
    let scene_dist = length(ray_dir);
    ray_dir = ray_dir / max(scene_dist, 1e-4);

    let t_near = max(u.fog_start, 0.001);
    let t_far = min(u.fog_end, scene_dist);
    let dt = (t_far - t_near) / f32(MAX_STEPS);

    var transmittance = 1.0;
    var fog_rgb = vec3<f32>(0.0, 0.0, 0.0);
    let w_light = normalize(u.light_dir.xyz);
    let sun_rgb = u.light_color.rgb;

    if (t_far > t_near && dt > 0.0) {
        for (var i = 0u; i < MAX_STEPS; i++) {
            let t = t_near + (f32(i) + 0.5) * dt;
            let sample_pos = cam + ray_dir * t;
            let height_att = exp(-max(0.0, sample_pos.y) * u.fog_height_falloff);
            let n3 = sample_pos * 0.12 + vec3<f32>(0.0, u.time * 0.03, u.time * 0.02);
            let noise = fbm3(n3);
            let sigma = max(0.0, u.fog_density * height_att * (0.35 + 0.65 * noise));

            let cos_t = dot(ray_dir, w_light);
            let phase = henyey_greenstein(cos_t, u.hg_g);
            let extinction = sigma;
            let tr_step = exp(-extinction * dt);
            let scatter_col = sun_rgb * phase + u.fog_color.rgb * 0.2;
            fog_rgb = fog_rgb + transmittance * (1.0 - tr_step) * scatter_col;
            transmittance = transmittance * tr_step;
        }
    }

    let opacity = clamp(1.0 - transmittance, 0.0, 1.0);
    var out_c = vec4<f32>(fog_rgb, opacity);

    let prev = textureLoad(fog_prev, vec2<i32>(gid.xy), 0);
    out_c = mix(out_c, prev, clamp(u.temporal_blend, 0.0, 0.999));

    textureStore(fog_out, vec2<i32>(gid.xy), out_c);
}

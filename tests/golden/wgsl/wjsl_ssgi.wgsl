struct GBufferPixel {
    position: vec4<f32>,
    normal: vec4<f32>,
    depth: f32,
    geometry_source: f32,
    _pad: vec2<f32>,
}

struct AtmosphereParams {
    sun_direction: vec3<f32>,
    _pad0: f32,
    sun_color: vec3<f32>,
    _pad1: f32,
    sky_top_color: vec3<f32>,
    _pad2: f32,
    sky_horizon_color: vec3<f32>,
    _pad3: f32,
    ground_color: vec3<f32>,
    fog_density: f32,
    fog_color: vec3<f32>,
    sun_size: f32,
    sun_intensity: f32,
    _pad4: f32,
    _pad5: f32,
    _pad6: f32,
}

struct WaterParams {
    water_color: vec3<f32>,
    time: f32,
    deep_color: vec3<f32>,
    wave_speed: f32,
    wave_amplitude: f32,
    wave_frequency: f32,
    transparency: f32,
    reflectivity: f32,
    absorption_rate: f32,
    water_material_id: f32,
    specular_power: f32,
    specular_intensity: f32,
}

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

struct SmokeParams {
    source_position: vec3<f32>,
    time: f32,
    smoke_color: vec3<f32>,
    density_scale: f32,
    rise_speed: f32,
    spread_rate: f32,
    max_height: f32,
    absorption: f32,
    scatter_strength: f32,
    phase_g: f32,
    noise_scale: f32,
    step_count: f32,
    light_dir: vec3<f32>,
    _pad0: f32,
    light_color: vec3<f32>,
    wind_strength: f32,
    wind_dir: vec2<f32>,
    plume_radius: f32,
    _pad1: f32,
}

struct LightingParams {
    sun_dir: vec3<f32>,
    _pad0: f32,
    sun_color: vec3<f32>,
    sun_intensity: f32,
    sky_color: vec3<f32>,
    _pad1: f32,
    ground_color: vec3<f32>,
    ambient_intensity: f32,
    gi_samples: u32,
    gi_intensity: f32,
    frame_count: u32,
    _pad2: f32,
    prev_vp_matrix: mat4x4<f32>,
    temporal_blend: f32,
    _pad3: f32,
    _pad4: f32,
    _pad5: f32,
    camera_pos: vec3<f32>,
    _pad6: f32,
}

struct RaymarchParams {
    world_size: vec3<f32>,
    voxel_size: f32,
    max_steps: u32,
    svo_depth: u32,
    node_count: u32,
    frame_index: u32,
}

struct CompositeParams {
    exposure: f32,
    gamma: f32,
    bloom_threshold: f32,
    vignette_strength: f32,
}

struct BloomExtractParams {
    threshold: f32,
    soft_knee: f32,
    _pad0: f32,
    _pad1: f32,
}

struct BloomBlurParams {
    direction: vec2<f32>,
    _pad0: f32,
    _pad1: f32,
}

struct BloomCombineParams {
    bloom_strength: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
}

struct HudParams {
    health: f32,
    max_health: f32,
    ammo: f32,
    max_ammo: f32,
    damage_flash: f32,
    crosshair_size: f32,
    energy: f32,
    max_energy: f32,
    fps: f32,
    kill_count: f32,
    debug_enabled: f32,
    crosshair_material_id: f32,
    camera_pos: vec3<f32>,
    nearest_enemy_dist: f32,
    collision_blocked: f32,
    camera_yaw: f32,
    camera_pitch: f32,
    crosshair_hit: f32,
    level: f32,
    xp_current: f32,
    xp_to_next: f32,
    skill_points: f32,
    inventory_count: f32,
    inventory_capacity: f32,
    weapon_form: f32,
    wave_number: f32,
}

struct ParticleSimParams {
    delta_time: f32,
    gravity: f32,
    _pad0: f32,
    _pad1: f32,
    wind: vec3<f32>,
    turbulence: f32,
    particle_count: f32,
    time: f32,
    spawn_type_id: f32,
    _pad2: f32,
}

struct ParticleEmitterParams {
    position: vec3<f32>,
    radius: f32,
}

struct ParticleRenderParams {
    splat_scale: f32,
    depth_bias: f32,
    vertical_fov_rad: f32,
    max_splat_radius_px: f32,
}

struct HeatSource {
    position: vec3<f32>,
    radius: f32,
    intensity: f32,
    rise_height: f32,
    _pad0: f32,
    _pad1: f32,
}

struct HeatControlParams {
    source_count: f32,
    time: f32,
    distortion_scale: f32,
    wave_speed: f32,
    wave_frequency: f32,
    falloff_power: f32,
    max_offset_pixels: f32,
    _pad0: f32,
}

struct ScreenSizeParams {
    width: u32,
    height: u32,
    _pad0: f32,
    _pad1: f32,
}

struct UpscaleParams {
    src_width: u32,
    src_height: u32,
    dst_width: u32,
    dst_height: u32,
    render_scale: f32,
    sharpness: f32,
    frame_count: u32,
    blend_factor: f32,
}

struct HiZParams {
    src_width: u32,
    src_height: u32,
    dst_width: u32,
    dst_height: u32,
}

struct CloudParams {
    coverage: f32,
    density: f32,
    shell_bottom: f32,
    shell_top: f32,
    phase_g: f32,
    absorption: f32,
    silver_intensity: f32,
    wind_speed: f32,
    wind_dir: vec3<f32>,
    time: f32,
    width: u32,
    height: u32,
    near_march_scale: f32,
    _pad0: f32,
}

struct HeightFogParams {
    base_height: f32,
    scale_height: f32,
    density: f32,
    _pad0: f32,
    fog_color: vec3<f32>,
    sun_scatter_strength: f32,
    ambient_strength: f32,
    _pad1: f32,
    _pad2: f32,
    _pad3: f32,
    sun_dir: vec3<f32>,
    width: u32,
    height: u32,
    _pad4: f32,
    _pad5: f32,
    _pad6: f32,
}

struct MotionVectorParams {
    width: u32,
    height: u32,
    jitter_x: f32,
    jitter_y: f32,
    prev_jitter_x: f32,
    prev_jitter_y: f32,
    frame_index: u32,
    _pad0: f32,
}

struct GiParams {
    inv_view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    screen_width: u32,
    screen_height: u32,
    sample_count: u32,
    frame_index: u32,
    max_distance: f32,
    intensity: f32,
    temporal_blend: f32,
    _pad: f32,
}

@group(0) @binding(0) var<uniform> gi: GiParams;
@group(0) @binding(1) var<storage, read> gbuffer: array<f32>;
@group(0) @binding(2) var<storage, read> color_buffer: array<u32>;
@group(0) @binding(3) var<storage, read> depth_buffer: array<f32>;
@group(0) @binding(4) var<storage, read> prev_indirect: array<u32>;
@group(0) @binding(5) var<storage, read_write> out_indirect: array<u32>;
const LINEAR_STEPS: u32 = 20u;

fn gbuffer_base(pixel_idx: u32) -> u32 {
    return pixel_idx * 12u;
}

fn gbuf_position(g: GBufferPixel) -> vec3<f32> {
    return g.position.xyz;
}

fn gbuf_normal(g: GBufferPixel) -> vec3<f32> {
    return g.normal.xyz;
}

fn gbuf_material_id(g: GBufferPixel) -> u32 {
    return u32(g.normal.w);
}

fn generate_ray(pixel: vec2<f32>, cam: CameraUniforms) -> vec3<f32> {
    let ndc = vec2(
            (2.0 * pixel.x / cam.screen_size.x - 1.0),
            (1.0 - 2.0 * pixel.y / cam.screen_size.y)
        );
        let clip_near = vec4(ndc.x, ndc.y, -1.0, 1.0);
        let clip_far  = vec4(ndc.x, ndc.y,  1.0, 1.0);
        let world_near_h = cam.inv_view * (cam.inv_proj * clip_near);
        let world_far_h  = cam.inv_view * (cam.inv_proj * clip_far);
        let world_near = world_near_h.xyz / world_near_h.w;
        let world_far  = world_far_h.xyz  / world_far_h.w;
        return normalize(world_far - world_near);
}

fn hash31(p: vec3<f32>) -> f32 {
    let q = fract(p * 0.1031);
        let r = dot(q, vec3(12.9898, 78.233, 45.54321));
        return fract(sin(r) * 43758.5453);
}

fn hash33(p: vec3<f32>) -> vec3<f32> {
    var q = vec3(
            dot(p, vec3(127.1, 311.7, 74.7)),
            dot(p, vec3(269.5, 183.3, 246.1)),
            dot(p, vec3(113.5, 271.9, 124.6))
        );
        return fract(sin(q) * 43758.5453123);
}

fn noise3d(p: vec3<f32>) -> f32 {
    let i = floor(p);
        let f = fract(p);
        let u = f * f * (3.0 - 2.0 * f);
    
        let n000 = hash31(i + vec3(0.0, 0.0, 0.0));
        let n100 = hash31(i + vec3(1.0, 0.0, 0.0));
        let n010 = hash31(i + vec3(0.0, 1.0, 0.0));
        let n110 = hash31(i + vec3(1.0, 1.0, 0.0));
        let n001 = hash31(i + vec3(0.0, 0.0, 1.0));
        let n101 = hash31(i + vec3(1.0, 0.0, 1.0));
        let n011 = hash31(i + vec3(0.0, 1.0, 1.0));
        let n111 = hash31(i + vec3(1.0, 1.0, 1.0));
    
        let x0 = mix(n000, n100, u.x);
        let x1 = mix(n010, n110, u.x);
        let x2 = mix(n001, n101, u.x);
        let x3 = mix(n011, n111, u.x);
        let y0 = mix(x0, x1, u.y);
        let y1 = mix(x2, x3, u.y);
        return mix(y0, y1, u.z);
}

fn fbm(p: vec3<f32>, octaves: i32) -> f32 {
    var value = 0.0;
        var amplitude = 0.5;
        var frequency = 1.0;
        var pos = p;
        for (var i = 0; i < octaves; i = i + 1) {
            value = value + amplitude * noise3d(pos * frequency);
            amplitude = amplitude * 0.5;
            frequency = frequency * 2.0;
        }
        return value;
}

fn world_pos_from_depth(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    let ndc = vec4<f32>(uv * 2.0 - 1.0, depth, 1.0);
        let ndc_flipped = vec4<f32>(ndc.x, -ndc.y, ndc.z, 1.0);
        var world = gi.inv_view_proj * ndc_flipped;
        return world.xyz / world.w;
}

fn project_view_to_uv(view_p: vec3<f32>) -> vec3<f32> {
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
        let sin_theta = sqrt(u.y);
        let cos_theta = sqrt(1.0 - u.y);
        return vec3<f32>(cos(phi) * sin_theta, sin(phi) * sin_theta, cos_theta);
}

fn rotate_hammersley(base: vec2<f32>, pix: vec2<u32>, frame: u32) -> vec2<f32> {
    let f = f32(frame % 1024u);
        let a = (f * 0.0245436926 + f32(pix.x & 127u) * 0.01 + f32(pix.y & 127u) * 0.013) * 6.28318530718;
        let c = cos(a);
        let s = sin(a);
        return vec2<f32>(base.x * c - base.y * s, base.x * s + base.y * c);
}

fn read_color_rgb(idx: u32) -> vec3<f32> {
    return vec3<f32>(
            bitcast<f32>(color_buffer[idx * 4u]),
            bitcast<f32>(color_buffer[idx * 4u + 1u]),
            bitcast<f32>(color_buffer[idx * 4u + 2u]),
        );
}

fn trace_gi_ray(world_pos: vec3<f32>, world_n: vec3<f32>, dir_world: vec3<f32>, full_w: u32, full_h: u32) -> vec3<f32> {
    let view_start = (gi.view * vec4<f32>(world_pos, 1.0)).xyz;
        let dir_view = normalize((gi.view * vec4<f32>(dir_world, 0.0)).xyz);
        let step_len = gi.max_distance / f32(LINEAR_STEPS);
    
        for (var s = 1u; s <= LINEAR_STEPS; s++) {
            let t = f32(s) * step_len;
            let p_v = view_start + dir_view * t;
            let p_uvz = project_view_to_uv(p_v);
            let p_uv = p_uvz.xy;
            let ray_z = p_uvz.z;
    
            if (p_uv.x <= 0.0 || p_uv.x >= 1.0 || p_uv.y <= 0.0 || p_uv.y >= 1.0) {
                break;
            }
    
            let px = u32(clamp(p_uv.x * f32(full_w), 0.0, f32(full_w - 1u)));
            let py = u32(clamp(p_uv.y * f32(full_h), 0.0, f32(full_h - 1u)));
            let pixel_idx = py * full_w + px;
            let scene_d = gbuffer[pixel_idx * 12u + 8u];
    
            if (ray_z >= scene_d) {
                let radiance = read_color_rgb(pixel_idx);
                let cos_theta = max(dot(world_n, dir_world), 0.0);
                return radiance * cos_theta;
            }
        }
        return vec3<f32>(0.0, 0.0, 0.0);
}

fn pack_rgb_to_u32(c: vec3<f32>) -> vec4<u32> {
    return vec4<u32>(
            bitcast<u32>(c.r),
            bitcast<u32>(c.g),
            bitcast<u32>(c.b),
            bitcast<u32>(1.0),
        );
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let full_w = gi.screen_width;
        let full_h = gi.screen_height;
        let half_w = max(full_w / 2u, 1u);
        let half_h = max(full_h / 2u, 1u);
    
        if (id.x >= half_w || id.y >= half_h) {
            return;
        }
    
        let full_px_x = min(id.x * 2u + 1u, full_w - 1u);
        let full_px_y = min(id.y * 2u + 1u, full_h - 1u);
        let full_idx = full_px_y * full_w + full_px_x;
    
        let uv = vec2<f32>(
            (f32(full_px_x) + 0.5) / f32(full_w),
            (f32(full_px_y) + 0.5) / f32(full_h)
        );
    
        let gb_base = full_idx * 12u;
        let normal = vec3<f32>(
            gbuffer[gb_base + 4u],
            gbuffer[gb_base + 5u],
            gbuffer[gb_base + 6u],
        );
        let depth = gbuffer[gb_base + 8u];
    
        let out_idx = id.y * half_w + id.x;
    
        if (depth >= 0.99999 || depth <= 0.0) {
            out_indirect[out_idx * 4u] = bitcast<u32>(0.0);
            out_indirect[out_idx * 4u + 1u] = bitcast<u32>(0.0);
            out_indirect[out_idx * 4u + 2u] = bitcast<u32>(0.0);
            out_indirect[out_idx * 4u + 3u] = bitcast<u32>(1.0);
            return;
        }
    
        let N = normalize(normal);
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
            accum += trace_gi_ray(world_pos, N, dir_w, full_w, full_h);
        }
    
        let gi_rgb = (accum / f32(max(n_samples, 1u))) * gi.intensity;
    
        let prev_r = bitcast<f32>(prev_indirect[out_idx * 4u]);
        let prev_g = bitcast<f32>(prev_indirect[out_idx * 4u + 1u]);
        let prev_b = bitcast<f32>(prev_indirect[out_idx * 4u + 2u]);
        let prev = vec3<f32>(prev_r, prev_g, prev_b);
    
        let a = clamp(gi.temporal_blend, 0.0, 1.0);
        let blended = prev * (1.0 - a) + gi_rgb * a;
    
        out_indirect[out_idx * 4u] = bitcast<u32>(blended.r);
        out_indirect[out_idx * 4u + 1u] = bitcast<u32>(blended.g);
        out_indirect[out_idx * 4u + 2u] = bitcast<u32>(blended.b);
        out_indirect[out_idx * 4u + 3u] = bitcast<u32>(1.0);
}


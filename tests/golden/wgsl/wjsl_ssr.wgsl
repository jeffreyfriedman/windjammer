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

struct SsrParams {
    inv_view_proj: mat4x4<f32>,
    view_matrix: mat4x4<f32>,
    proj_matrix: mat4x4<f32>,
    width: u32,
    height: u32,
    step_size: f32,
    max_distance: f32,
    edge_border: f32,
    roughness_cutoff: f32,
    _pad0: f32,
    _pad1: f32,
}

@group(0) @binding(0) var<uniform> ssr: SsrParams;
@group(0) @binding(1) var<storage, read> gbuffer: array<f32>;
@group(0) @binding(2) var<storage, read> depth_buffer: array<f32>;
@group(0) @binding(3) var<storage, read> color_buffer: array<u32>;
@group(0) @binding(4) var<storage, read_write> output_ssr: array<u32>;
const LINEAR_STEPS: u32 = 32u;
const BINARY_STEPS: u32 = 8u;

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

fn world_pos_from_depth(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    let ndc = vec4<f32>(uv * 2.0 - 1.0, depth, 1.0);
        let ndc_flipped = vec4<f32>(ndc.x, -ndc.y, ndc.z, 1.0);
        var world = ssr.inv_view_proj * ndc_flipped;
        return world.xyz / world.w;
}

fn project_view_to_uv(view_p: vec3<f32>) -> vec3<f32> {
    let clip = ssr.proj_matrix * vec4<f32>(view_p, 1.0);
        let inv_w = 1.0 / clip.w;
        let ndc_xy = clip.xy * inv_w;
        let ndc_z = clip.z * inv_w;
        let uv = vec2<f32>(ndc_xy.x * 0.5 + 0.5, (-ndc_xy.y) * 0.5 + 0.5);
        return vec3<f32>(uv, ndc_z);
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

fn pack_rgba_to_u32(c: vec4<f32>) -> u32 {
    let r = u32(clamp(c.r * 255.0, 0.0, 255.0));
        let g = u32(clamp(c.g * 255.0, 0.0, 255.0));
        let b = u32(clamp(c.b * 255.0, 0.0, 255.0));
        let a = u32(clamp(c.a * 255.0, 0.0, 255.0));
        return r | (g << 8u) | (b << 16u) | (a << 24u);
}

fn read_color_rgb(px: u32, py: u32, w: u32) -> vec3<f32> {
    let idx = py * w + px;
        return vec3<f32>(
            bitcast<f32>(color_buffer[idx * 4u]),
            bitcast<f32>(color_buffer[idx * 4u + 1u]),
            bitcast<f32>(color_buffer[idx * 4u + 2u]),
        );
}

fn trace_ssr(uv: vec2<f32>, world_pos: vec3<f32>, N: vec3<f32>, roughness: f32) -> vec4<f32> {
    let width = ssr.width;
        let height = ssr.height;
    
        let edge_att = edge_fade_factor(uv, ssr.edge_border);
        let rough_att = roughness_fade_factor(roughness, ssr.roughness_cutoff);
        let fade = edge_att * rough_att;
    
        if (fade < 0.001) {
            return vec4<f32>(0.0, 0.0, 0.0, 0.0);
        }
    
        let view_start = (ssr.view_matrix * vec4<f32>(world_pos, 1.0)).xyz;
        let n_view = normalize((ssr.view_matrix * vec4<f32>(N, 0.0)).xyz);
        let v_view = normalize(-view_start);
        let r_view = reflect(-v_view, n_view);
    
        var t_near = 0.0;
        var t_far = ssr.max_distance;
        var hit_uv = uv;
        var hit = false;
        var prev_t = 0.0;
    
        for (var i = 1u; i <= LINEAR_STEPS; i++) {
            let raw_t = (f32(i) / f32(LINEAR_STEPS)) * t_far;
            let t = max(raw_t, ssr.step_size);
            let p_v = view_start + r_view * t;
            let p_uvz = project_view_to_uv(p_v);
            let p_uv = p_uvz.xy;
            let ray_ndc_z = p_uvz.z;
    
            if (p_uv.x <= 0.0 || p_uv.x >= 1.0 || p_uv.y <= 0.0 || p_uv.y >= 1.0) {
                break;
            }
    
            let px = u32(clamp(p_uv.x * f32(width), 0.0, f32(width - 1u)));
            let py = u32(clamp(p_uv.y * f32(height), 0.0, f32(height - 1u)));
            let pixel_idx = py * width + px;
            let scene_d = gbuffer[pixel_idx * 12u + 8u];
    
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
    
        var lo = t_near;
        var hi = t_far;
        for (var b = 0u; b < BINARY_STEPS; b++) {
            let tm = (lo + hi) * 0.5;
            let p_v = view_start + r_view * tm;
            let p_uvz = project_view_to_uv(p_v);
            let p_uv = p_uvz.xy;
            let ray_ndc_z = p_uvz.z;
    
            if (p_uv.x <= 0.0 || p_uv.x >= 1.0 || p_uv.y <= 0.0 || p_uv.y >= 1.0) {
                hi = tm;
                continue;
            }
    
            let px = u32(clamp(p_uv.x * f32(width), 0.0, f32(width - 1u)));
            let py = u32(clamp(p_uv.y * f32(height), 0.0, f32(height - 1u)));
            let pixel_idx = py * width + px;
            let scene_d = gbuffer[pixel_idx * 12u + 8u];
    
            if (ray_ndc_z >= scene_d) {
                hi = tm;
                hit_uv = p_uv;
            } else {
                lo = tm;
            }
        }
    
        let hit_px = u32(clamp(hit_uv.x * f32(width), 0.0, f32(width - 1u)));
        let hit_py = u32(clamp(hit_uv.y * f32(height), 0.0, f32(height - 1u)));
        let refl = read_color_rgb(hit_px, hit_py, width);
        return vec4<f32>(refl * fade, fade);
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let width = ssr.width;
        let height = ssr.height;
    
        if (id.x >= width || id.y >= height) {
            return;
        }
    
        let pixel_idx = id.y * width + id.x;
        let uv = vec2<f32>((f32(id.x) + 0.5) / f32(width), (f32(id.y) + 0.5) / f32(height));
    
        let gb_base = pixel_idx * 12u;
        let normal = vec3<f32>(
            gbuffer[gb_base + 4u],
            gbuffer[gb_base + 5u],
            gbuffer[gb_base + 6u],
        );
        let depth = gbuffer[gb_base + 8u];
    
        if (depth >= 0.99999) {
            output_ssr[pixel_idx] = pack_rgba_to_u32(vec4<f32>(0.0, 0.0, 0.0, 0.0));
            return;
        }
    
        let N = normalize(normal);
        let roughness = 0.5;
        let world_pos = world_pos_from_depth(uv, depth);
    
        let color = trace_ssr(uv, world_pos, N, roughness);
        output_ssr[pixel_idx] = pack_rgba_to_u32(color);
}


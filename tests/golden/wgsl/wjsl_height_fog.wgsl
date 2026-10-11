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

struct GBufferPixel {
    position: vec4<f32>,
    normal: vec4<f32>,
    depth: f32,
    geometry_source: f32,
    _pad: vec2<f32>,
}

@group(0) @binding(0) var<uniform> fog: HeightFogParams;
@group(0) @binding(1) var<uniform> camera: CameraUniforms;
@group(0) @binding(2) var<storage, read> gbuffer: array<GBufferPixel>;
@group(0) @binding(3) var<storage, read_write> color_output: array<vec4<f32>>;
const VOL_PI: f32 = 3.14159265;

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

fn vol_hg_phase(cos_theta: f32, g: f32) -> f32 {
    let g2 = g * g;
        let denom = 1.0 + g2 - 2.0 * g * cos_theta;
        return (1.0 - g2) / (4.0 * VOL_PI * pow(max(denom, 1e-6), 1.5));
}

fn vol_beer_lambert(extinction: f32, segment_length: f32) -> f32 {
    return exp(-extinction * segment_length);
}

fn vol_march_transmittance_steps(origin: vec3<f32>, dir: vec3<f32>, t_min: f32, t_max: f32, steps: u32, absorption_scale: f32, density_scale: f32) -> f32 {
    if (t_max <= t_min) {
            return 1.0;
        }
        let step_len = (t_max - t_min) / f32(steps);
        var transmittance = 1.0;
        var t = t_min + step_len * 0.5;
        for (var i = 0u; i < steps; i++) {
            let pos = origin + dir * t;
            let density = density_scale;
            transmittance = transmittance * vol_beer_lambert(density * absorption_scale, step_len);
            if (transmittance < 0.001) {
                break;
            }
            t = t + step_len;
        }
        return transmittance;
}

fn vol_height_fog_density(world_y: f32, base_height: f32, scale_height: f32, density: f32) -> f32 {
    let h = max(world_y - base_height, 0.0);
        return density * exp(-h / max(scale_height, 0.001));
}

fn vol_sample_lut2d(lut: array<f32>, width: u32, height: u32, u: f32, v: f32) -> vec3<f32> {
    let w = max(width, 1u);
        let h = max(height, 1u);
        let fu = clamp(u, 0.0, 1.0) * f32(w - 1u);
        let fv = clamp(v, 0.0, 1.0) * f32(h - 1u);
        let x0 = u32(floor(fu));
        let y0 = u32(floor(fv));
        let x1 = min(x0 + 1u, w - 1u);
        let y1 = min(y0 + 1u, h - 1u);
        let tx = fu - f32(x0);
        let ty = fv - f32(y0);
        let idx00 = (y0 * w + x0) * 3u;
        let idx10 = (y0 * w + x1) * 3u;
        let idx01 = (y1 * w + x0) * 3u;
        let idx11 = (y1 * w + x1) * 3u;
        let c00 = vec3(lut[idx00], lut[idx00 + 1u], lut[idx00 + 2u]);
        let c10 = vec3(lut[idx10], lut[idx10 + 1u], lut[idx10 + 2u]);
        let c01 = vec3(lut[idx01], lut[idx01 + 1u], lut[idx01 + 2u]);
        let c11 = vec3(lut[idx11], lut[idx11 + 1u], lut[idx11 + 2u]);
        let c0 = mix(c00, c10, tx);
        let c1 = mix(c01, c11, tx);
        return mix(c0, c1, ty);
}

fn vol_height_fog_blend(background: vec3<f32>, sun_scatter: vec3<f32>, ambient: vec3<f32>, alpha: f32) -> vec3<f32> {
    return background * alpha + sun_scatter + ambient;
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let width = fog.width;
        let height = fog.height;
        if (id.x >= width || id.y >= height) { return; }
    
        let pixel_idx = id.y * width + id.x;
        let scene = color_output[pixel_idx];
        let gbuf = gbuffer[pixel_idx];
        let depth = gbuf.depth;
        let mat = gbuf.normal.w;
        if ((mat == 0.0) || (depth >= camera.far_plane)) {
            return;
        }
    
        let world_y = camera.position.y - depth * 0.1;
        let density = vol_height_fog_density(world_y, fog.base_height, fog.scale_height, fog.density);
        let alpha = exp(-density * depth * 0.01);
        let sun_scatter = fog.fog_color * fog.sun_scatter_strength * (1.0 - alpha);
        let ambient = fog.fog_color * fog.ambient_strength;
        let result = vol_height_fog_blend(scene.rgb, sun_scatter, ambient, alpha);
        color_output[pixel_idx] = vec4(result, 1.0);
}


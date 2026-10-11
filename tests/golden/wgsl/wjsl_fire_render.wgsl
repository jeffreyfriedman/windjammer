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

struct FireRenderParams {
    source_position: vec3<f32>,
    _pad0: f32,
    max_height: f32,
    absorption: f32,
    emission_strength: f32,
    time: f32,
    screen_width: u32,
    screen_height: u32,
    _pad1: u32,
    _pad2: u32,
}

@group(0) @binding(0) var<uniform> fire_render_params: FireRenderParams;
@group(0) @binding(1) var<uniform> camera: CameraUniforms;
@group(0) @binding(2) var<storage, read> temperature_grid: array<f32>;
@group(0) @binding(3) var<storage, read> gbuffer: array<GBufferPixel>;
@group(0) @binding(4) var<storage, read_write> color_output: array<vec4<f32>>;
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

fn grid_index(x: u32, y: u32, z: u32) -> u32 {
    return z * 64u * 64u + y * 64u + x;
}

fn blackbody_color(temperature: f32) -> vec3<f32> {
    let t = clamp(temperature, 0.0, 5.0);
        let r = smoothstep(0.0, 0.8, t);
        let g = smoothstep(0.5, 2.5, t) * 0.8;
        let b = smoothstep(2.0, 4.0, t) * 0.4;
        return vec3(r, g, b);
}

fn sample_temperature(world_pos: vec3<f32>) -> f32 {
    let cell_size = fire_render_params.max_height / 64.0;
        let rel = world_pos - fire_render_params.source_position;
        let gx = rel.x / cell_size + 32.0;
        let gy = rel.y / cell_size;
        let gz = rel.z / cell_size + 32.0;
    
        let ix = u32(clamp(gx, 0.0, 63.0));
        let iy = u32(clamp(gy, 0.0, 63.0));
        let iz = u32(clamp(gz, 0.0, 63.0));
    
        return temperature_grid[grid_index(ix, iy, iz)];
}

fn intersect_box(ray_origin: vec3<f32>, ray_dir: vec3<f32>) -> vec2<f32> {
    let half_size = fire_render_params.max_height * 0.5;
        let box_min = fire_render_params.source_position - vec3(half_size, 0.0, half_size);
        let box_max = fire_render_params.source_position + vec3(half_size, fire_render_params.max_height, half_size);
    
        let inv_dir = 1.0 / ray_dir;
        let t0 = (box_min - ray_origin) * inv_dir;
        let t1 = (box_max - ray_origin) * inv_dir;
        let tmin_v = min(t0, t1);
        let tmax_v = max(t0, t1);
        let tmin = max(max(tmin_v.x, tmin_v.y), tmin_v.z);
        let tmax = min(min(tmax_v.x, tmax_v.y), tmax_v.z);
    
        return vec2(max(tmin, 0.0), tmax);
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= fire_render_params.screen_width || id.y >= fire_render_params.screen_height) {
            return;
        }
    
        let pixel_idx = id.y * fire_render_params.screen_width + id.x;
    
        // Reconstruct ray
        let u = (f32(id.x) + 0.5) / f32(fire_render_params.screen_width);
        let v = (f32(id.y) + 0.5) / f32(fire_render_params.screen_height);
        let ndc_x = u * 2.0 - 1.0;
        let ndc_y = 1.0 - v * 2.0;
    
        let inv_proj = camera.inv_proj;
        let inv_view = camera.inv_view;
        let clip = vec4(ndc_x, ndc_y, 1.0, 1.0);
        let view_pos = inv_proj * clip;
        let view_dir = normalize(view_pos.xyz / view_pos.w);
        let world_dir4 = inv_view * vec4(view_dir, 0.0);
        let ray_dir = normalize(world_dir4.xyz);
        let ray_origin = camera.position;
    
        let scene_depth = gbuffer[pixel_idx].depth;
    
        // Intersect fire volume
        let t_range = intersect_box(ray_origin, ray_dir);
        if (t_range.x >= t_range.y) { return; }
    
        let t_start = t_range.x;
        var t_end_val = t_range.y;
        if (scene_depth > 0.0) {
            t_end_val = min(t_range.y, scene_depth);
        }
        if (t_start >= t_end_val) { return; }
    
        // Raymarch through fire
        let step_size = (t_end_val - t_start) / 32.0;
        var accumulated_color = vec3(0.0, 0.0, 0.0);
        var transmittance = 1.0;
    
        for (var i = 0u; i < 32u; i = i + 1u) {
            let t = t_start + (f32(i) + 0.5) * step_size;
            let sample_pos = ray_origin + ray_dir * t;
    
            let temp = sample_temperature(sample_pos);
            if (temp > 0.01) {
                let fire_color = blackbody_color(temp);
                let density = temp * fire_render_params.absorption;
                let absorption_val = exp(-density * step_size);
                let emission = fire_color * temp * fire_render_params.emission_strength * step_size;
    
                accumulated_color = accumulated_color + emission * transmittance;
                transmittance = transmittance * absorption_val;
    
                if (transmittance < 0.01) { break; }
            }
        }
    
        // Composite over scene
        let scene_color = color_output[pixel_idx];
        let final_color = scene_color.xyz * transmittance + accumulated_color;
        color_output[pixel_idx] = vec4(final_color, 1.0);
}


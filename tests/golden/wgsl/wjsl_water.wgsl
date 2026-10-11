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

@group(0) @binding(0) var<uniform> water_params: WaterParams;
@group(0) @binding(1) var<uniform> camera: CameraUniforms;
@group(0) @binding(2) var<storage, read> gbuffer: array<GBufferPixel>;
@group(0) @binding(3) var<storage, read_write> color_output: array<vec4<f32>>;
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

fn wave_height(pos: vec2<f32>, t: f32) -> f32 {
    let f = water_params.wave_frequency;
        let s = water_params.wave_speed;
        let a = water_params.wave_amplitude;
    
        // 3-octave Gerstner-inspired sum
        var h = sin(pos.x * f + t * s) * a;
        h = h + sin(pos.y * f * 0.7 + t * s * 1.3) * a * 0.5;
        h = h + sin((pos.x + pos.y) * f * 1.3 + t * s * 0.8) * a * 0.25;
        return h;
}

fn wave_normal(pos: vec2<f32>, t: f32) -> vec3<f32> {
    let eps = 0.1;
        let hc = wave_height(pos, t);
        let hx = wave_height(pos + vec2(eps, 0.0), t);
        let hz = wave_height(pos + vec2(0.0, eps), t);
        let dx = (hx - hc) / eps;
        let dz = (hz - hc) / eps;
        return normalize(vec3(-dx, 1.0, -dz));
}

fn fresnel_schlick(cos_theta: f32, r0: f32) -> f32 {
    return r0 + (1.0 - r0) * pow(1.0 - clamp(cos_theta, 0.0, 1.0), 5.0);
}

fn sky_reflection(reflect_dir: vec3<f32>) -> vec3<f32> {
    let y = reflect_dir.y;
        let sky_top = vec3(0.4, 0.6, 1.0);
        let sky_horizon = vec3(0.7, 0.8, 0.95);
        let t = clamp(y, 0.0, 1.0);
        return mix(sky_horizon, sky_top, t);
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let width = u32(camera.screen_size.x);
        let height = u32(camera.screen_size.y);
        if (id.x >= width || id.y >= height) {
            return;
        }
    
        let pixel_idx = id.y * width + id.x;
        if (pixel_idx >= arrayLength(&gbuffer)) {
            return;
        }
    
        let gbuf = gbuffer[pixel_idx];
        let mat = gbuf.normal.w;
        if (mat != water_params.water_material_id) {
            return;
        }
    
        let world_pos = gbuf.position.xyz;
        let depth = gbuf.depth;
        let surface_normal = gbuf.normal.xyz;
    
        // Animated wave normal
        let wave_n = wave_normal(world_pos.xz, water_params.time);
        let perturbed_normal = normalize(surface_normal + wave_n * 0.3);
    
        // View direction
        let view_dir = normalize(camera.position - world_pos);
        let cos_theta = max(dot(view_dir, perturbed_normal), 0.0);
    
        // Fresnel reflection factor
        let fresnel = fresnel_schlick(cos_theta, 0.02) * water_params.reflectivity;
    
        // Reflection
        let reflect_dir = reflect(-view_dir, perturbed_normal);
        let reflected_color = sky_reflection(reflect_dir);
    
        // Depth-based absorption: deeper water absorbs more light → shifts to deep_color
        let absorption = 1.0 - exp(-depth * water_params.absorption_rate);
        let water_base = mix(water_params.water_color, water_params.deep_color, absorption);
    
        // Specular highlight (sun on water)
        let sun_dir = normalize(vec3(0.5, 0.8, 0.3));
        let half_vec = normalize(view_dir + sun_dir);
        let spec = pow(max(dot(perturbed_normal, half_vec), 0.0), water_params.specular_power);
        let specular = spec * water_params.specular_intensity;
    
        // Blend water color with sky reflection via Fresnel
        var final_color = mix(water_base, reflected_color, fresnel);
        final_color = final_color + vec3(specular);
    
        // Transparency: blend with existing scene color behind water
        let scene = color_output[pixel_idx];
        let blended = mix(scene.xyz, final_color, water_params.transparency);
    
        color_output[pixel_idx] = vec4(blended, 1.0);
}


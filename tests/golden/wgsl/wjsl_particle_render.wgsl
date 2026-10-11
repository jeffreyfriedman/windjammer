struct GBufferPixel {
    position: vec4<f32>,
    normal: vec4<f32>,
    depth: f32,
    geometry_source: f32,
    _pad: vec2<f32>,
}

struct Particle {
    position: vec3<f32>,
    velocity: vec3<f32>,
    color: vec4<f32>,
    life: f32,
    max_life: f32,
    size: f32,
    type_id: u32,
}

struct ParticleParams {
    delta_time: f32,
    gravity: f32,
    wind_x: f32,
    wind_y: f32,
    wind_z: f32,
    turbulence: f32,
    particle_count: u32,
    time: f32,
    spawn_type_id: u32,
}

struct ParticleEmitterUniforms {
    position: vec3<f32>,
    radius: f32,
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

@group(0) @binding(0) var<storage, read> particles: array<Particle>;
@group(0) @binding(1) var<uniform> camera: CameraUniforms;
@group(0) @binding(2) var<uniform> render_params: ParticleRenderParams;
@group(0) @binding(3) var<storage, read> gbuffer: array<GBufferPixel>;
@group(0) @binding(4) var<storage, read_write> color_output: array<vec4<f32>>;
@group(0) @binding(5) var<uniform> screen_size: vec2<u32>;
const TYPE_FIRE: u32 = 0u;
const TYPE_SMOKE: u32 = 1u;
const TYPE_SPARK: u32 = 2u;
const TYPE_DEBRIS: u32 = 3u;
const TYPE_BLOOD: u32 = 4u;
const TYPE_WATER: u32 = 5u;

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

fn project_world_to_clip(world_pos: vec4<f32>) -> vec4<f32> {
    let view_pos = camera.view_matrix * world_pos;
        return camera.proj_matrix * view_pos;
}

fn scene_depth_linear(pixel_idx: u32, width: u32, height: u32) -> f32 {
    let max_idx = width * height;
        if (pixel_idx >= max_idx) {
            return camera.far_plane * 2.0;
        }
        if (pixel_idx >= arrayLength(&gbuffer)) {
            return camera.far_plane * 2.0;
        }
        return gbuffer[pixel_idx].depth;
}

fn splat_contrib(dist_norm: f32, softness: f32) -> f32 {
    let edge = clamp(softness, 0.15, 1.0);
        let t = clamp((1.0 - dist_norm) / edge, 0.0, 1.0);
        return t * t * (3.0 - 2.0 * t);
}

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
        let width = screen_size.x;
        let height = screen_size.y;
        let max_pixels = width * height;
    
        let pcount = u32(arrayLength(&particles));
        if (idx >= pcount) {
            return;
        }
    
        let part = particles[idx];
        if (part.life <= 0.0) {
            return;
        }
    
        let world = vec4(part.position.x, part.position.y, part.position.z, 1.0);
        let clip = project_world_to_clip(world);
    
        if (clip.w <= 1e-5) {
            return;
        }
    
        let ndc = clip.xyz / clip.w;
        if (ndc.z < 0.0 || ndc.z > 1.0) {
            return;
        }
    
        let screen_x = (ndc.x * 0.5 + 0.5) * f32(width);
        let screen_y = (1.0 - (ndc.y * 0.5 + 0.5)) * f32(height);
    
        if (screen_x < -32.0 || screen_x > f32(width) + 32.0 || screen_y < -32.0 || screen_y > f32(height) + 32.0) {
            return;
        }
    
        let cam_to_p = part.position - camera.position;
        let particle_depth = length(cam_to_p);
    
        let half_fov = max(render_params.vertical_fov_rad * 0.5, 0.01);
        let dist = max(particle_depth, camera.near_plane);
        let radius_px = (part.size * render_params.splat_scale * f32(height)) / (dist * tan(half_fov));
        let radius_clamped = min(max(radius_px, 1.0), render_params.max_splat_radius_px);
    
        let min_x = i32(floor(screen_x - radius_clamped));
        let max_x = i32(ceil(screen_x + radius_clamped));
        let min_y = i32(floor(screen_y - radius_clamped));
        let max_y = i32(ceil(screen_y + radius_clamped));
    
        let min_x_c = max(min_x, 0);
        let max_x_c = min(max_x, i32(width) - 1);
        let min_y_c = max(min_y, 0);
        let max_y_c = min(max_y, i32(height) - 1);
    
        let rgb = part.color.rgb * part.color.a;
        let soft = 0.55 + 0.35 * clamp(part.size * 4.0, 0.0, 1.0);
    
        for (var py = min_y_c; py <= max_y_c; py++) {
            for (var px = min_x_c; px <= max_x_c; px++) {
                let dx = f32(px) + 0.5 - screen_x;
                let dy = f32(py) + 0.5 - screen_y;
                let d = sqrt(dx * dx + dy * dy);
                let dn = d / radius_clamped;
                if (dn > 1.0) {
                    continue;
                }
    
                let pidx = u32(py) * width + u32(px);
                if (pidx >= max_pixels) {
                    continue;
                }
    
                let scene_d = scene_depth_linear(pidx, width, height);
                if (particle_depth >= scene_d - render_params.depth_bias && scene_d > 0.0 && scene_d < camera.far_plane * 1.5) {
                    continue;
                }
    
                let falloff = splat_contrib(dn, soft);
                if (falloff <= 1e-4) {
                    continue;
                }
    
                let add = vec4(rgb * falloff, 0.0);
                let prev = color_output[pidx];
                color_output[pidx] = prev + add;
            }
        }
}


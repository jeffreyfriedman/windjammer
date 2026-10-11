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

@group(0) @binding(0) var<uniform> smoke_params: SmokeParams;
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

fn smoke_density(world_pos: vec3<f32>) -> f32 {
    let rel = world_pos - smoke_params.source_position;
        let height = rel.y;
        if (height < 0.0 || height > smoke_params.max_height) {
            return 0.0;
        }
    
        let height_norm = height / smoke_params.max_height;
        let cone_radius = smoke_params.plume_radius + height_norm * smoke_params.spread_rate;
        let horiz_dist = length(vec2(rel.x, rel.z));
        if (horiz_dist > cone_radius * 1.5) {
            return 0.0;
        }
    
        let radial_falloff = 1.0 - smoothstep(cone_radius * 0.4, cone_radius, horiz_dist);
        let vertical_profile = smoothstep(0.0, 0.15, height_norm)
                             * (1.0 - smoothstep(0.7, 1.0, height_norm));
    
        let wind_offset = vec3(
            smoke_params.wind_dir.x * smoke_params.wind_strength * height_norm,
            0.0,
            smoke_params.wind_dir.y * smoke_params.wind_strength * height_norm
        );
        let noise_pos = (world_pos + wind_offset) * smoke_params.noise_scale
                      + vec3(0.0, -smoke_params.time * smoke_params.rise_speed, 0.0);
        let n = fbm(noise_pos, 4);
        let turbulence = clamp(n * 2.0 - 0.3, 0.0, 1.0);
    
        return radial_falloff * vertical_profile * turbulence * smoke_params.density_scale;
}

fn hg_phase(cos_theta: f32, g: f32) -> f32 {
    let g2 = g * g;
        let denom = 1.0 + g2 - 2.0 * g * cos_theta;
        return (1.0 - g2) / (4.0 * 3.14159265 * pow(denom, 1.5));
}

fn light_transmittance(pos: vec3<f32>) -> f32 {
    let sun_dir = normalize(smoke_params.light_dir);
        let light_step = 0.5;
        var transmittance = 1.0;
        var light_pos = pos;
        for (var i = 0; i < 6; i = i + 1) {
            light_pos = light_pos + sun_dir * light_step;
            let d = smoke_density(light_pos);
            transmittance = transmittance * exp(-d * smoke_params.absorption * light_step);
            if (transmittance < 0.01) {
                break;
            }
        }
        return transmittance;
}

fn intersect_smoke_volume(ray_origin: vec3<f32>, ray_dir: vec3<f32>) -> vec2<f32> {
    let src = smoke_params.source_position;
        let max_radius = smoke_params.plume_radius + smoke_params.spread_rate + 2.0;
    
        let t_y_min = (src.y - ray_origin.y) / max(ray_dir.y, 1e-6);
        let t_y_max = (src.y + smoke_params.max_height - ray_origin.y) / max(ray_dir.y, 1e-6);
        let t_near = max(min(t_y_min, t_y_max), 0.0);
        let t_far = max(t_y_min, t_y_max);
        if (t_near > t_far || t_far < 0.0) {
            return vec2(-1.0, -1.0);
        }
    
        return vec2(max(t_near, 0.0), t_far);
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let width = u32(camera.screen_size.x);
        let height = u32(camera.screen_size.y);
        if (id.x >= width || id.y >= height) {
            return;
        }
    
        let pixel_idx = id.y * width + id.x;
        let gb_len = arrayLength(&gbuffer);
        if (pixel_idx >= gb_len) {
            return;
        }
    
        let scene_depth = gbuffer[pixel_idx].depth;
        let pixel = vec2<f32>(f32(id.x) + 0.5, f32(id.y) + 0.5);
        let ray_dir = generate_ray(pixel, camera);
        let ray_origin = camera.position;
    
        let bounds = intersect_smoke_volume(ray_origin, ray_dir);
        if (bounds.x < 0.0) {
            return;
        }
    
        let t_start = bounds.x;
        let t_end = min(bounds.y, scene_depth);
        if (t_start >= t_end) {
            return;
        }
    
        let steps = u32(smoke_params.step_count);
        let step_size = (t_end - t_start) / f32(steps);
        let sun_dir = normalize(smoke_params.light_dir);
        let cos_angle = dot(ray_dir, sun_dir);
        let phase = hg_phase(cos_angle, smoke_params.phase_g);
    
        var transmittance = 1.0;
        var in_scatter = vec3(0.0, 0.0, 0.0);
    
        for (var i = 0u; i < steps; i = i + 1u) {
            let t = t_start + (f32(i) + 0.5) * step_size;
            let sample_pos = ray_origin + ray_dir * t;
            let d = smoke_density(sample_pos);
            if (d < 0.001) {
                continue;
            }
    
            let sample_transmittance = exp(-d * smoke_params.absorption * step_size);
            let lt = light_transmittance(sample_pos);
            let scatter_contribution = smoke_params.smoke_color
                * smoke_params.light_color
                * lt * phase * smoke_params.scatter_strength * d;
            in_scatter = in_scatter + scatter_contribution * transmittance * step_size;
            transmittance = transmittance * sample_transmittance;
    
            if (transmittance < 0.01) {
                break;
            }
        }
    
        let scene = color_output[pixel_idx];
        let composited = scene.xyz * transmittance + in_scatter;
        color_output[pixel_idx] = vec4(composited, 1.0);
}


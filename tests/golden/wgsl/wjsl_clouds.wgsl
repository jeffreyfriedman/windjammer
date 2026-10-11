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

@group(0) @binding(0) var<uniform> cloud_params: CloudParams;
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

fn cloud_density(world_pos: vec3<f32>) -> f32 {
    let cam_y = camera.position.y;
        let altitude = world_pos.y;
        if (altitude < cloud_params.shell_bottom || altitude > cloud_params.shell_top) {
            return 0.0;
        }
        let shell_norm = (altitude - cloud_params.shell_bottom)
            / max(cloud_params.shell_top - cloud_params.shell_bottom, 1.0);
        let wind_offset = cloud_params.wind_dir * cloud_params.wind_speed * cloud_params.time * 0.01;
        let sample_pos = world_pos * 0.0003 + wind_offset;
        let n = fbm(sample_pos, 5);
        let coverage_mask = smoothstep(1.0 - cloud_params.coverage, 1.0, n);
        let height_profile = smoothstep(0.0, 0.2, shell_norm) * (1.0 - smoothstep(0.85, 1.0, shell_norm));
        return coverage_mask * height_profile * cloud_params.density;
}

fn intersect_cloud_shell(ray_origin: vec3<f32>, ray_dir: vec3<f32>) -> vec2<f32> {
    let bottom = cloud_params.shell_bottom;
        let top = cloud_params.shell_top;
        let t_bottom = (bottom - ray_origin.y) / max(ray_dir.y, 1e-5);
        let t_top = (top - ray_origin.y) / max(ray_dir.y, 1e-5);
        let t_min = min(t_bottom, t_top);
        let t_max = max(t_bottom, t_top);
        if (t_max < 0.0) {
            return vec2(-1.0, -1.0);
        }
        return vec2(max(t_min, 0.0), t_max);
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let width = cloud_params.width;
        let height = cloud_params.height;
        if (id.x >= width || id.y >= height) { return; }
    
        let pixel_idx = id.y * width + id.x;
        let scene = color_output[pixel_idx];
        let gbuf = gbuffer[pixel_idx];
        let depth = gbuf.depth;
        let mat = gbuf.normal.w;
        let has_geometry = (mat > 0.0) && (depth < camera.far_plane) && (scene.a > 0.0);
    
        let pixel = vec2<f32>(f32(id.x), f32(id.y));
        let ray_dir = generate_ray(pixel + vec2(0.5, 0.5), camera);
        let ray_origin = camera.position;
    
        let shell = intersect_cloud_shell(ray_origin, ray_dir);
        if (shell.x < 0.0) {
            return;
        }
    
        var t_end = shell.y;
        if (has_geometry) {
            t_end = min(t_end, depth);
        }
        if (t_end <= shell.x) {
            return;
        }
    
        let steps = 12u;
        let step_len = (t_end - shell.x) / f32(steps);
        var transmittance = 1.0;
        var scattered = vec3(0.0);
        let sun_dir = normalize(vec3(1.0, 0.4, 0.2));
        var t = shell.x + step_len * 0.5;
    
        for (var i = 0u; i < steps; i++) {
            let pos = ray_origin + ray_dir * t;
            let density = cloud_density(pos);
            if (density > 0.0001) {
                let light_t = vol_march_transmittance_steps(
                    pos, sun_dir, 0.0, 2000.0, 4u, cloud_params.absorption, density
                );
                let cos_theta = dot(ray_dir, sun_dir);
                let phase = vol_hg_phase(cos_theta, cloud_params.phase_g);
                let sun_contrib = vec3(1.0, 0.95, 0.85) * light_t * phase * cloud_params.silver_intensity;
                let ambient = vec3(0.4, 0.45, 0.55) * 0.15;
                let sample_radiance = (sun_contrib + ambient) * density;
                scattered = scattered + transmittance * sample_radiance * step_len;
                transmittance = transmittance * vol_beer_lambert(density * cloud_params.absorption, step_len);
            }
            if (transmittance < 0.01) { break; }
            t = t + step_len;
        }
    
        let out_rgb = scene.rgb * transmittance + scattered;
        color_output[pixel_idx] = vec4(out_rgb, 1.0);
}


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

@group(0) @binding(0) var<uniform> params: AtmosphereParams;
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

fn hash_2d(p: vec2<f32>) -> f32 {
    let h = dot(p, vec2(127.1, 311.7));
        return fract(sin(h) * 43758.5453);
}

fn hash_3d(p: vec3<f32>) -> f32 {
    let h = dot(p, vec3(127.1, 311.7, 74.7));
        return fract(sin(h) * 43758.5453);
}

fn value_noise(p: vec3<f32>) -> f32 {
    let i = floor(p);
        let f = fract(p);
        let u = f * f * (3.0 - 2.0 * f);
    
        let n000 = hash_3d(i + vec3(0.0, 0.0, 0.0));
        let n100 = hash_3d(i + vec3(1.0, 0.0, 0.0));
        let n010 = hash_3d(i + vec3(0.0, 1.0, 0.0));
        let n110 = hash_3d(i + vec3(1.0, 1.0, 0.0));
        let n001 = hash_3d(i + vec3(0.0, 0.0, 1.0));
        let n101 = hash_3d(i + vec3(1.0, 0.0, 1.0));
        let n011 = hash_3d(i + vec3(0.0, 1.0, 1.0));
        let n111 = hash_3d(i + vec3(1.0, 1.0, 1.0));
    
        let n00 = mix(n000, n100, u.x);
        let n10 = mix(n010, n110, u.x);
        let n01 = mix(n001, n101, u.x);
        let n11 = mix(n011, n111, u.x);
    
        let n0 = mix(n00, n10, u.y);
        let n1 = mix(n01, n11, u.y);
    
        return mix(n0, n1, u.z);
}

fn fbm(p: vec3<f32>, octaves: i32) -> f32 {
    var value = 0.0;
        var amplitude = 0.5;
        var pos = p;
        for (var i = 0; i < octaves; i++) {
            value += amplitude * value_noise(pos);
            pos = pos * 2.1;
            amplitude *= 0.5;
        }
        return value;
}

fn starfield(dir: vec3<f32>) -> vec3<f32> {
    var stars = vec3(0.0);
    
        let grid_scale = 120.0;
        let cell = floor(dir * grid_scale);
        let cell_fract = fract(dir * grid_scale);
        let r = hash_3d(cell);
    
        if (r > 0.92) {
            let center = vec2(hash_3d(cell + vec3(1.0, 0.0, 0.0)),
                              hash_3d(cell + vec3(0.0, 1.0, 0.0)));
            let dist = length(cell_fract.xy - center);
            let brightness = smoothstep(0.12, 0.0, dist) * (0.4 + r * 0.6);
            let temp = hash_3d(cell + vec3(0.0, 0.0, 1.0));
            var color = vec3(1.0);
            if (temp < 0.3) {
                color = vec3(0.7, 0.8, 1.0);
            } else if (temp < 0.5) {
                color = vec3(1.0, 0.9, 0.7);
            } else if (temp < 0.65) {
                color = vec3(1.0, 0.6, 0.4);
            }
            let twinkle = 0.7 + 0.3 * sin(r * 100.0 + hash_3d(cell * 3.0) * 6.28);
            stars = color * brightness * twinkle;
        }
    
        let grid2 = 60.0;
        let cell2 = floor(dir * grid2);
        let cf2 = fract(dir * grid2);
        let r2 = hash_3d(cell2 + vec3(42.0, 0.0, 0.0));
        if (r2 > 0.97) {
            let c2 = vec2(hash_3d(cell2 + vec3(7.0, 0.0, 0.0)),
                           hash_3d(cell2 + vec3(0.0, 7.0, 0.0)));
            let d2 = length(cf2.xy - c2);
            let b2 = smoothstep(0.06, 0.0, d2) * 1.5;
            stars += vec3(0.8, 0.85, 1.0) * b2;
        }
    
        return stars;
}

fn nebula(dir: vec3<f32>) -> vec3<f32> {
    let p = dir * 2.5;
    
        let n1 = fbm(p + vec3(0.0, 0.0, 0.0), 4);
        let n2 = fbm(p + vec3(5.2, 1.3, 0.0), 4);
        let n3 = fbm(p + vec3(0.0, 3.7, 2.1), 3);
    
        let deep_blue = vec3(0.02, 0.03, 0.12) * smoothstep(0.3, 0.7, n1);
        let purple = vec3(0.06, 0.01, 0.10) * smoothstep(0.4, 0.8, n2);
        let magenta = vec3(0.04, 0.005, 0.03) * smoothstep(0.5, 0.9, n3);
    
        let galactic_band = exp(-8.0 * dir.y * dir.y) * 0.015;
        let band_color = vec3(0.03, 0.02, 0.06) * galactic_band;
    
        return deep_blue + purple + magenta + band_color;
}

fn sun_and_corona(ray_dir: vec3<f32>) -> vec3<f32> {
    let sun_dir = normalize(params.sun_direction);
        let cos_sun = dot(ray_dir, sun_dir);
        let s = params.sun_size;
    
        let disc = smoothstep(1.0 - s * 0.5, 1.0, cos_sun);
        let inner_corona = smoothstep(1.0 - s * 4.0, 1.0 - s * 0.3, cos_sun) * 0.4;
        let outer_corona = smoothstep(1.0 - s * 16.0, 1.0 - s * 2.0, cos_sun) * 0.08;
    
        let sun_term = disc + inner_corona + outer_corona;
    
        let perp = ray_dir - sun_dir * cos_sun;
        let streak_x = abs(perp.x);
        let streak_y = abs(perp.y);
        let flare_h = exp(-streak_y * 40.0 / s) * exp(-streak_x * 8.0 / s) * 0.12;
        let flare_v = exp(-streak_x * 40.0 / s) * exp(-streak_y * 8.0 / s) * 0.06;
        let flare = (flare_h + flare_v) * smoothstep(1.0 - s * 20.0, 1.0 - s * 3.0, cos_sun);
    
        return params.sun_color * (sun_term + flare) * params.sun_intensity;
}

fn compute_sky_color(ray_dir: vec3<f32>) -> vec3<f32> {
    let y = ray_dir.y;
        let up_blend = clamp(y, 0.0, 1.0);
        var base = mix(params.sky_horizon_color, params.sky_top_color, up_blend);
        let down_blend = clamp(-y, 0.0, 1.0);
        base = mix(base, params.ground_color, down_blend);
    
        let stars = starfield(ray_dir);
        let neb = nebula(ray_dir);
        let sun = sun_and_corona(ray_dir);
    
        return base + stars + neb + sun;
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
    
        let scene = color_output[pixel_idx];
        let gbuf = gbuffer[pixel_idx];
        let mat = gbuf.normal.w;
        let depth = gbuf.depth;
        let no_geometry = (mat == 0.0) || (depth >= camera.far_plane) || (scene.a == 0.0);
    
        if (no_geometry) {
            if (scene.a > 0.5) {
                // Interior: the lighting pass already filled this pixel with
                // an ambient gradient. Keep it and apply gentle fog to push
                // missed-ray areas toward a neutral interior color.
                let fogged = mix(scene.xyz, params.fog_color, 0.3);
                color_output[pixel_idx] = vec4(fogged, 1.0);
            } else {
                // Truly empty pixel (e.g. through a window to space).
                let pixel = vec2<f32>(f32(id.x), f32(id.y));
                let ray_dir = generate_ray(pixel + vec2(0.5, 0.5), camera);
                let sky = compute_sky_color(ray_dir);
                color_output[pixel_idx] = vec4(sky, 1.0);
            }
            return;
        }
    
        let fog_factor = 1.0 - exp(-depth * params.fog_density);
        let lit = mix(scene.xyz, params.fog_color, fog_factor);
        color_output[pixel_idx] = vec4(lit, 1.0);
}


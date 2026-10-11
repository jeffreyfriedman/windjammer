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

struct AreaLightData {
    corner0: vec3<f32>,
    _pad0: f32,
    corner1: vec3<f32>,
    _pad1: f32,
    corner2: vec3<f32>,
    _pad2: f32,
    corner3: vec3<f32>,
    _pad3: f32,
    color: vec3<f32>,
    _pad4: f32,
    intensity: f32,
    _pad5: f32,
    _pad6: f32,
    _pad7: f32,
}

struct AreaLightParams {
    light_count: u32,
    width: u32,
    height: u32,
    _pad: u32,
}

@group(0) @binding(0) var<uniform> params: AreaLightParams;
@group(0) @binding(1) var<storage, read> gbuffer: array<u32>;
@group(0) @binding(2) var<storage, read_write> color_buffer: array<u32>;
@group(0) @binding(3) var<storage, read> lights: array<AreaLightData>;
@group(0) @binding(4) var<uniform> camera: CameraUniforms;
const PI: f32 = 3.14159265;
const MAX_AREA_LIGHTS: u32 = 16u;

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

fn fresnel_schlick(cos_theta: f32, f0: vec3<f32>) -> vec3<f32> {
    return f0 + (1.0 - f0) * pow(1.0 - cos_theta, 5.0);
}

fn distribution_ggx(n: vec3<f32>, h: vec3<f32>, roughness: f32) -> f32 {
    let a = roughness * roughness;
        let a2 = a * a;
        let n_dot_h = max(dot(n, h), 0.0);
        let n_dot_h2 = n_dot_h * n_dot_h;
        let denom = n_dot_h2 * (a2 - 1.0) + 1.0;
        return a2 / max(PI * denom * denom, 0.0001);
}

fn geometry_schlick_ggx(n_dot_v: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
        let k = (r * r) / 8.0;
        return n_dot_v / (n_dot_v * (1.0 - k) + k);
}

fn geometry_smith(n: vec3<f32>, v: vec3<f32>, l: vec3<f32>, roughness: f32) -> f32 {
    let n_dot_v = max(dot(n, v), 0.0);
        let n_dot_l = max(dot(n, l), 0.0);
        return geometry_schlick_ggx(n_dot_v, roughness) * geometry_schlick_ggx(n_dot_l, roughness);
}

fn pbr_area_light(albedo: vec3<f32>, roughness: f32, metallic: f32, n: vec3<f32>, v: vec3<f32>, l: vec3<f32>, radiance: vec3<f32>) -> vec3<f32> {
    let h = normalize(v + l);
        var f0 = vec3<f32>(0.04, 0.04, 0.04);
        f0 = mix(f0, albedo, metallic);
        let n_dot_l = max(dot(n, l), 0.0);
        let n_dot_v = max(dot(n, v), 0.0);
        let k_d = (1.0 - fresnel_schlick(max(dot(h, v), 0.0), f0)) * (1.0 - metallic);
        let diffuse = ((k_d * albedo) / PI) * radiance * n_dot_l;
        let ndf = distribution_ggx(n, h, roughness);
        let g = geometry_smith(n, v, l, roughness);
        let f = fresnel_schlick(max(dot(h, v), 0.0), f0);
        let specular = (ndf * g * f) / (4.0 * n_dot_v * n_dot_l + 0.0001) * radiance * n_dot_l;
        return diffuse + specular;
}

fn quad_center(light: AreaLightData) -> vec3<f32> {
    return 0.25 * (light.corner0 + light.corner1 + light.corner2 + light.corner3);
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= params.width || id.y >= params.height) {
            return;
        }
    
        let pixel_idx = id.y * params.width + id.x;
        let gb_base = pixel_idx * 4u;
    
        let gb0 = bitcast<f32>(gbuffer[gb_base]);
        let gb1 = bitcast<f32>(gbuffer[gb_base + 1u]);
        let gb2 = bitcast<f32>(gbuffer[gb_base + 2u]);
        let gb3 = bitcast<f32>(gbuffer[gb_base + 3u]);
    
        let depth = gb3;
        if (depth >= 0.99999) {
            return;
        }
    
        let n = normalize(vec3<f32>(gb0, gb1, gb2));
    
        let uv = vec2<f32>(
            (f32(id.x) + 0.5) / f32(params.width),
            (f32(id.y) + 0.5) / f32(params.height)
        );
        let ndc = vec4<f32>(uv.x * 2.0 - 1.0, -(uv.y * 2.0 - 1.0), depth, 1.0);
        let inv_vp = camera.inv_view * camera.inv_proj;
        var world_pos4 = inv_vp * ndc;
        let world_pos = world_pos4.xyz / world_pos4.w;
    
        let v = normalize(camera.position - world_pos);
    
        let albedo = vec3<f32>(0.5, 0.5, 0.5);
        let roughness = 0.5;
        let metallic = 0.0;
    
        var total_light = vec3<f32>(0.0, 0.0, 0.0);
        let count = min(params.light_count, MAX_AREA_LIGHTS);
    
        for (var i = 0u; i < MAX_AREA_LIGHTS; i++) {
            if (i >= count) {
                break;
            }
            let light = lights[i];
            let center = quad_center(light);
            let to_center = center - world_pos;
            let distance = length(to_center);
            let l = to_center / max(distance, 0.001);
    
            let attenuation = 1.0 / max(distance * distance, 0.01);
            let e1 = light.corner1 - light.corner0;
            let e2 = light.corner2 - light.corner0;
            let quad_n = normalize(cross(e1, e2));
            let facing = max(dot(quad_n, -l), 0.0);
            let area = length(e1) * length(e2);
            let radiance = light.color * light.intensity * attenuation * facing * (1.0 + area * 0.1);
    
            total_light += pbr_area_light(albedo, roughness, metallic, n, v, l, radiance);
        }
    
        let existing = bitcast<vec4<f32>>(vec4<u32>(
            color_buffer[pixel_idx * 4u],
            color_buffer[pixel_idx * 4u + 1u],
            color_buffer[pixel_idx * 4u + 2u],
            color_buffer[pixel_idx * 4u + 3u],
        ));
        let result = existing.rgb + total_light;
        color_buffer[pixel_idx * 4u] = bitcast<u32>(result.r);
        color_buffer[pixel_idx * 4u + 1u] = bitcast<u32>(result.g);
        color_buffer[pixel_idx * 4u + 2u] = bitcast<u32>(result.b);
}


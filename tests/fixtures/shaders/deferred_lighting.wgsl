// Deferred Lighting Pass (Fullscreen Compute)
//
// Reads G-buffer targets and accumulates lighting from:
// - Directional sun light with CSM shadows
// - N point lights with quadratic attenuation
// - M spot lights with cone falloff
// - Hemisphere ambient lighting
//
// Output: HDR color buffer (RGBA16Float storage buffer packed as vec4<f32>)

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

struct SunLight {
    direction: vec3<f32>,
    _pad0: f32,
    color: vec3<f32>,
    intensity: f32,
}

struct AmbientLight {
    sky_color: vec3<f32>,
    _pad0: f32,
    ground_color: vec3<f32>,
    ambient_intensity: f32,
}

struct PointLight {
    position: vec3<f32>,
    radius: f32,
    color: vec3<f32>,
    intensity: f32,
}

struct SpotLight {
    position: vec3<f32>,
    radius: f32,
    direction: vec3<f32>,
    inner_cone: f32,
    color: vec3<f32>,
    outer_cone: f32,
    intensity: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
}

struct LightCounts {
    point_light_count: u32,
    spot_light_count: u32,
    width: u32,
    height: u32,
}

// G-buffer inputs (as textures)
@group(0) @binding(0) var gbuf_albedo_metallic: texture_2d<f32>;
@group(0) @binding(1) var gbuf_normal_roughness: texture_2d<f32>;
@group(0) @binding(2) var gbuf_emission_ao: texture_2d<f32>;
@group(0) @binding(3) var gbuf_depth: texture_depth_2d;

// Uniforms
@group(0) @binding(4) var<uniform> camera: CameraUniforms;
@group(0) @binding(5) var<uniform> sun: SunLight;
@group(0) @binding(6) var<uniform> ambient: AmbientLight;
@group(0) @binding(7) var<uniform> light_counts: LightCounts;

// Dynamic light arrays
@group(1) @binding(0) var<storage, read> point_lights: array<PointLight>;
@group(1) @binding(1) var<storage, read> spot_lights: array<SpotLight>;

// Output HDR color
@group(1) @binding(2) var<storage, read_write> output_color: array<u32>;

// Shadow maps (cascade depth textures + sampler)
struct ShadowCascadeData {
    light_view_proj: mat4x4<f32>,
    split_near: f32,
    split_far: f32,
    bias: f32,
    _pad: f32,
}

@group(2) @binding(0) var shadow_cascade_0: texture_depth_2d;
@group(2) @binding(1) var shadow_cascade_1: texture_depth_2d;
@group(2) @binding(2) var shadow_cascade_2: texture_depth_2d;
@group(2) @binding(3) var shadow_cascade_3: texture_depth_2d;
@group(2) @binding(4) var shadow_sampler: sampler_comparison;
@group(2) @binding(5) var<uniform> shadow_cascades: array<ShadowCascadeData, 4>;
@group(2) @binding(6) var<uniform> shadow_enabled: u32;

// Octahedral decode
fn octahedral_decode(e: vec2<f32>) -> vec3<f32> {
    let p = e * 2.0 - 1.0;
    let z = 1.0 - abs(p.x) - abs(p.y);
    var n: vec3<f32>;
    if (z >= 0.0) {
        n = vec3<f32>(p.x, p.y, z);
    } else {
        let signs = vec2<f32>(
            select(-1.0, 1.0, p.x >= 0.0),
            select(-1.0, 1.0, p.y >= 0.0)
        );
        n = vec3<f32>((1.0 - abs(p.y)) * signs.x, (1.0 - abs(p.x)) * signs.y, z);
    }
    return normalize(n);
}

// Reconstruct world position from depth
fn world_pos_from_depth(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    let ndc = vec4<f32>(uv * 2.0 - 1.0, depth, 1.0);
    let ndc_flipped = vec4<f32>(ndc.x, -ndc.y, ndc.z, 1.0);
    var world_pos = camera.inv_proj * ndc_flipped;
    world_pos = world_pos / world_pos.w;
    world_pos = camera.inv_view * world_pos;
    return world_pos.xyz;
}

// PBR functions
fn fresnel_schlick(cos_theta: f32, f0: vec3<f32>) -> vec3<f32> {
    let t = 1.0 - cos_theta;
    let t2 = t * t;
    let t5 = t2 * t2 * t;
    return f0 + (vec3<f32>(1.0) - f0) * t5;
}

fn distribution_ggx(n_dot_h: f32, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let d = n_dot_h * n_dot_h * (a2 - 1.0) + 1.0;
    return a2 / (3.14159265 * d * d + 0.0001);
}

fn geometry_smith(n_dot_v: f32, n_dot_l: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = r * r / 8.0;
    let g1 = n_dot_v / (n_dot_v * (1.0 - k) + k);
    let g2 = n_dot_l / (n_dot_l * (1.0 - k) + k);
    return g1 * g2;
}

fn cook_torrance(
    N: vec3<f32>, V: vec3<f32>, L: vec3<f32>,
    albedo: vec3<f32>, roughness: f32, metallic: f32,
) -> vec3<f32> {
    let H = normalize(V + L);
    let n_dot_v = max(dot(N, V), 0.001);
    let n_dot_l = max(dot(N, L), 0.0);
    let n_dot_h = max(dot(N, H), 0.0);
    let v_dot_h = max(dot(V, H), 0.0);

    let f0 = mix(vec3<f32>(0.04), albedo, metallic);
    let F = fresnel_schlick(v_dot_h, f0);
    let D = distribution_ggx(n_dot_h, roughness);
    let G = geometry_smith(n_dot_v, n_dot_l, roughness);

    let specular = (D * F * G) / (4.0 * n_dot_v * n_dot_l + 0.0001);
    let kd = (vec3<f32>(1.0) - F) * (1.0 - metallic);
    let diffuse = kd * albedo / 3.14159265;

    return (diffuse + specular) * n_dot_l;
}

fn hemisphere_ambient(normal: vec3<f32>, albedo: vec3<f32>) -> vec3<f32> {
    let sky_factor = normal.y * 0.5 + 0.5;
    let sky_ambient = mix(ambient.ground_color, ambient.sky_color, sky_factor);
    return albedo * sky_ambient * ambient.ambient_intensity;
}

fn point_light_attenuation(distance: f32, radius: f32) -> f32 {
    let d = max(distance, 0.001);
    let att = 1.0 / (d * d);
    let falloff = 1.0 - smoothstep(radius * 0.75, radius, distance);
    return att * falloff;
}

// PCF shadow sampling with 4-tap Poisson disk
fn sample_shadow_cascade(cascade_idx: u32, world_pos: vec3<f32>, normal: vec3<f32>) -> f32 {
    if shadow_enabled == 0u {
        return 1.0;
    }

    let cascade = shadow_cascades[cascade_idx];

    // Bias based on surface angle to light
    let bias = cascade.bias + max(cascade.bias * 3.0 * (1.0 - dot(normal, normalize(-sun.direction))), 0.0);

    // Project world position into shadow map space
    let light_clip = cascade.light_view_proj * vec4<f32>(world_pos + normal * bias, 1.0);
    let light_ndc = light_clip.xyz / light_clip.w;
    let shadow_uv = vec2<f32>(light_ndc.x * 0.5 + 0.5, -light_ndc.y * 0.5 + 0.5);
    let shadow_depth = light_ndc.z;

    // Out of bounds check
    if shadow_uv.x < 0.0 || shadow_uv.x > 1.0 || shadow_uv.y < 0.0 || shadow_uv.y > 1.0 {
        return 1.0;
    }

    // 4-tap PCF with Poisson offsets
    let texel_size = 1.0 / 2048.0;
    let offsets = array<vec2<f32>, 4>(
        vec2<f32>(-0.94201624, -0.39906216),
        vec2<f32>(0.94558609, -0.76890725),
        vec2<f32>(-0.09418410, -0.92938870),
        vec2<f32>(0.34495938, 0.29387760),
    );

    var shadow = 0.0;
    for (var i = 0u; i < 4u; i++) {
        let sample_uv = shadow_uv + offsets[i] * texel_size * 2.0;
        var s: f32;
        if cascade_idx == 0u {
            s = textureSampleCompare(shadow_cascade_0, shadow_sampler, sample_uv, shadow_depth);
        } else if cascade_idx == 1u {
            s = textureSampleCompare(shadow_cascade_1, shadow_sampler, sample_uv, shadow_depth);
        } else if cascade_idx == 2u {
            s = textureSampleCompare(shadow_cascade_2, shadow_sampler, sample_uv, shadow_depth);
        } else {
            s = textureSampleCompare(shadow_cascade_3, shadow_sampler, sample_uv, shadow_depth);
        }
        shadow += s;
    }

    return shadow / 4.0;
}

// Select cascade based on view-space depth
fn select_cascade(view_depth: f32) -> u32 {
    for (var i = 0u; i < 4u; i++) {
        if view_depth < shadow_cascades[i].split_far {
            return i;
        }
    }
    return 3u;
}

fn pack_rgba_to_u32(c: vec4<f32>) -> u32 {
    let r = u32(clamp(c.r * 255.0, 0.0, 255.0));
    let g = u32(clamp(c.g * 255.0, 0.0, 255.0));
    let b = u32(clamp(c.b * 255.0, 0.0, 255.0));
    let a = u32(clamp(c.a * 255.0, 0.0, 255.0));
    return r | (g << 8u) | (b << 16u) | (a << 24u);
}

// IBL support
@group(3) @binding(0) var ibl_diffuse: texture_cube<f32>;
@group(3) @binding(1) var ibl_specular: texture_cube<f32>;
@group(3) @binding(2) var brdf_lut: texture_2d<f32>;
@group(3) @binding(3) var ibl_sampler: sampler;
@group(3) @binding(4) var<uniform> ibl_enabled: u32;

fn ibl_ambient(
    N: vec3<f32>, V: vec3<f32>,
    albedo: vec3<f32>, metallic: f32, roughness: f32, ao: f32,
) -> vec3<f32> {
    if ibl_enabled == 0u {
        return vec3<f32>(0.0);
    }

    let f0 = mix(vec3<f32>(0.04), albedo, metallic);
    let n_dot_v = max(dot(N, V), 0.0);

    // Diffuse irradiance
    let irradiance = textureSample(ibl_diffuse, ibl_sampler, N).rgb;
    let kd = (vec3<f32>(1.0) - fresnel_schlick(n_dot_v, f0)) * (1.0 - metallic);
    let diffuse = kd * albedo * irradiance;

    // Specular IBL with split-sum
    let R = reflect(-V, N);
    let max_mip = f32(textureNumLevels(ibl_specular) - 1u);
    let prefiltered = textureSampleLevel(ibl_specular, ibl_sampler, R, roughness * max_mip).rgb;

    let brdf_uv = vec2<f32>(n_dot_v, roughness);
    let brdf = textureSample(brdf_lut, ibl_sampler, brdf_uv).rg;
    let specular = prefiltered * (f0 * brdf.x + brdf.y);

    return (diffuse + specular) * ao;
}

// ACES tonemapping
fn aces_tonemap(color: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((color * (a * color + b)) / (color * (c * color + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}

fn linear_to_srgb(c: vec3<f32>) -> vec3<f32> {
    return pow(c, vec3<f32>(1.0 / 2.2));
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let width = light_counts.width;
    let height = light_counts.height;

    if (id.x >= width || id.y >= height) {
        return;
    }

    let pixel = vec2<i32>(i32(id.x), i32(id.y));
    let uv = vec2<f32>((f32(id.x) + 0.5) / f32(width), (f32(id.y) + 0.5) / f32(height));

    // Sample G-buffer
    let albedo_metallic = textureLoad(gbuf_albedo_metallic, pixel, 0);
    let normal_roughness = textureLoad(gbuf_normal_roughness, pixel, 0);
    let emission_ao = textureLoad(gbuf_emission_ao, pixel, 0);
    let depth = textureLoad(gbuf_depth, pixel, 0);

    // Early out for sky pixels
    if (depth >= 1.0) {
        let idx = id.y * width + id.x;
        output_color[idx] = pack_rgba_to_u32(vec4<f32>(0.0, 0.0, 0.0, 0.0));
        return;
    }

    // Unpack G-buffer
    let albedo = albedo_metallic.rgb;
    let metallic = albedo_metallic.a;
    let N = octahedral_decode(normal_roughness.rg);
    let roughness = normal_roughness.b;
    let emission = emission_ao.rgb;
    let ao = emission_ao.a;

    let world_pos = world_pos_from_depth(uv, depth);
    let V = normalize(camera.position - world_pos);

    // Compute view-space depth for cascade selection
    let view_pos = camera.view_matrix * vec4<f32>(world_pos, 1.0);
    let view_depth = -view_pos.z;

    // Shadow factor from CSM
    let cascade_idx = select_cascade(view_depth);
    let shadow_factor = sample_shadow_cascade(cascade_idx, world_pos, N);

    // Sun directional light (modulated by shadow)
    let L_sun = normalize(-sun.direction);
    let sun_contrib = cook_torrance(N, V, L_sun, albedo, roughness, metallic)
                      * sun.color * sun.intensity * shadow_factor;

    // Point lights
    var point_contrib = vec3<f32>(0.0);
    let num_points = min(light_counts.point_light_count, 128u);
    for (var i = 0u; i < num_points; i++) {
        let pl = point_lights[i];
        let to_light = pl.position - world_pos;
        let dist = length(to_light);
        if (dist > pl.radius) {
            continue;
        }
        let L = normalize(to_light);
        let att = point_light_attenuation(dist, pl.radius);
        point_contrib += cook_torrance(N, V, L, albedo, roughness, metallic)
                         * pl.color * pl.intensity * att;
    }

    // Spot lights
    var spot_contrib = vec3<f32>(0.0);
    let num_spots = min(light_counts.spot_light_count, 64u);
    for (var i = 0u; i < num_spots; i++) {
        let sl = spot_lights[i];
        let to_light = sl.position - world_pos;
        let dist = length(to_light);
        if (dist > sl.radius) {
            continue;
        }
        let L = normalize(to_light);
        let theta = dot(L, normalize(-sl.direction));
        let epsilon = sl.inner_cone - sl.outer_cone;
        let cone_att = clamp((theta - sl.outer_cone) / max(epsilon, 0.001), 0.0, 1.0);
        let dist_att = point_light_attenuation(dist, sl.radius);
        spot_contrib += cook_torrance(N, V, L, albedo, roughness, metallic)
                        * sl.color * sl.intensity * dist_att * cone_att;
    }

    // Ambient: IBL when available, hemisphere ambient as fallback
    var ambient_contrib = vec3<f32>(0.0);
    if ibl_enabled > 0u {
        ambient_contrib = ibl_ambient(N, V, albedo, metallic, roughness, ao);
    } else {
        ambient_contrib = hemisphere_ambient(N, albedo) * ao;
    }

    let hdr_color = sun_contrib + point_contrib + spot_contrib + ambient_contrib + emission;

    // Tonemap and gamma correct for output
    let tonemapped = aces_tonemap(hdr_color);
    let srgb = linear_to_srgb(tonemapped);

    let idx = id.y * width + id.x;
    output_color[idx] = pack_rgba_to_u32(vec4<f32>(srgb, 1.0));
}

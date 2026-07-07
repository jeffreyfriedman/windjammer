// Point Light Shader - PBR Lighting
//
// Computes lighting contribution from a point light:
// - Inverse square attenuation
// - Lambert diffuse + GGX specular (energy-conserving)
// - Input: GBuffer-like fragment data (position, normal, albedo, roughness, metallic)
// - Output: RGB lighting contribution
//
// Workgroup: 1x1 for single-fragment tests; can be 8x8 for full-screen

struct FragmentData {
    position: vec3<f32>,
    _pad1: f32,
    normal: vec3<f32>,
    _pad2: f32,
    albedo: vec3<f32>,
    _pad3: f32,
    roughness: f32,
    metallic: f32,
    _pad4: f32,
    _pad5: f32,
}

struct PointLightParams {
    position: vec3<f32>,
    _pad1: f32,
    color: vec3<f32>,
    _pad2: f32,
    intensity: f32,
    radius: f32,
    _pad3: f32,
    _pad4: f32,
}

@group(0) @binding(0) var<storage, read> fragment: array<FragmentData>;
@group(0) @binding(1) var<uniform> light: PointLightParams;
@group(0) @binding(2) var<storage, read_write> output: array<vec4<f32>>;

const PI: f32 = 3.14159265;

// Fresnel-Schlick approximation
fn fresnel_schlick(cos_theta: f32, f0: vec3<f32>) -> vec3<f32> {
    return f0 + (1.0 - f0) * pow(1.0 - cos_theta, 5.0);
}

// GGX/Trowbridge-Reitz normal distribution
fn distribution_ggx(n: vec3<f32>, h: vec3<f32>, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let n_dot_h = max(dot(n, h), 0.0);
    let n_dot_h2 = n_dot_h * n_dot_h;
    let denom = n_dot_h2 * (a2 - 1.0) + 1.0;
    denom = PI * denom * denom;
    return a2 / max(denom, 0.0001);
}

// Schlick-GGX geometry
fn geometry_schlick_ggx(n_dot_v: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = (r * r) / 8.0;
    return n_dot_v / (n_dot_v * (1.0 - k) + k);
}

fn geometry_smith(n: vec3<f32>, v: vec3<f32>, l: vec3<f32>, roughness: f32) -> f32 {
    let n_dot_v = max(dot(n, v), 0.0);
    let n_dot_l = max(dot(n, l), 0.0);
    let ggx_v = geometry_schlick_ggx(n_dot_v, roughness);
    let ggx_l = geometry_schlick_ggx(n_dot_l, roughness);
    return ggx_v * ggx_l;
}

// PBR: diffuse + specular for a single light
fn pbr_lighting(
    albedo: vec3<f32>,
    roughness: f32,
    metallic: f32,
    n: vec3<f32>,
    v: vec3<f32>,
    l: vec3<f32>,
    radiance: vec3<f32>
) -> vec3<f32> {
    let h = normalize(v + l);
    var f0 = vec3<f32>(0.04, 0.04, 0.04);
    f0 = mix(f0, albedo, metallic);

    let n_dot_l = max(dot(n, l), 0.0);
    let n_dot_v = max(dot(n, v), 0.0);

    // Diffuse (Lambert, energy-conserving with metallic)
    let k_d = (1.0 - fresnel_schlick(max(dot(h, v), 0.0), f0)) * (1.0 - metallic);
    let diffuse = ((k_d * albedo) / PI) * radiance * n_dot_l;

    // Specular (Cook-Torrance)
    let ndf = distribution_ggx(n, h, roughness);
    let g = geometry_smith(n, v, l, roughness);
    let f = fresnel_schlick(max(dot(h, v), 0.0), f0);
    let numerator = ndf * g * f;
    let denominator = 4.0 * n_dot_v * n_dot_l + 0.0001;
    let specular = (numerator / denominator) * radiance * n_dot_l;

    return diffuse + specular;
}

@compute @workgroup_size(1)
fn main() {
    let frag = fragment[0];
    let pos = frag.position;
    let n = normalize(frag.normal);
    let albedo = frag.albedo;
    let roughness = max(frag.roughness, 0.04);
    let metallic = frag.metallic;

    let light_vec = light.position - pos;
    let distance = length(light_vec);
    let l = light_vec / max(distance, 0.001);

    // Inverse square attenuation
    let attenuation = 1.0 / max(distance * distance, 0.01);
    let radiance = light.color * light.intensity * attenuation;

    // View direction (from fragment toward camera)
    // When pos is at origin, use +Z to avoid normalize(0)
    let to_camera = select(-pos, vec3<f32>(0.0, 0.0, 1.0), length(pos) < 0.001);
    let v = normalize(to_camera);

    let color = pbr_lighting(albedo, roughness, metallic, n, v, l, radiance);
    output[0] = vec4<f32>(color, 1.0);
}

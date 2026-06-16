// Area Light Shader - Quad Light with Soft Approximation
//
// Approximates a quad area light by sampling the center (or multiple points).
// For simplicity: use center point + distance-based soft falloff.
// Full LTC would require more complex integration.
//
// Quad: 4 corners (vec3 each). We use center + averaged direction for soft falloff.

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

struct AreaLightParams {
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
}

@group(0) @binding(0) var<storage, read> fragment: array<FragmentData>;
@group(0) @binding(1) var<uniform> light: AreaLightParams;
@group(0) @binding(2) var<storage, read_write> output: array<vec4<f32>>;

const PI: f32 = 3.14159265;

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
    let k_d = (1.0 - fresnel_schlick(max(dot(h, v), 0.0), f0)) * (1.0 - metallic);
    let diffuse = ((k_d * albedo) / PI) * radiance * n_dot_l;
    let ndf = distribution_ggx(n, h, roughness);
    let g = geometry_smith(n, v, l, roughness);
    let f = fresnel_schlick(max(dot(h, v), 0.0), f0);
    let specular = (ndf * g * f) / (4.0 * n_dot_v * n_dot_l + 0.0001) * radiance * n_dot_l;
    return diffuse + specular;
}

fn quad_center() -> vec3<f32> {
    return 0.25 * (light.corner0 + light.corner1 + light.corner2 + light.corner3);
}

@compute @workgroup_size(1)
fn main() {
    let frag = fragment[0];
    let pos = frag.position;
    let n = normalize(frag.normal);
    let albedo = frag.albedo;
    let roughness = max(frag.roughness, 0.04);
    let metallic = frag.metallic;

    let center = quad_center();
    let to_center = center - pos;
    let distance = length(to_center);
    let l = to_center / max(distance, 0.001);

    let attenuation = 1.0 / max(distance * distance, 0.01);
    let e1 = light.corner1 - light.corner0;
    let e2 = light.corner2 - light.corner0;
    let quad_n = normalize(cross(e1, e2));
    let n_dot_l_quad = max(dot(quad_n, -l), 0.0);
    let n_dot_l_frag = max(dot(n, l), 0.0);
    let area = length(e1) * length(e2);
    let radiance = light.color * light.intensity * attenuation * n_dot_l_quad * n_dot_l_frag * (1.0 + area * 0.1);

    let to_camera = select(-pos, vec3<f32>(0.0, 0.0, 1.0), length(pos) < 0.001);
    let v = normalize(to_camera);

    let color = pbr_lighting(albedo, roughness, metallic, n, v, l, radiance);
    output[0] = vec4<f32>(color, 1.0);
}

struct Material {
    base_color: vec4<f32>,
    metallic: f32,
    roughness: f32,
    ao: f32,
    emissive: vec3<f32>,
}

struct Light {
    position: vec3<f32>,
    color: vec3<f32>,
    intensity: f32,
}

struct CameraUniforms {
    view_matrix: mat4x4<f32>,
    proj_matrix: mat4x4<f32>,
    position: vec3<f32>,
    screen_size: vec2<f32>,
    near_plane: f32,
    far_plane: f32,
}

@group(0) @binding(0) var<uniform> material: Material;
@group(0) @binding(1) var<uniform> camera: CameraUniforms;
@group(0) @binding(2) var<uniform> light: Light;
fn pbr_lighting(albedo: vec3<f32>, metallic: f32, roughness: f32, ao: f32, n: vec3<f32>, v: vec3<f32>, l: vec3<f32>, radiance: vec3<f32>) -> vec3<f32> {
    let h = normalize(v + l);
        var f0 = vec3(0.04);
        f0 = mix(f0, albedo, metallic);
        let n_dot_l = max(dot(n, l), 0.0);
        let n_dot_v = max(dot(n, v), 0.0);
        let diffuse = (vec3(1.0) - f0) * (1.0 - metallic) * albedo / 3.14159265;
        return diffuse * radiance * n_dot_l * ao;
}

@fragment
fn main(@location(0) world_pos: vec3<f32>, @location(1) normal: vec3<f32>, @location(2) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let n = normalize(normal);
        let v = normalize(camera.position - world_pos);
        let l = normalize(light.position - world_pos);
        let dist = length(light.position - world_pos);
        let attenuation = 1.0 / (dist * dist);
        let radiance = light.color * light.intensity * attenuation;
        let albedo = material.base_color.rgb;
        let lit = pbr_lighting(albedo, material.metallic, material.roughness,
                              material.ao, n, v, l, radiance);
        let ambient = vec3(0.03) * albedo * material.ao;
        return vec4(lit + ambient + material.emissive, material.base_color.a);
}


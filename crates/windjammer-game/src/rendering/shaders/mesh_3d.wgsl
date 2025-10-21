// 3D Mesh Shader with Phong Lighting

struct Camera {
    view_proj: mat4x4<f32>,
    view_pos: vec3<f32>,
}

struct Light {
    position: vec3<f32>,
    color: vec3<f32>,
    intensity: f32,
}

struct Material {
    albedo: vec4<f32>,
    metallic: f32,
    roughness: f32,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(1) @binding(0)
var<uniform> light: Light;

@group(2) @binding(0)
var<uniform> material: Material;

@group(2) @binding(1)
var texture_sampler: sampler;

@group(2) @binding(2)
var texture_diffuse: texture_2d<f32>;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    // Transform position to clip space
    out.clip_position = camera.view_proj * vec4<f32>(vertex.position, 1.0);
    
    // Pass world space position and normal
    out.world_position = vertex.position;
    out.world_normal = normalize(vertex.normal);
    out.tex_coords = vertex.tex_coords;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample texture
    let tex_color = textureSample(texture_diffuse, texture_sampler, in.tex_coords);
    let base_color = material.albedo * tex_color;
    
    // Phong lighting
    let ambient = 0.1;
    
    // Diffuse
    let light_dir = normalize(light.position - in.world_position);
    let diffuse = max(dot(in.world_normal, light_dir), 0.0);
    
    // Specular
    let view_dir = normalize(camera.view_pos - in.world_position);
    let reflect_dir = reflect(-light_dir, in.world_normal);
    let spec = pow(max(dot(view_dir, reflect_dir), 0.0), 32.0);
    let specular = spec * (1.0 - material.roughness);
    
    // Combine
    let lighting = (ambient + diffuse + specular) * light.intensity;
    let final_color = base_color.rgb * light.color * lighting;
    
    return vec4<f32>(final_color, base_color.a);
}


// Deferred Rendering - Geometry Pass
// Renders scene geometry to G-Buffer (position, normal, albedo, material properties)

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tangent: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) world_tangent: vec3<f32>,
};

struct Camera {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
    position: vec3<f32>,
    _padding: f32,
};

struct Model {
    transform: mat4x4<f32>,
    normal_matrix: mat4x4<f32>,
};

struct Material {
    albedo: vec4<f32>,
    metallic: f32,
    roughness: f32,
    ao: f32,
    _padding: f32,
    emissive: vec3<f32>,
    emissive_strength: f32,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(1) @binding(0)
var<uniform> model: Model;

@group(2) @binding(0)
var<uniform> material: Material;

@group(2) @binding(1)
var albedo_texture: texture_2d<f32>;

@group(2) @binding(2)
var albedo_sampler: sampler;

@group(2) @binding(3)
var normal_map: texture_2d<f32>;

@group(2) @binding(4)
var normal_sampler: sampler;

@group(2) @binding(5)
var metallic_roughness_texture: texture_2d<f32>;

@group(2) @binding(6)
var metallic_roughness_sampler: sampler;

struct GBufferOutput {
    @location(0) position: vec4<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) albedo: vec4<f32>,
    @location(3) material: vec4<f32>,
    @location(4) emissive: vec4<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    
    // Transform position to world space
    let world_pos = model.transform * vec4<f32>(input.position, 1.0);
    output.world_position = world_pos.xyz;
    
    // Transform to clip space
    output.clip_position = camera.view_proj * world_pos;
    
    // Transform normal to world space
    output.world_normal = normalize((model.normal_matrix * vec4<f32>(input.normal, 0.0)).xyz);
    
    // Transform tangent to world space
    output.world_tangent = normalize((model.normal_matrix * vec4<f32>(input.tangent, 0.0)).xyz);
    
    // Pass through UV
    output.uv = input.uv;
    
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> GBufferOutput {
    var output: GBufferOutput;
    
    // Sample textures
    let albedo_sample = textureSample(albedo_texture, albedo_sampler, input.uv);
    let normal_sample = textureSample(normal_map, normal_sampler, input.uv);
    let metallic_roughness = textureSample(metallic_roughness_texture, metallic_roughness_sampler, input.uv);
    
    // Output position
    output.position = vec4<f32>(input.world_position, 1.0);
    
    // Calculate normal from normal map
    let tangent_normal = normal_sample.xyz * 2.0 - 1.0;
    let bitangent = cross(input.world_normal, input.world_tangent);
    let tbn = mat3x3<f32>(
        input.world_tangent,
        bitangent,
        input.world_normal
    );
    let world_normal = normalize(tbn * tangent_normal);
    output.normal = vec4<f32>(world_normal, 1.0);
    
    // Output albedo
    output.albedo = albedo_sample * material.albedo;
    
    // Output material properties (metallic, roughness, AO)
    output.material = vec4<f32>(
        metallic_roughness.b * material.metallic,  // Metallic
        metallic_roughness.g * material.roughness, // Roughness
        metallic_roughness.r * material.ao,        // AO
        1.0
    );
    
    // Output emissive
    output.emissive = vec4<f32>(
        material.emissive * material.emissive_strength,
        material.emissive_strength
    );
    
    return output;
}


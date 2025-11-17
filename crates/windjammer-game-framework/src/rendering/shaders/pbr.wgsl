// Physically-Based Rendering (PBR) Shader
// Implements metallic-roughness workflow with full PBR lighting

struct Camera {
    view_proj: mat4x4<f32>,
    view_pos: vec3<f32>,
}

struct Light {
    position: vec3<f32>,
    color: vec3<f32>,
    intensity: f32,
    light_type: u32, // 0 = directional, 1 = point, 2 = spot
    direction: vec3<f32>,
    range: f32,
    inner_angle: f32,
    outer_angle: f32,
}

struct Material {
    base_color: vec4<f32>,
    metallic: f32,
    roughness: f32,
    emissive: vec3<f32>,
    emissive_strength: f32,
    normal_strength: f32,
    occlusion_strength: f32,
    alpha_cutoff: f32,
    alpha_mode: u32, // 0 = opaque, 1 = mask, 2 = blend
    has_base_color_texture: u32,
    has_metallic_roughness_texture: u32,
    has_normal_texture: u32,
    has_occlusion_texture: u32,
    has_emissive_texture: u32,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(1) @binding(0)
var<uniform> light: Light;

@group(2) @binding(0)
var<uniform> material: Material;

@group(2) @binding(1)
var texture_sampler: sampler;

// Texture slots
@group(2) @binding(2)
var texture_base_color: texture_2d<f32>;

@group(2) @binding(3)
var texture_metallic_roughness: texture_2d<f32>;

@group(2) @binding(4)
var texture_normal: texture_2d<f32>;

@group(2) @binding(5)
var texture_occlusion: texture_2d<f32>;

@group(2) @binding(6)
var texture_emissive: texture_2d<f32>;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coords: vec2<f32>,
    @location(3) tangent: vec4<f32>, // w component is handedness
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) tex_coords: vec2<f32>,
    @location(3) world_tangent: vec3<f32>,
    @location(4) world_bitangent: vec3<f32>,
}

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    // Transform position to clip space
    out.clip_position = camera.view_proj * vec4<f32>(vertex.position, 1.0);
    
    // Pass world space position
    out.world_position = vertex.position;
    
    // Transform normal to world space
    out.world_normal = normalize(vertex.normal);
    
    // Transform tangent to world space
    out.world_tangent = normalize(vertex.tangent.xyz);
    
    // Calculate bitangent (handedness is in tangent.w)
    out.world_bitangent = cross(out.world_normal, out.world_tangent) * vertex.tangent.w;
    
    out.tex_coords = vertex.tex_coords;
    
    return out;
}

// Constants
const PI: f32 = 3.14159265359;

// Normal Distribution Function (GGX/Trowbridge-Reitz)
fn distribution_ggx(N: vec3<f32>, H: vec3<f32>, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let NdotH = max(dot(N, H), 0.0);
    let NdotH2 = NdotH * NdotH;
    
    let nom = a2;
    var denom = (NdotH2 * (a2 - 1.0) + 1.0);
    denom = PI * denom * denom;
    
    return nom / denom;
}

// Geometry Function (Schlick-GGX)
fn geometry_schlick_ggx(NdotV: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = (r * r) / 8.0;
    
    let nom = NdotV;
    let denom = NdotV * (1.0 - k) + k;
    
    return nom / denom;
}

// Smith's method for geometry obstruction
fn geometry_smith(N: vec3<f32>, V: vec3<f32>, L: vec3<f32>, roughness: f32) -> f32 {
    let NdotV = max(dot(N, V), 0.0);
    let NdotL = max(dot(N, L), 0.0);
    let ggx2 = geometry_schlick_ggx(NdotV, roughness);
    let ggx1 = geometry_schlick_ggx(NdotL, roughness);
    
    return ggx1 * ggx2;
}

// Fresnel-Schlick approximation
fn fresnel_schlick(cosTheta: f32, F0: vec3<f32>) -> vec3<f32> {
    return F0 + (1.0 - F0) * pow(clamp(1.0 - cosTheta, 0.0, 1.0), 5.0);
}

// Sample normal map and transform to world space
fn sample_normal_map(
    tex_coords: vec2<f32>,
    world_normal: vec3<f32>,
    world_tangent: vec3<f32>,
    world_bitangent: vec3<f32>,
    strength: f32
) -> vec3<f32> {
    if (material.has_normal_texture == 0u) {
        return world_normal;
    }
    
    // Sample normal map (in tangent space, range [0,1])
    var tangent_normal = textureSample(texture_normal, texture_sampler, tex_coords).xyz;
    
    // Convert from [0,1] to [-1,1]
    tangent_normal = tangent_normal * 2.0 - 1.0;
    
    // Apply normal strength
    tangent_normal.x *= strength;
    tangent_normal.y *= strength;
    tangent_normal = normalize(tangent_normal);
    
    // TBN matrix (tangent space to world space)
    let TBN = mat3x3<f32>(
        world_tangent,
        world_bitangent,
        world_normal
    );
    
    // Transform to world space
    return normalize(TBN * tangent_normal);
}

// Calculate light attenuation
fn calculate_attenuation(light_pos: vec3<f32>, frag_pos: vec3<f32>, range: f32) -> f32 {
    let distance = length(light_pos - frag_pos);
    
    // Inverse square law with smooth falloff
    let attenuation = 1.0 / (distance * distance);
    
    // Smooth cutoff at range
    let cutoff = 1.0 - smoothstep(range * 0.8, range, distance);
    
    return attenuation * cutoff;
}

// Calculate spot light cone attenuation
fn calculate_spot_attenuation(
    light_dir: vec3<f32>,
    spot_dir: vec3<f32>,
    inner_angle: f32,
    outer_angle: f32
) -> f32 {
    let cos_angle = dot(normalize(-light_dir), normalize(spot_dir));
    let cos_inner = cos(inner_angle);
    let cos_outer = cos(outer_angle);
    
    return smoothstep(cos_outer, cos_inner, cos_angle);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample textures
    var base_color = material.base_color;
    if (material.has_base_color_texture != 0u) {
        base_color *= textureSample(texture_base_color, texture_sampler, in.tex_coords);
    }
    
    // Alpha testing
    if (material.alpha_mode == 1u && base_color.a < material.alpha_cutoff) {
        discard;
    }
    
    // Sample metallic-roughness
    var metallic = material.metallic;
    var roughness = material.roughness;
    if (material.has_metallic_roughness_texture != 0u) {
        let mr = textureSample(texture_metallic_roughness, texture_sampler, in.tex_coords);
        metallic *= mr.b; // Blue channel = metallic
        roughness *= mr.g; // Green channel = roughness
    }
    
    // Sample ambient occlusion
    var ao = 1.0;
    if (material.has_occlusion_texture != 0u) {
        ao = textureSample(texture_occlusion, texture_sampler, in.tex_coords).r;
        ao = mix(1.0, ao, material.occlusion_strength);
    }
    
    // Sample emissive
    var emissive = material.emissive * material.emissive_strength;
    if (material.has_emissive_texture != 0u) {
        emissive *= textureSample(texture_emissive, texture_sampler, in.tex_coords).rgb;
    }
    
    // Sample normal map
    let N = sample_normal_map(
        in.tex_coords,
        in.world_normal,
        in.world_tangent,
        in.world_bitangent,
        material.normal_strength
    );
    
    // View direction
    let V = normalize(camera.view_pos - in.world_position);
    
    // Calculate light direction based on light type
    var L: vec3<f32>;
    var light_distance: f32;
    var attenuation: f32 = 1.0;
    
    if (light.light_type == 0u) {
        // Directional light
        L = normalize(-light.direction);
        light_distance = 0.0;
    } else if (light.light_type == 1u) {
        // Point light
        L = normalize(light.position - in.world_position);
        light_distance = length(light.position - in.world_position);
        attenuation = calculate_attenuation(light.position, in.world_position, light.range);
    } else {
        // Spot light
        L = normalize(light.position - in.world_position);
        light_distance = length(light.position - in.world_position);
        let dist_attenuation = calculate_attenuation(light.position, in.world_position, light.range);
        let spot_attenuation = calculate_spot_attenuation(L, light.direction, light.inner_angle, light.outer_angle);
        attenuation = dist_attenuation * spot_attenuation;
    }
    
    let H = normalize(V + L);
    
    // Calculate F0 (surface reflection at zero incidence)
    // For dielectrics, F0 is around 0.04
    // For metals, F0 is the albedo color
    var F0 = vec3<f32>(0.04);
    F0 = mix(F0, base_color.rgb, metallic);
    
    // Cook-Torrance BRDF
    let NDF = distribution_ggx(N, H, roughness);
    let G = geometry_smith(N, V, L, roughness);
    let F = fresnel_schlick(max(dot(H, V), 0.0), F0);
    
    // Specular term
    let NdotL = max(dot(N, L), 0.0);
    let NdotV = max(dot(N, V), 0.0);
    let numerator = NDF * G * F;
    let denominator = 4.0 * NdotV * NdotL + 0.0001; // Add epsilon to prevent divide by zero
    let specular = numerator / denominator;
    
    // Diffuse term (Lambertian)
    let kS = F; // Specular contribution
    var kD = vec3<f32>(1.0) - kS; // Diffuse contribution
    kD *= 1.0 - metallic; // Metals have no diffuse
    
    let diffuse = kD * base_color.rgb / PI;
    
    // Combine diffuse and specular
    let radiance = light.color * light.intensity * attenuation;
    let Lo = (diffuse + specular) * radiance * NdotL;
    
    // Ambient term (simple ambient for now, IBL would be better)
    let ambient = vec3<f32>(0.03) * base_color.rgb * ao;
    
    // Final color
    var color = ambient + Lo + emissive;
    
    // Tone mapping (simple Reinhard)
    color = color / (color + vec3<f32>(1.0));
    
    // Gamma correction
    color = pow(color, vec3<f32>(1.0 / 2.2));
    
    return vec4<f32>(color, base_color.a);
}



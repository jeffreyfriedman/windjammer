// Deferred Rendering - Lighting Pass
// Calculates lighting using G-Buffer data

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct Camera {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
    position: vec3<f32>,
    _padding: f32,
};

struct Light {
    position: vec3<f32>,
    light_type: u32,      // 0 = point, 1 = directional, 2 = spot
    color: vec3<f32>,
    intensity: f32,
    range: f32,
    inner_angle: f32,
    outer_angle: f32,
    _padding: f32,
};

struct LightingParams {
    num_lights: u32,
    ambient_color: vec3<f32>,
    ambient_intensity: f32,
    _padding: vec3<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(0) @binding(1)
var<uniform> lighting_params: LightingParams;

@group(1) @binding(0)
var g_position: texture_2d<f32>;

@group(1) @binding(1)
var g_normal: texture_2d<f32>;

@group(1) @binding(2)
var g_albedo: texture_2d<f32>;

@group(1) @binding(3)
var g_material: texture_2d<f32>;

@group(1) @binding(4)
var g_emissive: texture_2d<f32>;

@group(1) @binding(5)
var g_depth: texture_depth_2d;

@group(2) @binding(0)
var<storage, read> lights: array<Light>;

// Fullscreen quad vertex shader
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var output: VertexOutput;
    
    // Generate fullscreen quad
    let x = f32((vertex_index & 1u) << 1u);
    let y = f32((vertex_index & 2u));
    
    output.position = vec4<f32>(x * 2.0 - 1.0, y * 2.0 - 1.0, 0.0, 1.0);
    output.uv = vec2<f32>(x, 1.0 - y);
    
    return output;
}

// PBR lighting functions
const PI: f32 = 3.14159265359;

fn distribution_ggx(N: vec3<f32>, H: vec3<f32>, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let NdotH = max(dot(N, H), 0.0);
    let NdotH2 = NdotH * NdotH;
    
    let num = a2;
    var denom = (NdotH2 * (a2 - 1.0) + 1.0);
    denom = PI * denom * denom;
    
    return num / denom;
}

fn geometry_schlick_ggx(NdotV: f32, roughness: f32) -> f32 {
    let r = (roughness + 1.0);
    let k = (r * r) / 8.0;
    
    let num = NdotV;
    let denom = NdotV * (1.0 - k) + k;
    
    return num / denom;
}

fn geometry_smith(N: vec3<f32>, V: vec3<f32>, L: vec3<f32>, roughness: f32) -> f32 {
    let NdotV = max(dot(N, V), 0.0);
    let NdotL = max(dot(N, L), 0.0);
    let ggx2 = geometry_schlick_ggx(NdotV, roughness);
    let ggx1 = geometry_schlick_ggx(NdotL, roughness);
    
    return ggx1 * ggx2;
}

fn fresnel_schlick(cosTheta: f32, F0: vec3<f32>) -> vec3<f32> {
    return F0 + (1.0 - F0) * pow(clamp(1.0 - cosTheta, 0.0, 1.0), 5.0);
}

fn calculate_pbr_lighting(
    position: vec3<f32>,
    normal: vec3<f32>,
    albedo: vec3<f32>,
    metallic: f32,
    roughness: f32,
    ao: f32,
    view_dir: vec3<f32>,
    light: Light
) -> vec3<f32> {
    var light_dir: vec3<f32>;
    var attenuation: f32 = 1.0;
    
    // Calculate light direction and attenuation based on light type
    if (light.light_type == 0u) {
        // Point light
        let light_vec = light.position - position;
        let distance = length(light_vec);
        light_dir = normalize(light_vec);
        
        // Inverse square falloff with range
        attenuation = 1.0 / (distance * distance);
        attenuation *= smoothstep(light.range, 0.0, distance);
    } else if (light.light_type == 1u) {
        // Directional light
        light_dir = normalize(-light.position);
        attenuation = 1.0;
    } else {
        // Spot light
        let light_vec = light.position - position;
        let distance = length(light_vec);
        light_dir = normalize(light_vec);
        
        // Inverse square falloff
        attenuation = 1.0 / (distance * distance);
        attenuation *= smoothstep(light.range, 0.0, distance);
        
        // Spot cone attenuation
        let spot_dir = normalize(-light.position);
        let theta = dot(light_dir, spot_dir);
        let epsilon = light.inner_angle - light.outer_angle;
        let spot_intensity = clamp((theta - light.outer_angle) / epsilon, 0.0, 1.0);
        attenuation *= spot_intensity;
    }
    
    // PBR calculations
    let H = normalize(view_dir + light_dir);
    
    // Calculate F0 (surface reflection at zero incidence)
    var F0 = vec3<f32>(0.04);
    F0 = mix(F0, albedo, metallic);
    
    // Cook-Torrance BRDF
    let NDF = distribution_ggx(normal, H, roughness);
    let G = geometry_smith(normal, view_dir, light_dir, roughness);
    let F = fresnel_schlick(max(dot(H, view_dir), 0.0), F0);
    
    let kS = F;
    var kD = vec3<f32>(1.0) - kS;
    kD *= 1.0 - metallic;
    
    let NdotL = max(dot(normal, light_dir), 0.0);
    let NdotV = max(dot(normal, view_dir), 0.0);
    
    let numerator = NDF * G * F;
    let denominator = 4.0 * NdotV * NdotL + 0.0001;
    let specular = numerator / denominator;
    
    // Combine diffuse and specular
    let radiance = light.color * light.intensity * attenuation;
    return (kD * albedo / PI + specular) * radiance * NdotL;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Sample G-Buffer
    let position = textureLoad(g_position, vec2<i32>(input.position.xy), 0).xyz;
    let normal = normalize(textureLoad(g_normal, vec2<i32>(input.position.xy), 0).xyz);
    let albedo = textureLoad(g_albedo, vec2<i32>(input.position.xy), 0).rgb;
    let material = textureLoad(g_material, vec2<i32>(input.position.xy), 0);
    let emissive = textureLoad(g_emissive, vec2<i32>(input.position.xy), 0).rgb;
    
    let metallic = material.r;
    let roughness = material.g;
    let ao = material.b;
    
    // Calculate view direction
    let view_dir = normalize(camera.position - position);
    
    // Start with ambient lighting
    var Lo = lighting_params.ambient_color * lighting_params.ambient_intensity * albedo * ao;
    
    // Add contribution from each light
    for (var i = 0u; i < lighting_params.num_lights; i = i + 1u) {
        Lo += calculate_pbr_lighting(
            position,
            normal,
            albedo,
            metallic,
            roughness,
            ao,
            view_dir,
            lights[i]
        );
    }
    
    // Add emissive
    Lo += emissive;
    
    return vec4<f32>(Lo, 1.0);
}


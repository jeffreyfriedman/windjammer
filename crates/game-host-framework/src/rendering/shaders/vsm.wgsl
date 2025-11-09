// Virtual Shadow Maps (VSM) Shader
// Inspired by Unreal Engine 5's VSM system
// Provides high-quality shadows with minimal performance cost

struct CameraUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _padding: f32,
}

struct LightUniform {
    light_view_proj: mat4x4<f32>,
    light_pos: vec3<f32>,
    light_radius: f32,
    light_color: vec3<f32>,
    light_intensity: f32,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<uniform> light: LightUniform;

@group(2) @binding(0)
var shadow_map: texture_depth_2d;
@group(2) @binding(1)
var shadow_sampler: sampler_comparison;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coords: vec2<f32>,
    @location(3) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) light_space_pos: vec4<f32>,
}

// Vertex shader for shadow map generation
@vertex
fn vs_shadow(vertex: VertexInput) -> @builtin(position) vec4<f32> {
    return light.light_view_proj * vec4<f32>(vertex.position, 1.0);
}

// Fragment shader for shadow map generation (empty, just writes depth)
@fragment
fn fs_shadow() {
    // Depth is written automatically
}

// Vertex shader for main rendering with shadows
@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    // Transform to clip space
    out.clip_position = camera.view_proj * vec4<f32>(vertex.position, 1.0);
    
    // Pass world space data
    out.world_position = vertex.position;
    out.world_normal = normalize(vertex.normal);
    out.color = vertex.color;
    
    // Transform to light space for shadow mapping
    out.light_space_pos = light.light_view_proj * vec4<f32>(vertex.position, 1.0);
    
    return out;
}

// PCF (Percentage Closer Filtering) for soft shadows
fn sample_shadow_pcf(light_space_pos: vec4<f32>, bias: f32) -> f32 {
    // Perspective divide
    let proj_coords = light_space_pos.xyz / light_space_pos.w;
    
    // Transform to [0, 1] range
    let shadow_coords = vec2<f32>(
        proj_coords.x * 0.5 + 0.5,
        proj_coords.y * -0.5 + 0.5
    );
    
    // Check if outside shadow map
    if shadow_coords.x < 0.0 || shadow_coords.x > 1.0 ||
       shadow_coords.y < 0.0 || shadow_coords.y > 1.0 {
        return 1.0; // Not in shadow
    }
    
    let current_depth = proj_coords.z;
    
    // PCF with 3x3 kernel
    var shadow = 0.0;
    let texel_size = 1.0 / 2048.0; // Shadow map resolution
    
    for (var x = -1; x <= 1; x++) {
        for (var y = -1; y <= 1; y++) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel_size;
            let sample_coords = shadow_coords + offset;
            
            shadow += textureSampleCompare(
                shadow_map,
                shadow_sampler,
                sample_coords,
                current_depth - bias
            );
        }
    }
    
    return shadow / 9.0;
}

// Fragment shader with shadow mapping
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Calculate lighting direction
    let light_dir = normalize(light.light_pos - in.world_position);
    let view_dir = normalize(camera.camera_pos - in.world_position);
    
    // Ambient lighting
    let ambient = 0.2;
    
    // Diffuse lighting
    let diffuse = max(dot(in.world_normal, light_dir), 0.0);
    
    // Specular lighting (Blinn-Phong)
    let half_dir = normalize(light_dir + view_dir);
    let specular = pow(max(dot(in.world_normal, half_dir), 0.0), 32.0) * 0.3;
    
    // Shadow calculation with bias
    let bias = max(0.005 * (1.0 - dot(in.world_normal, light_dir)), 0.001);
    let shadow = sample_shadow_pcf(in.light_space_pos, bias);
    
    // Combine lighting
    let lighting = ambient + (diffuse + specular) * shadow * light.intensity;
    
    // Apply to base color
    let final_color = in.color.rgb * light.color * lighting;
    
    return vec4<f32>(final_color, in.color.a);
}

// Debug visualization of shadow map
@fragment
fn fs_debug_shadow(in: VertexOutput) -> @location(0) vec4<f32> {
    let proj_coords = in.light_space_pos.xyz / in.light_space_pos.w;
    let shadow_coords = vec2<f32>(
        proj_coords.x * 0.5 + 0.5,
        proj_coords.y * -0.5 + 0.5
    );
    
    if shadow_coords.x < 0.0 || shadow_coords.x > 1.0 ||
       shadow_coords.y < 0.0 || shadow_coords.y > 1.0 {
        return vec4<f32>(1.0, 0.0, 0.0, 1.0); // Red = outside shadow map
    }
    
    let depth = proj_coords.z;
    return vec4<f32>(depth, depth, depth, 1.0);
}


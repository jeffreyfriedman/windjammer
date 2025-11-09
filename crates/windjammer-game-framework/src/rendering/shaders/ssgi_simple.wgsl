//! Simplified Screen-Space Global Illumination (SSGI)
//!
//! This shader implements a basic SSGI technique that approximates
//! indirect lighting by sampling the screen-space G-buffer.
//!
//! Algorithm:
//! 1. For each pixel, sample neighboring pixels in a hemisphere around the normal
//! 2. Accumulate indirect lighting from those samples
//! 3. Combine with direct lighting
//!
//! This is a simplified version suitable for real-time games.

struct CameraUniform {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
    view_projection: mat4x4<f32>,
    position: vec3<f32>,
    _padding: f32,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

// G-buffer textures
@group(1) @binding(0)
var g_position: texture_2d<f32>;
@group(1) @binding(1)
var g_normal: texture_2d<f32>;
@group(1) @binding(2)
var g_albedo: texture_2d<f32>;
@group(1) @binding(3)
var g_sampler: sampler;

// SSGI parameters
struct SSGIParams {
    num_samples: u32,
    sample_radius: f32,
    intensity: f32,
    _padding: f32,
}

@group(2) @binding(0)
var<uniform> ssgi_params: SSGIParams;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Fullscreen triangle
    var out: VertexOutput;
    let x = f32((vertex_index & 1u) << 1u);
    let y = f32((vertex_index & 2u));
    out.position = vec4<f32>(x * 2.0 - 1.0, y * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2<f32>(x, 1.0 - y);
    return out;
}

// Generate hemisphere sample direction
fn get_hemisphere_sample(index: u32, num_samples: u32, normal: vec3<f32>) -> vec3<f32> {
    let golden_angle = 2.399963; // Golden angle in radians
    let theta = f32(index) * golden_angle;
    let phi = acos(1.0 - 2.0 * (f32(index) + 0.5) / f32(num_samples));
    
    // Spherical to Cartesian
    let x = sin(phi) * cos(theta);
    let y = sin(phi) * sin(theta);
    let z = cos(phi);
    
    let sample_dir = vec3<f32>(x, y, z);
    
    // Orient to normal (simple approach)
    if (dot(sample_dir, normal) < 0.0) {
        return -sample_dir;
    }
    return sample_dir;
}

// Project world position to screen space
fn world_to_screen(world_pos: vec3<f32>) -> vec2<f32> {
    let clip_pos = camera.view_projection * vec4<f32>(world_pos, 1.0);
    let ndc = clip_pos.xy / clip_pos.w;
    return ndc * 0.5 + 0.5;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample G-buffer
    let position = textureSample(g_position, g_sampler, in.uv).xyz;
    let normal = textureSample(g_normal, g_sampler, in.uv).xyz;
    let albedo = textureSample(g_albedo, g_sampler, in.uv).rgb;
    
    // Skip if no geometry (background)
    if (length(normal) < 0.1) {
        return vec4<f32>(albedo, 1.0);
    }
    
    var indirect_lighting = vec3<f32>(0.0);
    let num_samples = ssgi_params.num_samples;
    
    // Sample hemisphere around normal
    for (var i = 0u; i < num_samples; i++) {
        let sample_dir = get_hemisphere_sample(i, num_samples, normal);
        let sample_pos = position + sample_dir * ssgi_params.sample_radius;
        
        // Project to screen space
        let screen_pos = world_to_screen(sample_pos);
        
        // Check if sample is on screen
        if (screen_pos.x < 0.0 || screen_pos.x > 1.0 || 
            screen_pos.y < 0.0 || screen_pos.y > 1.0) {
            continue;
        }
        
        // Sample G-buffer at that position
        let sample_albedo = textureSample(g_albedo, g_sampler, screen_pos).rgb;
        let sample_normal = textureSample(g_normal, g_sampler, screen_pos).xyz;
        
        // Skip if no geometry
        if (length(sample_normal) < 0.1) {
            continue;
        }
        
        // Accumulate indirect lighting
        let ndotl = max(dot(normal, sample_dir), 0.0);
        indirect_lighting += sample_albedo * ndotl;
    }
    
    // Average and scale
    indirect_lighting /= f32(num_samples);
    indirect_lighting *= ssgi_params.intensity;
    
    // Simple direct lighting
    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.3));
    let ambient = 0.2;
    let diffuse = max(dot(normal, light_dir), 0.0) * 0.8;
    let direct_lighting = (ambient + diffuse);
    
    // Combine direct and indirect lighting
    let final_color = albedo * (direct_lighting + indirect_lighting);
    
    return vec4<f32>(final_color, 1.0);
}

// Debug visualization modes
@fragment
fn fs_debug_position(in: VertexOutput) -> @location(0) vec4<f32> {
    let position = textureSample(g_position, g_sampler, in.uv).xyz;
    return vec4<f32>(position * 0.1, 1.0); // Scale for visibility
}

@fragment
fn fs_debug_normal(in: VertexOutput) -> @location(0) vec4<f32> {
    let normal = textureSample(g_normal, g_sampler, in.uv).xyz;
    return vec4<f32>(normal * 0.5 + 0.5, 1.0); // Map to 0-1 range
}

@fragment
fn fs_debug_albedo(in: VertexOutput) -> @location(0) vec4<f32> {
    let albedo = textureSample(g_albedo, g_sampler, in.uv);
    return albedo;
}


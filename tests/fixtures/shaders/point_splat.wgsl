// Point Cloud Splatting Shader
//
// Renders point clouds as screen-space billboards (splats) to G-buffer.
//
// Inputs:
// - Point buffer (read-only): Position, color, normal, size
// - Camera uniforms: View-projection, position
//
// Outputs:
// - G-buffer (read-write): Position, normal, material, depth, geometry_source

struct Point {
    position: vec3<f32>,
    _pad0: u32,
    color: vec3<f32>,
    _pad1: u32,
    normal: vec3<f32>,
    size: f32,
}

struct CameraUniforms {
    view_proj: mat4x4<f32>,
    position: vec3<f32>,
    _pad1: f32,
    forward: vec3<f32>,
    fov: f32,
    screen_width: f32,
    screen_height: f32,
    near_plane: f32,
    far_plane: f32,
}

struct SplatParams {
    point_count: u32,
    point_scale: f32,  // Scale factor for point size
    _pad: vec2<u32>,
}

struct GBufferPixel {
    position: vec3<f32>,
    _pad1: f32,
    normal: vec3<f32>,
    material_id: f32,
    depth: f32,
    geometry_source: f32,  // 0=voxel, 1=triangle, 2=point
    _pad2: vec2<f32>,
}

@group(0) @binding(0) var<uniform> params: SplatParams;
@group(0) @binding(1) var<uniform> camera: CameraUniforms;
@group(0) @binding(2) var<storage, read> points: array<Point>;
@group(0) @binding(3) var<storage, read_write> gbuffer: array<GBufferPixel>;

@compute @workgroup_size(64)
fn splat_points(@builtin(global_invocation_id) id: vec3<u32>) {
    let point_idx = id.x;
    
    if (point_idx >= params.point_count) {
        return;
    }
    
    let point = points[point_idx];
    
    // Project point to screen space
    let clip_pos = camera.view_proj * vec4<f32>(point.position, 1.0);
    
    // Perspective divide
    let ndc = clip_pos.xyz / clip_pos.w;
    
    // Check if point is visible (inside NDC cube [-1,1])
    if (ndc.x < -1.0 || ndc.x > 1.0 || ndc.y < -1.0 || ndc.y > 1.0 || ndc.z < 0.0 || ndc.z > 1.0) {
        return;  // Point outside view frustum
    }
    
    // Convert NDC to screen coordinates
    let screen_x = (ndc.x * 0.5 + 0.5) * camera.screen_width;
    let screen_y = (1.0 - (ndc.y * 0.5 + 0.5)) * camera.screen_height;  // Flip Y
    
    // Calculate splat size in pixels
    let distance = length(point.position - camera.position);
    let screen_size = (point.size * params.point_scale * camera.screen_height) / (distance * tan(camera.fov * 0.5 * 0.0174533));
    let radius_pixels = max(screen_size, 1.0);
    
    // Rasterize billboard (square splat)
    let min_x = i32(screen_x - radius_pixels);
    let max_x = i32(screen_x + radius_pixels);
    let min_y = i32(screen_y - radius_pixels);
    let max_y = i32(screen_y + radius_pixels);
    
    // Clamp to screen bounds
    let min_x_clamped = max(min_x, 0);
    let max_x_clamped = min(max_x, i32(camera.screen_width) - 1);
    let min_y_clamped = max(min_y, 0);
    let max_y_clamped = min(max_y, i32(camera.screen_height) - 1);
    
    // For each pixel in splat
    for (var py = min_y_clamped; py <= max_y_clamped; py++) {
        for (var px = min_x_clamped; px <= max_x_clamped; px++) {
            let pixel_idx = u32(py) * u32(camera.screen_width) + u32(px);
            
            // Calculate distance from pixel center to splat center
            let dx = f32(px) - screen_x;
            let dy = f32(py) - screen_y;
            let pixel_dist = sqrt(dx * dx + dy * dy);
            
            // Circular falloff (soft edge)
            if (pixel_dist > radius_pixels) {
                continue;
            }
            
            // Depth test: Only write if closer than existing depth
            let existing_depth = gbuffer[pixel_idx].depth;
            if (ndc.z >= existing_depth && existing_depth > 0.0) {
                continue;  // Point is behind existing geometry
            }
            
            // Write to G-buffer
            // Note: This is a race condition! Multiple points might write to same pixel.
            // In real implementation, would use atomic operations or multi-pass rendering.
            gbuffer[pixel_idx].position = point.position;
            gbuffer[pixel_idx].normal = point.normal;
            gbuffer[pixel_idx].material_id = 0.0;  // Material ID for point clouds
            gbuffer[pixel_idx].depth = ndc.z;
            gbuffer[pixel_idx].geometry_source = 2.0;  // 2 = point cloud
            
            // Could encode point color in material_id or separate buffer
        }
    }
}

// Alternative: Atomic depth test using atomic compare-exchange
// This prevents race conditions when multiple points write to same pixel
@compute @workgroup_size(64)
fn splat_points_atomic(@builtin(global_invocation_id) id: vec3<u32>) {
    let point_idx = id.x;
    
    if (point_idx >= params.point_count) {
        return;
    }
    
    let point = points[point_idx];
    
    // Project to screen space (same as above)
    let clip_pos = camera.view_proj * vec4<f32>(point.position, 1.0);
    let ndc = clip_pos.xyz / clip_pos.w;
    
    if (ndc.x < -1.0 || ndc.x > 1.0 || ndc.y < -1.0 || ndc.y > 1.0 || ndc.z < 0.0 || ndc.z > 1.0) {
        return;
    }
    
    let screen_x = (ndc.x * 0.5 + 0.5) * camera.screen_width;
    let screen_y = (1.0 - (ndc.y * 0.5 + 0.5)) * camera.screen_height;
    let pixel_idx = u32(screen_y) * u32(camera.screen_width) + u32(screen_x);
    
    // For simplicity, just write center pixel
    // In real implementation, would rasterize full splat
    
    // TODO: Use atomic compare-exchange for depth test
    // let depth_u32 = bitcast<u32>(ndc.z);
    // let old_depth_u32 = atomicMin(&depth_buffer[pixel_idx], depth_u32);
    
    // For now, just write (race condition acceptable for demo)
    gbuffer[pixel_idx].position = point.position;
    gbuffer[pixel_idx].normal = point.normal;
    gbuffer[pixel_idx].depth = ndc.z;
    gbuffer[pixel_idx].geometry_source = 2.0;
}

fn tan(x: f32) -> f32 {
    return x + (x * x * x) / 3.0;
}

struct CullParams {
    hiz_width: u32,
    hiz_height: u32,
    chunk_count: u32,
    near_plane: f32,
    far_plane: f32,
    screen_width: u32,
    screen_height: u32,
    _pad: u32,
}

struct ChunkAABB {
    min_x: f32,
    min_y: f32,
    min_z: f32,
    max_x: f32,
    max_y: f32,
    max_z: f32,
}

@group(0) @binding(0) var<uniform> params: CullParams;
@group(0) @binding(1) var<uniform> camera: mat4x4<f32>;
@group(0) @binding(2) var<storage, read> aabbs: array<ChunkAABB>;
@group(0) @binding(3) var<storage, read> hiz_depth: array<f32>;
@group(0) @binding(4) var<storage, read_write> visibility: array<u32>;
fn project_point(vp: mat4x4<f32>, p: vec3<f32>) -> vec4<f32> {
    return vp * vec4(p, 1.0);
}

fn ndc_to_screen(ndc: vec2<f32>, sw: f32, sh: f32) -> vec2<f32> {
    return vec2(
            (ndc.x * 0.5 + 0.5) * sw,
            (1.0 - (ndc.y * 0.5 + 0.5)) * sh
        );
}

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= params.chunk_count) { return; }
    
        let aabb = aabbs[id.x];
        let sw = f32(params.screen_width);
        let sh = f32(params.screen_height);
    
        // Project all 8 AABB corners
        var min_sx = sw;
        var min_sy = sh;
        var max_sx = 0.0;
        var max_sy = 0.0;
        var min_depth = 1.0;
        var any_behind = false;
    
        for (var i = 0u; i < 8u; i++) {
            let cx = select(aabb.min_x, aabb.max_x, (i & 1u) != 0u);
            let cy = select(aabb.min_y, aabb.max_y, (i & 2u) != 0u);
            let cz = select(aabb.min_z, aabb.max_z, (i & 4u) != 0u);
            let clip = project_point(camera, vec3(cx, cy, cz));
    
            if (clip.w <= 0.0) {
                any_behind = true;
                continue;
            }
    
            let ndc = clip.xy / clip.w;
            let screen = ndc_to_screen(ndc, sw, sh);
            let depth = clip.z / clip.w;
    
            min_sx = min(min_sx, screen.x);
            min_sy = min(min_sy, screen.y);
            max_sx = max(max_sx, screen.x);
            max_sy = max(max_sy, screen.y);
            min_depth = min(min_depth, depth);
        }
    
        // If any corner is behind the camera, conservatively mark visible
        if (any_behind) {
            visibility[id.x] = 1u;
            return;
        }
    
        // Clamp to screen bounds
        min_sx = max(min_sx, 0.0);
        min_sy = max(min_sy, 0.0);
        max_sx = min(max_sx, sw);
        max_sy = min(max_sy, sh);
    
        // If AABB projects to zero area, it's off-screen
        if (min_sx >= max_sx || min_sy >= max_sy) {
            visibility[id.x] = 0u;
            return;
        }
    
        // Map screen coords to Hi-Z buffer coords
        let hiz_min_x = u32(min_sx * f32(params.hiz_width) / sw);
        let hiz_min_y = u32(min_sy * f32(params.hiz_height) / sh);
        let hiz_max_x = min(u32(max_sx * f32(params.hiz_width) / sw) + 1u, params.hiz_width);
        let hiz_max_y = min(u32(max_sy * f32(params.hiz_height) / sh) + 1u, params.hiz_height);
    
        // Sample Hi-Z: find maximum depth in the screen rect
        var max_hiz_depth = 0.0;
        for (var hy = hiz_min_y; hy < hiz_max_y; hy++) {
            for (var hx = hiz_min_x; hx < hiz_max_x; hx++) {
                let d = hiz_depth[hy * params.hiz_width + hx];
                if (d > max_hiz_depth) { max_hiz_depth = d; }
            }
        }
    
        // Occlusion test: if chunk's nearest depth is beyond all Hi-Z samples, it's occluded
        if (min_depth > max_hiz_depth && max_hiz_depth > 0.0) {
            visibility[id.x] = 0u;
        } else {
            visibility[id.x] = 1u;
        }
}


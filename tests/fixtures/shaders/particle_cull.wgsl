// GPU Particle Frustum Culling Shader
//
// Tests each particle's bounding sphere against 6 frustum planes.
// Updates visible count (atomic counter).
// Skips dead and off-screen particles.

struct Particle {
    position: vec3<f32>,
    _padding1: f32,
    velocity: vec3<f32>,
    _padding2: f32,
    color: vec4<f32>,
    size: f32,
    lifetime: f32,
    age: f32,
    _padding3: f32,
}

// Frustum plane: ax + by + cz + d = 0
struct FrustumPlane {
    normal: vec3<f32>,      // (a, b, c)
    distance: f32,          // d
}

struct CullingParams {
    // 6 frustum planes (left, right, bottom, top, near, far)
    planes: array<FrustumPlane, 6>,  // 6 × 16 bytes = 96 bytes
    particle_count: u32,              // 4 bytes
    _padding1: u32,
    _padding2: u32,
    _padding3: u32,
    _padding4: vec4<u32>,             // 16 bytes
}

struct VisibilityData {
    visible_count: atomic<u32>,
    _padding: array<u32, 7>,  // 32 bytes total (8 × u32)
}

@group(0) @binding(0) var<storage, read> particles: array<Particle>;
@group(0) @binding(1) var<storage, read_write> visibility: VisibilityData;
@group(0) @binding(2) var<uniform> params: CullingParams;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    
    // Bounds check
    if (idx >= params.particle_count) {
        return;
    }
    
    let p = particles[idx];
    
    // Skip dead particles
    if (p.age >= p.lifetime) {
        return;
    }
    
    // Bounding sphere radius (half size)
    let radius = p.size * 0.5;
    
    // Test against 6 frustum planes
    var visible = true;
    
    for (var i = 0u; i < 6u; i++) {
        let plane = params.planes[i];
        
        // Distance from sphere center to plane
        let dist = dot(plane.normal, p.position) + plane.distance;
        
        // If sphere is entirely behind plane (negative side), it's outside
        if (dist < -radius) {
            visible = false;
            break;
        }
    }
    
    // If visible, increment counter
    if (visible) {
        atomicAdd(&visibility.visible_count, 1u);
    }
}

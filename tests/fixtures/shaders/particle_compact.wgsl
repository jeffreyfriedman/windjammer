// GPU Particle Compaction Shader
//
// Counts alive particles and builds alive_indices array.
// Updates indirect draw args (vertex_count = 6 × alive_count).
// Eliminates CPU readback overhead.

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

// DrawIndirect buffer (wgpu::DrawIndirect format)
struct DrawIndirectArgs {
    vertex_count: atomic<u32>,      // 6 × alive_count (updated by GPU)
    instance_count: u32,            // Always 1
    first_vertex: u32,              // Always 0
    first_instance: u32,            // Always 0
}

struct CompactParams {
    capacity: u32,
    _padding1: u32,
    _padding2: u32,
    _padding3: u32,
    // Total: 16 bytes (vec4 equivalent)
}

@group(0) @binding(0) var<storage, read> particles: array<Particle>;
@group(0) @binding(1) var<storage, read_write> indirect_args: DrawIndirectArgs;
@group(0) @binding(2) var<storage, read_write> alive_indices: array<atomic<u32>>;
@group(0) @binding(3) var<uniform> params: CompactParams;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    
    // Bounds check
    if (idx >= params.capacity) {
        return;
    }
    
    let p = particles[idx];
    
    // If particle is alive, add to vertex count
    if (p.age < p.lifetime) {
        atomicAdd(&indirect_args.vertex_count, 6u);
        // Note: alive_indices would store particle index for sorting (future)
        // For now, just count vertices (6 per particle)
    }
}

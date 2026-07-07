// GPU Particle Sorting Shader
//
// Bitonic sort for depth ordering (back-to-front rendering).
// Compare-exchange pattern: log2(N)² passes.

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

struct SortParams {
    camera_pos: vec3<f32>,      // Camera position for depth calculation
    _padding1: f32,
    stage: u32,                 // Current sort stage (2^stage = distance)
    sort_pass: u32,             // Current pass within stage (renamed from 'pass')
    particle_count: u32,        // Number of particles to sort
    _padding2: u32,
}

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<storage, read_write> sort_keys: array<f32>;  // Depth keys
@group(0) @binding(2) var<storage, read_write> sort_indices: array<u32>;  // Particle indices
@group(0) @binding(3) var<uniform> params: SortParams;

// Compute depth keys (camera distance squared)
@compute @workgroup_size(64)
fn compute_keys(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    if (idx >= params.particle_count) {
        return;
    }
    
    let p = particles[idx];
    
    // Skip dead particles (infinite depth)
    if (p.age >= p.lifetime) {
        sort_keys[idx] = 999999.0;
        sort_indices[idx] = idx;
        return;
    }
    
    // Compute depth (distance squared from camera)
    let dx = p.position.x - params.camera_pos.x;
    let dy = p.position.y - params.camera_pos.y;
    let dz = p.position.z - params.camera_pos.z;
    let depth_sq = dx * dx + dy * dy + dz * dz;
    
    sort_keys[idx] = depth_sq;
    sort_indices[idx] = idx;
}

// Bitonic sort compare-exchange pass
@compute @workgroup_size(64)
fn bitonic_sort(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    if (idx >= params.particle_count) {
        return;
    }
    
    // Bitonic sort: compare-exchange at distance 2^stage
    let stage_dist = 1u << params.stage;
    let pass_dist = 1u << params.sort_pass;
    
    // Determine partner index for compare-exchange
    let box_idx = idx / (stage_dist * 2u);
    let in_box_idx = idx % (stage_dist * 2u);
    
    // Ascending or descending based on box
    let ascending = (box_idx % 2u) == 0u;
    
    // Partner index (XOR with pass distance)
    let partner = idx ^ pass_dist;
    
    if (partner >= params.particle_count || idx >= partner) {
        return;
    }
    
    // Compare keys
    let key1 = sort_keys[idx];
    let key2 = sort_keys[partner];
    
    // Swap if needed (descending = back-to-front)
    let should_swap = select(key1 < key2, key1 > key2, ascending);
    
    if (should_swap) {
        // Swap keys
        let temp_key = key1;
        sort_keys[idx] = key2;
        sort_keys[partner] = temp_key;
        
        // Swap indices
        let temp_idx = sort_indices[idx];
        sort_indices[idx] = sort_indices[partner];
        sort_indices[partner] = temp_idx;
    }
}

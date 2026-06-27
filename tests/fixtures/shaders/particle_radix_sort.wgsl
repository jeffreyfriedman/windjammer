// Sprint 28: Radix Sort Shaders
// O(kN) sorting algorithm (4-bit radix, 4 passes for 16-bit keys)

struct Particle {
    position: vec3<f32>,
    velocity: vec3<f32>,
    color: vec4<f32>,
    size: f32,
    age: f32,
    lifetime: f32,
    _padding: f32,
}

struct RadixParams {
    particle_count: u32,
    radix_pass: u32,   // 0-3 (current pass) - renamed from 'pass' (WGSL reserved keyword)
    radix_shift: u32,  // 0, 4, 8, 12 (bits to shift)
    _padding: u32,
}

// Phase 0: Compute Depth Keys (distance from camera)
struct CameraParams {
    position: vec3<f32>,
    _padding: f32,
}

@group(0) @binding(0) var<storage, read> particles: array<Particle>;
@group(0) @binding(1) var<storage, read_write> depth_keys: array<u32>;
@group(0) @binding(2) var<uniform> camera_params: CameraParams;

@compute @workgroup_size(64)
fn compute_keys(@builtin(global_invocation_id) id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    let idx = id.x;
    let total_particles = num_workgroups.x * 64u;
    
    if (idx >= total_particles) {
        return;
    }
    
    let particle = particles[idx];
    
    // Compute distance squared from camera
    let dx = particle.position.x - camera_params.position.x;
    let dy = particle.position.y - camera_params.position.y;
    let dz = particle.position.z - camera_params.position.z;
    let dist_sq = dx * dx + dy * dy + dz * dz;
    
    // Dead particles get max distance (render last)
    if (particle.age >= particle.lifetime) {
        depth_keys[idx] = 0xFFFFu;  // Max 16-bit value
    } else {
        // Convert to 16-bit key (sufficient for sorting)
        // Scale distance squared to fit in 16 bits
        // Assuming max distance of ~1600 units (dist_sq ~ 2.56M)
        // Scale factor: 2.56M / 65535 ≈ 39
        let key_16bit = u32(min(dist_sq / 40.0, 65535.0));
        depth_keys[idx] = key_16bit;
    }
}

// Phase 1: Histogram (Count keys per bin)
@group(0) @binding(0) var<storage, read> keys_for_histogram: array<u32>;
@group(0) @binding(1) var<storage, read_write> histograms: array<atomic<u32>>;
@group(0) @binding(2) var<uniform> histogram_params: RadixParams;

@compute @workgroup_size(64)
fn histogram(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    if (idx >= histogram_params.particle_count) {
        return;
    }
    
    let key = keys_for_histogram[idx];
    
    // Extract 4-bit radix digit at current pass
    let digit = (key >> histogram_params.radix_shift) & 0xFu;
    
    // Increment global histogram bin (shared across all workgroups)
    atomicAdd(&histograms[digit], 1u);
}

// Phase 2: Scan (Prefix sum to compute offsets from histogram)
// NOTE: Only 16 bins, so single workgroup is sufficient
@group(0) @binding(0) var<storage, read> histogram_counts: array<u32>;
@group(0) @binding(1) var<storage, read_write> scan_offsets: array<atomic<u32>>;

@compute @workgroup_size(16)  // One thread per bin
fn scan(@builtin(local_invocation_id) local_id: vec3<u32>) {
    let bin_id = local_id.x;
    
    // Compute exclusive prefix sum (each bin gets sum of all previous bins)
    var sum = 0u;
    for (var i = 0u; i < bin_id; i = i + 1u) {
        sum = sum + histogram_counts[i];
    }
    
    // Store offset for this bin
    atomicStore(&scan_offsets[bin_id], sum);
}

// Phase 3: Scatter (Reorder particles by sorted positions)
@group(0) @binding(0) var<storage, read> keys_in: array<u32>;
@group(0) @binding(1) var<storage, read> indices_in: array<u32>;
@group(0) @binding(2) var<storage, read_write> offsets: array<atomic<u32>>;
@group(0) @binding(3) var<storage, read_write> keys_out: array<u32>;
@group(0) @binding(4) var<storage, read_write> indices_out: array<u32>;
@group(0) @binding(5) var<uniform> params_scatter: RadixParams;

@compute @workgroup_size(64)
fn scatter(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    if (idx >= params_scatter.particle_count) {
        return;
    }
    
    // Get key and extract digit
    let key = keys_in[idx];
    let digit = (key >> params_scatter.radix_shift) & 0xFu;
    
    // Atomically get output position from offset
    let pos = atomicAdd(&offsets[digit], 1u);
    
    // Write to sorted position
    keys_out[pos] = key;
    indices_out[pos] = indices_in[idx];
}

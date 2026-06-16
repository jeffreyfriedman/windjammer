// GPU Particle Update Compute Shader
//
// Updates particles: velocity integration, lifetime, forces, culling.
// Workgroup size: 64 threads (standard for compute)

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

struct UpdateParams {
    gravity: vec3<f32>,
    dt: f32,
    particle_count: u32,
    _padding: vec3<u32>,
}

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<uniform> params: UpdateParams;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    if (idx >= params.particle_count) {
        return;
    }
    
    var p = particles[idx];
    
    // Skip dead particles
    if (p.age >= p.lifetime) {
        return;
    }
    
    // Apply forces (gravity)
    let accel = params.gravity;
    
    // Integrate velocity
    p.velocity += accel * params.dt;
    
    // Integrate position
    p.position += p.velocity * params.dt;
    
    // Update age
    p.age += params.dt;
    
    // Write back
    particles[idx] = p;
}

// GPU Particle Initialization Compute Shader
//
// Initializes particles with spawn parameters.
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

struct SpawnerParams {
    position: vec3<f32>,
    _padding1: f32,
    velocity: vec3<f32>,
    _padding2: f32,
    color: vec4<f32>,
    lifetime: f32,
    count: u32,
    _padding3: vec3<f32>,
    _padding4: vec3<f32>,
}

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<uniform> spawner: SpawnerParams;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    if (idx >= spawner.count) {
        return;
    }
    
    // Initialize particle
    particles[idx].position = spawner.position;
    particles[idx].velocity = spawner.velocity;
    particles[idx].color = spawner.color;
    particles[idx].size = 1.0;
    particles[idx].lifetime = spawner.lifetime;
    particles[idx].age = 0.0;
}

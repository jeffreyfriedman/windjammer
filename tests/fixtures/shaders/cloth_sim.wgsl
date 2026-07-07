// Position-Based Dynamics (PBD) cloth — GPU compute
// Predict → Gauss–Seidel constraint projection (single global order) → collisions → pins → finalize velocities.
// Group 0: uniforms + vertex buffers. Group 1: constraints + pins + collision primitives.

struct ClothSimUniforms {
    dt: f32,
    damping: f32,
    stiffness: f32,
    _pad0: f32,
    gravity: vec3<f32>,
    _pad_g: f32,
    wind: vec3<f32>,
    _pad_w: f32,
    vertex_count: u32,
    distance_constraint_count: u32,
    bending_constraint_count: u32,
    sphere_count: u32,
    capsule_count: u32,
    grid_width: u32,
    grid_height: u32,
    _pad_tail: u32,
}

struct DistanceConstraint {
    i0: u32,
    i1: u32,
    rest_length: f32,
    _pad: f32,
}

struct SphereCollider {
    center_radius: vec4<f32>, // xyz, radius in w
}

struct CapsuleCollider {
    a_radius: vec4<f32>, // endpoint A xyz, radius w
    b_xyz_pad: vec4<f32>, // endpoint B xyz, w unused
}

@group(0) @binding(0) var<uniform> sim: ClothSimUniforms;
@group(0) @binding(1) var<storage, read_write> positions: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read_write> velocities: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read_write> predicted: array<vec4<f32>>;
@group(0) @binding(4) var<storage, read_write> prev_positions: array<vec4<f32>>;

// Distance constraints first [0..distance_count), bending [distance_count..distance_count+bending_count)
@group(1) @binding(0) var<storage, read> all_constraints: array<DistanceConstraint>;
@group(1) @binding(1) var<storage, read> pin_positions: array<vec4<f32>>;
@group(1) @binding(2) var<storage, read> spheres: array<SphereCollider>;
@group(1) @binding(3) var<storage, read> capsules: array<CapsuleCollider>;

fn pin_active(i: u32) -> bool {
    return pin_positions[i].w > 0.5;
}

// --- Predict: integrate velocity (gravity + wind), predict position ---

@compute @workgroup_size(256)
fn cloth_predict(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= sim.vertex_count) {
        return;
    }
    if (pin_active(i)) {
        predicted[i] = positions[i];
        return;
    }
    let dt = sim.dt;
    let g = sim.gravity;
    let w = sim.wind;
    let x = positions[i].xyz;
    let inv_mass = positions[i].w;
    if (inv_mass <= 0.0) {
        predicted[i] = positions[i];
        return;
    }
    var v = velocities[i].xyz;
    v = v + (g + w) * dt;
    let x_pred = x + v * dt;
    predicted[i] = vec4<f32>(x_pred, inv_mass);
}

// Gauss–Seidel: one invocation loops all constraints in order (no atomics).

@compute @workgroup_size(1)
fn cloth_solve_constraints(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x != 0u) {
        return;
    }
    let s = sim.stiffness;
    var ci = 0u;
    while (ci < sim.distance_constraint_count) {
        let c = all_constraints[ci];
        project_distance_edge(c.i0, c.i1, c.rest_length, s);
        ci = ci + 1u;
    }
    ci = 0u;
    while (ci < sim.bending_constraint_count) {
        let c = all_constraints[sim.distance_constraint_count + ci];
        project_distance_edge(c.i0, c.i1, c.rest_length, s);
        ci = ci + 1u;
    }
}

fn project_distance_edge(i0: u32, i1: u32, rest: f32, stiffness: f32) {
    if (pin_active(i0) && pin_active(i1)) {
        return;
    }
    var p0 = predicted[i0].xyz;
    var p1 = predicted[i1].xyz;
    let w0 = select(predicted[i0].w, 0.0, pin_active(i0));
    let w1 = select(predicted[i1].w, 0.0, pin_active(i1));
    let wsum = w0 + w1;
    if (wsum < 1e-8) {
        return;
    }
    let d = p0 - p1;
    let len = length(d);
    if (len < 1e-8) {
        return;
    }
    let n = d / len;
    let c = len - rest;
    let lambda = (c / wsum) * stiffness;
    let corr0 = -w0 * lambda * n;
    let corr1 = w1 * lambda * n;
    p0 = p0 + corr0;
    p1 = p1 + corr1;
    predicted[i0] = vec4<f32>(p0, predicted[i0].w);
    predicted[i1] = vec4<f32>(p1, predicted[i1].w);
}

// --- Sphere / capsule collision (post projection) ---

fn closest_point_on_segment(p: vec3<f32>, a: vec3<f32>, b: vec3<f32>) -> vec3<f32> {
    let ab = b - a;
    let denom = dot(ab, ab);
    if (denom < 1e-12) {
        return a;
    }
    let t = clamp(dot(p - a, ab) / denom, 0.0, 1.0);
    return a + ab * t;
}

@compute @workgroup_size(256)
fn cloth_collisions(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= sim.vertex_count) {
        return;
    }
    if (pin_active(i)) {
        return;
    }
    var p = predicted[i].xyz;
    var si = 0u;
    while (si < sim.sphere_count) {
        let s = spheres[si].center_radius;
        let center = s.xyz;
        let r = s.w;
        let off = p - center;
        let dist = length(off);
        if (dist < r && dist > 1e-8) {
            p = center + normalize(off) * r;
        } else if (dist <= 1e-8) {
            p = center + vec3<f32>(0.0, r, 0.0);
        }
        si = si + 1u;
    }
    var ci = 0u;
    while (ci < sim.capsule_count) {
        let cap = capsules[ci];
        let a = cap.a_radius.xyz;
        let b = cap.b_xyz_pad.xyz;
        let r = cap.a_radius.w;
        let closest = closest_point_on_segment(p, a, b);
        let off = p - closest;
        let dist = length(off);
        if (dist < r && dist > 1e-8) {
            p = closest + normalize(off) * r;
        } else if (dist <= 1e-8) {
            p = closest + vec3<f32>(0.0, r, 0.0);
        }
        ci = ci + 1u;
    }
    predicted[i] = vec4<f32>(p, predicted[i].w);
}

@compute @workgroup_size(256)
fn cloth_apply_pins(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= sim.vertex_count) {
        return;
    }
    if (pin_active(i)) {
        predicted[i] = vec4<f32>(pin_positions[i].xyz, predicted[i].w);
    }
}

@compute @workgroup_size(256)
fn cloth_finalize(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= sim.vertex_count) {
        return;
    }
    let dt = sim.dt;
    if (dt < 1e-8) {
        return;
    }
    let x0 = prev_positions[i].xyz;
    let x1 = predicted[i].xyz;
    let damp = sim.damping;
    var v = (x1 - x0) / dt;
    v = v * (1.0 - damp);
    velocities[i] = vec4<f32>(v, 0.0);
    positions[i] = vec4<f32>(x1, positions[i].w);
}

// SDF Evaluate Compute Shader
//
// Evaluates an SDF scene at grid points, producing voxel data.
// The scene is described as a flat instruction buffer: each instruction
// specifies a primitive or CSG operation with parameters.
//
// Instruction format (per instruction, 16 floats = 64 bytes):
//   [0]: opcode (f32 cast to u32)
//   [1]: material_id (f32 cast to u32, for primitives)
//   [2-15]: parameters (depends on opcode)
//
// Opcodes:
//   0: END
//   1: SPHERE   (cx, cy, cz, radius)
//   2: BOX      (cx, cy, cz, hx, hy, hz)
//   3: CYLINDER (cx, cy, cz, radius, half_height)
//   4: TORUS    (cx, cy, cz, major_r, minor_r)
//   5: PLANE    (nx, ny, nz, offset)
//   6: CAPSULE  (ax, ay, az, bx, by, bz, radius)
//   7: CONE     (cx, cy, cz, radius, height)
//   8: ROUNDED_BOX (cx, cy, cz, hx, hy, hz, rounding)
//
//   // CSG operations pop two values from the stack
//   20: UNION
//   21: INTERSECT
//   22: DIFFERENCE
//   23: SMOOTH_UNION (k)
//   24: SMOOTH_INTERSECT (k)
//   25: SMOOTH_DIFFERENCE (k)
//
//   // Domain transforms modify the query point
//   30: TRANSLATE (tx, ty, tz)
//   31: ROTATE_Y (angle)
//   32: ROTATE_X (angle)
//   33: ROTATE_Z (angle)
//   34: REPEAT (sx, sy, sz)
//   35: MIRROR (axis_x, axis_y, axis_z)  // 1.0 to mirror, 0.0 to keep
//   36: PUSH_POINT
//   37: POP_POINT

struct EvalParams {
    grid_size: vec3<u32>,   // voxel grid dimensions
    _pad1: u32,
    grid_origin: vec3<f32>, // world-space origin of grid
    _pad2: f32,
    voxel_size: f32,        // world units per voxel
    instruction_count: u32,
    _pad3: vec2<f32>,
}

@group(0) @binding(0) var<uniform> params: EvalParams;
@group(0) @binding(1) var<storage, read> instructions: array<f32>;  // instruction_count * 16
@group(0) @binding(2) var<storage, read_write> voxels: array<u32>;  // grid_size.x * y * z

// === SDF Primitives ===
fn sdf_sphere(p: vec3<f32>, center: vec3<f32>, radius: f32) -> f32 {
    return length(p - center) - radius;
}

fn sdf_box(p: vec3<f32>, center: vec3<f32>, half_ext: vec3<f32>) -> f32 {
    let d = abs(p - center) - half_ext;
    return length(max(d, vec3<f32>(0.0))) + min(max(d.x, max(d.y, d.z)), 0.0);
}

fn sdf_rounded_box(p: vec3<f32>, center: vec3<f32>, half_ext: vec3<f32>, rounding: f32) -> f32 {
    return sdf_box(p, center, half_ext - vec3<f32>(rounding)) - rounding;
}

fn sdf_cylinder(p: vec3<f32>, center: vec3<f32>, radius: f32, half_height: f32) -> f32 {
    let d = p - center;
    let rd = length(d.xz) - radius;
    let vd = abs(d.y) - half_height;
    return length(max(vec2<f32>(rd, vd), vec2<f32>(0.0))) + min(max(rd, vd), 0.0);
}

fn sdf_torus(p: vec3<f32>, center: vec3<f32>, major_r: f32, minor_r: f32) -> f32 {
    let d = p - center;
    let q = vec2<f32>(length(d.xz) - major_r, d.y);
    return length(q) - minor_r;
}

fn sdf_plane(p: vec3<f32>, normal: vec3<f32>, offset: f32) -> f32 {
    return dot(p, normal) - offset;
}

fn sdf_capsule(p: vec3<f32>, a: vec3<f32>, b: vec3<f32>, radius: f32) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
    return length(pa - ba * h) - radius;
}

fn sdf_cone(p: vec3<f32>, center: vec3<f32>, radius: f32, height: f32) -> f32 {
    let d = p - center;
    let q = length(d.xz);
    let len = sqrt(radius * radius + height * height);
    let kx = height / len;
    let ky = radius / len;
    let dot_val = q * kx + d.y * ky;
    if (dot_val < 0.0) {
        return length(vec2<f32>(q, d.y));
    } else if (dot_val > height) {
        let base_dy = d.y - height;
        let base_dq = q - radius;
        if (base_dq > 0.0) {
            return sqrt(base_dq * base_dq + base_dy * base_dy);
        }
        return abs(base_dy);
    }
    return q * kx + d.y * ky - height * kx;
}

// === CSG Operations ===
fn csg_union(d1: f32, d2: f32) -> f32 { return min(d1, d2); }
fn csg_intersect(d1: f32, d2: f32) -> f32 { return max(d1, d2); }
fn csg_difference(d1: f32, d2: f32) -> f32 { return max(d1, -d2); }

fn csg_smooth_union(d1: f32, d2: f32, k: f32) -> f32 {
    let h = clamp(0.5 + 0.5 * (d2 - d1) / k, 0.0, 1.0);
    return mix(d2, d1, h) - k * h * (1.0 - h);
}

fn csg_smooth_intersect(d1: f32, d2: f32, k: f32) -> f32 {
    let h = clamp(0.5 - 0.5 * (d2 - d1) / k, 0.0, 1.0);
    return mix(d2, d1, h) + k * h * (1.0 - h);
}

fn csg_smooth_difference(d1: f32, d2: f32, k: f32) -> f32 {
    let h = clamp(0.5 - 0.5 * (d2 + d1) / k, 0.0, 1.0);
    return mix(d1, -d2, h) + k * h * (1.0 - h);
}

// === Domain Transforms ===
fn domain_rotate_y(p: vec3<f32>, angle: f32) -> vec3<f32> {
    let c = cos(angle); let s = sin(angle);
    return vec3<f32>(p.x * c + p.z * s, p.y, -p.x * s + p.z * c);
}

fn domain_rotate_x(p: vec3<f32>, angle: f32) -> vec3<f32> {
    let c = cos(angle); let s = sin(angle);
    return vec3<f32>(p.x, p.y * c + p.z * s, -p.y * s + p.z * c);
}

fn domain_rotate_z(p: vec3<f32>, angle: f32) -> vec3<f32> {
    let c = cos(angle); let s = sin(angle);
    return vec3<f32>(p.x * c + p.y * s, -p.x * s + p.y * c, p.z);
}

fn domain_repeat(p: vec3<f32>, spacing: vec3<f32>) -> vec3<f32> {
    return p - round(p / spacing) * spacing;
}

fn domain_mirror(p: vec3<f32>, axes: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        select(p.x, abs(p.x), axes.x > 0.5),
        select(p.y, abs(p.y), axes.y > 0.5),
        select(p.z, abs(p.z), axes.z > 0.5)
    );
}

// === SDF Scene Evaluator ===
// Uses a stack-based approach: primitives push distance values,
// CSG ops pop two and push result, domain ops modify the query point.

const MAX_STACK_SIZE: u32 = 16u;
const MAX_POINT_STACK: u32 = 8u;

fn evaluate_scene(world_pos: vec3<f32>) -> vec2<f32> {
    // Returns vec2(distance, material_id)
    var dist_stack: array<f32, 16>;
    var mat_stack: array<u32, 16>;
    var point_stack: array<vec3<f32>, 8>;
    var sp = 0u;       // stack pointer
    var pp = 0u;       // point stack pointer
    var p = world_pos; // current query point

    for (var i = 0u; i < params.instruction_count; i++) {
        let base = i * 16u;
        let opcode = u32(instructions[base]);
        let material = u32(instructions[base + 1u]);

        if (opcode == 0u) { break; }

        // Primitives: evaluate and push
        if (opcode == 1u) {
            // SPHERE
            let center = vec3<f32>(instructions[base + 2u], instructions[base + 3u], instructions[base + 4u]);
            let radius = instructions[base + 5u];
            let d = sdf_sphere(p, center, radius);
            if (sp < MAX_STACK_SIZE) { dist_stack[sp] = d; mat_stack[sp] = material; sp++; }
        } else if (opcode == 2u) {
            // BOX
            let center = vec3<f32>(instructions[base + 2u], instructions[base + 3u], instructions[base + 4u]);
            let half_ext = vec3<f32>(instructions[base + 5u], instructions[base + 6u], instructions[base + 7u]);
            let d = sdf_box(p, center, half_ext);
            if (sp < MAX_STACK_SIZE) { dist_stack[sp] = d; mat_stack[sp] = material; sp++; }
        } else if (opcode == 3u) {
            // CYLINDER
            let center = vec3<f32>(instructions[base + 2u], instructions[base + 3u], instructions[base + 4u]);
            let radius = instructions[base + 5u];
            let half_h = instructions[base + 6u];
            let d = sdf_cylinder(p, center, radius, half_h);
            if (sp < MAX_STACK_SIZE) { dist_stack[sp] = d; mat_stack[sp] = material; sp++; }
        } else if (opcode == 4u) {
            // TORUS
            let center = vec3<f32>(instructions[base + 2u], instructions[base + 3u], instructions[base + 4u]);
            let major = instructions[base + 5u];
            let minor = instructions[base + 6u];
            let d = sdf_torus(p, center, major, minor);
            if (sp < MAX_STACK_SIZE) { dist_stack[sp] = d; mat_stack[sp] = material; sp++; }
        } else if (opcode == 5u) {
            // PLANE
            let normal = vec3<f32>(instructions[base + 2u], instructions[base + 3u], instructions[base + 4u]);
            let offset = instructions[base + 5u];
            let d = sdf_plane(p, normal, offset);
            if (sp < MAX_STACK_SIZE) { dist_stack[sp] = d; mat_stack[sp] = material; sp++; }
        } else if (opcode == 6u) {
            // CAPSULE
            let a = vec3<f32>(instructions[base + 2u], instructions[base + 3u], instructions[base + 4u]);
            let b = vec3<f32>(instructions[base + 5u], instructions[base + 6u], instructions[base + 7u]);
            let radius = instructions[base + 8u];
            let d = sdf_capsule(p, a, b, radius);
            if (sp < MAX_STACK_SIZE) { dist_stack[sp] = d; mat_stack[sp] = material; sp++; }
        } else if (opcode == 7u) {
            // CONE
            let center = vec3<f32>(instructions[base + 2u], instructions[base + 3u], instructions[base + 4u]);
            let radius = instructions[base + 5u];
            let height = instructions[base + 6u];
            let d = sdf_cone(p, center, radius, height);
            if (sp < MAX_STACK_SIZE) { dist_stack[sp] = d; mat_stack[sp] = material; sp++; }
        } else if (opcode == 8u) {
            // ROUNDED_BOX
            let center = vec3<f32>(instructions[base + 2u], instructions[base + 3u], instructions[base + 4u]);
            let half_ext = vec3<f32>(instructions[base + 5u], instructions[base + 6u], instructions[base + 7u]);
            let rounding = instructions[base + 8u];
            let d = sdf_rounded_box(p, center, half_ext, rounding);
            if (sp < MAX_STACK_SIZE) { dist_stack[sp] = d; mat_stack[sp] = material; sp++; }
        }
        // CSG operations
        else if (opcode >= 20u && opcode <= 25u && sp >= 2u) {
            sp--;
            let d2 = dist_stack[sp];
            let m2 = mat_stack[sp];
            sp--;
            let d1 = dist_stack[sp];
            let m1 = mat_stack[sp];

            var result = d1;
            var result_mat = m1;

            if (opcode == 20u) {
                result = csg_union(d1, d2);
                result_mat = select(m1, m2, d2 < d1);
            } else if (opcode == 21u) {
                result = csg_intersect(d1, d2);
                result_mat = select(m2, m1, d1 > d2);
            } else if (opcode == 22u) {
                result = csg_difference(d1, d2);
                result_mat = m1;
            } else if (opcode == 23u) {
                let k = instructions[base + 2u];
                result = csg_smooth_union(d1, d2, k);
                result_mat = select(m1, m2, d2 < d1);
            } else if (opcode == 24u) {
                let k = instructions[base + 2u];
                result = csg_smooth_intersect(d1, d2, k);
                result_mat = select(m2, m1, d1 > d2);
            } else if (opcode == 25u) {
                let k = instructions[base + 2u];
                result = csg_smooth_difference(d1, d2, k);
                result_mat = m1;
            }

            dist_stack[sp] = result;
            mat_stack[sp] = result_mat;
            sp++;
        }
        // Domain transforms
        else if (opcode == 30u) {
            let t = vec3<f32>(instructions[base + 2u], instructions[base + 3u], instructions[base + 4u]);
            p = p - t;
        } else if (opcode == 31u) {
            p = domain_rotate_y(p, instructions[base + 2u]);
        } else if (opcode == 32u) {
            p = domain_rotate_x(p, instructions[base + 2u]);
        } else if (opcode == 33u) {
            p = domain_rotate_z(p, instructions[base + 2u]);
        } else if (opcode == 34u) {
            let s = vec3<f32>(instructions[base + 2u], instructions[base + 3u], instructions[base + 4u]);
            p = domain_repeat(p, s);
        } else if (opcode == 35u) {
            let axes = vec3<f32>(instructions[base + 2u], instructions[base + 3u], instructions[base + 4u]);
            p = domain_mirror(p, axes);
        } else if (opcode == 36u) {
            // PUSH_POINT
            if (pp < MAX_POINT_STACK) { point_stack[pp] = p; pp++; }
        } else if (opcode == 37u) {
            // POP_POINT
            if (pp > 0u) { pp--; p = point_stack[pp]; }
        }
    }

    if (sp > 0u) {
        return vec2<f32>(dist_stack[sp - 1u], f32(mat_stack[sp - 1u]));
    }
    return vec2<f32>(1000.0, 0.0);  // no result
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let gx = id.x;
    let gy = id.y;

    if (gx >= params.grid_size.x || gy >= params.grid_size.y) { return; }

    // Process all Z values for this X,Y
    for (var gz = 0u; gz < params.grid_size.z; gz++) {
        let world_pos = params.grid_origin + vec3<f32>(
            f32(gx) * params.voxel_size + params.voxel_size * 0.5,
            f32(gy) * params.voxel_size + params.voxel_size * 0.5,
            f32(gz) * params.voxel_size + params.voxel_size * 0.5
        );

        let result = evaluate_scene(world_pos);
        let distance = result.x;
        let material = u32(result.y);

        // Threshold: if distance < 0, voxel is inside the surface
        let voxel_idx = gx + gy * params.grid_size.x + gz * params.grid_size.x * params.grid_size.y;
        if (distance < 0.0) {
            voxels[voxel_idx] = material;
        } else {
            voxels[voxel_idx] = 0u;
        }
    }
}

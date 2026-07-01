// TressFX-style strand simulation: Verlet integration, distance constraints,
// global shape blend, wind + turbulence, capsule collision (head).

struct SimUniforms {
  gravity: vec4<f32>,
  wind_dir: vec4<f32>,
  capsule_a: vec4<f32>,
  capsule_b: vec4<f32>,
  dt: f32,
  wind_strength: f32,
  turbulence: f32,
  time: f32,
  capsule_radius: f32,
  shape_blend: f32,
  total_vertices: u32,
  num_strands: u32,
  verts_per_strand: u32,
  _pad: u32,
}

struct ConstraintUniforms {
  length_stiffness: f32,
  damping: f32,
  bend_stiffness: f32,
  shape_strength: f32,
  constraint_iters: u32,
  _p0: u32,
  _p1: u32,
  _p2: u32,
}

struct Strand {
  start: u32,
  count: u32,
}

@group(0) @binding(0) var<uniform> sim: SimUniforms;
@group(0) @binding(1) var<storage, read_write> positions: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read_write> positions_prev: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read> rest_positions: array<vec4<f32>>;

@group(1) @binding(0) var<uniform> constraint: ConstraintUniforms;
@group(1) @binding(1) var<storage, read> strands: array<Strand>;

fn capsule_sdf(p: vec3<f32>, a: vec3<f32>, b: vec3<f32>, r: f32) -> f32 {
  let pa = p - a;
  let ba = b - a;
  let denom = max(dot(ba, ba), 1e-8);
  let h = clamp(dot(pa, ba) / denom, 0.0, 1.0);
  return length(pa - ba * h) - r;
}

fn turbulence_wind(base: vec3<f32>, p: vec3<f32>, t: f32) -> vec3<f32> {
  let s = sin(p.x * 3.1 + t) + sin(p.y * 2.7 + t * 1.3) + sin(p.z * 2.9 + t * 0.7);
  let off = vec3<f32>(
    sin(t * 1.1 + p.z),
    cos(t * 0.9 + p.x),
    sin(t * 1.05 + p.y),
  );
  return base + off * s * 0.15;
}

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let strand = gid.x;
  if (strand >= sim.num_strands) {
    return;
  }

  let s = strands[strand];
  let start = s.start;
  let count = s.count;
  if (count < 2u) {
    return;
  }

  let g = sim.gravity.xyz;
  let dt = sim.dt;
  let dt2 = dt * dt;
  let damp = clamp(constraint.damping, 0.0, 0.9999);
  let wind_base = sim.wind_dir.xyz * sim.wind_strength;

  // --- Verlet integration per vertex on this strand ---
  for (var i = 0u; i < count; i = i + 1u) {
    let vi = start + i;
    if (vi >= sim.total_vertices) {
      continue;
    }

    var x = positions[vi].xyz;
    let x0 = positions_prev[vi].xyz;

    // Root vertex follows rest (kinematic anchor)
    if (i == 0u) {
      let rest = rest_positions[vi].xyz;
      positions_prev[vi] = vec4<f32>(rest, 1.0);
      positions[vi] = vec4<f32>(rest, 1.0);
      continue;
    }

    let wind = turbulence_wind(wind_base, x, sim.time) * sim.turbulence;
    let accel = g + wind;

    var v = (x - x0) * (1.0 - damp);
    var x_new = x + v + accel * dt2;

    positions_prev[vi] = vec4<f32>(x, 1.0);
    positions[vi] = vec4<f32>(x_new, 1.0);
  }

  // --- Iterative distance + shape constraints (Jacobi-style along chain) ---
  let rest_len_scale = 1.0;
  let len_k = constraint.length_stiffness;
  let shape_k = constraint.shape_strength * sim.shape_blend;
  let bend_k = constraint.bend_stiffness;

  for (var iter = 0u; iter < constraint.constraint_iters; iter = iter + 1u) {
    // Length constraints between consecutive vertices
    for (var i = 0u; i < count - 1u; i = i + 1u) {
      let i0 = start + i;
      let i1 = start + i + 1u;
      let p0 = positions[i0].xyz;
      let p1 = positions[i1].xyz;
      let r0 = rest_positions[i0].xyz;
      let r1 = rest_positions[i1].xyz;
      let rest_len = length(r1 - r0) * rest_len_scale;
      if (rest_len < 1e-6) {
        continue;
      }
      let d = p1 - p0;
      let len = length(d);
      if (len < 1e-8) {
        continue;
      }
      let err = (len - rest_len) / len;
      let corr = d * err * 0.5 * len_k;

      if (i > 0u) {
        positions[i0] = vec4<f32>(p0 + corr, 1.0);
      }
      positions[i1] = vec4<f32>(p1 - corr, 1.0);
    }

    // Global shape: blend toward rest (skip root)
    if (shape_k > 1e-5) {
      for (var i = 1u; i < count; i = i + 1u) {
        let vi = start + i;
        let p = positions[vi].xyz;
        let r = rest_positions[vi].xyz;
        let np = mix(p, r, shape_k);
        positions[vi] = vec4<f32>(np, 1.0);
      }
    }

    // Simple bend: reduce deviation from straight line between neighbors (3-point)
    if (bend_k > 1e-5 && count > 2u) {
      for (var i = 1u; i < count - 1u; i = i + 1u) {
        let im = start + i - 1u;
        let vi = start + i;
        let ip = start + i + 1u;
        let prev = positions[im].xyz;
        let cur = positions[vi].xyz;
        let next = positions[ip].xyz;
        let mid = 0.5 * (prev + next);
        let nc = mix(cur, mid, bend_k * 0.25);
        positions[vi] = vec4<f32>(nc, 1.0);
      }
    }
  }

  // --- Capsule collision (push outside head capsule) ---
  let cap_a = sim.capsule_a.xyz;
  let cap_b = sim.capsule_b.xyz;
  let rad = sim.capsule_radius;

  for (var i = 1u; i < count; i = i + 1u) {
    let vi = start + i;
    var p = positions[vi].xyz;
    let sdf = capsule_sdf(p, cap_a, cap_b, rad);
    if (sdf < 0.0) {
      let pa = p - cap_a;
      let ba = cap_b - cap_a;
      let denom = max(dot(ba, ba), 1e-8);
      let h = clamp(dot(pa, ba) / denom, 0.0, 1.0);
      let closest = cap_a + ba * h;
      let n = normalize(p - closest);
      p = closest + n * (rad + 1e-4);
      positions[vi] = vec4<f32>(p, 1.0);
    }
  }
}

// Kajiya-Kay anisotropic hair: billboard strand segments + dual specular lobes.

struct HairRenderUniforms {
  view: mat4x4<f32>,
  proj: mat4x4<f32>,
  camera_pos: vec4<f32>,
  light_dir: vec4<f32>,
  hair_color: vec4<f32>,
  spec_shift_1: f32,
  spec_shift_2: f32,
  spec_width: f32,
  strand_width: f32,
  num_strands: u32,
  verts_per_strand: u32,
  strand_vertex_base: u32,
  _pad: u32,
}

@group(0) @binding(0) var<uniform> hair: HairRenderUniforms;
@group(0) @binding(1) var<storage, read> strand_positions: array<vec4<f32>>;

struct VertexOutput {
  @builtin(position) clip_pos: vec4<f32>,
  @location(0) tangent: vec3<f32>,
  @location(1) view_dir: vec3<f32>,
  @location(2) light_dir_v: vec3<f32>,
  @location(3) tip_fade: f32,
  @location(4) color: vec3<f32>,
}

fn safe_normalize(v: vec3<f32>) -> vec3<f32> {
  let l = length(v);
  if (l < 1e-8) {
    return vec3<f32>(0.0, 1.0, 0.0);
  }
  return v / l;
}

@vertex
fn vs_main(
  @builtin(vertex_index) vertex_index: u32,
  @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
  let corners = array<vec2<f32>, 6>(
    vec2<f32>(-1.0, 0.0),
    vec2<f32>(1.0, 0.0),
    vec2<f32>(1.0, 1.0),
    vec2<f32>(-1.0, 0.0),
    vec2<f32>(1.0, 1.0),
    vec2<f32>(-1.0, 1.0),
  );
  let uv = corners[vertex_index % 6u];

  let segs_per_strand = hair.verts_per_strand - 1u;
  let strand_id = instance_index / segs_per_strand;
  let seg = instance_index % segs_per_strand;

  let base = hair.strand_vertex_base + strand_id * hair.verts_per_strand;
  let i0 = base + seg;
  let i1 = base + seg + 1u;

  let p0 = strand_positions[i0].xyz;
  let p1 = strand_positions[i1].xyz;

  let tangent = safe_normalize(p1 - p0);
  let mid = 0.5 * (p0 + p1);
  let seg_len = length(p1 - p0);
  let cam = hair.camera_pos.xyz;
  let to_cam = safe_normalize(cam - mid);
  let w = hair.strand_width * 0.5;
  var side = cross(tangent, to_cam);
  let sl = length(side);
  if (sl < 1e-5) {
    side = cross(tangent, vec3<f32>(0.0, 1.0, 0.0));
  }
  side = safe_normalize(side);
  let thick = tangent * uv.y * seg_len * 0.5;

  let world_pos = mid + side * (uv.x * w) + thick;

  var out: VertexOutput;
  out.clip_pos = hair.proj * hair.view * vec4<f32>(world_pos, 1.0);
  out.tangent = tangent;
  out.view_dir = safe_normalize(cam - world_pos);
  out.light_dir_v = safe_normalize(hair.light_dir.xyz);
  let denom = max(f32(segs_per_strand), 1.0);
  let along = (f32(seg) + uv.y) / denom;
  // Thin strand tips (alpha test friendly)
  out.tip_fade = 1.0 - smoothstep(0.55, 1.0, along);
  out.color = hair.hair_color.xyz;
  return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  let T = safe_normalize(in.tangent);
  let V = in.view_dir;
  let L = in.light_dir_v;

  // Diffuse wrap using normal approx: orthogonal to tangent in (T,L) plane
  let B = safe_normalize(cross(T, L));
  let N = safe_normalize(cross(B, T));
  let wrap = clamp(dot(N, L) * 0.5 + 0.5, 0.0, 1.0);
  let rim = 1.0 - abs(dot(T, L));
  let diffuse = mix(wrap, rim, 0.35);

  // Kajiya-Kay: two shifted specular lobes along hair fiber (Marschner primary/secondary style)
  let cos_tl = clamp(dot(T, L), -1.0, 1.0);
  let cos_tv = clamp(dot(T, V), -1.0, 1.0);
  let sin_tl = sin(acos(cos_tl));
  let sin_tv = sin(acos(cos_tv));
  let theta_d = asin(clamp(sin_tl * cos_tv - cos_tl * sin_tv, -1.0, 1.0));

  let w = max(hair.spec_width, 2.0);
  let s1 = pow(max(0.0, sin(theta_d + hair.spec_shift_1)), w);
  let s2 = pow(max(0.0, sin(theta_d + hair.spec_shift_2)), w * 1.5);
  let spec = s1 * 0.6 + s2 * 0.4;

  let rgb = in.color * diffuse + vec3<f32>(spec * 1.25);
  let a = clamp(in.tip_fade, 0.0, 1.0);
  if (a < 0.04) {
    discard;
  }
  return vec4<f32>(rgb, a);
}

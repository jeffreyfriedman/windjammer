# RFC: Windjammer Shader Language (WJSL)

**Status:** Draft  
**Author:** Windjammer Team  
**Date:** 2026-03-15  
**Target:** Extends existing WGSL backend (`windjammer/src/codegen/wgsl/`)

---

## 1. Motivation

### 1.1 Why WJSL?

Current WGSL authoring in the Windjammer ecosystem is **painful**. Developers spend hours debugging "colored stripes instead of a scene" or black screens—often caused by subtle type mismatches, struct layout errors, or binding conflicts that WGSL's minimal tooling doesn't catch until runtime.

**Goal:** 80% of shader power with 20% of complexity.

### 1.2 Current Pain Points vs. Proposed Solutions

| Pain Point | Current WGSL Experience | WJSL Solution |
|------------|-------------------------|---------------|
| **Type mismatches** | Host sends `vec2<f32>` (1280.0, 720.0), shader expects `vec2<u32>`. Bit reinterpretation → pixel_idx = 1.15 billion → black screen. | **Type-safe bindings**: Compiler validates CPU/GPU type correspondence. Error: "Host sends vec2&lt;f32&gt;, shader expects vec2&lt;u32&gt;. Use vec2&lt;f32&gt; or add @host_type(u32) for bitcast." |
| **Manual struct padding** | Every struct needs `_pad1: f32`, `_pad2: vec2<f32>` to satisfy WGSL's vec3=16-byte alignment. Easy to get wrong. | **Automatic layout**: `@align`/`@size` inferred. Compiler inserts padding. Error if manual padding conflicts. |
| **Binding conflicts** | Duplicate `@binding(0)` in same group → silent wrong buffer at runtime. | **Binding validation**: "Duplicate @binding(0) in @group(0), already used by 'camera'. Use @binding(1) for 'params'." |
| **Scalar-vector ops** | `vec3 + f32` compiles in WGSL but produces garbage (broadcast rules unclear). | **Explicit broadcast**: "vec3 + f32 not allowed. Did you mean vec3 + vec3(f32, f32, f32)?" |
| **Debugging** | WGSL error points to generated line 47; source is .wjsl line 12. | **Source maps**: WJSL → WGSL with line mapping for error reporting. |
| **Boilerplate** | Repetitive `@group(0) @binding(N) var<uniform> name: Type;` for every resource. | **Convenience syntax**: `uniform camera: CameraUniforms @group(0) @binding(0)` |

### 1.3 Design Principles

1. **Build on WGSL** — WJSL compiles to WGSL. We do not replace the WGSL backend; we extend it with a friendlier source language.
2. **Type safety first** — Catch CPU/GPU type mismatches, layout errors, and binding conflicts at compile time.
3. **Ergonomic defaults** — Automatic struct padding, sensible type inference, clear error messages.
4. **Zero runtime cost** — WJSL is compile-time only. Generated WGSL is what runs on the GPU.

---

## 2. Syntax Design

### 2.1 Entry Points

#### Vertex Shaders

```wjsl
@vertex
fn main(
    @location(0) position: vec3,
    @location(1) normal: vec3,
    @location(2) uv: vec2
) -> VertexOutput {
    let world_pos = model_matrix * vec4(position, 1.0);
    return VertexOutput(
        proj_view * world_pos,
        world_pos.xyz,
        normal,
        uv
    );
}
```

#### Fragment Shaders

```wjsl
@fragment
fn main(
    @location(0) world_pos: vec3,
    @location(1) normal: vec3,
    @location(2) uv: vec2
) -> @location(0) vec4 {
    let n = normalize(normal);
    let lit = pbr_lighting(albedo, metallic, roughness, n, v, l, radiance);
    return vec4(lit, 1.0);
}
```

#### Compute Shaders

```wjsl
@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= screen_size.x || id.y >= screen_size.y) { return; }
    let pixel_idx = id.y * screen_size.x + id.x;
    output[pixel_idx] = compute_color(pixel_idx);
}
```

### 2.2 Uniform and Storage Bindings

```wjsl
// Uniform buffer
@group(0) @binding(0) uniform camera: CameraUniforms;

// Storage buffer (read-only)
@group(0) @binding(1) storage read svo_nodes: array<u32>;

// Storage buffer (read-write)
@group(0) @binding(2) storage read_write gbuffer: array<GBufferPixel>;

// Texture and sampler
@group(0) @binding(3) texture_2d albedo_map: texture_2d<f32>;
@group(0) @binding(4) sampler tex_sampler: sampler;
```

**Equivalent WGSL output:**

```wgsl
@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<storage, read> svo_nodes: array<u32>;
@group(0) @binding(2) var<storage, read_write> gbuffer: array<GBufferPixel>;
@group(0) @binding(3) var albedo_map: texture_2d<f32>;
@group(0) @binding(4) var tex_sampler: sampler;
```

### 2.3 Struct Declarations

```wjsl
// Automatic padding (compiler inserts _padN for alignment)
struct CameraUniforms {
    view_matrix: mat4x4,
    proj_matrix: mat4x4,
    position: vec3,      // vec3 needs 16-byte alignment → auto _pad
    screen_size: vec2,
    near_plane: f32,
    far_plane: f32,
}

// Explicit layout when needed (e.g., matching CPU struct)
struct RayMarchParams {
    world_size: vec3,
    @align(16) max_steps: u32,
    svo_depth: u32,
    node_count: u32,
}
```

---

## 3. Type System

### 3.1 Scalars

| WJSL | WGSL | Notes |
|------|------|-------|
| `f32` | `f32` | Default float |
| `u32` | `u32` | Unsigned (indices, flags) |
| `i32` | `i32` | Signed |
| `bool` | `bool` | 4 bytes in WGSL |

### 3.2 Vectors

| WJSL | WGSL |
|------|------|
| `vec2` | `vec2<f32>` (default) |
| `vec3` | `vec3<f32>` |
| `vec4` | `vec4<f32>` |
| `vec2<f32>`, `vec2<u32>`, `vec2<i32>` | Explicit element type |
| `vec3<u32>` | e.g., `global_invocation_id` |
| `vec4<f32>` | RGBA, positions |

**Swizzling:** `color.rgb`, `pos.xy`, `normal.xyz` (same as WGSL)

### 3.3 Matrices

| WJSL | WGSL |
|------|------|
| `mat2x2` | `mat2x2<f32>` |
| `mat3x3` | `mat3x3<f32>` |
| `mat4x4` | `mat4x4<f32>` |

### 3.4 Textures and Samplers

| WJSL | WGSL |
|------|------|
| `texture_2d<f32>` | `texture_2d<f32>` |
| `texture_2d<u32>` | `texture_2d<u32>` |
| `texture_cube<f32>` | `texture_cube<f32>` |
| `texture_3d<f32>` | `texture_3d<f32>` |
| `sampler` | `sampler` |
| `sampler_comparison` | `sampler_comparison` |

### 3.5 Struct Layout Annotations

```wjsl
struct Example {
    a: vec3,           // 12 bytes, 16-byte aligned
    @align(8) b: vec2,  // Explicit alignment
    @size(16) c: u32,   // Explicit size (e.g., for padding)
}
```

- **`@align(N)`** — Override alignment for this field.
- **`@size(N)`** — Override size (for explicit padding blocks).
- **Default:** Compiler uses WGSL rules (vec2=8, vec3=16, vec4=16, etc.).

### 3.6 Type Checking Rules (Compile-Time Validation)

WJSL performs compile-time type checking before codegen. The following rules are enforced:

#### Binary Operations

| Left | Op | Right | Result | Notes |
|------|-----|-------|--------|-------|
| `vec3` | `+` `-` | `vec3` | `vec3` | ✅ Same-size vectors |
| `vec3` | `+` `-` | `f32` | **ERROR** | ❌ Use `vec3 + vec3(1.0, 1.0, 1.0)` |
| `vec2` | `+` `-` | `vec2` | `vec2` | ✅ |
| `vec4` | `+` `-` | `vec4` | `vec4` | ✅ |
| `vec3` | `+` `-` | `vec2` | **ERROR** | ❌ Vector sizes must match |
| `f32` | `*` | `vec3` | `vec3` | ✅ Scalar multiplication |
| `vec3` | `*` | `f32` | `vec3` | ✅ Scalar multiplication |
| `mat4x4` | `*` | `vec4` | `vec4` | ✅ Matrix-vector multiply |
| `mat3x3` | `*` | `vec3` | `vec3` | ✅ |

**Error message for vec + scalar:**
```
Cannot add f32 to vec3. Did you mean vec3 + vec3(f32, f32, f32)?
```

#### Binding Validation

- **Unique bindings per group:** `@binding(N)` must be unique within each `@group(G)`.
- **Error:** `Duplicate @binding(0) in @group(0): 'b' conflicts with 'a'`
- Different groups may reuse binding numbers: `@group(0) @binding(0)` and `@group(1) @binding(0)` are valid.

#### Function Signatures

- Return type must match the type of the `return` expression.
- Struct field access (`material.base_color`) is type-checked against struct declarations.
- Swizzles (`.rgb`, `.xy`, `.xyz`) are validated.

#### Implementation

- **Module:** `windjammer/src/wjsl/type_checker.rs`
- **API:** `type_check_wjsl(source: &str) -> Result<()>`
- **Integration:** Type checking runs automatically in `transpile_wjsl()` before codegen.

---

## 4. Three Example Shaders

### 4.1 Simple PBR Fragment Shader

```wjsl
// pbr_fragment.wjsl
// PBR lighting with one directional light + ambient

struct Material {
    base_color: vec4,
    metallic: f32,
    roughness: f32,
    ao: f32,
    emissive: vec3,
}

struct Light {
    position: vec3,
    color: vec3,
    intensity: f32,
}

struct CameraUniforms {
    view_matrix: mat4x4,
    proj_matrix: mat4x4,
    position: vec3,
    screen_size: vec2,
    near_plane: f32,
    far_plane: f32,
}

@group(0) @binding(0) uniform material: Material;
@group(0) @binding(1) uniform camera: CameraUniforms;
@group(0) @binding(2) uniform light: Light;

fn fresnel_schlick(cos_theta: f32, f0: vec3) -> vec3 {
    return f0 + (vec3(1.0) - f0) * pow(1.0 - cos_theta, 5.0);
}

fn distribution_ggx(n: vec3, h: vec3, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let n_dot_h = max(dot(n, h), 0.0);
    let n_dot_h2 = n_dot_h * n_dot_h;
    let denom = n_dot_h2 * (a2 - 1.0) + 1.0;
    return a2 / (3.14159265 * denom * denom);
}

fn geometry_smith(n: vec3, v: vec3, l: vec3, roughness: f32) -> f32 {
    let n_dot_v = max(dot(n, v), 0.0);
    let n_dot_l = max(dot(n, l), 0.0);
    let ggx_v = n_dot_v / (n_dot_v * (1.0 - roughness) + roughness);
    let ggx_l = n_dot_l / (n_dot_l * (1.0 - roughness) + roughness);
    return ggx_v * ggx_l;
}

fn pbr_lighting(albedo: vec3, metallic: f32, roughness: f32, ao: f32,
                n: vec3, v: vec3, l: vec3, radiance: vec3) -> vec3 {
    let h = normalize(v + l);
    var f0 = vec3(0.04);
    f0 = mix(f0, albedo, metallic);
    let ndf = distribution_ggx(n, h, roughness);
    let g = geometry_smith(n, v, l, roughness);
    let f = fresnel_schlick(max(dot(h, v), 0.0), f0);
    let n_dot_l = max(dot(n, l), 0.0);
    let n_dot_v = max(dot(n, v), 0.0);
    let specular = (ndf * g * f) / (4.0 * n_dot_v * n_dot_l + 0.0001);
    let kd = (vec3(1.0) - f) * (1.0 - metallic);
    let diffuse = kd * albedo / 3.14159265;
    return (diffuse + specular) * radiance * n_dot_l * ao;
}

@fragment
fn main(
    @location(0) world_pos: vec3,
    @location(1) normal: vec3,
    @location(2) uv: vec2
) -> @location(0) vec4 {
    let n = normalize(normal);
    let v = normalize(camera.position - world_pos);
    let l = normalize(light.position - world_pos);
    let dist = length(light.position - world_pos);
    let attenuation = 1.0 / (dist * dist);
    let radiance = light.color * light.intensity * attenuation;
    let albedo = material.base_color.rgb;
    let lit = pbr_lighting(albedo, material.metallic, material.roughness,
                          material.ao, n, v, l, radiance);
    let ambient = vec3(0.03) * albedo * material.ao;
    return vec4(lit + ambient + material.emissive, material.base_color.a);
}
```

### 4.2 Visibility Compute Shader (VGS / SVO)

```wjsl
// visibility_compute.wjsl
// Reads SVO, outputs G-buffer (position, normal, material_id, depth)
// For VGS (Visibility Buffer / Geometry Buffer System)

struct CameraUniforms {
    view_matrix: mat4x4,
    proj_matrix: mat4x4,
    inv_view: mat4x4,
    inv_proj: mat4x4,
    position: vec3,
    screen_size: vec2,
    near_plane: f32,
    far_plane: f32,
}

struct RayMarchParams {
    world_size: vec3,
    max_steps: u32,
    svo_depth: u32,
    node_count: u32,
}

struct GBufferPixel {
    position: vec3,
    normal: vec3,
    material_id: f32,
    depth: f32,
    geometry_source: f32,
}

@group(0) @binding(0) uniform camera: CameraUniforms;
@group(0) @binding(1) uniform params: RayMarchParams;
@group(0) @binding(2) storage read svo_nodes: array<u32>;
@group(0) @binding(3) storage read_write gbuffer: array<GBufferPixel>;

fn svo_get_material(node: u32) -> u32 { return node & 0xFFu; }
fn svo_is_leaf(node: u32) -> bool { return (node & 0x100u) != 0u; }
fn svo_child_ptr(node: u32) -> u32 { return node >> 9u; }

fn get_octant(p: vec3, center: vec3) -> u32 {
    var idx = 0u;
    if (p.x >= center.x) { idx |= 1u; }
    if (p.y >= center.y) { idx |= 2u; }
    if (p.z >= center.z) { idx |= 4u; }
    return idx;
}

fn ray_aabb(origin: vec3, dir: vec3, box_min: vec3, box_max: vec3) -> vec2 {
    let eps = 1e-7;
    let safe_dir = select(dir, vec3(eps), abs(dir) < vec3(eps));
    let inv_dir = 1.0 / safe_dir;
    let t1 = (box_min - origin) * inv_dir;
    let t2 = (box_max - origin) * inv_dir;
    let tmin = min(t1, t2);
    let tmax = max(t1, t2);
    let t_near = max(max(tmin.x, tmin.y), tmin.z);
    let t_far = min(min(tmax.x, tmax.y), tmax.z);
    return vec2(t_near, t_far);
}

fn lookup_svo(pos: vec3, world_size: f32) -> u32 {
    if (params.node_count == 0u) { return 0u; }
    var node_idx = 0u;
    var node_min = vec3(0.0);
    var node_size = world_size;
    for (var depth = 0u; depth < params.svo_depth; depth++) {
        if (node_idx >= params.node_count) { return 0u; }
        let node = svo_nodes[node_idx];
        if (svo_is_leaf(node)) { return svo_get_material(node); }
        let half = node_size * 0.5;
        let center = node_min + vec3(half);
        let octant = get_octant(pos, center);
        node_idx = svo_child_ptr(node) + octant;
        node_min = node_min + vec3(
            select(0.0, half, (octant & 1u) != 0u),
            select(0.0, half, (octant & 2u) != 0u),
            select(0.0, half, (octant & 4u) != 0u)
        );
        node_size = half;
    }
    return node_idx < params.node_count ? svo_get_material(svo_nodes[node_idx]) : 0u;
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let width = u32(camera.screen_size.x);
    let height = u32(camera.screen_size.y);
    if (id.x >= width || id.y >= height) { return; }
    let pixel_idx = id.y * width + id.x;

    // Ray from pixel through inverse projection
    let uv = vec2(f32(id.x) / f32(width), f32(id.y) / f32(height));
    let ndc = vec2(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    let view_ray = normalize(vec3(ndc.x, ndc.y, 1.0));
    let world_ray = (camera.inv_view * vec4(view_ray, 0.0)).xyz;
    let origin = camera.position;
    let world_min = vec3(0.0);
    let world_max = params.world_size;
    let t = ray_aabb(origin, world_ray, world_min, world_max);
    if (t.x > t.y || t.y < 0.0) {
        gbuffer[pixel_idx] = GBufferPixel(vec3(0.0), vec3(0.0), 0.0, 1e10, 0.0);
        return;
    }
    var t_cur = max(t.x, 0.0);
    let step = 0.5;
    for (var i = 0u; i < params.max_steps; i++) {
        let pos = origin + world_ray * t_cur;
        let mat = lookup_svo(pos, params.world_size.x);
        if (mat > 0u) {
            let hit_pos = pos;
            let normal = vec3(0.0, 1.0, 0.0); // Simplified; full impl would compute from voxel faces
            gbuffer[pixel_idx] = GBufferPixel(hit_pos, normal, f32(mat), t_cur, 0.0);
            return;
        }
        t_cur += step;
    }
    gbuffer[pixel_idx] = GBufferPixel(vec3(0.0), vec3(0.0), 0.0, 1e10, 0.0);
}
```

### 4.3 Post-Process Shader (Tonemapping, Bloom, Vignette)

```wjsl
// post_process.wjsl
// Full-screen pass: tonemap (ACES), optional bloom, vignette

struct PostProcessParams {
    exposure: f32,
    bloom_strength: f32,
    vignette_intensity: f32,
    vignette_radius: f32,
}

struct FullscreenVertex {
    @builtin(vertex_index) vertex_id: u32,
}

struct VertexOutput {
    @builtin(position) position: vec4,
    @location(0) uv: vec2,
}

@group(0) @binding(0) uniform params: PostProcessParams;
@group(0) @binding(1) storage read color_input: array<vec4>;
@group(0) @binding(2) storage read bloom_input: array<vec4>;
@group(0) @binding(3) uniform screen_size: vec2<u32>;

fn tonemap_aces(color: vec3) -> vec3 {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((color * (a * color + b)) / (color * (c * color + d) + e), vec3(0.0), vec3(1.0));
}

fn apply_vignette(color: vec3, uv: vec2, intensity: f32, radius: f32) -> vec3 {
    let center = uv - vec2(0.5);
    let dist = length(center);
    let vignette = 1.0 - smoothstep(radius, 1.0, dist) * intensity;
    return color * vignette;
}

@vertex
fn vs_main(in: FullscreenVertex) -> VertexOutput {
    let x = f32((in.vertex_id << 1u) & 2u) - 1.0;
    let y = f32(in.vertex_id & 2u) - 1.0;
    let uv = vec2((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return VertexOutput(vec4(x, y, 0.0, 1.0), uv);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4 {
    let width = screen_size.x;
    let height = screen_size.y;
    let uv = in.uv;
    let px = u32(uv.x * f32(width));
    let py = u32(uv.y * f32(height));
    let idx = py * width + px;
    var color = color_input[idx].rgb * params.exposure;
    color = color + bloom_input[idx].rgb * params.bloom_strength;
    color = tonemap_aces(color);
    color = apply_vignette(color, uv, params.vignette_intensity, params.vignette_radius);
    return vec4(color, 1.0);
}
```

---

## 5. Compilation Pipeline

### 5.1 Overview

```
.wjsl source
    │
    ▼
┌─────────────────────┐
│ WJSL Parser         │  (new: wjsl_parser)
│ - Lexer             │
│ - Grammar           │
│ - AST               │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ WJSL Analyzer       │  (new: wjsl_analyzer)
│ - Type checking     │
│ - Binding validation│
│ - Layout validation │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ WGSL Codegen        │  (existing: codegen/wgsl/)
│ - Reuse WgslBackend │
│ - Or WJSL→WJ AST    │
│   then WJ→WGSL      │
└──────────┬──────────┘
           │
           ▼
.wgsl output
    │
    ▼
wgpu / GPU
```

### 5.2 Integration with Existing Backend

**Option A: WJSL → Windjammer AST → WGSL**

- Parse `.wjsl` into a shader-specific AST.
- Convert WJSL AST to Windjammer `Program` (structs, extern let, functions).
- Feed to existing `WgslBackend::generate()`.
- **Pro:** Reuses all existing WGSL codegen (structs, types, validation).
- **Con:** WJSL AST → Windjammer AST mapping may have impedance mismatch.

**Option B: WJSL → WGSL Direct**

- Parse `.wjsl` into WJSL AST.
- Implement WJSL-specific codegen that emits WGSL strings directly.
- **Pro:** Full control over WJSL-specific features (better errors, source maps).
- **Con:** Duplicates some logic from `WgslBackend`.

**Recommendation:** Start with **Option A** for MVP. Add WJSL parser that produces Windjammer-compatible AST. Use `wj build --target wgsl` for `.wjsl` files (new extension handling). If impedance mismatch is too high, introduce Option B incrementally.

### 5.3 CLI Integration

```bash
# Compile single WJSL file
wj build shader.wjsl --target wgsl --output shaders/

# Compile directory
wj build src_wj/shaders/ --target wgsl --output shaders/

# With source maps (for debugging)
wj build shader.wjsl --target wgsl --output shaders/ --source-map
```

### 5.4 Source Maps

- Emit `.wgsl.map` (or inline `//# sourceMappingURL=`) mapping generated WGSL lines to WJSL source.
- Enables: "Error at shader.wgsl:47" → "Error at shader.wjsl:12:34".
- Format: JSON source map (same as JavaScript).

---

## 6. Error Messages

### 6.1 Type Errors

**Scalar-vector mismatch:**

```
Error: vec3 + f32 not allowed
  --> shader.wjsl:24:18
   |
24 |     let color = base_color + intensity;
   |                  ^^^^^^^^^^^^^^^^^^^^^
   |
   = help: Did you mean vec3 + vec3(intensity, intensity, intensity)?
   = help: Or use vec3(intensity) to broadcast?
```

**Type mismatch at binding:**

```
Error: Binding 'screen_size' type mismatch
  --> shader.wjsl:18:1
   |
18 | @group(0) @binding(5) uniform screen_size: vec2<u32>;
   |                                          ^^^^^^^^^
   |
   = note: Host (Rust) sends vec2<f32>. Shader expects vec2<u32>.
   = help: Change to vec2<f32> and use u32(screen_size.x) for indexing.
   = help: Or add @host_type(f32) to accept bitcast from host.
```

### 6.2 Binding Errors

**Duplicate binding:**

```
Error: Duplicate @binding(0) in @group(0)
  --> shader.wjsl:12:1
   |
12 | @group(0) @binding(0) uniform camera: CameraUniforms;
   |            ^^^^^^^^^
   |
   = note: @binding(0) already used by 'lighting_params' at line 8.
   = help: Use @binding(1) for 'camera'.
```

### 6.3 Layout Errors

**Struct padding incorrect:**

```
Error: Struct 'GBufferPixel' layout incorrect
  --> shader.wjsl:6:1
   |
 6 | struct GBufferPixel {
 7 |     position: vec3,
 8 |     normal: vec3,
 9 |     material_id: f32,
   |     ...
   |
   = note: Struct is 17 bytes but must be 16-byte aligned.
   = help: Add padding after 'material_id' or use @align(16) on next field.
   = help: Compiler can auto-insert padding if you remove manual _pad fields.
```

### 6.4 Bounds Check Hints

```
Warning: storage buffer 'gbuffer' indexed without bounds check
  --> shader.wjsl:45:22
   |
45 |     let hit = gbuffer[pixel_idx];
   |                  ^^^^^
   |
   = help: Add: if (pixel_idx >= arrayLength(&gbuffer)) { return; }
```

---

## 7. Implementation Phases

### Phase 1: Parser + Basic Codegen (MVP)

- [ ] WJSL lexer and parser
- [ ] AST for structs, uniforms, functions
- [ ] Map to Windjammer AST or direct WGSL emit
- [ ] `wj build *.wjsl --target wgsl`
- [ ] One example shader (e.g., minimal compute) compiling end-to-end

### Phase 2: Type System + Validation

- [ ] Type checker (scalar-vector, binding types)
- [ ] Struct layout validation
- [ ] Binding uniqueness validation
- [ ] Error message formatting

### Phase 3: Ergonomics

- [ ] Source maps
- [ ] Convenience syntax (uniform/storage shorthand)
- [ ] Default f32 for vec2/vec3/vec4
- [ ] Bounds check warnings

### Phase 4: Integration

- [ ] ShaderGraph / wj-game plugin awareness of .wjsl
- [ ] Hot reload for .wjsl files
- [ ] Documentation and migration guide from raw WGSL

---

## 8. References

- **Existing WGSL backend:** `windjammer/src/codegen/wgsl/`
- **WGSL spec:** https://www.w3.org/TR/WGSL/
- **WGSL Transpiler Status:** `windjammer/WGSL_TRANSPILER_STATUS.md`
- **Shader metadata:** `windjammer/src/codegen/wgsl/shader_metadata.rs`
- **Pain point docs:** `docs/SCREEN_SIZE_TYPE_MISMATCH_BUG.md`, `docs/TDD_BLACK_SCREEN_FIX_FINAL.md`

---

## 9. Open Questions

1. **WJSL vs. Windjammer for shaders:** Should WJSL be a separate language or a subset of Windjammer with shader-specific decorators? Current RFC assumes separate grammar for maximum control.
2. **Module system:** Should WJSL support `import` for shared structs/functions across shaders?
3. **Conditional compilation:** `#ifdef`-style for different quality levels or platforms?
4. **Standard library:** Built-in `pbr_lighting()`, `tonemap_aces()`, etc., or user-defined only?

---

*This RFC builds on the existing WGSL backend. It does not replace it—WJSL compiles to WGSL, and raw WGSL remains fully supported.*

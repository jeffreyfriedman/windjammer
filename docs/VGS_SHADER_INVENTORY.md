# VGS Shader Inventory

**Purpose**: Roadmap for .wjsl conversion and shader DSL work.  
**Last updated**: 2026-03-15

---

## Current VGS Shaders

### 1. vgs_visibility.wgsl ✅ COMPLETE

**Location**: 
- `windjammer-game/windjammer-game-core/shaders/vgs_visibility.wgsl`
- `breach-protocol/runtime_host/shaders/vgs_visibility.wgsl`

**Purpose**: GPU-driven frustum culling and LOD selection.

**Inputs**:
- Cluster buffer (read-only)
- Camera uniforms (view-proj, position, FOV)
- VisibilityParams (cluster_count, lod_bias)

**Outputs**:
- Cluster instances (read-write): visibility flags + selected LOD

**Key logic**:
- Extract 6 frustum planes from view-proj matrix
- AABB vs frustum test
- Screen-space error calculation
- LOD selection (0–3 based on pixel error)

**Lines**: ~200

---

### 2. vgs_expansion.wgsl ✅ COMPLETE

**Location**:
- `windjammer-game/windjammer-game-core/shaders/vgs_expansion.wgsl`
- `breach-protocol/runtime_host/shaders/vgs_expansion.wgsl`

**Purpose**: GPU-driven cluster-to-triangle expansion.

**Inputs**:
- Cluster buffer (read-only)
- Cluster instances (read-only, from visibility pass)
- Source triangles (read-only)
- ExpansionParams (cluster_count, max_triangles)

**Outputs**:
- Output vertices (read-write)
- Output indices (read-write)
- Draw count (atomic)

**Key logic**:
- Atomic reserve space per visible cluster
- Copy triangle vertices/indices
- Flat normal calculation
- `expand_clusters_indirect` variant (stub)

**Lines**: ~165

---

### 3. vgs_rasterization.wgsl ❌ MISSING (STUB)

**Referenced by**: `vgs_rasterization.wj`, `vgs_rasterization.rs`  
**Expected path**: `shaders/vgs_rasterization.wgsl`  
**Status**: **File does not exist** – VGSRasterizer loads a non-existent shader.

**Intended purpose**: Draw expanded triangles to G-buffer via hardware rasterization.

**Current workaround**: `vgs_rasterization.wj` uses CPU-side `draw_clusters()` with placeholder logic.

---

## Related Shaders (Non-VGS)

### SVO / Voxel Pipeline

| Shader | Purpose | VGS-related? |
|--------|---------|--------------|
| `voxel_raymarch.wgsl` | SVO ray march, G-buffer | No (voxel path) |
| `voxel_lighting.wgsl` | Lighting with SVO lookup | No |
| `voxel_denoise.wgsl` | Denoise voxel output | No |
| `voxel_composite.wgsl` | Composite passes | No |

### SVO Test/Debug Shaders

| Shader | Purpose |
|--------|---------|
| `test_svo_simple.wgsl` | Minimal SVO test |
| `test_svo_buffer_raw.wgsl` | Raw buffer read |
| `test_svo_lookup.wgsl` | lookup_svo validation |
| `debug_svo_dump.wgsl` | SVO structure dump |

### Other Visibility

| Shader | Purpose |
|--------|---------|
| `particle_cull.wgsl` | Particle visibility (VisibilityData struct) |

---

## Dependencies Between Shaders

```
VGS Pipeline Flow:
┌─────────────────────────────────────────────────────────────────┐
│  vgs_visibility.wgsl                                            │
│  (Cluster buffer + Camera → Visibility flags + LOD)              │
└─────────────────────────────┬───────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  vgs_expansion.wgsl                                              │
│  (Visible clusters + Triangles → Vertex/Index buffers)            │
└─────────────────────────────┬───────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  vgs_rasterization.wgsl  [MISSING]                               │
│  (Vertex/Index buffers → G-buffer)                               │
└─────────────────────────────────────────────────────────────────┘
```

**Shared structs** (must stay in sync):
- `Cluster` (visibility + expansion)
- `ClusterInstance`
- `Triangle`, `Vertex`
- `CameraUniforms`, `VisibilityParams`, `ExpansionParams`

---

## Priority Order for .wjsl Conversion

### 1. vgs_visibility.wgsl (HIGH)

**Priority**: High  
**Pain**: Frustum math, buffer layout, LOD logic  
**Complexity**: ~200 lines, 6 frustum planes, AABB test  
**Benefit**: Type-safe Cluster/Camera structs, compile-time binding checks

### 2. vgs_expansion.wgsl (HIGH)

**Priority**: High  
**Pain**: Atomic append, buffer bounds, Triangle/Vertex layout  
**Complexity**: ~165 lines, atomicAdd, multi-binding  
**Benefit**: Catch buffer overflow at compile time, validate struct layout

### 3. voxel_raymarch.wgsl (MEDIUM)

**Priority**: Medium  
**Pain**: SVO traversal, ray-AABB, buffer bounds  
**Complexity**: ~230 lines, lookup_svo, march_svo  
**Benefit**: Used for Breach Protocol; type-safe SVO params

### 4. vgs_rasterization.wgsl (CREATE FIRST)

**Priority**: Critical  
**Status**: **File does not exist** – create as first .wjsl shader  
**Purpose**: Vertex/fragment shader for drawing expanded triangles  
**Benefit**: Unblocks full VGS pipeline, validates .wjsl for vertex/fragment

### 5. voxel_lighting.wgsl (MEDIUM)

**Priority**: Medium  
**Pain**: Duplicated SVO lookup logic from raymarch  
**Benefit**: Shared SVO module, single source of truth

---

## Recommended Conversion Order

1. **Create** `vgs_rasterization.wjsl` (new) – validates vertex/fragment pipeline  
2. **Convert** `vgs_visibility.wgsl` → `vgs_visibility.wjsl`  
3. **Convert** `vgs_expansion.wgsl` → `vgs_expansion.wjsl`  
4. **Convert** `voxel_raymarch.wgsl` → `voxel_raymarch.wjsl`  
5. **Convert** `voxel_lighting.wgsl` → `voxel_lighting.wjsl` (share SVO module)

---

## Summary

| Shader | Status | Lines | Priority |
|--------|--------|-------|----------|
| vgs_visibility.wgsl | ✅ Complete | ~200 | High |
| vgs_expansion.wgsl | ✅ Complete | ~165 | High |
| vgs_rasterization.wgsl | ❌ Missing | N/A | Critical (create) |
| voxel_raymarch.wgsl | ✅ Complete | ~230 | Medium |
| voxel_lighting.wgsl | ✅ Complete | ~200 | Medium |

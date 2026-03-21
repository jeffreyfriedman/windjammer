# NVIDIA Path Tracing Fork of Godot: Analysis for Windjammer Engine

**Date:** March 13, 2026  
**Sources:** 80.lv articles, NVIDIA GDC 2026, NVIDIA Developer Blog, GitHub (NVIDIA-RTX/godot)  
**Mission:** Extract actionable learnings for Windjammer's rendering pipeline

---

## 1. Executive Summary

NVIDIA released a path tracing fork of Godot at GDC 2026 (March 2026), demonstrating their commitment to bringing real-time path tracing to open-source game engines. While the Godot fork itself has minimal public documentation, NVIDIA's broader RTX ecosystem—ReSTIR PT, RTX Mega Geometry, DLSS denoising—provides a clear roadmap for advanced rendering.

**Key Takeaway:** Windjammer's voxel raymarch + SVO pipeline is architecturally sound. The highest-impact improvements are: (1) better denoising, (2) hybrid BVH for mesh rendering, and (3) optional path tracing mode for high-quality output.

---

## 2. NVIDIA's Approach: Technical Details

### 2.1 Path Tracing Techniques

| Technique | Description | NVIDIA Use Case |
|-----------|-------------|----------------|
| **ReSTIR** | Reservoir-based Spatiotemporal Importance Resampling | Direct lighting from millions of lights; 6×–60× speedup |
| **ReSTIR GI** | Path resampling for indirect illumination | Real-time GI at 1 spp with 9×–166× MSE improvement |
| **ReSTIR PT** | Path resampling at any bounce | Mirror reflections, glossy surfaces (RTX Dynamic Illumination SDK) |
| **Hardware RT** | RT cores for BVH traversal | Primary intersection acceleration |
| **DLSS** | AI-based denoising + upscaling | Frame generation, noise reduction |

### 2.2 Integration with Existing Renderer

- **Hybrid rendering:** Path tracing runs alongside rasterization
- **Progressive refinement:** Low spp initially, accumulate over frames
- **Denoising required:** Path tracing at 1–8 spp produces noisy images; denoiser is mandatory
- **Godot fork:** Adds path tracing as optional render mode; existing Godot materials/scenes work

### 2.3 Performance Optimizations

1. **RTX Mega Geometry**
   - Compresses geometry into clusters, reuses per frame
   - 100× faster acceleration structure build
   - Alan Wake 2: 5–20% FPS boost, 300 MB VRAM reduction
   - **Foliage system:** Partitioned top-level acceleration structures; millions of animated foliage elements per frame

2. **ReSTIR**
   - 8 rays per pixel for millions of dynamic lights
   - Spatiotemporal resampling across pixels and frames
   - Reservoir sampling for importance-weighted light selection

3. **DLSS 4.5**
   - Dynamic Multi Frame Generation (RTX 50 Series)
   - AI denoising reduces required spp
   - Second-generation transformer model for super resolution

### 2.4 Denoising Strategy

- **Primary:** DLSS (proprietary, NVIDIA GPUs)
- **Alternative:** OIDN (Intel Open Image Denoise) — open-source, CPU/GPU, Apache 2.0
- **Auxiliary buffers:** Albedo, normal, depth improve denoising quality
- **Temporal accumulation:** Reuse history across frames (Windjammer already does this)

### 2.5 Real-Time Path Tracing Approach

1. **Low spp (1–4) per frame** with temporal accumulation
2. **ReSTIR** for efficient light sampling
3. **Hardware acceleration** (RT cores) for BVH traversal
4. **Denoising** to produce clean output from noisy input
5. **Progressive refinement** for static scenes

---

## 3. Windjammer vs. NVIDIA: Pipeline Comparison

### 3.1 Windjammer Current Pipeline

```
Raymarch (SVO) → Lighting (GI) → Denoise → Composite → Screen
     │                │              │
     │                │              └─ 5×5 a-trous wavelet + temporal
     │                └─ Cosine hemisphere sampling, shadow rays
     └─ Sparse Voxel Octree traversal, DDA-style stepping
```

**Strengths:**
- SVO is memory-efficient for voxel worlds
- Temporal denoising already implemented
- Edge-aware filtering (color, normal, depth)
- One-bounce diffuse GI in lighting pass
- Compute shader-based (wgpu) — portable

**Gaps:**
- No hardware ray tracing (software raymarch only)
- No BVH for mesh geometry
- Simple bilateral/a-trous denoise (no ML)
- No path tracing mode
- No ReSTIR-style importance resampling

### 3.2 NVIDIA Pipeline (Inferred)

```
Raster/RT → Path Trace (ReSTIR) → DLSS Denoise → Composite → Screen
     │              │                    │
     │              └─ BVH + RT cores      └─ AI denoising
     └─ Hybrid: raster for primary, RT for secondary
```

---

## 4. Key Technologies Deep Dive

### 4.1 ReSTIR (Reservoir-based Spatiotemporal Importance Resampling)

**Paper:** NVIDIA Research, 2020–2021  
**Purpose:** Render millions of dynamic lights with ~8 rays per pixel

**How it works:**
1. Generate candidate light samples (M per pixel)
2. Reservoir sampling selects one sample per pixel
3. Spatial resampling: share samples with neighboring pixels
4. Temporal resampling: reuse samples from previous frame
5. Unbiased estimate with MIS (Multiple Importance Sampling)

**Windjammer relevance:** Medium. Our lighting uses cosine hemisphere + shadow rays. ReSTIR would help with many dynamic lights. Lower priority than denoising/BVH.

### 4.2 RTX Mega Geometry

**Concepts:**
- Geometry clustering and compression
- Per-frame instance updates
- Partitioned top-level acceleration structures
- 100× faster BVH build

**Windjammer relevance:** Low. Our VGS (Voxel Geometry Structure) / SVO already handles dense voxel data efficiently. Mega Geometry targets mesh-heavy foliage; we could study LOD/instancing concepts for future mesh integration.

### 4.3 OIDN (Open Image Denoise)

**Source:** https://www.openimagedenoise.org/  
**License:** Apache 2.0  
**Platforms:** Intel CPU/GPU, NVIDIA, AMD

**Features:**
- Deep learning-based denoising
- Works with 1 spp to converged images
- Optional albedo/normal buffers
- C/C++ API, GPU-accelerated

**Windjammer relevance:** High. We have G-buffer (position, normal, material). OIDN could significantly improve quality over our a-trous wavelet. Integration: FFI from Windjammer runtime.

### 4.4 wgpu Ray Tracing

**Status (2025):**
- Vulkan: VK_KHR_ray_query supported
- DXR: PR merged for DirectX 12
- Metal: Acceleration structures working

**Windjammer relevance:** High for mesh rendering. Our voxel pipeline uses compute raymarch; adding BVH + hardware RT would accelerate mesh intersections. wgpu ray tracing is still evolving—track for future.

---

## 5. Integration Strategy for Windjammer

### 5.1 Architecture Principles

1. **Preserve voxel pipeline:** SVO raymarch stays primary for voxel worlds
2. **Additive, not replacement:** New features as optional modes
3. **TDD for each component:** Shader TDD framework, integration tests
4. **Backend-agnostic where possible:** OIDN works on CPU; ML denoising may need GPU-specific paths

### 5.2 Integration Opportunities

| Opportunity | Windjammer Fit | Effort | Impact |
|-------------|----------------|--------|--------|
| **Better denoising** | Already have temporal + a-trous | Medium | High |
| **OIDN integration** | G-buffer compatible | Medium | High |
| **BVH for meshes** | Hybrid renderer (voxels + meshes) | High | High |
| **Hardware RT** | wgpu support emerging | High | Medium |
| **Path tracer mode** | Offline/screenshots | Medium | Medium |
| **ReSTIR** | Many lights use case | High | Medium |
| **Mega Geometry concepts** | VGS LOD/instancing | Low | Low |

---

## 6. Implementation Tasks (TDD Approach)

### P0 — Immediate (Denoising)

| Task | TDD Approach | Acceptance Criteria |
|------|--------------|---------------------|
| Improve bilateral filter kernel | `test_denoise_reduces_noise` — compare variance before/after | Variance reduction > 50% |
| Temporal accumulation for static scenes | `test_temporal_accumulates_over_frames` | History weight increases over N frames |
| Research OIDN integration | Document FFI requirements, buffer formats | Spec document |

### P1 — Short-term (Hybrid Rendering)

| Task | TDD Approach | Acceptance Criteria |
|------|--------------|---------------------|
| BVH acceleration structure | `test_bvh_intersection_correct` | Ray-triangle hits match reference |
| Hardware ray tracing (wgpu) | `test_rt_pipeline_initializes` | Pipeline creation succeeds |
| Benchmark BVH vs VGS | `bench_bvh_vs_svo_intersection` | Document perf tradeoffs |

### P2 — Long-term (Path Tracer)

| Task | TDD Approach | Acceptance Criteria |
|------|--------------|---------------------|
| Basic path tracer | `test_path_tracer_converges` | Cornel box matches reference |
| ReSTIR sampling | `test_restir_unbiased` | Variance lower than naive |
| Offline rendering pipeline | `test_offline_render_saves` | PNG output matches expected |

### P2 — RTX Mega Geometry Concepts

| Task | TDD Approach | Acceptance Criteria |
|------|--------------|---------------------|
| Study foliage rendering | Document partitioned TLAS approach | Summary doc |
| Apply to VGS hierarchy | `test_vgs_instancing` | Instance reuse works |

---

## 7. References

### Papers & Research
- [ReSTIR: Spatiotemporal Reservoir Resampling](https://research.nvidia.com/publication/2020-07_Spatiotemporal-reservoir-resampling) — NVIDIA, 2020
- [ReSTIR GI: Path Resampling for Real-Time Path Tracing](https://research.nvidia.com/publication/2021-06_ReSTIR-GI%3A-Path) — NVIDIA, 2021

### NVIDIA Resources
- [NVIDIA RTX Innovations GDC 2026](https://developer.nvidia.com/blog/nvidia-rtx-innovations-are-powering-the-next-era-of-game-development/)
- [RTX Mega Geometry Vulkan Samples](https://developer.nvidia.com/blog/nvidia-rtx-mega-geometry-now-available-with-new-vulkan-samples/)
- [NVIDIA RTX Kit](https://developer.nvidia.com/rtx-kit) — ReSTIR PT, Path Tracing, etc.

### Open Source
- [NVIDIA-RTX/godot](https://github.com/NVIDIA-RTX/godot) — Path tracing fork (minimal public docs)
- [Intel Open Image Denoise](https://www.openimagedenoise.org/) — OIDN library
- [wgpu Ray Tracing Spec](https://github.com/gfx-rs/wgpu/blob/trunk/docs/api-specs/ray_tracing.md)

### Articles
- [80.lv: NVIDIA Launches Path Tracing Fork of Godot](https://80.lv/articles/nvidia-launches-path-tracing-fork-of-godot-engine)
- [80.lv: NVIDIA RTX Updates GDC 2026](https://80.lv/articles/nvidia-reveals-updates-for-rtx-titles-platform-features)

---

## 8. Conclusion

NVIDIA's path tracing work validates that **real-time path tracing is achievable** with the right combination of:
1. Efficient sampling (ReSTIR)
2. Hardware acceleration (RT cores)
3. Aggressive denoising (DLSS/OIDN)
4. Temporal accumulation

Windjammer's voxel pipeline already employs (4). Our highest-leverage improvements are **(1) better denoising** (OIDN or improved filters) and **(2) hybrid BVH for meshes** when we add mesh rendering. Path tracing mode and ReSTIR are valuable but lower priority.

**Recommended next step:** Improve the voxel denoise shader (better kernel, variance-guided blending) and prototype OIDN integration for a quality comparison.

# OIDN Integration Plan for Windjammer Rendering Pipeline

**Date:** March 14, 2026  
**Context:** NVIDIA analysis identified OIDN as #1 priority for visual quality improvement.  
**Source:** [NVIDIA_PATH_TRACING_ANALYSIS.md](./NVIDIA_PATH_TRACING_ANALYSIS.md)

---

## 1. Executive Summary

**OIDN (Intel Open Image Denoise)** is an open-source, ML-based denoising library (Apache 2.0) used in Blender, Unreal, and Unity. It offers significantly better quality than our current 5×5 a-trous wavelet filter.

**Key Finding:** Integration is **feasible but has constraints**. The `oidn-wgpu` crate requires wgpu 27; Windjammer uses wgpu 0.19. We have two paths: (1) use the `oidn` crate with manual GPU→CPU→GPU buffer copy, or (2) upgrade wgpu first. For real-time 60 FPS, OIDN GPU mode is borderline (~5–15ms); CPU mode is too slow (50–200ms).

**Recommendation:** P0 = improve a-trous + add albedo/normal; P1 = prototype OIDN for quality comparison; consider SVGF as GPU-native alternative.

---

## 2. OIDN Requirements & API

### 2.1 Buffer Formats

| Buffer | Purpose | OIDN Format | Windjammer Source |
|--------|---------|-------------|-------------------|
| **Color** | HDR beauty (required) | `OIDN_FORMAT_FLOAT3` (RGB float32) | `color_buffer` (vec4<f32>) → extract RGB |
| **Albedo** | Optional, improves quality | `OIDN_FORMAT_FLOAT3` | Not yet output; derive from `material_id` + lookup |
| **Normal** | Optional, improves quality | `OIDN_FORMAT_FLOAT3` | `gbuffer.normal` (vec3<f32>) ✅ |

**Compatibility:** Straightforward. RGB extraction from `vec4<f32>` → `[f32; width*height*3]`. Normals already in view space; OIDN expects world/view-space normals.

### 2.2 API Characteristics

- **C99 API** + C++11 wrapper
- **Device types:** CPU, GPU (CUDA, SYCL, HIP, Metal)
- **Object model:** `OIDNDevice`, `OIDNBuffer`, `OIDNFilter` (reference-counted)
- **Commit semantics:** Parameters must be committed before use
- **Thread safety:** API calls thread-safe; operations on same device serialized
- **Quality modes:** `Fast`, `Balanced`, `High` (for real-time, use Fast or Balanced)

### 2.3 Key Filter Parameters

```c
// RT (ray tracing) filter
oidnSetFilterImage(filter, "color",  colorBuf,  OIDN_FORMAT_FLOAT3, w, h, ...);
oidnSetFilterImage(filter, "albedo", albedoBuf, OIDN_FORMAT_FLOAT3, w, h, ...);
oidnSetFilterImage(filter, "normal", normalBuf, OIDN_FORMAT_FLOAT3, w, h, ...);
oidnSetFilterImage(filter, "output", outputBuf, OIDN_FORMAT_FLOAT3, w, h, ...);
oidnSetFilterBool(filter, "hdr", true);  // HDR input
oidnSetFilterInt(filter, "quality", OIDN_QUALITY_FAST);  // or BALANCED, HIGH
```

### 2.4 System Requirements (OIDN 2.4.x)

| Platform | CPU | GPU |
|----------|-----|-----|
| **CPU** | Intel 64 (SSE4.1+), ARM64, AMD | — |
| **NVIDIA** | — | Turing+ (RTX 20 series+) |
| **AMD** | — | RDNA 2+ |
| **Intel** | — | Xe/Xe2/Xe3 |
| **Apple** | — | Silicon (Metal) |

---

## 3. Rust Integration Options

### 3.1 Available Crates

| Crate | Version | OIDN | wgpu | Notes |
|-------|---------|------|------|-------|
| **oidn** | 2.3.3 | 2.x | — | CPU-only, simple API, RayTracing filter |
| **oidn-wgpu** | 2.4.1 | 2.4.x | **27** | Texture denoise, GPU backends; **wgpu 27** |
| **oidn2-sys** | 0.0.1 | 2.x | — | Raw FFI bindings |
| **oidn-wgpu-interop** | 0.4.0 | — | — | Shared buffers; wgpu 28 |

**Windjammer wgpu version:** 0.19 (see `windjammer-game-core/Cargo.toml`)

**Version mismatch:** `oidn-wgpu` requires wgpu 27. Windjammer is on 0.19. Upgrading wgpu is a major undertaking (API changes, breaking changes).

### 3.2 Integration Paths

**Path A: `oidn` crate + manual buffer copy**
- Use `oidn::Device`, `oidn::RayTracing` filter
- Read `color_buffer` from GPU → `Vec<f32>` (RGB)
- Optionally: read albedo, normal from GPU
- Run OIDN on CPU
- Write result back to GPU buffer
- **Pros:** Works with current wgpu 0.19, no dependency upgrade
- **Cons:** GPU→CPU→GPU round trip; CPU denoising ~50–200ms

**Path B: Upgrade wgpu + use `oidn-wgpu`**
- Upgrade Windjammer to wgpu 27 (or latest)
- Use `oidn-wgpu::denoise_texture()` for GPU denoising
- **Pros:** GPU denoising, potentially faster
- **Cons:** Major upgrade effort; `oidn-wgpu` still does readback → OIDN → upload (GPU→CPU→GPU for most backends)

**Path C: Direct FFI**
- Use `oidn2-sys` or raw `oidn.h` bindings
- Create OIDN buffers from wgpu buffer pointers (requires shared memory / external memory)
- **Complexity:** OIDN supports `oidnNewSharedBufferFromFD`, `FromWin32Handle`, `FromMetal` for shared memory; wgpu 0.19 may not expose these easily

**Recommendation:** Path A for prototyping. Validate quality improvement; if significant, consider Path B when wgpu upgrade is planned.

---

## 4. Buffer Format Compatibility

### 4.1 Windjammer Current Pipeline

```
Raymarch → G-buffer (position, normal, material_id, depth)
         → Lighting → color_buffer (HDR vec4<f32>)
         → Denoise (a-trous) → denoise_output
         → Composite (tonemap) → ldr_output → Screen
```

**Storage buffers:**
- `color_buffer`: `array<vec4<f32>>` — RGB in [0, ∞), alpha usually 1.0
- `gbuffer`: `array<GBufferPixel>` — position, normal, material_id, depth
- `denoise_output`: `array<vec4<f32>>` — denoised HDR

### 4.2 Conversion for OIDN

| OIDN Input | Windjammer Source | Conversion |
|------------|-------------------|------------|
| Color (float3) | `color_buffer[i].rgb` | Direct copy; HDR values OK |
| Normal (float3) | `gbuffer[i].normal` | Direct copy |
| Albedo (float3) | Not yet | Add: `material_lookup(gbuffer[i].material_id).rgb` |

**Albedo derivation:** Add `albedo_buffer` output from lighting pass. Lighting already has `get_material_color(material_id)`; we can write albedo per-pixel in a separate pass or extend lighting.

### 4.3 OIDN Expected Output

- `OIDN_FORMAT_FLOAT3` RGB float32
- Same dimensions as input
- We pack back to `vec4<f32>(rgb, 1.0)` for composite pass

---

## 5. Pipeline Integration Points

### Option 1: Replace A-Trous (Denoise HDR Before Tonemap) ✅ Recommended

```
Raymarch → Lighting → OIDN (replace a-trous) → Composite → Screen
```

**Rationale:** Denoise HDR before tonemap preserves dynamic range; OIDN is trained on HDR. Our composite does ACES tonemap; denoising before tonemap is correct.

### Option 2: Denoise LDR After Composite

```
Raymarch → Lighting → A-Trous → Composite → OIDN → Screen
```

**Rationale:** Denoise final LDR. Simpler (no HDR handling), but OIDN is less effective on LDR; we lose HDR denoising benefits.

**Recommendation:** Option 1.

---

## 6. Performance Analysis

### 6.1 OIDN Performance (from research)

| Mode | Resolution | Time | 60 FPS Budget |
|------|------------|------|----------------|
| **CPU** | 1920×1080 | 50–200 ms | ❌ Exceeds 16.67 ms |
| **GPU** | 1920×1080 | 5–15 ms | ⚠️ Borderline |
| **GPU (Fast)** | 1920×1080 | ~5 ms | ✅ Fits |

**Note:** OIDN is designed for interactive preview, not real-time gaming. Quality modes: Fast (fastest), Balanced, High (best quality).

### 6.2 Buffer Copy Overhead

**oidn-wgpu** (and Path A) flow:
1. GPU → CPU: read texture/buffer (~2–5 ms for 1080p RGBA32F)
2. OIDN execute: 5–50 ms (CPU) or 5–15 ms (GPU)
3. CPU → GPU: upload result (~2–5 ms)

**Total GPU→CPU→GPU:** ~9–60 ms. For real-time, GPU OIDN + minimal copy is essential.

### 6.3 Real-Time Viability

**CPU path:** Not viable for 60 FPS (50–200 ms).

**GPU path:** Viable with `OIDN_QUALITY_FAST` or `BALANCED`. Requires:
- OIDN built with CUDA/HIP/Metal
- GPU device creation
- Shared memory or async copy to minimize latency

**Alternative:** Use OIDN for **screenshot/bake mode** or **quality toggle** (e.g., "High Quality" mode at 30 FPS).

---

## 7. TDD Tasks

### 7.1 OIDN Initialization

```rust
// test_oidn_initializes
#[test]
fn test_oidn_initializes() {
    let device = oidn::Device::new();
    // No panic; device created
}
```

### 7.2 Basic Denoise

```rust
// test_oidn_denoise_basic
#[test]
fn test_oidn_denoise_basic() {
    let device = oidn::Device::new();
    let mut filter = oidn::RayTracing::new(&device);
    let w = 64;
    let h = 64;
    let input: Vec<f32> = (0..w*h*3).map(|i| f32::from(i % 256) / 255.0).collect();
    let mut output = vec![0.0f32; w * h * 3];
    filter.srgb(false).filter(&input, &mut output).expect("denoise");
    // Output should differ from input (denoised)
    assert_ne!(input, output);
}
```

### 7.3 With Albedo

```rust
// test_oidn_with_albedo
// Pass albedo buffer; verify no panic; quality improvement (manual inspection)
```

### 7.4 With Normals

```rust
// test_oidn_with_normals
// Pass normal buffer; verify no panic; quality improvement (manual inspection)
```

### 7.5 Performance

```rust
// test_oidn_performance_acceptable
#[test]
fn test_oidn_performance_acceptable() {
    let mut total_ms = 0.0;
    for _ in 0..10 {
        let start = Instant::now();
        // ... denoise 1920x1080 ...
        total_ms += start.elapsed().as_secs_f64() * 1000.0;
    }
    let avg_ms = total_ms / 10.0;
    assert!(avg_ms < 16.67, "OIDN must fit 60 FPS budget: {} ms", avg_ms);
}
```

### 7.6 Buffer Format

```rust
// test_oidn_vec4_to_float3_conversion
// vec4<f32> RGB → [f32; n*3] round-trip
```

---

## 8. Alternatives

### 8.1 SVGF (Spatiotemporal Variance-Guided Filtering)

**Source:** NVIDIA Research, HPG 2017  
**Paper:** [Spatiotemporal Variance-Guided Filtering](https://research.nvidia.com/publication/2017-07_spatiotemporal-variance-guided-filtering-real-time-reconstruction-path-traced)

**Characteristics:**
- **Real-time:** ~10 ms @ 1080p on modern GPU
- **GPU-native:** Implementable in WGSL compute shaders
- **Pipeline:** Pre-blend → à trous wavelet (multi-scale) → albedo modulation → TAA
- **Quality:** 5–47% better SSIM than prior filters; 10× more temporally stable

**Windjammer fit:** Our current denoise is already temporal + a-trous. SVGF adds variance-guided weights and hierarchical filtering. **No external dependency.**

**Pros:** Real-time, GPU-native, no FFI, fits our stack  
**Cons:** More complex than OIDN; requires variance estimation

### 8.2 Improved A-Trous

**Current:** 5×5 kernel, single pass, edge-aware (color, normal, depth)

**Improvements:**
- **More passes:** 3–5 à trous iterations (increasing step size)
- **Variance-guided:** Use luminance variance to guide filter weights
- **Larger kernel:** 7×7 or 9×9 with proper weighting
- **Adaptive:** Reduce blur in high-detail regions

**Effort:** Low–medium; shader-only changes.

### 8.3 Bilateral++

- Enhanced bilateral filter with better edge handling
- Variance-guided sigma
- GPU-friendly

---

## 9. Recommendations

### P0 (Immediate)

1. **Improve current a-trous filter**
   - Add 3–5 à trous iterations (multi-scale)
   - Add variance-guided blending (optional)
   - TDD: `test_denoise_reduces_variance`, `test_denoise_preserves_edges`

2. **Add albedo buffer support**
   - Output albedo from lighting pass (material color per pixel)
   - Use in denoise for edge-aware weighting
   - Enables future OIDN/SVGF with albedo

3. **Add normal buffer to denoise**
   - Already in use; ensure format is correct for future OIDN

### P1 (Short-Term)

1. **Prototype OIDN integration**
   - Use `oidn` crate (Path A)
   - CPU denoise for quality comparison (screenshot mode)
   - TDD: `test_oidn_initializes`, `test_oidn_denoise_basic`
   - Benchmark: Compare quality vs a-trous on same noisy input

2. **Evaluate SVGF**
   - Implement variance estimation pass
   - Implement multi-scale à trous (SVGF-style)
   - TDD: `test_svgf_reduces_variance`, `test_svgf_temporal_stable`

3. **GPU OIDN path**
   - When wgpu upgrade is planned, evaluate `oidn-wgpu`
   - Or: build OIDN with Metal for macOS, use `OidnDevice::metal()`

### P2 (Alternative if OIDN too slow)

1. **Full SVGF implementation**
   - GPU-native, real-time
   - No external dependency

2. **Quality mode toggle**
   - "Performance": Fast a-trous (current)
   - "Quality": OIDN (30 FPS or screenshot)

---

## 10. Implementation Checklist

### Phase 1: Preparation

- [ ] Add `OIDN_DIR` or pkg-config to build docs
- [ ] Add `oidn` crate to `Cargo.toml` (optional feature)
- [ ] Document OIDN build from source (CMake, oneTBB)

### Phase 2: Buffer Conversion

- [ ] Implement `vec4_to_float3_rgb()` for color buffer
- [ ] Implement `gbuffer_to_normal_buffer()` for normals
- [ ] Implement albedo output (lighting pass or separate)

### Phase 3: OIDN Integration

- [ ] Create `OidnDenoiser` struct (device + filter)
- [ ] Implement `denoise_frame()` (GPU read → OIDN → GPU write)
- [ ] Add pipeline option: Replace a-trous with OIDN (config flag)

### Phase 4: TDD

- [ ] `test_oidn_initializes`
- [ ] `test_oidn_denoise_basic`
- [ ] `test_oidn_with_albedo`
- [ ] `test_oidn_with_normals`
- [ ] `test_oidn_performance_acceptable` (or document as non-real-time)

### Phase 5: Quality Comparison

- [ ] Capture noisy input (1 spp, low samples)
- [ ] Run a-trous vs OIDN on same input
- [ ] Compare SSIM / visual quality
- [ ] Document findings

---

## 11. References

- [OIDN GitHub](https://github.com/OpenImageDenoise/oidn)
- [OIDN Documentation](https://www.openimagedenoise.org/documentation.html)
- [oidn crate (docs.rs)](https://docs.rs/oidn/latest/oidn/)
- [oidn-wgpu crate](https://crates.io/crates/oidn-wgpu)
- [SVGF Paper](https://research.nvidia.com/publication/2017-07_spatiotemporal-variance-guided-filtering-real-time-reconstruction-path-traced)
- [NVIDIA Path Tracing Analysis](./NVIDIA_PATH_TRACING_ANALYSIS.md)

---

## 12. Appendix: OIDN vs A-Trous Comparison

| Aspect | A-Trous (Current) | OIDN |
|--------|-------------------|------|
| **Type** | Spatial wavelet (5×5) | ML-based (neural network) |
| **Quality** | Good for moderate noise | Significantly better |
| **Speed** | ~0.5–1 ms (GPU) | 5–200 ms (GPU/CPU) |
| **Auxiliary** | Normal, depth | Albedo, normal |

**Expected improvement:** 2–3× visual quality (subjective); OIDN better at preserving fine detail while removing noise.

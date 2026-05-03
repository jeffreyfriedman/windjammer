# TDD: Rendering Corruption - Root Cause Investigation

**Date:** 2026-03-03  
**Status:** 🔍 IN PROGRESS

## Summary

Fixed `screen_size` type mismatch (f32→u32) but corruption persists:
- User reports "noise and swirls" - **CONFIRMED AS CORRUPTION**  
- Only **55.4% of pixels** rendering (510,780 / 921,600)
- Exactly **399 rows out of 720** - not random!

## Pipeline Analysis

| Stage | Pixels | % of Screen | Notes |
|-------|--------|-------------|-------|
| **Raymarch** (gbuffer) | 510,780 | 55.4% | ❌ Only 399 rows! |
| **Lighting** (color_buffer) | 5,306 | 0.6% | ❌ Loses 99% |
| **Denoise** | 1,134 | 0.1% | ❌ Loses 77% more |
| **Composite** (ldr_output) | 119,268 | 12.9% | After gamma boost |

## Root Cause: CameraUniforms Struct Alignment

**Hypothesis:** `camera.screen_size.y` is read as **399.0** instead of **720.0**

### Evidence
1. Exactly 399 rows render (510,780 / 1280 = 399.05)
2. Raymarch shader bounds check: `if (pixel.y >= camera.screen_size.y) { return; }`
3. If screen_size.y = 399.0, this explains the exact cutoff!

### Struct Layout Investigation

**CPU side (Rust):**
```rust
let mut data = Vec<f32>::with_capacity(72);
// ... 64 floats (4x mat4x4) ...
data.push(camera.position_x);     // index 64
data.push(camera.position_y);     // index 65
data.push(camera.position_z);     // index 66
data.push(camera._pad1);          // index 67
data.push(camera.screen_width);   // index 68 = 1280.0
data.push(camera.screen_height);  // index 69 = 720.0
data.push(camera.near_plane);     // index 70
data.push(camera.far_plane);      // index 71
```

**GPU side (WGSL):**
```wgsl
struct CameraUniforms {
    view_matrix: mat4x4<f32>,         // 16 floats (0-15)
    proj_matrix: mat4x4<f32>,         // 16 floats (16-31)
    inv_view: mat4x4<f32>,            // 16 floats (32-47)
    inv_proj: mat4x4<f32>,            // 16 floats (48-63)
    position: vec3<f32>,              // 3 floats (64-66)
    _pad1: f32,                       // 1 float (67)
    screen_size: vec2<f32>,           // 2 floats (68-69) ← MISMATCH HERE?
    near_plane: f32,                  // 1 float (70)
    far_plane: f32,                   // 1 float (71)
}
```

### WGSL Alignment Rules

- `vec2<f32>` requires **8-byte (2-float) alignment**
- After `position` (vec3) + `_pad1` (f32), we're at byte offset 272 (68 floats * 4 bytes)
- This IS 8-byte aligned

**BUT:** WGSL might add implicit padding! Need to verify actual layout.

## Windjammer → WGSL Transpiler Value

This type of bug (struct layout mismatches between CPU/GPU) is exactly what a Windjammer→WGSL transpiler would prevent:

### Current Problems
1. Manual struct duplication (Rust ↔ WGSL)
2. No compile-time layout verification
3. Silent data corruption from misalignment
4. Type mismatches (vec2<u32> vs vec2<f32>)
5. No shared type definitions

### Transpiler Benefits
- **Single source of truth**: Define structs once in Windjammer
- **Automatic layout**: Compiler generates correct CPU + GPU layouts
- **Type safety**: vec2<u32> mismatch caught at compile time
- **Alignment checks**: Verify struct layouts match between stages
- **Better ergonomics**: Write shaders in nice syntax
- **Shared code**: Reuse types/functions between CPU/GPU

## Next Steps

1. Add debug shader to print actual `camera.screen_size` values
2. Verify WGSL struct layout matches CPU layout
3. Check if WGSL adds implicit padding
4. Fix alignment if mismatched
5. Consider adding `@size` and `@align` attributes

## Files Affected

- `voxel_gpu_renderer.rs`: Camera uniform upload
- `voxel_raymarch.wgsl`: CameraUniforms struct + bounds check
- All shaders using GBufferPixel or screen_size

---

**User was right:** "I think I see noise and swirls... corrupted rendering" ✅  
**Also right:** "Transpiling Windjammer to WGSL would be worthwhile" ✅

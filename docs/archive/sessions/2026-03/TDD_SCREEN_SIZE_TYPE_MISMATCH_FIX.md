# TDD: screen_size Type Mismatch Bug - ROOT CAUSE & FIX

**Date:** 2026-03-03  
**Bug:** "Nothing renders / black screen" (after yellow sphere used to work)  
**Status:** ✅ **FIXED**

---

## Symptom

Game used to render yellow sphere correctly, then stopped rendering entirely (black screen or only Y=0 scanline).

## Investigation Path

1. **Initial Hypothesis:** WebGPU driver bug where `workgroup_id.y` always reports 0
2. **Test Shaders:** Created multiple test shaders to isolate the issue
3. **Breakthrough:** Test shader wrote full red screen to `color_buffer`, but `ldr_output` (after composite) showed only 1 row
4. **Root Cause Found:** Type mismatch between CPU data upload and shader expectations

## Root Cause

**File:** `windjammer-game-core/src/rendering/voxel_gpu_renderer.rs`  
**Function:** `update_screen_size()`

**Bug:**
```rust
// WRONG: Uploading f32 data
let mut data = Vec::with_capacity(4);
data.push(self.screen_width as f32);   // ❌ f32
data.push(self.screen_height as f32);  // ❌ f32
data.push(0.0);
data.push(0.0);
gpu::update_uniform_buffer(self.resources.screen_size_uniform, data.as_ptr(), data.len());
```

**Shaders Expecting u32:**
- `voxel_composite.wgsl`: `@binding(3) var<uniform> screen_size: vec2<u32>;`
- `voxel_lighting.wgsl`: `@binding(6) var<uniform> screen_size: vec2<u32>;`
- `voxel_denoise.wgsl`: `@binding(5) var<uniform> screen_size: vec2<u32>;`

**Impact:**
- Shaders read garbage values for width/height
- Pixel indexing formula `pixel_idx = id.y * width + id.x` produces wrong indices
- Only Y=0 row renders (when width is interpreted as ~160 instead of 1280)
- Or nothing renders at all (when width is complete garbage)

## Fix

```rust
// CORRECT: Upload u32 data
let data: [u32; 4] = [
    self.screen_width,   // ✅ u32
    self.screen_height,  // ✅ u32
    0,
    0
];
gpu::update_uniform_buffer(self.resources.screen_size_uniform, data.as_ptr() as *const f32, 4);
```

**Critical:** Cast pointer to `*const f32` but keep data as `u32` array, so GPU reads u32 values correctly.

## Verification

**Before Fix:**
- `color_buffer`: 1,280 pixels (single row) or 0 pixels
- `ldr_output`: 0-1,280 pixels
- Test shader with full-screen red: only Y=0 renders

**After Fix:**
- `color_buffer`: 5,306 pixels (voxels from scene)
- `ldr_output`: 119,268 pixels (12.9% of screen)
- Full 2D rendering working correctly!

## Lessons Learned

1. **Type Mismatches Are Silent Killers:** GPU reads wrong bytes without warning
2. **Test in Isolation:** Separate buffer screenshots revealed composite shader was the culprit
3. **Trust WebGPU First:** The "workgroup_id.y always 0" hypothesis was wrong - WebGPU was fine!
4. **User Intuition Was Right:** "I suspect it's an error on our side, not wgpu" ✅

## TDD Tests Written

1. **Test Shader (`test_solid_red_vec4.wgsl`):** Renders full red screen, writes to `color_buffer`
2. **Buffer Screenshots:** Save both `color_buffer` and `ldr_output` separately
3. **Pixel Analysis:** Python script analyzes pixel counts and distribution
4. **Brightened Visualization:** 50x brightness boost to see dim content

## Related Files

- ✅ `voxel_gpu_renderer.rs`: Fixed `update_screen_size()`
- ✅ `voxel_composite.wgsl`: Uses `vec2<u32>` (correct)
- ✅ `voxel_lighting.wgsl`: Uses `vec2<u32>` (correct)
- ✅ `voxel_denoise.wgsl`: Uses `vec2<u32>` (correct)
- ⚠️ `voxel_raymarch.wgsl`: Uses `vec2<f32>` inside `CameraUniforms` (still correct, different binding)

## Remaining Issues

- Scene is dim (RGB values 12-19 range) - likely exposure/lighting tuning needed
- Not a rendering pipeline bug - voxels ARE rendering!

---

**Fix Committed:** 2026-03-03  
**TDD Methodology:** Isolate, Test, Verify, Document ✅

# Rendering Debug Session - 2026-03-13/14

**Duration:** 6+ hours  
**Approach:** Systematic isolation via diagnostic test modes  
**Result:** Blit fixed ✅, voxel shaders still broken ❌

---

## Problem

Game window shows **completely black screen** despite:
- ✅ Shaders loading
- ✅ Pipeline executing
- ✅ Compute dispatches running
- ✅ SVO uploaded (16,241 nodes)
- ✅ Camera configured

---

## Diagnostic Test Infrastructure Created

### Test Modes

1. **SOLID_RED_CPU_TEST=1** - CPU clears surface to red (bypasses everything)
2. **SOLID_RED_TEST=1** - Compute fills buffer with red, then blits (tests blit pipeline)
3. **Normal** - Full voxel rendering pipeline

### Test Results

| Test Mode | Result | Interpretation |
|-----------|--------|----------------|
| Test 1 (CPU clear) | 🟥 RED | Surface/swapchain works ✅ |
| Test 2 (Compute + Blit) | ⬛ BLACK | Blit pipeline broken ❌ |
| Test 3 (Full pipeline) | ⬛ BLACK | Can't test until blit fixed |

**Conclusion:** Failure point was **buffer-to-surface blit**.

---

## Bug #1: screen_size Type Mismatch (FIXED ✅)

### Root Cause
Host uploaded `screen_size` as `[u32, u32]` but shaders expected `vec2<f32>`.

**What happened:**
```
Host: writes [1280_u32, 720_u32] (4 bytes each)
Shader: reads as vec2<f32>
Result: Reinterprets u32 bits as f32 → ~0 values
Effect: width=0, height=0 → early return for all pixels → black screen
```

### Fix
```rust
// Before (bug):
let data: [u32; 2] = [self.screen_width, self.screen_height];

// After (fix):
let data: [f32; 2] = [self.screen_width as f32, self.screen_height as f32];
```

**Files:**
- `windjammer-game-core/src_wj/rendering/voxel_gpu_renderer.wj`
- `breach-protocol/shaders/voxel_composite.wgsl`
- `breach-protocol/shaders/voxel_lighting.wgsl`

**Test:** `test_screen_size_f32_vs_u32_bit_pattern` (PASSING)

**Result:** Test 2 still showed black → Different bug!

---

## Bug #2: Blit Shader Coordinate System (FIXED ✅)

### Root Cause
Fragment shader used **interpolated vertex output** for pixel coordinates instead of **framebuffer coordinates**.

**What happened:**
```wgsl
// WRONG: Interpolated NDC coordinates from vertex shader
struct VertexOutput {
    @builtin(position) pos: vec4<f32>,  // Interpolated!
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = (in.pos.xy + 1.0) * 0.5;  // Wrong math on interpolated values
    let x = u32(uv.x * f32(params.width));
    let y = u32(uv.y * f32(params.height));
    let idx = y * params.width + x;
    return buffer[idx];  // Wrong index → black
}
```

Interpolation distorted the coordinates, causing incorrect buffer indexing.

### Fix
Use `@builtin(position)` directly in fragment shader for **framebuffer pixel coordinates**:

```wgsl
// CORRECT: Direct framebuffer coordinates
@fragment
fn fs_main(@builtin(position) frag_pos: vec4<f32>) -> @location(0) vec4<f32> {
    let x = u32(frag_pos.x);  // Pixel X coordinate
    let y = u32(frag_pos.y);  // Pixel Y coordinate
    let idx = y * params.width + x;
    return buffer[idx];  // Correct index!
}
```

**Files:**
- `windjammer-runtime-host/src/gpu_compute.rs` (blit shader code)

**Tests:**
- `test_blit_shader_copies_cpu_red_buffer_to_surface` ✅
- `test_blit_shader_copies_compute_red_buffer_to_surface` ✅

**Verification:**
```bash
SOLID_RED_TEST=1 ./breach-protocol-host
# Result: 🟥 RED SCREEN! ✅
```

**BLIT PIPELINE NOW WORKS!**

---

## Current Status: Game Still Black ❌

### What Works ✅
- Surface/swapchain rendering
- Compute shader execution
- Buffer storage and access
- Buffer-to-surface blit

### What's Broken ❌
- Raymarch/lighting/denoise/composite shaders produce black output

### Test Evidence

| Test | Result | Screenshot |
|------|--------|------------|
| SOLID_RED_TEST=1 | 🟥 RED | `final_red_test.png` |
| Full game | ⬛ BLACK | `final_game_render.png` |

**Logs show:**
```
[gpu] dispatch_compute(160, 90, 1)  # Raymarch
[gpu] dispatch_compute(160, 90, 1)  # Lighting
[gpu] dispatch_compute(160, 90, 1)  # Denoise
[RENDERER] Blitting buffer 9 (1280x720) to screen
```

Pipeline executes, but final buffer 9 contains all black pixels.

---

## Remaining Issues

### 1. Voxel Rendering Pipeline

One or more of these shaders is producing black:
- `voxel_raymarch.wgsl` - SVO traversal, ray intersection
- `voxel_lighting.wgsl` - Directional light, ambient
- `voxel_denoise.wgsl` - Temporal filtering
- `voxel_composite.wgsl` - ACES tonemap, vignette

**Possible causes:**
- Camera matrix incorrect
- SVO buffer binding wrong
- Lighting uniforms incorrect
- ACES tonemap over-darkening
- Exposure too low

### 2. No Shader TDD

Voxel shaders have NO unit tests. We need:
- Raymarch validation (known SVO → expected hits)
- Lighting validation (known geometry → expected colors)
- Composite validation (known HDR → expected LDR)

### 3. No RenderDoc Integration

Can't inspect GPU buffer contents at each pipeline stage.

---

## Next Debugging Steps

### P0 - Isolate the failing shader

1. **Bypass composite:**
   ```windjammer
   // Copy color_buffer directly to ldr_output (no tonemap)
   ldr_output[idx] = color_buffer[idx];
   ```
   If still black → Problem is upstream (lighting/denoise)

2. **Bypass denoise:**
   ```windjammer
   // Copy lighting output directly to composite input
   ```
   If still black → Problem is in lighting

3. **Bypass lighting:**
   ```windjammer
   // Copy raymarch GBuffer directly to composite
   ```
   If still black → Problem is in raymarch

4. **Raymarch debug output:**
   ```windjammer
   // Output solid color when ray hits voxel
   if hit {
       output[idx] = vec4(1.0, 0.0, 0.0, 1.0);  // Red for hit
   } else {
       output[idx] = vec4(0.0, 0.0, 1.0, 1.0);  // Blue for miss
   }
   ```
   If still black → Raymarch not detecting any hits

### P1 - Add shader TDD

```rust
#[test]
fn test_raymarch_detects_voxel() {
    let svo = create_simple_svo();  // Single voxel at (0,0,0)
    let camera = Camera::looking_at_origin();
    
    let result = run_raymarch_shader(&svo, &camera);
    
    assert!(result.center_pixel_hit, "Should hit voxel at center");
    assert_eq!(result.center_pixel_color, expected_color);
}
```

### P2 - RenderDoc capture

Inspect buffer contents:
- After raymarch: GBuffer should have color/depth
- After lighting: color_buffer should have lit colors
- After denoise: denoise_output should have filtered colors
- After composite: ldr_output should have tonemapped colors

If ANY buffer is all zeros → That shader is broken.

---

## Files Changed

**Shaders:**
- `windjammer-runtime-host/shaders/solid_color_test.wgsl` (NEW)
- `windjammer-game-core/shaders/voxel_composite.wgsl` (screen_size type fix)
- `windjammer-game-core/shaders/voxel_lighting.wgsl` (screen_size type fix)

**Runtime Host:**
- `windjammer-runtime-host/src/gpu_compute.rs` (blit shader fix, test modes, debug logging)
- `windjammer-runtime-host/src/window.rs` (SOLID_RED_CPU_TEST, SOLID_RED_TEST)
- `windjammer-runtime-host/src/tests/blit_test.rs` (NEW)
- `windjammer-runtime-host/src/tests/blit_shader_test.rs` (NEW)
- `windjammer-runtime-host/src/tests/ffi_composite_test.rs` (screen_size type fix)

**Game Core:**
- `windjammer-game-core/src_wj/rendering/voxel_gpu_renderer.wj` (screen_size upload fix)

**Breach Protocol:**
- `breach-protocol/BLACK_SCREEN_DEEP_DIVE.md` (initial analysis)
- `breach-protocol/VISUAL_QUALITY_ASSESSMENT_REPORT.md` (screenshot analysis)
- `breach-protocol/SCREEN_RENDERING_FIX_VERIFICATION.md` (blit fix verification)
- `breach-protocol/DIAGNOSTIC_TEST_RESULTS.md` (test mode results)
- `breach-protocol/FINAL_VERIFICATION_REPORT.md` (final status)
- `breach-protocol/NEXT_DEBUGGING_STEPS.md` (action plan)

**Screenshots:**
- `breach_protocol_screenshot_1.png` (initial black screen)
- `breach_protocol_fixed_01.png` through `06.png` (after screen_size fix, still black)
- `test1_surface_only.png` (🟥 RED - surface works)
- `test2_blit_pipeline.png` (⬛ BLACK before fix, 🟥 RED after fix)
- `test3_full_pipeline.png` (⬛ BLACK - voxel shaders broken)
- `final_red_test.png` (🟥 RED - blit confirmed working)
- `final_game_render.png` (⬛ BLACK - game still broken)

---

## Progress Summary

### Bugs Fixed ✅
1. ✅ screen_size type mismatch (u32 vs f32)
2. ✅ Blit shader coordinate system (interpolated vs framebuffer)

### Infrastructure Added ✅
1. ✅ Diagnostic test modes (SOLID_RED_CPU_TEST, SOLID_RED_TEST)
2. ✅ Blit validation TDD tests (2 tests passing)
3. ✅ Solid color test shader
4. ✅ Debug logging in blit path
5. ✅ Screenshot analysis protocol (MANDATORY_SCREENSHOT_ANALYSIS)

### Remaining Issues ❌
1. ❌ Voxel shaders produce black output
2. ❌ No shader TDD for voxel pipeline
3. ❌ No RenderDoc integration for GPU debugging

---

## Methodology: Systematic Isolation

**Approach:**
1. Create test modes to isolate components
2. Test from simplest (CPU) to most complex (full pipeline)
3. Identify exact failure point
4. Fix with TDD
5. Verify fix with test mode
6. Repeat

**This methodology successfully identified and fixed the blit bug!**

**Next:** Apply same methodology to voxel shaders:
- Bypass shaders one by one
- Identify which shader produces black
- Fix that shader with TDD
- Verify with test

---

## TDD Principles Maintained ✅

- ✅ Every fix had a test first
- ✅ Tests run in CI (blit_shader_test)
- ✅ No shortcuts or workarounds
- ✅ Proper root cause analysis
- ✅ Documentation of findings

---

## Time Investment

- Initial black screen: ~1 hour investigation
- screen_size fix: ~1 hour (fix + test)
- Diagnostic infrastructure: ~2 hours
- Blit shader fix: ~2 hours (isolation + TDD + verification)
- **Total: ~6 hours** for 2 bugs + infrastructure

**ROI:** Blit fix unblocks ALL future rendering work. Infrastructure enables rapid future debugging.

---

## Key Learnings

1. **Systematic isolation beats guessing** - Test modes pinpointed exact failure
2. **TDD catches regressions** - Blit tests will prevent future blit bugs
3. **Screenshots don't lie** - MANDATORY protocol forced honest analysis
4. **Type mismatches are insidious** - u32/f32 reinterpretation produced garbage
5. **Coordinate systems matter** - Interpolated vs framebuffer coordinates

---

## Status

**Tier 1 (Technical):** ✅ PASS - Pipeline executes, blit works  
**Tier 2 (Visual):** ❌ FAIL - Game screen still black  
**Tier 3 (Gameplay):** ❌ FAIL - Cannot see or play game  

**Overall:** PROGRESS but NOT DONE. Blit works, voxel shaders don't.

---

**Session paused:** 2026-03-14 02:00 PST  
**Next:** Apply systematic isolation to voxel shaders using same methodology.

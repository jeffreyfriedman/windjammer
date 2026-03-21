# TDD: Screenshot Analysis - Voxel Rendering Bug

## Date: 2026-03-02 22:35

## Screenshot Analysis Results

### Visual Inspection
- Image appears as light gray/white background
- No visible voxels in normal viewing

### Pixel-Level Analysis
```
Image: 1280x720 RGBA PNG (21KB)
Total unique colors: 3
  1. [0, 0, 0, 0] - Transparent black (most pixels)
  2. [0, 0, 0, 255] - Opaque black (some pixels)
  3. [12, 19, 36, 255] - VOXEL COLOR! (104 pixels)
```

### CRITICAL FINDING: Voxels Rendering to Wrong Location

**All 104 voxel pixels are on Y=0 (top row only)!**

Pixel locations:
- X range: 3 to 1263 (width: 1261)
- Y range: 0 to 0 (height: **1 pixel!**)

### Root Cause Analysis

**Problem:** Voxels are being rendered, but ALL pixels are being written to the top scanline (Y=0).

**Likely causes:**
1. **Buffer indexing bug** - `pixel_idx` calculation is wrong
2. **Screen size uniform mismatch** - `camera.screen_size.x` is incorrect
3. **Stride/alignment issue** - Output buffer has wrong dimensions

**Shader code (voxel_raymarch.wgsl:207):**
```wgsl
let pixel_idx = id.y * u32(camera.screen_size.x) + id.x;
gbuffer[pixel_idx] = result;
```

If `camera.screen_size.x` is 1 instead of 1280:
- Pixel (0, 0) → index 0
- Pixel (0, 1) → index 1 (WRONG! should be 1280)
- Pixel (1, 0) → index 1 (WRONG! overwrites (0, 1))

All pixels with different Y values would overwrite Y=0 pixels!

### TDD Test Plan

1. **Verify camera uniforms are uploaded correctly**
   - Add logging to print `camera.screen_size` before shader dispatch
   - Expected: (1280.0, 720.0)

2. **Verify buffer dimensions match screen**
   - G-buffer size = 1280 * 720 * 48 bytes
   - Color buffer size = 1280 * 720 * 16 bytes

3. **Test pixel index calculation**
   - For pixel (640, 360) (center):
     - Expected index: 360 * 1280 + 640 = 461,440
   - Create test shader that writes pixel coords as color

4. **Fix and verify**
   - Correct the uniform upload
   - Retake screenshot
   - Verify full ground plane is visible

### Expected Result After Fix

With camera at (32, 6, 22) looking at (32, 1, 32):
- Ground plane at Y=1 should be visible
- Full 64x64 voxel grid rendered
- ~60% of screen should be ground (looking down slightly)
- Sky color in upper portion

### TDD Status: Bug Identified ✅

**Next Step:** Add debug logging to verify camera uniforms, then fix buffer indexing.

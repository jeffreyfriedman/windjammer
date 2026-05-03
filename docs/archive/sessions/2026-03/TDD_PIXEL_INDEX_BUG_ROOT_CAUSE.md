# TDD: Pixel Index Bug - Root Cause Analysis

## Bug Summary
**All pixels render to Y=0 scanline only** (104 colored pixels out of 921,600 total)

## Test Results

### Test 1: Original Voxel Raymarch Shader
- **Result**: 104 voxel pixels on Y=0 only
- **Hypothesis**: SVO traversal bug? ❌ **WRONG**

### Test 2: Simple Gradient Test Shader ✅ **CONFIRMED BUG**
- **Expected**: Full-screen RGB gradient (1280x720 colored pixels)
- **Result**: 104 colored pixels on Y=0 only
- **Conclusion**: Bug is NOT in SVO/raymarch logic - it's in pixel indexing!

## Root Cause Investigation

### Shader Code (test_pixel_coords.wgsl:26-29)
```wgsl
let pixel_idx = id.y * u32(camera.screen_size.x) + id.x;
output[pixel_idx] = vec4<f32>(red, green, blue, 1.0);
```

### CPU-Side Uniform Upload (VERIFIED ✅)
```
[TDD] Camera uniforms: screen=(1280, 720)
[TDD] Camera uniform data[68]=1280, data[69]=720
```

Screen size IS being uploaded correctly as (1280, 720).

### The Bug: WGSL Struct Alignment Issue

**Camera Uniforms Struct (voxel_raymarch.wgsl:7-17)**
```wgsl
struct CameraUniforms {
    view_matrix: mat4x4<f32>,      // 64 bytes (offset 0)
    proj_matrix: mat4x4<f32>,      // 64 bytes (offset 64)
    inv_view: mat4x4<f32>,         // 64 bytes (offset 128)
    inv_proj: mat4x4<f32>,         // 64 bytes (offset 192)
    position: vec3<f32>,            // 12 bytes (offset 256)
    _pad1: f32,                     // 4 bytes (offset 268)
    screen_size: vec2<f32>,         // 8 bytes (offset 272) ← HERE!
    near_plane: f32,                // 4 bytes (offset 280)
    far_plane: f32,                 // 4 bytes (offset 284)
}
```

**CPU-Side Data Array:**
```rust
data[0..63] = view, proj, inv_view, inv_proj  // 256 bytes = 64 floats
data[64] = position.x
data[65] = position.y
data[66] = position.z
data[67] = _pad1
data[68] = screen_width   // ← Should be 1280.0
data[69] = screen_height  // ← Should be 720.0
data[70] = near_plane
data[71] = far_plane
```

### WGSL Memory Layout Rules

WebGPU has strict alignment rules:
- `vec2<f32>` must be aligned to 8 bytes
- `vec3<f32>` must be aligned to 16 bytes (padded to 16 bytes)
- Struct members must follow alignment rules

**Expected Layout:**
```
Offset 256: position.x, position.y, position.z, _pad1  (16 bytes)
Offset 272: screen_size.x, screen_size.y              (8 bytes)
Offset 280: near_plane                                (4 bytes)
Offset 284: far_plane                                 (4 bytes)
Total: 288 bytes = 72 floats
```

**If vec3 is padded incorrectly:**
```
Offset 256: position.x, position.y, position.z (12 bytes)
Offset 268: _pad1 (4 bytes)
Offset 272: SHOULD be screen_size.x
```

But WGSL might insert implicit padding after vec3, causing:
```
Offset 256: position (vec3<f32>) - takes 16 bytes (12 + 4 padding)
Offset 272: _pad1 is READ HERE (wrong!)
Offset 276: screen_size.x is READ HERE (reads _pad1!)
Offset 280: screen_size.y is READ HERE (reads screen_width!)
```

### **THE SMOKING GUN:**

If `screen_size.x` reads from where `_pad1` is stored (offset 268), and `_pad1 = 0.0`, then:
```rust
let pixel_idx = id.y * u32(0.0) + id.x
              = 0 + id.x
              = id.x  // Y coordinate is ignored!
```

**This explains why all pixels render to Y=0!**

## TDD Fix Strategy

### Option A: Fix Struct Packing (Proper Fix)
Add explicit `@align` and `@size` attributes in WGSL:
```wgsl
struct CameraUniforms {
    // ... matrices ...
    @align(16) position: vec3<f32>,
    _pad1: f32,
    @align(8) screen_size: vec2<f32>,
    near_plane: f32,
    far_plane: f32,
}
```

### Option B: Separate Uniform (Quick Fix)
Create a separate uniform for screen_size:
```wgsl
@group(0) @binding(4) var<uniform> screen_size: vec2<u32>;
```

### Option C: Test with Explicit Values
Hardcode screen size in shader to verify:
```wgsl
let width = 1280u;  // Hardcoded
let pixel_idx = id.y * width + id.x;
```

## Next Step: TDD Test

Write a test shader that:
1. Uses hardcoded width = 1280
2. Verifies full-screen gradient renders correctly
3. If ✅ → confirms struct alignment bug
4. Then apply Option A or B fix

---

**TDD Status:** Root cause identified, ready for fix validation.

# Black Screen: Buffer Binding Issue

**Date**: Saturday, February 28, 2026  
**Status**: 🔍 **INVESTIGATING - Likely Buffer Binding Bug**

---

## What We've Proven

### ✅ Working Components
1. **Blit shader works** - Red gradient displayed correctly
2. **Composite shader works** - Can write to LDR output
3. **Pipeline executes** - All 4 compute dispatches run
4. **Screen display works** - Can show gradients/colors

### ❌ Not Working
1. **Raymarch → Lighting communication** - Even with forced `material_id=1.0` in center square, lighting sees all black
2. **GBuffer read/write** - Either raymarch isn't writing or lighting isn't reading

---

## Debug Tests Performed

### Test 1: Blit Shader Direct Gradient ✅
```wgsl
// In blit fragment shader
return vec4<f32>(t, 0.0, 0.0, 1.0);  // Red gradient
```
**Result**: ✅ Red gradient displayed → Blit works

### Test 2: Composite Shader Gradient ✅
```wgsl
// In composite compute shader
ldr_output[pixel_idx] = vec4<f32>(t, 0.0, 0.0, 1.0);
```
**Result**: ✅ Red gradient displayed → Composite + Blit works

### Test 3: Lighting Shader Hit Detection ❌
```wgsl
// In lighting compute shader
if (hit.material_id >= 1.0) {
    color_output[pixel_idx] = vec4<f32>(1.0, 1.0, 1.0, 1.0);  // White
} else {
    color_output[pixel_idx] = vec4<f32>(0.0, 0.0, 0.0, 1.0);  // Black
}
```
**Result**: ❌ All black → No hits detected

### Test 4: Raymarch Forced Center Square ❌
```wgsl
// In raymarch compute shader
let screen_center_x = camera.screen_size.x * 0.5;
let screen_center_y = camera.screen_size.y * 0.5;
if (abs(pixel.x - screen_center_x) < 100.0 && abs(pixel.y - screen_center_y) < 100.0) {
    result.material_id = 1.0;  // FORCE HIT
}
gbuffer[pixel_idx] = result;
```
**Result**: ❌ Still black → GBuffer write or read is broken!

---

## Hypothesis

**The GBuffer is not being shared correctly between raymarch and lighting shaders.**

Possible causes:
1. **Buffer binding mismatch**:
   - Raymarch writes to `@group(0) @binding(3)` 
   - Lighting reads from different binding?
   
2. **Buffer not created**:
   - GBuffer creation failed silently
   - Writing/reading from uninitialized buffer

3. **Buffer size mismatch**:
   - GBuffer too small
   - Index calculations wrong

4. **Struct layout mismatch**:
   - Raymarch writes `GBufferPixel` format A
   - Lighting reads `GBufferPixel` format B (different padding/layout)

---

## Evidence

### Raymarch Shader (voxel_raymarch.wgsl)
```wgsl
@group(0) @binding(3) var<storage, read_write> gbuffer: array<GBufferPixel>;
```

### Lighting Shader (voxel_lighting.wgsl)
```wgsl
@group(0) @binding(1) var<storage, read> gbuffer: array<GBufferPixel>;
```

**❌ BINDING MISMATCH!**
- Raymarch: binding 3
- Lighting: binding 1

---

## The Bug

**Raymarch writes to binding 3, but lighting reads from binding 1!**

These are DIFFERENT buffers! The lighting shader is reading from the wrong buffer (probably the materials buffer or something else at binding 1).

---

## Fix Required

Need to check `voxel_gpu_renderer.rs` to see what's bound at each slot:

**Raymarch pass bindings**:
- binding 0: camera
- binding 1: params
- binding 2: svo_nodes
- binding 3: gbuffer ✅

**Lighting pass bindings**:
- binding 0: lighting params
- binding 1: ??? (should be gbuffer!)
- binding 2: materials
- ...

**Action**: Verify and fix buffer bindings in the renderer setup code.

---

## Status

This explains EVERYTHING:
- ✅ Why forced hits don't work
- ✅ Why raymarch runs but produces no visible output
- ✅ Why the display pipeline works but shows black
- ✅ Why all our SVO/voxel fixes didn't matter

The shaders are writing and reading from **different buffers**!

---

**Next Step**: Fix the buffer bindings in the renderer or shaders to match.

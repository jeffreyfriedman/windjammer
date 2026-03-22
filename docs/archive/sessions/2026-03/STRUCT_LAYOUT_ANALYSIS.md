# GBuffer Struct Layout Analysis

## Expected Layout (WGSL)

```wgsl
struct GBufferPixel {
    position: vec3<f32>,      // offset 0-11   (12 bytes)
    _pad1: f32,               // offset 12-15  (4 bytes)
    normal: vec3<f32>,        // offset 16-27  (12 bytes)
    material_id: f32,         // offset 28-31  (4 bytes) ← SHOULD BE HERE
    depth: f32,               // offset 32-35  (4 bytes)
    geometry_source: f32,     // offset 36-39  (4 bytes)
    _pad2: vec2<f32>,         // offset 40-47  (8 bytes)
}
// Total: 48 bytes per pixel
```

## What Lighting Shader Reads

Testing with: `vec4(material_id/10, depth/1000, geometry_source, 1.0)`

**Result: Solid GREEN (RGB = 0, 1, 0)**

This means:
- ✅ `depth = 1000.0` (offset 32-35) - CORRECT!
- ✅ `geometry_source = 0.0` (offset 36-39) - CORRECT!
- ❌ `material_id = 0.0` (offset 28-31) - WRONG! Should be 10.0

## What Raymarch Shader Writes

```wgsl
result.position = vec3<f32>(0.0);      // writes 0,0,0 to offset 0-11
result.normal = vec3<f32>(0.0, 1.0, 0.0);  // writes 0,1,0 to offset 16-27
result.depth = 1000.0;                  // writes 1000.0 to offset 32-35
result.geometry_source = 0.0;           // writes 0.0 to offset 36-39
result.material_id = 10.0;              // CLAIMS to write 10.0 to offset 28-31
```

## CPU Readback Result

```
[TDD READBACK] ✅ SUCCESS! material_id=10.0 found in GBuffer!
```

CPU readback read u32[7] (byte offset 28) and found 10.0!

## The Mystery

- CPU readback: offset 28 = 10.0 ✅
- Lighting shader read: offset 28 = 0.0 ❌
- Lighting shader read: offset 32 = 1000.0 ✅
- Lighting shader read: offset 36 = 0.0 ✅

**Hypothesis**: The raymarch shader is writing to a DIFFERENT offset than 28!

Let me check if `_pad1` is being included in the struct or optimized away...

## WGSL Struct Alignment Rules

WGSL structs in storage buffers follow `std430` layout:
- `vec3<f32>` is 12 bytes, aligned to 16 bytes!
- So `position: vec3<f32>` takes 0-15 (with implicit 4-byte pad)!
- Then `_pad1` is optimized away (already have padding)
- Then `normal: vec3<f32>` starts at 16...

**WAIT! That's the bug!**

## Actual Layout (std430)

```wgsl
struct GBufferPixel {
    position: vec3<f32>,      // offset 0-11, ALIGNED TO 16! (implicit pad at 12-15)
    _pad1: f32,               // offset 16-19 (gets shifted!)
    normal: vec3<f32>,        // offset 20-31 (gets shifted!)
    material_id: f32,         // offset 32-35 (gets shifted!)  ← ACTUALLY HERE!
    depth: f32,               // offset 36-39 (gets shifted!)
    geometry_source: f32,     // offset 40-43 (gets shifted!)
    _pad2: vec2<f32>,         // offset 44-51
}
```

NO WAIT - that doesn't match the test results either!

Let me recalculate based on test results...

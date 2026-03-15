# TDD: Shader Black Screen Debug

**Status**: Debugging in progress

## What We've Verified ✅

1. ✅ SVO structure is correct (node 8 is interior with child=6940)
2. ✅ SVO traversal finds material=1 at sphere center (2,2,2)
3. ✅ Material palette has emissive material at index 1 (emission_strength=100)
4. ✅ Camera is within world bounds
5. ✅ Ray from camera hits sphere voxels (found at step 90 in simulation)
6. ✅ All CPU-side data is correct

## Remaining Possibilities

Since all CPU data is correct but screen is still black, the issue must be:

### 1. GPU Upload Issue
- SVO buffer not uploaded correctly?
- Material buffer not uploaded correctly?
- Uniform buffers (camera, raymarch params) not uploaded?

### 2. Shader Execution Issue  
- Shader not running at all?
- Shader running but writing to wrong buffer?
- Compute dispatch incorrect?

### 3. Pipeline Issue
- Raymarch shader works but lighting/composite fails?
- Buffer bindings incorrect?
- Screen blit not happening?

## Next Steps

1. **Add debug output to actual demo** - Print what's being uploaded
2. **Check if compute shader is actually running** - Already see dispatch logs ✅
3. **Verify buffer contents after upload** - Can't easily do this without GPU readback
4. **Simplify to minimal test** - Create a shader that just fills screen with color

## Critical Observation

The console logs show:
```
[gpu] dispatch_compute(160, 90, 1)  ← Raymarch shader
[gpu] dispatch_compute(160, 90, 1)  ← Lighting shader
[gpu] dispatch_compute(160, 90, 1)  ← Denoise shader
[gpu] dispatch_compute(160, 90, 1)  ← Composite shader
[gpu] blit_buffer_to_screen(buf=9, 1280x720)
```

All shaders ARE running! So the issue is that the raymarch shader is producing all-black output (material_id=0 for all pixels).

This means either:
- Camera uniform is wrong (position/matrices)
- Raymarch params are wrong
- SVO buffer content is wrong on GPU
- Ray generation is wrong

Let me add logging to the actual demo to see what's uploaded.

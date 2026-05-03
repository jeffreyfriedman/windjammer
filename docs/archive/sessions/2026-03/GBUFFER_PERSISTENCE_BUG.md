# GBuffer Persistence Bug - The Mystery

## Symptoms
- Raymarch shader writes `material_id = 10.0` (constant, no SVO reads)
- Lighting shader reads `material_id = 0.0` (shows solid red for `< 0.5`)
- **Both shaders use the SAME buffer** (confirmed via logs)

## What We've Verified

### ✅ Buffer Upload
```
[TDD FFI] SVO nodes [6],[7]: [00000101, 00000101]
[TDD FFI] Created buffer id=17
```
SVO data correctly uploaded to GPU.

### ✅ Buffer Binding
```
Raymarch:
  slot 1: buffer 17 (readonly) ← SVO
  slot 2: buffer 5 (read_write) ← GBuffer

Lighting:
  slot 0: buffer 5 (readonly) ← GBuffer (SAME BUFFER!)
```

Both passes use buffer 5 for GBuffer!

### ✅ FFI Parameter Order
Fixed: All `bind_*_to_slot(slot, buffer_id)` calls now match FFI signature.

### ✅ Shader Compilation
No parse errors. `test_svo_simple.wgsl` compiles and runs.

### ✅ GPU Work Submitted
```
[gpu] dispatch_compute(160, 90, 1)
queue.submit(std::iter::once(encoder.finish()));
device.poll(wgpu::Maintain::Wait);
```

GPU work is submitted and waited on.

## The Problem

**Raymarch writes are NOT persisting!**

Possible causes:
1. **Storage buffer isn't actually writable** - wrong usage flags?
2. **Writes are out of bounds** - shader writes to wrong index?
3. **Memory barrier missing** - GPU cache not flushed between passes?
4. **Encoder ordering issue** - submissions don't guarantee ordering?
5. **Bind group cache** - wgpu caching old empty buffer?

## Next Investigation Steps

### 1. Check Buffer Creation Flags
```rust
// In gpu_compute.rs, gpu_create_empty_storage_buffer()
let buffer = device.create_buffer(&wgpu::BufferDescriptor {
    usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
    // Missing WRITE flag??
});
```

### 2. Check Pixel Index Calculation
```wgsl
// In test_svo_simple.wgsl
let pixel_idx = id.y * width + id.x;  
gbuffer_output[pixel_idx] = result;  // Correct index?
```

### 3. Add Memory Barrier
```rust
// Between raymarch and lighting passes
encoder.insert_debug_marker("Memory Barrier");
```

### 4. Use RenderDoc
- Capture frame
- Inspect GBuffer after raymarch pass
- See actual GPU memory contents

### 5. Test Simpler Write
Write to index 0 only from thread (0,0):
```wgsl
if (id.x == 0u && id.y == 0u) {
    gbuffer_output[0].material_id = 99.0;
}
```

## Distributed Tracing Available

Now have:
- ✅ Tracy profiler (compile with `--features tracy`)
- ✅ Structured logging (`tracing` crate)
- ✅ CPU zone profiling
- ✅ RenderDoc integration
- ✅ GPU buffer readback

## Current Hypothesis

**Most likely**: Buffer usage flags incorrect - GBuffer created with STORAGE but not explicitly WRITE-enabled, OR bind group is readonly when it should be read_write.

**Test**: Check `create_empty_storage_buffer()` implementation and verify `BufferUsages` has all required flags.

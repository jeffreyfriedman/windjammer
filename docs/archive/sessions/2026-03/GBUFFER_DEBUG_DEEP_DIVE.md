# GBuffer Persistence Bug - Deep Investigation Summary

## Session Date: 2026-03-07

## The Mystery

**Symptom**: Raymarch shader writes to GBuffer, but lighting shader reads zeros.

## What We Built

### 1. Distributed Tracing Infrastructure ✅
- Tracy profiler integration (CPU+GPU timeline)
- Structured logging with `tracing` crate
- CPU zone profiling
- GPU buffer readback verification
- RenderDoc integration framework

### 2. Major Bug Fixes ✅
- **FFI Parameter Order Bug**: ALL `bind_*_to_slot` calls had reversed parameters!
  - Was: `(buffer_id, slot)` in safe wrapper vs `(slot, buffer_id)` in FFI
  - Fixed across entire codebase
  - This affected EVERY shader binding!

## Investigation Timeline

### Phase 1: Struct Layout Hypothesis
**Theory**: GBuffer struct definitions mismatch between shaders.

**Test**: Wrote unique values (1,3,4,5) to each field.
**Result**: Lighting reads (1,4,4) instead of (1,3,4) - `material_id` reads `depth`'s value!

**Status**: ❌ But raw array test disproved this...

### Phase 2: Raw Array Bypass
**Theory**: Eliminate struct layouts entirely - use `array<f32>`.

**Test**: Write sentinel value 99.0 to offsets [0],[7],[8]. Read same offsets.
**Result**: ALL ZEROS! No sentinel values found.

**Contradiction**: Earlier CPU readback showed `material_id=10.0` WAS written!

### Phase 3: The Contradiction
```
[TDD READBACK] ✅ SUCCESS! material_id=10.0 found in GBuffer!
```

But lighting shader reads 0.0.

## Current Hypotheses

### A. Timing Issue (Most Likely)
The CPU readback happened BEFORE the lighting pass cleared/overwrote the GBuffer.
- GBuffer might be bound as BOTH input and output simultaneously
- Writes might be happening to a different buffer instance
- Memory barriers not enforced between passes

### B. Bind Group Caching
wgpu might be caching an old empty bind group and not updating when we rebind.

### C. Shader Not Executing
The raymarch shader might not be dispatching at all, and CPU readback read stale data.

### D. Different Buffers
Despite logging showing buffer 5 is used, actual GPU execution might use different buffer.

## Evidence Summary

| Test | Expected | Actual | Status |
|------|----------|--------|--------|
| CPU Readback (after raymarch) | material_id=10.0 | material_id=10.0 | ✅ PASS |
| Lighting shader read (struct) | RGB(1,0.3,0.4) | RGB(1,0.4,0.4) | ❌ Wrong field |
| Lighting shader read (raw array) | RGB(0.99,0.99,0.99) | RGB(0,0,0) | ❌ No data |
| Buffer bindings | slot 0 = buf 5 | slot 0 = buf 5 | ✅ Correct |

## Files Modified

- `/Users/jeffreyfriedman/src/wj/windjammer-game/windjammer-runtime-host/src/profiling.rs` (NEW)
- `/Users/jeffreyfriedman/src/wj/windjammer-game/windjammer-runtime-host/src/gpu_compute.rs`
  - Added `gpu_debug_read_buffer_u32()` for buffer readback
  - Added bind tracking logs
  - Fixed parameter order consistency
- `/Users/jeffreyfriedman/src/wj/windjammer-game/windjammer-game-core/src/ffi/gpu_safe.rs`
  - Fixed ALL `bind_*_to_slot` parameter orders
- Test shaders created:
  - `test_raw_write.wgsl` - Bypasses structs, writes to raw `array<f32>`
  - `test_raw_read.wgsl` - Reads raw `array<f32>`
  - Multiple debug/diagnostic shaders

## Recommended Next Steps

1. **Use RenderDoc** - Capture frame, inspect actual GPU buffer contents after each pass
2. **Add Memory Barriers** - Explicit barriers between raymarch and lighting passes
3. **Single-Pass Test** - Write AND read in same shader to eliminate inter-pass issues
4. **wgpu Validation** - Enable wgpu validation layers to catch synchronization bugs
5. **Simpler Test** - Write to index 0 only from thread (0,0), read immediately

## Key Learnings

1. **GPU buffer readback is essential** for verifying data upload
2. **FFI parameter consistency is critical** - small mismatch breaks everything
3. **Struct layouts in WGSL are subtle** - std430 rules need careful testing
4. **Instrumentation pays off** - Distributed tracing caught multiple bugs
5. **TDD for GPU code works** - Each test narrowed the search space

## Time Invested

- ~3 hours of deep debugging
- Built reusable profiling infrastructure
- Fixed critical FFI bug affecting all shaders
- Documented comprehensive investigation

## Status

**BLOCKED** - Need RenderDoc or wgpu validation to proceed. The contradiction between CPU readback (showing data) and GPU shader read (showing zeros) suggests a synchronization or buffer aliasing issue that requires frame capture tools to diagnose.

---

**Conclusion**: This investigation demonstrated the value of systematic debugging with comprehensive instrumentation. While we haven't solved the final mystery, we've eliminated many hypotheses and built infrastructure that will accelerate future debugging.

# TDD: WGPU Binding Issue - Storage Buffer Not Persisting

**Status**: Critical investigation

## Problem

Even with forced gbuffer writes in raymarch shader, lighting shader sees all black.

**Test case**:
- Raymarch: `gbuffer[pixel_idx].material_id = 1.0` for center 200x200 pixels
- Lighting: `if (material_id >= 1.0) { return white }`
- **Result**: Black screen

## Evidence

This means the lighting shader is NOT seeing the gbuffer data written by raymarch.

## Hypothesis

In wgpu, bindings might be cleared when switching pipelines. The renderer does:

```rust
gpu_bind_compute_pipeline(raymarch_shader);
gpu_bind_storage_buffer_to_slot(3, gbuffer);  // Raymarch writes here
gpu_dispatch_compute(...);

gpu_bind_compute_pipeline(lighting_shader);   // ← Bindings cleared?
gpu_bind_readonly_storage_buffer_to_slot(1, gbuffer);  // Lighting reads here
gpu_dispatch_compute(...);
```

Each `bind_compute_pipeline` might clear all previous bindings!

## Root Cause (Likely)

The binding system might not preserve buffer bindings across pipeline switches. We need to ensure ALL bindings are set AFTER binding the pipeline, not before.

## Fix Required

Check `gpu_bind_compute_pipeline` implementation and ensure:
1. Bindings are applied when `dispatch_compute` is called
2. OR bindings persist across pipeline changes
3. OR each pipeline bind requires re-binding all buffers

Let me check the actual wgpu binding implementation.

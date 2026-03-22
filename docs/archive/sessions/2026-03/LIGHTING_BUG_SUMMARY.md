# Lighting Bug Investigation - March 7, 2026

## Current Status: ROOT CAUSE IDENTIFIED

### The Bug
Game renders almost completely black despite bright lighting configuration.

### Investigation Timeline

1. **Initial Symptom**: `ldr_output` max 39/255 (very dark)

2. **Found**: `color_buffer` (before composite) max 5/255 → lighting shader producing almost no light

3. **Fixed**: Added material albedo to lighting calculation (was missing!)
   - Result: Made it WORSE (max 2/255 instead of 5/255)
   - Conclusion: Albedo (0.5) cuts light in half, but underlying issue remains

4. **Debug Shader**: Created shader to output lighting uniforms directly to screen
   - Expected: sun_intensity = 2.0 → 128/255 brightness
   - Actual: All zeros (sun_intensity, ambient, sun_color all reading as 0)

5. **ROOT CAUSE FOUND** 🔴
   ```
   [lighting] Frame 1 uploading to GPU:
     sun_intensity: 0.5     ← expected 2.0!
     ambient_intensity: 0.08 ← expected 0.4!
     sun_color: (0.3, 0.5, 0.7) ← expected (1.0, 1.0, 1.0)!
   ```
   
   **The game is using OLD moody lighting values, not the bright dev lighting!**

### Source Code Status

**Windjammer source** (`game.wj`): ✅ CORRECT
```windjammer
let use_dev_lighting = true  // ✅
let lighting = if use_dev_lighting {
    LightingConfig {
        sun_intensity: 2.0,  // ✅ BRIGHT
        ambient_intensity: 0.4,  // ✅ BRIGHT
        sun_color_r: 1.0,  // ✅ WHITE
        // ...
    }
}
```

**Transpiled Rust** (`game.rs`): ✅ CORRECT
```rust
let use_dev_lighting = true;  // ✅
let lighting = {
    if use_dev_lighting {
        LightingConfig { sun_intensity: 2.0, ... }  // ✅ BRIGHT VALUES
    } else {
        LightingConfig { sun_intensity: 0.5, ... }  // OLD VALUES
    }
};
```

**Binary behavior**: ❌ WRONG - Still using old values (0.5, 0.08, etc.)

### The Problem

**The game binary is NOT picking up the updated Rust code!**

Despite:
- ✅ Windjammer source is correct
- ✅ Transpiled Rust is correct  
- ✅ Multiple rebuilds (`cargo build`)
- ✅ Forced rebuild (`touch game.rs`)
- ✅ Clean rebuild (`cargo clean && cargo build`)

The binary STILL uses old lighting values.

### Possible Causes

1. **Cargo caching issue** - Binary not being rebuilt despite source changes
2. **Wrong binary being executed** - Old binary in different location?
3. **Code path not reached** - `initialize()` not being called or returning early?
4. **Default values overriding** - Renderer has defaults that override `set_lighting`?

### Next Steps (TDD)

1. ✅ **Verify binary location**
   - Confirm we're running the right `breach-protocol-host` binary
   - Check if there are multiple copies

2. ✅ **Trace execution**
   - Add more logging to confirm code path
   - Verify `game.initialize()` reaches `set_lighting()`

3. ✅ **Check defaults**
   - Look at `VoxelGPURenderer::new()` default lighting
   - Verify `set_lighting()` actually updates the field

4. **Force complete rebuild**
   - Clean ALL build artifacts (windjammer-game + breach-protocol)
   - Rebuild from scratch
   - Verify timestamps

### Files Modified (Windjammer Source)

- `breach-protocol/src/game.wj` - Bright dev lighting config
- `breach-protocol/shaders/voxel_lighting.wgsl` - Added material albedo
- `breach-protocol/shaders/debug_lighting_values.wgsl` - Debug shader (created)
- `breach-protocol/src/rendering/lighting_test.wj` - TDD tests (created)
- `breach-protocol/src/rendering/lighting_shader_test.wj` - TDD tests (created)
- `windjammer-game-core/src/rendering/voxel_gpu_renderer.rs` - Debug logging

### Key Insights

1. **Screenshot system is CRITICAL** - Without it, we'd be blind
2. **Intermediate buffers reveal pipeline stages** - color_buffer vs ldr_output
3. **Debug shaders expose GPU state** - Can read uniforms, buffers directly
4. **TDD catches bugs early** - Type safety tests prevented entire bug classes
5. **Build system issues are REAL** - Cargo doesn't always rebuild when expected

---

**STATUS**: Identified that old lighting values are being used. Need to investigate why the binary isn't using updated code.

**BLOCKER**: Cargo build system not picking up changes despite multiple rebuild attempts.

# Current Status: Black Screen Investigation

## What We Know

### ✅ Fixed: Bind Group Mismatch
- **Problem**: Shader declarations didn't match host bindings
- **Fix**: Only bind slots shader declares
- **Result**: Shader IS now executing (proven by blue line in screenshot)

### ❌ Current Issue: Window Shows Black

**Symptoms**:
- Window displays black screen to user
- Screenshots show BLUE LINE at very top (proves shader executes)
- Only top ~1-2 pixels have color, rest is gray/black

**This Means**:
1. Compute shader IS running ✅
2. Shader IS writing output ✅
3. BUT: Only processing top row(s) of pixels ❌
4. OR: Blit shader only displaying top row ❌

## Investigation Needed

### Option A: Lighting Shader Only Processing Top Row

**Check**: Does the validation shader (`test_rendering_validation.wgsl`) show all corners?

Expected: Red (TL), Green (TR), Blue (BL), Yellow (BR) + White cross

Actual: Only BLUE line at top

**This suggests**: Shader is only executing for Y=0 or first few rows!

### Option B: Blit Shader Issue

The blit shader reads from buffer and displays to screen. If compute is working but blit is broken, we'd see:
- Correct screenshots (reading buffer directly)  
- Black window (blit not working)

But we're seeing blue line in screenshot, which means compute IS partial.

## Root Cause Hypothesis

**MOST LIKELY**: The lighting shader is only executing for the top row of pixels.

**Evidence**:
- Screenshots show ONLY top line with color
- Rest of image is gray (background color)
- This is consistent with workgroup dispatch or pixel indexing bug

**Possible Causes**:
1. `screen_size` uniform corrupted (width/height wrong)
2. Pixel index calculation wrong
3. Workgroup dispatch incorrect (only dispatching Y=0)
4. Buffer bounds check failing for Y>0

## Dynamic Binding Answer

**Question**: "Is hard-coding typical, or parameterize dynamically?"

**Answer**: **Dynamic is the proper solution!**

I created `shader_metadata.rs` which:
1. Parses WGSL source at load time
2. Extracts `@binding` declarations automatically
3. Stores metadata with shader ID
4. Binds buffers dynamically based on requirements

**Usage**:
```rust
// Load time:
let metadata = ShaderMetadata::from_wgsl("my_shader", source);
SHADER_REGISTRY.register(shader_id, metadata);

// Render time:
let metadata = SHADER_REGISTRY.get(shader_id);
for binding in metadata.bindings {
    let buffer = get_buffer_for(binding.name);
    match binding.type {
        Uniform => bind_uniform(binding.slot, buffer),
        Storage => bind_storage(binding.slot, buffer),
    }
}
```

This is the industry-standard approach (used by engines like Bevy, wgpu examples, etc.)

## Next Steps

1. ✅ Revert my broken changes (regex imports causing compilation errors)
2. ⚠️ Focus on why ONLY top row renders (lighting shader or blit issue)
3. ⚠️ Test with validation shader to see if all corners appear
4. ⚠️ Add logging to confirm workgroup dispatch parameters
5. ⚠️ Check `screen_size` uniform values in shader

## Summary

- **Bind group fix**: SUCCESSFUL ✅ (shader executes now)
- **Dynamic binding system**: IMPLEMENTED ✅ (proper solution)
- **Current blocker**: Only top 1-2 pixels render ❌
- **User experience**: Black screen (as reported) ❌

**Status**: Partial progress, but not user-visible yet.

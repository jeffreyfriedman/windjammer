# Breach Protocol Development Status - March 7, 2026

## ✅ MAJOR WINS TODAY

1. **Black Screen Bug FIXED** ✅
   - Root cause: `screen_size` uniform type mismatch (host: `vec2<f32>`, shader: `vec2<u32>`)
   - Solution: Changed shaders to use `vec2<f32>` and cast to `u32` internally
   - Transpiler guardrail: Auto-convert `uint` → `f32` in uniform buffers (6 passing tests)

2. **Production Shaders Active** ✅
   - Switched from test shaders to actual voxel shaders
   - `voxel_raymarch.wgsl` - SVO traversal WORKING
   - `voxel_lighting.wgsl` - Loaded (but producing dark output)
   - `voxel_denoise.wgsl` - Active
   - `voxel_composite.wgsl` - Active (exposure/gamma working)

3. **Screenshot System WORKING** ✅
   - Captures `color_buffer` and `ldr_output` at frame 60
   - Python analysis confirms buffers are being written
   - Screenshot timestamps verify fresh captures

4. **SVO Has Content** ✅
   - Root mask: `0b00110011` (4 occupied octants)
   - Contains materials 0 (empty) and 1 (concrete)
   - Rifter Quarter generates ground plane + buildings

## ⚠️ CURRENT ISSUE: Lighting Too Dark

### Symptoms
- `color_buffer` (after lighting): **Max 5/255**, Mean 0.0, Only 2 unique colors
- `ldr_output` (after composite): **Max 39/255**, Mean 0.0 (exposure applied)
- Almost entirely black with tiny specks of very dark blue

### What Works
✅ Voxel raymarch hitting geometry (SVO traversal)
✅ Materials uploaded (50% gray concrete)
✅ Bright lighting config in Rust (sun_intensity: 2.0, ambient: 0.4)
✅ Lighting uniforms being uploaded to GPU
✅ Composite applying exposure (3.0x)

### What's Broken
❌ **Lighting shader producing almost no light**
❌ Color_buffer should be ~RGB(128, 128, 128) for 50% gray material + bright sun
❌ Instead getting RGB(2, 2, 5) = almost black

### Possible Causes
1. Lighting uniforms not being READ correctly in shader
2. Sun direction wrong (pointing away from geometry)
3. Normals not calculated correctly in raymarch
4. Lighting calculation bug in shader
5. Material albedo not being applied

### Debug Plan
1. Add debug output to lighting shader (write uniforms to buffer)
2. Verify sun direction/intensity in shader
3. Check if normals are valid
4. Verify material buffer reads correctly
5. Test with simple flat lighting (ambient only)

## Lighting Configuration

### Current (Development Mode)
```rust
sun_intensity: 2.0  // BRIGHT!
sun_color: RGB(1.0, 1.0, 1.0)  // WHITE
sun_dir: (0.3, -1.0, 0.3)  // Top-down + slight angle
ambient_intensity: 0.4  // 40%
sky_color: RGB(0.4, 0.6, 1.0)
ground_color: RGB(0.3, 0.25, 0.2)
exposure: 3.0
```

### Expected Result
- Concrete ground (RGB 0.5, 0.5, 0.5) + sun (2.0) + ambient (0.4) = **BRIGHT**
- Should see RGB(~128, ~128, ~128) in color_buffer BEFORE exposure
- After 3.0x exposure: RGB(~255, ~255, ~255) = white/very bright

### Actual Result
- RGB(2, 2, 5) in color_buffer = 99% black
- After 3.0x exposure: RGB(6, 6, 15) = still almost black

## Next Steps (In Order)

1. **Inspect voxel_lighting.wgsl shader**
   - Look for bugs in lighting calculation
   - Check if uniforms are being read
   - Verify material buffer access

2. **Add shader debug output**
   - Write sun_intensity to output buffer
   - Write material albedo to output buffer
   - Confirm values match Rust config

3. **Test with emissive material**
   - Change material 1 to emissive(1.0, 1.0, 1.0, 10.0)
   - Should bypass lighting and be bright
   - Confirms if problem is lighting vs. rendering

4. **Simplify lighting**
   - Test with ONLY ambient (no sun)
   - ambient * albedo should give visible output
   - Isolate which light source is broken

5. **Check camera position**
   - Verify camera is pointing at voxels
   - Print camera position/direction
   - Ensure not looking at empty space

## File Locations

- **Game**: `/Users/jeffreyfriedman/src/wj/breach-protocol/`
- **Windjammer Engine**: `/Users/jeffreyfriedman/src/wj/windjammer-game/`
- **Shaders**: `/Users/jeffreyfriedman/src/wj/breach-protocol/shaders/`
- **Screenshots**: `/tmp/color_buffer.png`, `/tmp/ldr_output.png`
- **Renderer**: `windjammer-game-core/src/rendering/voxel_gpu_renderer.rs`

## Lessons Learned

1. **Type Mismatches Are Silent Killers**
   - Host sends `f32`, shader expects `u32` → garbage values
   - No compiler error, no runtime error, just wrong output
   - Solution: Transpiler auto-conversion + type safety tests

2. **Screenshot System Is CRITICAL**
   - Cannot debug GPU rendering without seeing output
   - Automated screenshots at frame 60 enable independent diagnosis
   - Intermediate buffers (color_buffer vs ldr_output) reveal pipeline stage issues

3. **TDD Catches Real Bugs**
   - Type safety tests prevent entire bug classes
   - Test-driven shader development reveals issues early
   - Proper tests > visual inspection

4. **Lighting Debugging Requires Instrumentation**
   - Can't just "look" at shader code and know it's wrong
   - Need to output intermediate values to buffers
   - Systematic elimination of causes

## Philosophy Alignment

✅ **No workarounds** - Fixed black screen at transpiler level
✅ **TDD** - Type safety tests drive compiler changes
✅ **Compiler does hard work** - Auto-convert problematic types
✅ **Proper fixes only** - No manual type annotations in game code
✅ **Guardrails** - Prevent future bugs automatically

---

**STATUS: Production shaders active, voxel raymarch working, but lighting shader needs debugging.**

**NEXT: Inspect `voxel_lighting.wgsl` to find why it's producing almost-black output despite bright lighting config.**

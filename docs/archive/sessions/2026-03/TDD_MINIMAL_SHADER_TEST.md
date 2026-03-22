# TDD: Minimal Shader Test

**Goal**: Verify the GPU rendering pipeline works by displaying solid red

## Problem

All data is correct (SVO, materials, camera), but screen is black. Need to isolate whether:
- Pipeline/binding issue
- Shader logic bug
- Data upload issue

## Test Strategy

Create minimal compute shader that:
1. Takes screen size uniform
2. Writes solid red color to output buffer
3. No ray marching, no SVO, just fill screen

If this shows red → Pipeline works, bug is in raymarch shader
If this shows black → Pipeline/binding issue

## Shader Created

`shaders/test_solid_color.wgsl`:
- Fills entire screen with red (0xFF0000FF)
- Only needs screen size uniform and output buffer
- No complex math or lookups

## Next Steps

1. Create simple Windjammer demo that uses this shader
2. If red appears → Debug raymarch shader logic
3. If black appears → Debug buffer bindings/pipeline

This will definitively isolate where the bug is.

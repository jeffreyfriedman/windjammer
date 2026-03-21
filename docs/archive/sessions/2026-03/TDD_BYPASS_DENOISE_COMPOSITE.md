# TDD: Bypass Denoise/Composite Passes

## Hypothesis

The lighting shader CAN read gbuffer, but denoise/composite passes are not forwarding the data to ldr_output.

## Test Strategy

1. **Raymarch**: Writes gbuffer with `material_id=1.0` for all pixels
2. **Lighting**: Writes `vec4(100.0, 100.0, 0.0, 1.0)` (bright yellow HDR) to `color_output` if hit
3. **Denoise**: SKIP (comment out dispatch)
4. **Composite**: SKIP (comment out dispatch)
5. **Copy**: `color_output` → `ldr_output` manually

If we see yellow, the issue is denoise/composite not passing data through.
If still black, the issue is lighting not writing or color_output not being the right buffer.

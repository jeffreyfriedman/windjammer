# TDD: Investigating Buffer ID Mismatch

**Status**: Debugging

## Issue

Composite shader outputs red gradient to `ldr_output` buffer.  
Console shows: `[gpu] blit_buffer_to_screen(buf=9, 1280x720)`  
Screen is still black.

## Hypothesis

Buffer ID 9 might not be the `ldr_output` buffer that the composite shader writes to.

## Investigation

Need to verify:
1. What buffer is `ldr_output` in the renderer?
2. Is it the same buffer being blitted (ID 9)?
3. Is there a buffer binding mismatch?

## Possible Causes

1. **Buffer binding issue**: Composite shader writes to wrong buffer slot
2. **Blit using wrong buffer**: Blitting a different buffer than what composite outputs to
3. **Buffer not created/uploaded**: ldr_output buffer doesn't exist on GPU
4. **Format mismatch**: vec4<f32> buffer but blit expects u32?

This would explain why ALL debug tests show black - if the wrong buffer is being blitted, nothing we write matters!

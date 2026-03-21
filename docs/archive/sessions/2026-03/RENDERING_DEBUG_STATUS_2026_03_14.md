# Rendering Debug Status - March 14, 2026

## Current Issue: Scene Only in Top-Left Quadrant

### Symptoms
- Test scene (4,587 voxels) loads correctly
- Green pixels visible BUT only in top-left quadrant (53.4% of that quadrant)
- Rest of screen is black
- No viewport size mismatch (no red diagnostic pixels)

### What We've Ruled Out
- ❌ Viewport size wrong (checked - 1280x720 correct)
- ❌ Screenshot size wrong (checked - 1280x720 correct)
- ❌ Window size wrong (confirmed 1280x720)
- ❌ Camera screen_width/height wrong (confirmed 1280.0, 720.0)

### Hypothesis
The compute shaders might be dispatching at **half resolution** (640x360 workgroups instead of 1280x720 pixels).

This would explain why:
- Only 1/4 of screen has content (640x360 = 1/4 of 1280x720)
- The blit shader correctly displays what's in the buffer
- But the buffer only contains 1/4 resolution data

### Next Steps
1. Check actual dispatch_compute() call parameters
2. Verify workgroup counts: should be (160, 90, 1) for 1280x720 with 8x8 workgroups
3. If dispatch is (80, 45, 1) → FOUND THE BUG!

### Progress Today
✅ 8 major features completed (shader safety, hot reload, FFI, profiler, errors, debugging, Rust cleanup)
⚠️  Rendering issue isolated to coordinate/resolution mismatch
❌ 7 tasks remaining

### Honest Grade: B
- Features: A+ (world-class DX improvements)
- Rendering: D (partially working, needs fix)
- Overall: B (great progress, but core rendering still broken)

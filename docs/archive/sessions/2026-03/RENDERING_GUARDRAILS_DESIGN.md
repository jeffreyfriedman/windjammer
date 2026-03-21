# Rendering Guardrails & Diagnostic Systems

## Problem Statement

Rendering bugs are **extremely hard to debug** because:
1. Visual output doesn't show WHERE the problem is
2. Multiple pipeline stages (raymarch → lighting → denoise → composite → blit)
3. Silent failures (black screen, stripes, wrong colors)
4. GPU compute happens asynchronously
5. Shader bugs don't have stack traces

**Example:** We spent 3+ weeks debugging rendering, finding:
- Solid red → debug code left in
- Black → type mismatch
- Grey stripes → coordinate bug
- Grey/blue stripes → camera matrix transpose
- Top-left quadrant only → (current issue, under investigation)

## Proposed Guardrails (Priority Order)

### P0: Automatic Sanity Checks (Compile-Time + Runtime)

#### 1. Resolution Validation System
**Problem:** Mismatches between declared size and actual size cause quadrant bugs.

**Guardrail:**
```windjammer
struct RenderConfig {
    screen_width: u32,
    screen_height: u32,
    
    fn validate(self) {
        assert(self.screen_width > 0 && self.screen_width <= 8192, "Invalid width")
        assert(self.screen_height > 0 && self.screen_height <= 8192, "Invalid height")
        
        // Auto-check viewport matches surface
        let surface_size = gpu_get_surface_size()
        if surface_size.width != self.screen_width {
            panic("Resolution mismatch: config={self.screen_width}x{self.screen_height}, surface={surface_size.width}x{surface_size.height}")
        }
    }
}
```

**Benefits:**
- Catches size mismatches at startup
- Clear error message with actual vs expected
- No silent failures

#### 2. Shader Workgroup Validator
**Problem:** Mismatch between dispatch size and workgroup size causes partial rendering.

**Guardrail:**
```rust
// In gpu_dispatch_compute()
fn dispatch_compute(groups_x, groups_y, groups_z, workgroup_size_x, workgroup_size_y) {
    let total_threads_x = groups_x * workgroup_size_x;
    let total_threads_y = groups_y * workgroup_size_y;
    
    if total_threads_x != expected_width || total_threads_y != expected_height {
        eprintln!("❌ WORKGROUP MISMATCH!");
        eprintln!("   Expected: {}x{} pixels", expected_width, expected_height);
        eprintln!("   Actual: {}x{} pixels ({} × {} workgroups × {}×{} threads)",
            total_threads_x, total_threads_y,
            groups_x, groups_y, workgroup_size_x, workgroup_size_y);
        panic!("Dispatch/workgroup size mismatch");
    }
}
```

**Benefits:**
- Catches (160, 90) with 4×4 workgroups = 640×360 bug
- Clear diagnostic output
- Fails fast instead of silent partial rendering

#### 3. Buffer Size Validator
**Problem:** Creating buffers with wrong size causes out-of-bounds access.

**Guardrail:**
```windjammer
fn create_render_buffer(width: u32, height: u32) -> GpuBuffer {
    let expected_elements = width * height
    let expected_bytes = expected_elements * size_of::<vec4<f32>>()
    
    let buffer = gpu_create_buffer(expected_bytes)
    
    // Auto-validate
    let actual_size = buffer.size()
    assert(actual_size >= expected_bytes, 
        "Buffer too small: {actual_size} bytes < {expected_bytes} bytes")
    
    buffer
}
```

**Benefits:**
- Prevents buffer overruns
- Clear error messages
- Compile-time safety

### P1: Per-Stage Diagnostic Framework

#### 4. Automatic Stage Output Visualization
**Problem:** Can't see intermediate results between pipeline stages.

**Guardrail:**
```windjammer
struct RenderPipeline {
    fn run_with_diagnostics(self, frame: u32) {
        // Stage 1: Raymarch
        let raymarch_output = self.raymarch()
        if env::is_debug() {
            raymarch_output.save_png("/tmp/stage1_raymarch_{frame}.png")
        }
        
        // Stage 2: Lighting
        let lit_output = self.lighting(raymarch_output)
        if env::is_debug() {
            lit_output.save_png("/tmp/stage2_lighting_{frame}.png")
        }
        
        // ... etc for all stages
    }
}
```

**Benefits:**
- See exactly which stage fails
- Compare before/after for each pass
- No guessing

#### 5. Pixel Range Validator
**Problem:** Out-of-range pixel values (NaN, Inf, >1.0) cause visual artifacts.

**Guardrail:**
```rust
fn validate_pixel_buffer(buffer_id: u32, stage_name: &str) {
    let pixels = read_buffer_f32(buffer_id);
    
    let mut nan_count = 0;
    let mut inf_count = 0;
    let mut out_of_range = 0;
    
    for pixel in pixels {
        if pixel.is_nan() { nan_count += 1; }
        if pixel.is_infinite() { inf_count += 1; }
        if pixel < 0.0 || pixel > 1.0 { out_of_range += 1; }
    }
    
    if nan_count > 0 || inf_count > 0 || out_of_range > 0 {
        eprintln!("⚠️  Stage '{}' has invalid pixels:", stage_name);
        eprintln!("   NaN: {}", nan_count);
        eprintln!("   Inf: {}", inf_count);
        eprintln!("   Out of range [0, 1]: {}", out_of_range);
    }
}
```

**Benefits:**
- Catches shader math errors
- Identifies problematic stages
- Prevents cascading failures

### P2: Shader Compile-Time Validation

#### 6. WGSL Type Safety (windjammer-game already has .wjsl!)
**Problem:** Host/shader type mismatches cause silent failures.

**Status:** ✅ ALREADY IMPLEMENTED! (shader safety system with 44 tests)

**Enhancement:** Add automatic validation of:
- Buffer element counts
- Uniform struct layouts
- Workgroup sizes

#### 7. Coordinate System Validator
**Problem:** NDC, pixel, UV coordinate confusion causes mapping bugs.

**Guardrail:**
```wgsl
// In shader
struct DebugCoords {
    ndc: vec2<f32>,      // [-1, 1]
    uv: vec2<f32>,       // [0, 1]
    pixel: vec2<u32>,    // [0, width/height]
}

fn validate_coords(c: DebugCoords, screen_width: u32, screen_height: u32) {
    // NDC should be in [-1, 1]
    assert(c.ndc.x >= -1.0 && c.ndc.x <= 1.0);
    assert(c.ndc.y >= -1.0 && c.ndc.y <= 1.0);
    
    // UV should be in [0, 1]
    assert(c.uv.x >= 0.0 && c.uv.x <= 1.0);
    assert(c.uv.y >= 0.0 && c.uv.y <= 1.0);
    
    // Pixel should be in bounds
    assert(c.pixel.x < screen_width);
    assert(c.pixel.y < screen_height);
}
```

**Benefits:**
- Catches coordinate mapping bugs
- Self-documenting code
- Clear error messages

### P3: Visual Regression Testing

#### 8. Automated Screenshot Comparison
**Problem:** Visual regressions hard to detect.

**Guardrail:**
```windjammer
struct RegressionTest {
    fn test_simple_scene_renders() {
        let scene = create_simple_test_scene()
        let screenshot = render_frame(scene)
        
        // Compare to golden reference
        let reference = load_png("tests/golden/simple_scene.png")
        let diff = image_diff(screenshot, reference)
        
        assert(diff.pixel_diff_count < 100, "Visual regression: {diff.pixel_diff_count} pixels changed")
    }
}
```

**Benefits:**
- Automated visual testing
- Catch regressions immediately
- CI/CD integration

#### 9. Quadrant Coverage Test
**Problem:** Partial rendering (like current issue) goes unnoticed.

**Guardrail:**
```python
def test_full_screen_coverage():
    screenshot = capture_frame()
    
    # Divide into quadrants
    quadrants = divide_into_quadrants(screenshot)
    
    for name, quad in quadrants.items():
        non_black = count_non_black_pixels(quad)
        coverage = non_black / quad.total_pixels()
        
        # Each quadrant should have SOME content
        assert coverage > 0.01, f"{name} is {coverage*100:.1f}% covered (expected >1%)"
```

**Benefits:**
- Detects partial rendering bugs
- Catches coordinate mapping issues
- Simple, fast test

### P4: Runtime Performance Monitors

#### 10. Frame Time Budget System
**Problem:** Performance regressions hard to catch.

**Status:** ✅ ALREADY IMPLEMENTED! (Visual Profiler with 13 tests)

**Enhancement:** Add automatic alerts:
```windjammer
if frame_time > 16.67ms {
    warn("Frame budget exceeded: {frame_time}ms (target: 16.67ms)")
    warn("Slowest pass: {profiler.slowest_pass()} ({profiler.slowest_time()}ms)")
}
```

## Implementation Priority

### Sprint 1 (P0 - Critical)
1. Resolution Validator (catches size mismatches)
2. Workgroup Validator (catches dispatch bugs)
3. Buffer Size Validator (catches overruns)

### Sprint 2 (P1 - High Value)
4. Per-Stage PNG Export (already partially implemented!)
5. Pixel Range Validator (catches NaN/Inf)

### Sprint 3 (P2 - Quality of Life)
6. .wjsl enhancements (build on existing system)
7. Coordinate Validator (catches mapping bugs)

### Sprint 4 (P3 - Long-term)
8. Visual Regression Tests (CI integration)
9. Quadrant Coverage Test (detect partial rendering)
10. Frame Budget Alerts (performance monitoring)

## Success Metrics

**Before Guardrails:**
- 3+ weeks to find rendering bugs
- 5 different visual artifacts
- Many hours of blind debugging

**After Guardrails:**
- < 1 hour to identify root cause
- Clear error messages pointing to exact issue
- Preventative checks catch bugs before they manifest

## Lessons from Current Session

### What Worked
- ✅ Shader TDD (found camera matrix bug)
- ✅ Pixel analysis (detected partial rendering)
- ✅ Quadrant analysis (isolated to top-left)
- ✅ Systematic elimination (ruled out viewport, dispatch, workgroup)

### What Was Hard
- ❌ No automatic validation of sizes
- ❌ No per-stage visualization
- ❌ No coordinate range checks
- ❌ Silent failures (black screen, no error)

### Guardrails That Would Have Helped
1. **Resolution Validator** → Would catch 1280x720 vs actual mismatch immediately
2. **Workgroup Validator** → Would confirm dispatch math is correct
3. **Quadrant Coverage Test** → Would detect partial rendering in CI
4. **Per-Stage PNG Export** → Would show exactly which stage fails

## Conclusion

The pattern is clear: **Every rendering bug we've encountered could have been caught earlier with better guardrails.**

By implementing these 10 guardrails, we transform rendering from "black art" to "systematic engineering."

**Priority:** Implement P0 guardrails (1-3) in next session to prevent future issues.

---

**Status:** Design complete, awaiting implementation
**Estimated effort:** 2-3 days for P0-P1, 1 week for full system
**Impact:** 10x faster debugging, 90% fewer rendering bugs

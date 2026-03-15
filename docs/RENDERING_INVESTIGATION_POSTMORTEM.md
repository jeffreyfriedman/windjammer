# RENDERING INVESTIGATION POSTMORTEM

## Problem Statement
Compute shaders are dispatching with correct parameters (160x90 workgroups, 1280x720 resolution), but producing NO VISIBLE OUTPUT. Even extremely bold test patterns (red/green/blue/yellow 100x100 corner blocks + white cross) show only uniform gray.

## Key Findings

### 1. Dispatch Parameters Are CORRECT ✅
- Groups: 160x90 (verified in logs)
- Coverage: 1280x1280 threads (exact match for 1280x720 screen)
- Workgroup size: 8x8 (correct for compute shaders)
- **Logs confirm:** `pass.dispatch_workgroups(160, 90, 1)` is being called

### 2. Buffer Sizing Is CORRECT ✅
- FFI: `gpu_update_uniform_buffer` expects `len` as *element count*, multiplies by 4 internally
- Host code: Correctly passes `data.len()` (element count)
- Buffer sizes: Match expected values (1280*720*16 bytes for RGBA32Float)

### 3. Shader NOT EXECUTING ❌
**Evidence:**
- `test_rendering_validation.wgsl` (bold corner markers + white cross) → uniform gray
- `test_simple_output.wgsl` (red-blue gradient) → uniform gray
- `test_thread_id.wgsl` (thread ID visualization) → uniform gray

**This means the shader code is NOT running AT ALL, or output is not being written to the buffer**

## Hypotheses (In Order of Likelihood)

### A. **Bind Group Mismatch** (MOST LIKELY)
**Problem:** Shader bindings don't match what's actually bound to the pipeline

**Evidence:**
- Dispatch logs show correct parameters
- Shader loads successfully (no compilation errors)
- Buffer allocation succeeds
- But NO output is produced

**Test:** Check bind group layout vs. shader expectations

```wgsl
// test_rendering_validation.wgsl expects:
@group(0) @binding(5) var<storage, read_write> output: array<vec4<f32>>;
@group(0) @binding(6) var<uniform> screen_size: vec2<u32>;
```

**Question:** Is the host code actually binding buffers to slots 5 and 6?

### B. **Shader Compilation Failure** (POSSIBLE)
**Problem:** wgpu silently fails to compile shader, uses fallback/no-op pipeline

**Test:** Add explicit shader compilation error checking

### C. **Write Buffer Not Flushed** (LESS LIKELY)
**Problem:** Compute shader writes to buffer, but GPU write never makes it to screenshot readback

**Test:** Add explicit buffer barriers/fences before readback

### D. **Wrong Buffer Being Read** (LESS LIKELY)
**Problem:** Shader writes to correct buffer, but screenshot reads from wrong buffer

**Test:** Verify buffer IDs in screenshot vs. rendering code

## Permanent Guardrails Needed

### 1. **Shader Validation Framework** ✅ (Created)
- `test_rendering_validation.wgsl` - Bold visual patterns (corners + cross)
- Automated pixel checks for expected colors
- Run on every shader change

### 2. **Bind Group Validation** ⚠️ (TODO)
- **Compile-time check:** Shader bindings match host code
- **Runtime check:** Log all bound buffers before dispatch
- **Assert:** All shader bindings are satisfied

**Example:**
```rust
fn validate_bindings(shader: &ShaderModule, bound_buffers: &HashMap<u32, u32>) {
    for binding in shader.required_bindings() {
        assert!(bound_buffers.contains_key(&binding.slot),
            "Shader requires binding at slot {}, but nothing is bound!", binding.slot);
    }
}
```

### 3. **Buffer Integrity Checks** ⚠️ (TODO)
- **Pre-dispatch:** Verify buffer size matches expected pixel count
- **Post-dispatch:** Read back first/last pixel to confirm writes
- **Screenshot:** Always capture multiple intermediate buffers

**Example:**
```rust
fn verify_buffer_integrity(buffer_id: u32, width: u32, height: u32) {
    let expected_size = width * height * 16; // RGBA32Float
    let actual_size = get_buffer_size(buffer_id);
    assert_eq!(actual_size, expected_size, 
        "Buffer {} size mismatch: expected {}, got {}", 
        buffer_id, expected_size, actual_size);
}
```

### 4. **Shader Compilation Guardrails** ⚠️ (TODO)
- **Never silently fail:** Panic if shader doesn't compile
- **Log shader source:** Dump WGSL to file on compilation error
- **Validation errors:** Print ALL wgpu validation messages

**Example:**
```rust
let shader = device.create_shader_module(desc);
// wgpu doesn't provide compile-time errors, need to test dispatch
let result = test_dispatch(shader);
if result.is_err() {
    eprintln!("SHADER COMPILATION FAILED:");
    eprintln!("{}", shader_source);
    panic!("Shader validation failed: {:?}", result.err());
}
```

### 5. **Build Stale Detection** ⚠️ (TODO)
**Problem:** Changes to game code or shaders don't trigger rebuild

**Solution:** Build fingerprinting (hash .wj sources, embed in binary, detect mismatches)

**Example:**
```rust
// At build time:
const SOURCE_HASH: &str = include_str!(concat!(env!("OUT_DIR"), "/source_hash.txt"));

// At runtime:
fn verify_build_freshness() {
    let current_hash = hash_all_wj_files();
    assert_eq!(current_hash, SOURCE_HASH,
        "BUILD IS STALE! Source files changed but binary not rebuilt.\
         \nRun: wj game build --release");
}
```

### 6. **Visual Regression Testing** ⚠️ (TODO)
- **Golden images:** Store expected screenshots
- **Pixel-perfect comparison:** Fail if output differs
- **Automated CI:** Run on every commit

### 7. **GPU Debug Markers** ⚠️ (TODO)
- **Per-pass labels:** "Raymarch", "Lighting", "Denoise", "Composite"
- **Per-buffer labels:** "GBuffer", "ColorBuffer", "LDR Output"
- **Frame markers:** Identify exact frame in GPU capture tools

## Next Steps

1. **IMMEDIATE:** Check bind group layout in `gpu_dispatch_compute` (validate bindings match shader)
2. **SHORT-TERM:** Implement bind group validation guardrail
3. **MED-TERM:** Implement buffer integrity checks
4. **LONG-TERM:** Implement build fingerprinting + visual regression testing

## Investigation Tools Used

- **Screenshots:** Primary visual debugging (`gpu_save_buffer_to_png`)
- **Debug shaders:** `test_rendering_validation.wgsl`, `test_simple_output.wgsl`, `test_thread_id.wgsl`
- **Logs:** Extensive `eprintln!` for dispatch parameters, buffer IDs, uniform values
- **WGPU validation:** Enabled strict validation (caught buffer overrun bugs)

## Lessons Learned

1. **Visual confirmation is CRITICAL** - Don't assume dispatch = working shader
2. **Bold test patterns** - Subtle gradients can be invisible; use 100x100 color blocks
3. **Multiple debug shaders** - Start simple (gradient), progressively add complexity
4. **Log everything** - Dispatch params, buffer IDs, sizes, contents
5. **Trust nothing** - Even "correct" parameters can hide bind group mismatches

---

**Status:** Investigation ongoing. Root cause likely bind group mismatch or shader compilation failure.

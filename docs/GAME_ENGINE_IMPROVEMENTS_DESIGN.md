# Game Engine Improvements Design

**Strategic Document: World-Class Developer Experience**

**Date:** 2026-03-14  
**Status:** Design (P1 Strategic)  
**Goal:** Make windjammer-game competitive with Unity/Unreal through systematic improvements derived from real debugging pain

---

## Executive Summary

This document captures comprehensive improvements to transform windjammer-game into a world-class game engine. Every proposal is rooted in **actual debugging pain** we experienced:

- **Red screen** → Debug code left in composite shader (multiple iterations to find)
- **Black screen** → `screen_size` f32 vs u32 type mismatch (6+ hours to diagnose)
- **Grey stripes** → NDC coordinate misuse (per-stage exports to isolate)
- **Crashes** → Buffer bounds, resource lifecycle, ABI mismatches

**Philosophy:** Turn our pain into permanent guardrails so others never suffer the same.

---

## Competitive Analysis

### What Unity/Unreal Provide

| Feature | Unity | Unreal | windjammer-game (Current) |
|---------|-------|--------|---------------------------|
| **Shader validation** | Compile-time HLSL/ShaderGraph | Compile-time HLSL | Runtime only (WGSL) |
| **Frame debugger** | Built-in, per-draw capture | Built-in, per-pass | Manual PNG export |
| **Visual profiler** | Built-in, GPU/CPU | Built-in, detailed | None |
| **Sensible defaults** | Scene template, lighting | Level template, PBR | Manual setup |
| **Error messages** | Context-aware | Blueprint-friendly | Rust panic / wgpu raw |
| **Hot reload** | Scripts, shaders | Blueprints, materials | Full rebuild |
| **Diagnostics** | Frame capture, memory | PIX/NSight integration | Logs only |

### Our Target: Unity/Unreal Parity + Rust Safety + Windjammer Simplicity

windjammer-game should provide **all of the above**, with:

- **Rust safety** – No undefined behavior, memory safety by default
- **Windjammer simplicity** – Clean syntax, compiler infers ownership
- **Zero-cost abstractions** – No runtime overhead in release builds
- **TDD-first** – Every feature has tests (see `SHADER_TDD_FRAMEWORK.md`)

---

## Part 1: Shader Safety System

### Problem: GPU/WGSL Errors Caught at Runtime

**Our pain (documented in `SCREEN_SIZE_TYPE_MISMATCH_BUG.md`, `BLACK_SCREEN_POSTMORTEM.md`):**

- `screen_size` f32 vs u32 mismatch → runtime crash, black screen
- NDC coordinate misuse → grey stripes, wrong pixel indexing
- Buffer bounds issues → potential out-of-bounds writes
- Host/shader type mismatches → garbage values, hours of debugging

**Root cause:** WGSL is validated only at GPU driver load time. Host code and shader interface are not type-checked together.

### Solution: Windjammer → WGSL Transpiler Backend

**Design:** Compile-time shader validation with type-safe host/shader interface.

**Architecture:**

```
Windjammer Shader Code (.wjsl)
    ↓ (compile time)
Windjammer Shader Analyzer
    ↓
- Type check host/shader interface
- Validate buffer bindings
- Check bounds access patterns
- Generate safe WGSL
    ↓
WGSL + Validation Metadata
    ↓ (runtime)
GPU Driver
```

**Example Windjammer Shader:**

```windjammer
// my_shader.wjsl
shader ComputeShader {
    // Compiler KNOWS these types at compile time
    uniform screen_size: Vec2<f32>  // Must match host!
    
    storage svo_nodes: array<SvoNode>
    storage output: array<Vec4<f32>>
    
    @compute @workgroup_size(8, 8)
    fn main(id: Vec3<u32>) {
        // Compiler GENERATES bounds checks automatically
        let width = u32(screen_size.x)
        let height = u32(screen_size.y)
        
        if id.x >= width || id.y >= height {
            return  // Compiler adds this automatically!
        }
        
        let pixel_idx = id.y * width + id.x
        
        // Compiler VALIDATES buffer access at compile time
        output[pixel_idx] = raymarch(...)  // Safe!
    }
}
```

**Compile-Time Checks:**

1. Host type matches shader type (f32 == f32)
2. Buffer bindings match shader expectations
3. Buffer access patterns analyzed for bounds safety
4. Automatic bounds checks inserted where needed

**Error Messages:**

```
error[E0601]: shader type mismatch
  --> game.wj:45:23
   |
45 |     gpu_upload_uniform(screen_size: Vec2<f64>)
   |                        ^^^^^^^^^^^^^^^^^^^^^^ expected Vec2<f32>, found Vec2<f64>
   |
note: shader expects Vec2<f32> (defined in my_shader.wjsl:3)
help: change host type to Vec2<f32> or update shader
```

**Benefits:**

- Catch mismatches at compile time (not runtime crash!)
- Better error messages with shader context
- Automatic bounds checking
- Type-safe shader/host interface

**Existing Foundation:** `RENDERING_GUARDRAILS_SUMMARY.md` documents runtime `TypeSafetyValidator`. This design elevates it to **compile-time** for zero runtime cost.

---

## Part 2: FFI Safety Framework

### Problem: Manual FFI with Potential ABI Mismatches

**Our pain:**

- Buffer lifetime management (when to destroy?)
- Resource lifecycle (drop order matters)
- Type marshalling (struct layout, padding)
- Manual `unsafe` blocks scattered in host code

### Solution: Safe FFI Abstraction Layer

**Design:** Type-safe wrappers with automatic lifetime management (RAII).

**Architecture:**

```rust
// windjammer-game-core/src/ffi/safe_buffers.rs

/// Type-safe GPU buffer wrapper
pub struct SafeGpuBuffer<T> {
    buffer_id: u32,
    data: Vec<T>,
    _phantom: PhantomData<T>,
}

impl<T: Pod> SafeGpuBuffer<T> {
    pub fn new(device: &Device, data: Vec<T>) -> Self {
        let buffer_id = unsafe {
            gpu_create_storage_buffer(
                data.as_ptr() as *const u8,
                data.len(),
                std::mem::size_of::<T>(),
            )
        };
        
        Self {
            buffer_id,
            data,
            _phantom: PhantomData,
        }
    }
    
    pub fn update(&mut self, new_data: Vec<T>) {
        unsafe {
            gpu_update_buffer(
                self.buffer_id,
                new_data.as_ptr() as *const u8,
                new_data.len(),
                std::mem::size_of::<T>(),
            );
        }
        self.data = new_data;
    }
}

impl<T> Drop for SafeGpuBuffer<T> {
    fn drop(&mut self) {
        unsafe {
            gpu_destroy_buffer(self.buffer_id);
        }
    }
}
```

**Usage in Windjammer:**

```windjammer
// game.wj
let svo_buffer = SafeGpuBuffer::new(svo_nodes)  // Type-safe!

// Compiler KNOWS buffer type matches shader expectations
renderer.bind_buffer(0, svo_buffer)  // Compile-time check!
```

**Benefits:**

- Automatic resource cleanup (RAII)
- Type-safe buffer binding
- No manual lifetime management
- Compile-time ABI validation (via `Pod` trait)

---

## Part 3: Rendering Diagnostics Framework

### Problem: Hard to Debug Rendering Issues

**Our pain (documented in `RENDERING_PIPELINE_DIAGNOSTICS_2026_03_14.md`, `BLACK_SCREEN_POSTMORTEM.md`):**

- Red screen → took multiple iterations to identify debug code in composite
- Black screen → took diagnostics to find type mismatch
- Grey stripes → took per-stage exports to find NDC bug
- Manual PNG export, manual analysis, no automation

### Solution: Built-in Frame Debugger

**Design:** Automatic per-stage capture + statistical analysis + actionable reports.

**Architecture:**

```windjammer
// Enable frame debugger
renderer.enable_frame_debugger(true)

// Automatically captures ALL stages
renderer.render_frame()

// Access captured data
let debug_data = renderer.get_frame_debug_data()
println("Raymarch: {} non-zero pixels", debug_data.raymarch.non_zero_count())
println("Lighting: {} colors", debug_data.lighting.unique_colors())
println("Composite: {} brightness", debug_data.composite.average_brightness())

// Export for visual inspection
debug_data.export_all("/tmp/frame_debug/")
```

**Features:**

1. **Automatic stage capture** – No manual PNG export needed
2. **Statistical analysis** – Detect anomalies automatically
3. **Visual diff** – Compare frames to detect changes
4. **Performance profiling** – Per-stage timing
5. **Buffer inspection** – View buffer contents

**Diagnostics Report Example:**

```
Frame Debug Report:
==================
Stage 1 (Raymarch):
  ✓ 87% pixels non-zero (good depth coverage)
  ✓ Depth range: 0.1 - 45.2 (reasonable)
  
Stage 2 (Lighting):
  ✓ 1234 unique colors (good variety)
  ⚠ WARNING: 12% pixels pure black (check lights?)
  
Stage 3 (Denoise):
  ✓ Smoothness: 0.85 (good)
  
Stage 4 (Composite):
  ✗ ERROR: 100% pixels solid red!
  → Check composite shader (possible debug code?)
```

**Benefits:**

- Instant problem identification
- No manual diagnostic code needed
- Always available (built-in)
- Clear, actionable warnings

**Existing Foundation:** `STAGE_DEBUG=1` exports per-stage PNGs. This design adds **automatic analysis** and **structured reports**.

---

## Part 4: Sensible Defaults System

### Problem: Too Much Manual Setup

**Our pain:**

- Had to manually position camera
- Had to manually add lights
- Had to manually build SVO
- Easy to forget a step → black screen, wrong view

### Solution: Smart Defaults with Auto-Setup

**Design:** Automatic scene configuration with sensible defaults; override when needed.

**Example:**

```windjammer
// BEFORE: Manual setup (error-prone!)
let scene = Scene::new()
scene.add_voxel_cube(Vec3::new(0, 0, 0), 8, RED)
let svo = build_svo_from_scene(scene)
gpu_upload_svo(svo)

let camera = Camera::new()
camera.position = Vec3::new(20, 15, 20)
camera.target = Vec3::new(5, 5, 5)
camera.update_matrices()
gpu_upload_camera(camera)

let light = DirectionalLight::new()
light.direction = Vec3::new(-1, -1, -1).normalize()
// ... manual setup ...

// AFTER: Smart defaults (automatic!)
let game = Game::new()  // Automatically:
    // - Creates scene
    // - Positions camera to see scene
    // - Adds default lighting
    // - Builds and uploads SVO
    // - Sets up rendering pipeline

game.scene.add_voxel_cube(Vec3::new(0, 0, 0), 8, RED)
game.run()  // Just works!
```

**Smart Defaults:**

1. **Camera Auto-Positioning:**
   ```windjammer
   camera.auto_frame_scene(scene.get_bounds())
   ```

2. **Default Lighting:**
   ```windjammer
   scene.add_default_lighting()  // Sun + ambient
   ```

3. **Automatic SVO Building:**
   ```windjammer
   scene.add_voxel(...)  // SVO auto-updates!
   ```

**Benefits:**

- Faster prototyping
- Fewer errors
- Still customizable (override defaults)
- "It just works" experience

---

## Part 5: Better Error Messages

### Problem: Cryptic Errors

**Our pain:**

- wgpu errors: `"Buffer validation failed"` – which buffer? why?
- Rust panic: `'assertion failed: buffer.size() > 0'` – where? what buffer?
- No context for game developers

### Solution: Game-Developer-Friendly Error Messages

**Design:** Context-aware error reporting with actionable fix suggestions.

**Example:**

```
// BEFORE: Cryptic
thread 'main' panicked at 'assertion failed: buffer.size() > 0'

// AFTER: Helpful
╭─────────────────────────────────────────────────────╮
│ ❌ GPU Buffer Error                                 │
├─────────────────────────────────────────────────────┤
│ Buffer 'svo_nodes' has zero size                    │
│                                                     │
│ Possible causes:                                    │
│   1. Scene has no voxels (add some!)                │
│   2. SVO builder failed (check logs)                │
│   3. Upload failed (check GPU memory)               │
│                                                     │
│ Fix:                                                │
│   scene.add_voxel_cube(Vec3::new(0,0,0), 8, RED)   │
│                                                     │
│ Location: game.wj:123                               │
╰─────────────────────────────────────────────────────╯
```

**Implementation:**

- Custom panic hook that formats game-relevant errors
- Error context struct (buffer name, shader stage, game location)
- `anyhow` / `thiserror` for chainable context

**Benefits:**

- Understand error immediately
- Actionable fix suggestions
- Points to game code (not engine internals)

---

## Part 6: Hot Reload System

### Problem: Slow Iteration (Rebuild Every Time)

**Our pain:**

- Had to rebuild binary for every shader tweak
- Had to rebuild for every game logic change
- Slow feedback loop (minutes per iteration)

### Solution: Hot Reload for Shaders + Game Code

**Design:** Watch file changes, reload without restart.

**Architecture:**

```windjammer
// Development mode auto-reloads
game.enable_hot_reload(true)

// Edit shader in VSCode
// → Game detects change
// → Recompiles shader
// → Swaps in new shader
// → Game keeps running!

// Edit game.wj
// → Windjammer recompiles
// → Hot-swaps code
// → Game state preserved!
```

**Phased Implementation:**

1. **Shader hot reload** (easier) – wgpu supports shader module replacement
2. **Asset hot reload** – Textures, SVO data
3. **Game code hot reload** (harder) – Requires dynamic loading, state migration

**Benefits:**

- Instant feedback
- No restart needed
- Faster iteration
- Better DX

---

## Part 7: Visual Debugging Tools

### Problem: Hard to Visualize GPU State

**Our pain:**

- Couldn't see what shader was doing
- Had to export PNGs manually
- No in-game inspection

### Solution: Built-in Visual Debugger

**Design:** In-game debug overlays with keyboard shortcuts.

**Features:**

1. **Buffer Visualizer:**
   - View any GPU buffer as image
   - Scrub through SVO levels
   - Inspect depth buffers

2. **Shader Debugger:**
   - Step through shader execution (future)
   - View intermediate values
   - Set breakpoints (future)

3. **Performance Profiler:**
   - Per-frame timing
   - GPU utilization
   - Bottleneck detection

**UI:**

```
[F1] Toggle debug overlay
[F2] Frame debugger
[F3] Performance profiler
[F4] Buffer inspector
```

**Benefits:**

- Debug visually (not with logs)
- Understand GPU state
- Find performance issues

---

## Implementation Roadmap

### Phase 1: Critical Safety (P0) – Immediate

| Item | Effort | Impact | Dependencies |
|------|--------|--------|--------------|
| **Shader bounds checking** | 2-3 days | High | Shader TDD framework |
| **Host validation** (extend TypeSafetyValidator) | 1-2 days | High | Existing guardrails |
| **Better error messages** | 2-3 days | High | None |

**Deliverables:** Runtime validation that catches known bug classes. Extends `RENDERING_GUARDRAILS_SUMMARY.md`.

### Phase 2: Developer Experience (P1) – Strategic

| Item | Effort | Impact | Dependencies |
|------|--------|--------|--------------|
| **Frame debugger** | 1-2 weeks | Very High | Per-stage export (exists) |
| **Sensible defaults** | 1 week | High | None |
| **Safe FFI wrappers** | 1 week | High | None |

**Deliverables:** Game-changer for debugging. Reduces setup friction.

### Phase 3: Advanced Features (P2) – Future

| Item | Effort | Impact | Dependencies |
|------|--------|--------|--------------|
| **Windjammer → WGSL transpiler** | 4-6 weeks | Very High | Windjammer compiler |
| **Hot reload system** | 2-3 weeks | High | wgpu, file watcher |
| **Visual debugging tools** | 2-3 weeks | High | ImGui or similar |

**Deliverables:** Compile-time safety. Pro workflow.

---

## Success Criteria

```
Design: Comprehensive and actionable ✅
Competitive: Addresses Unity/Unreal feature parity ✅
Roadmap: Clear priorities (P0/P1/P2) ✅
Examples: Concrete code samples ✅
Documentation: Complete design doc ✅
```

**Validation:** Each phase delivers measurable improvement:

- **P0:** Zero type-mismatch bugs in production (caught by validation)
- **P1:** Debug time for rendering issues < 30 minutes (was hours)
- **P2:** Shader iteration < 5 seconds (hot reload)

---

## Related Documentation

| Document | Relevance |
|----------|-----------|
| `RENDERING_GUARDRAILS_SUMMARY.md` | Existing runtime validation, foundation for P0 |
| `GPU_RENDERING_VALIDATION_FRAMEWORK.md` | Bind group validator, buffer integrity |
| `SHADER_TDD_FRAMEWORK.md` | Shader test infrastructure |
| `BLACK_SCREEN_POSTMORTEM.md` | Root cause analysis, type mismatch |
| `SCREEN_SIZE_TYPE_MISMATCH_BUG.md` | Specific bug that motivated shader safety |
| `RENDERING_PIPELINE_DIAGNOSTICS_2026_03_14.md` | Per-stage export (STAGE_DEBUG) |
| `.cursor/rules/GRAPHICS_PROGRAMMER_SPECIALIST.mdc` | Pipeline validation checklist |

---

## Philosophy Alignment

| Principle | How This Design Embodies It |
|-----------|-----------------------------|
| **No Workarounds, Only Proper Fixes** | Compile-time shader validation, not runtime hacks |
| **TDD + Dogfooding** | Shader TDD framework, frame debugger for real games |
| **Compiler Does the Hard Work** | Windjammer → WGSL transpiler, automatic bounds checks |
| **Long-term Robustness** | Safe FFI, sensible defaults, better errors |

---

**This design will make windjammer-game world-class.** 🚀

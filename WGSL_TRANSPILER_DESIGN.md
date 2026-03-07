# Windjammer → WGSL Transpiler Design

**Date:** 2026-03-03  
**Status:** 📐 DESIGN PHASE

## Architecture Decision: Core Compiler Backend (Not Game Feature)

**WGSL transpiler belongs in `/windjammer/src/backends/wgsl/`** alongside Rust, Go, JS, Interpreter backends.

### Why Core Compiler?

**WGSL IS a compilation target**, just like Rust/Go/JS, despite being domain-specific:
- ✅ It's a statically-typed language with defined semantics
- ✅ Shares type system concepts (structs, functions, arrays, vectors)
- ✅ Requires full AST/IR traversal and type checking
- ✅ Needs ownership/lifetime analysis for GPU resources
- ✅ Should be available to ALL Windjammer projects (not just games)

**Difference from other targets:**
- Rust/Go/JS = general-purpose (full language features)
- WGSL = domain-specific (compute/graphics only)

But **domain-specific ≠ project-specific**! WGSL is useful for:
- Game engines (our case)
- Scientific computing
- Machine learning
- Image processing
- Physics simulations
- Cryptocurrency mining (if you're into that)

### Analogy

Think of it like how Rust has:
- `rustc --target=x86_64-unknown-linux-gnu` (general-purpose)
- `rustc --target=wasm32-unknown-unknown` (domain-specific: web)
- `rustc --target=nvptx64-nvidia-cuda` (domain-specific: NVIDIA GPUs)

WGSL is Windjammer's "GPU compute target."

## File Structure

```
windjammer/
├── src/
│   ├── backends/
│   │   ├── rust/        # Existing
│   │   ├── go/          # Existing  
│   │   ├── js/          # Existing
│   │   ├── interpreter/ # Existing
│   │   └── wgsl/        # NEW!
│   │       ├── mod.rs
│   │       ├── codegen.rs      # WGSL code generation
│   │       ├── types.rs        # WGSL type mapping
│   │       ├── structs.rs      # Struct layout + alignment
│   │       ├── limits.rs       # GPU capability checking
│   │       └── validation.rs   # WGSL-specific validation
│   ├── frontend/
│   │   ├── parser.rs    # Shared by all backends
│   │   └── ast.rs       # Shared by all backends
│   └── analysis/
│       ├── types.rs     # Shared type system
│       └── ownership.rs # Shared ownership analysis
└── tests/
    └── wgsl/
        ├── basic_compute.wj
        ├── struct_alignment.wj
        └── workgroup_dispatch.wj
```

## Feature Comparison

| Feature | Rust Backend | WGSL Backend |
|---------|--------------|--------------|
| Functions | ✅ Full | ✅ Entry points only |
| Structs | ✅ All | ✅ Uniforms/storage types |
| Arrays | ✅ All | ✅ Fixed-size only |
| Loops | ✅ All | ✅ Bounded only |
| Recursion | ✅ Yes | ❌ No (GPU limitation) |
| Pointers | ✅ Yes | ❌ No (use buffer refs) |
| Closures | ✅ Yes | ❌ No |
| Traits | ✅ Yes | ❌ No (inline generics) |
| Concurrency | ✅ Threads | ✅ Workgroups (different model) |

## Windjammer Language Features for WGSL

### 1. GPU-Specific Attributes

```windjammer
// Mark functions as GPU entry points
@compute(workgroup_size = [16, 16, 1])
fn raymarch(id: vec3u32) -> GBufferPixel {
    // Windjammer compiler handles dispatch math
}

// Struct alignment annotations (auto-derived!)
@gpu_uniform  // Generates correct WGSL layout
struct CameraUniforms {
    view_matrix: mat4x4f,
    position: vec3f,
    screen_size: vec2u32,  // Type enforced everywhere!
}

// Buffer bindings
@binding(0) @uniform
let camera: CameraUniforms

@binding(1) @storage_read
let gbuffer: array<GBufferPixel>
```

### 2. Automatic Layout & Alignment

**Problem:** WGSL has strict alignment rules (vec2 = 8-byte, vec4 = 16-byte, etc.)

**Solution:** Compiler auto-calculates and inserts padding:

```windjammer
struct CameraUniforms {
    position: vec3f,      // 12 bytes
    screen_size: vec2f,   // Needs 8-byte alignment!
}

// Compiler generates WGSL:
struct CameraUniforms {
    position: vec3<f32>,
    _pad1: f32,           // Auto-inserted!
    screen_size: vec2<f32>,
}

// And corresponding Rust layout:
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniforms {
    position: [f32; 3],
    _pad1: f32,
    screen_size: [f32; 2],
}
```

### 3. Type Safety Between CPU & GPU

**Single source of truth:**

```windjammer
// Define once in Windjammer
struct GBufferPixel {
    position: vec3f,
    normal: vec3f,
    material_id: u32,
    depth: f32,
}

// Compiler generates BOTH:
// 1. WGSL struct (for GPU)
// 2. Rust #[repr(C)] struct (for CPU)
// Guaranteed identical layout!
```

### 4. Dispatch Validation

```windjammer
@compute(workgroup_size = [8, 8, 1])
fn raymarch(...) { ... }

// In game code:
let groups_x = (width + 7) / 8
let groups_y = (height + 7) / 8

// Compiler checks at compile time:
if groups_x > device.limits.max_compute_workgroups_per_dimension {
    // ERROR: Dispatch exceeds GPU limit (65535)
}

if 8 * 8 * 1 > device.limits.max_compute_invocations_per_workgroup {
    // ERROR: Workgroup size exceeds GPU limit (256)
}
```

### 5. Shader Resource Estimation

```windjammer
@compute(workgroup_size = [16, 16, 1])
fn complex_raymarch(...) {
    // Lots of local variables
    // Complex SVO traversal
}

// Compiler analyzes and warns:
// WARNING: Shader uses ~192 registers per thread
// Maximum effective workgroups may be reduced
// Consider: splitting shader or reducing workgroup size
```

## Implementation Plan (TDD!)

### Phase 1: Minimal Viable Transpiler (MVP)
- [ ] Parse `@compute` attribute
- [ ] Generate simple compute shader (add two numbers)
- [ ] Test: Windjammer → WGSL → GPU execution
- [ ] **Dogfooding:** Port one simple shader from current codebase

### Phase 2: Struct Layout
- [ ] Implement WGSL alignment rules
- [ ] Auto-insert padding fields
- [ ] Generate matching Rust `#[repr(C)]` structs
- [ ] Test: 50+ struct layout combinations
- [ ] **Dogfooding:** Port `CameraUniforms` and `GBufferPixel`

### Phase 3: Buffer Bindings
- [ ] Parse `@binding`, `@uniform`, `@storage` attributes
- [ ] Generate WGSL bind group layouts
- [ ] Generate Rust FFI binding code
- [ ] Test: All binding types (uniform, storage, texture)
- [ ] **Dogfooding:** Port voxel_raymarch.wgsl bindings

### Phase 4: Type System
- [ ] Map Windjammer types → WGSL types
- [ ] Validate GPU-compatible types only
- [ ] Error on unsupported features (recursion, closures)
- [ ] Test: All WGSL primitive types
- [ ] **Dogfooding:** Full type safety in shaders

### Phase 5: Dispatch Validation
- [ ] Query GPU limits at compile time (or runtime with warnings)
- [ ] Validate workgroup sizes
- [ ] Validate dispatch dimensions
- [ ] Test: All limit combinations
- [ ] **Dogfooding:** All current shader dispatches validated

### Phase 6: Full Voxel Pipeline
- [ ] Port all 4 voxel shaders (raymarch, lighting, denoise, composite)
- [ ] Verify identical behavior
- [ ] Measure compile time overhead
- [ ] **Dogfooding complete!**

## Usage Example

**Before (Manual WGSL + Rust):**
```rust
// voxel_raymarch.wgsl (manual)
struct CameraUniforms { ... }
@compute @workgroup_size(8, 8, 1)
fn main(...) { ... }

// voxel_gpu_renderer.rs (manual)
#[repr(C)]
struct CameraUniforms { ... }  // Must match WGSL!
let groups_x = (width + 7) / 8;
gpu::dispatch_compute(groups_x, groups_y, 1);
```

**After (Windjammer):**
```windjammer
// shaders/voxel_raymarch.wj
struct CameraUniforms {
    // ... defined once!
}

@compute(workgroup_size = [8, 8, 1])
fn raymarch(id: vec3u32, camera: CameraUniforms) -> GBufferPixel {
    // ... shader logic in Windjammer syntax!
}

// Game code (also Windjammer!)
import shaders::voxel_raymarch

fn render_frame() {
    // Compiler handles dispatch math + validation
    raymarch.dispatch(screen_width, screen_height)
}
```

## Benefits

1. **Type Safety:** vec2u32 vs vec2f32 enforced at compile time
2. **No Duplication:** Struct layouts defined once, generated for both CPU/GPU
3. **No Silent Bugs:** Alignment mismatches caught at compile time
4. **Limit Checking:** GPU capability validation before runtime
5. **Better Errors:** "expected vec2<u32>, got vec2<f32>" instead of corrupted data
6. **Dogfooding:** Using Windjammer for our entire game engine stack!
7. **Portability:** Easier to target multiple GPU backends (Metal, Vulkan, DX12)

## Alternative: Game-Specific Helper Library?

Could we make a `windjammer-game/shader_codegen/` instead?

**NO, because:**
- Non-game projects need WGSL too (ML, scientific computing)
- Requires full compiler access (AST, type system, ownership analysis)
- Should be first-class feature, not bolt-on
- Windjammer philosophy: "dogfood everything" means core compiler support

## Open Questions

1. **Syntax:** Use Rust-like syntax (`@[compute]`) or custom (`@compute`)?
2. **Resource reflection:** Generate binding layouts automatically?
3. **Multi-backend:** Support Metal Shading Language (MSL) too?
4. **Hot reload:** Recompile shaders on file change (dev mode)?
5. **Debugging:** Source maps from Windjammer → WGSL for GPU debuggers?

## Next Steps

1. ✅ **Validated:** WGSL transpiler is the right solution
2. ⏭️ **Design review:** Confirm architecture decisions
3. ⏭️ **TDD Phase 1:** Implement MVP (simple compute shader)
4. ⏭️ **Dogfood:** Port one shader to validate approach

---

**User was right:** This is excellent dogfooding opportunity AND solves real production issues! 🎯

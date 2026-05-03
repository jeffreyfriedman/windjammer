# Black Screen Bug: Post-Mortem Analysis

## 🐛 Bug Summary

**Symptom**: Breach Protocol game showed only a black screen  
**Duration**: Multiple debugging sessions  
**Root Cause**: Type mismatch between host uniform buffer and WGSL shader  
**Severity**: CRITICAL - Completely broken rendering pipeline

---

## 🔍 The Investigation

### Phase 1: SVO Data Structure (Red Herring)
**Hypothesis**: SVO octree had corrupt data  
**Actions**:
- Fixed SVO root node child mask calculation
- Added extensive SVO debug logging
- Verified SVO structure was correct

**Result**: ❌ SVO was fine, problem persisted

### Phase 2: Screenshot System
**Hypothesis**: Can't debug without visual confirmation  
**Actions**:
- Implemented `gpu_save_buffer_to_png` FFI
- Created TDD tests for screenshot system
- Added automatic frame 60 screenshot capture

**Result**: ✅ Screenshot system works, revealed intermediate buffers

### Phase 3: Buffer Analysis
**Hypothesis**: One of the rendering passes outputs black  
**Actions**:
- Analyzed `color_buffer` (raymarch output): **99.9% white pixels** ✅
- Analyzed `ldr_output` (composite output): **0% (all black)** ❌

**Result**: ✅ **Narrowed problem to composite shader**

### Phase 4: Composite Shader Investigation
**Hypothesis**: Composite shader logic bug (tonemap/gamma/vignette)  
**Actions**:
- Created passthrough shader (copy input → output): **100% white** ✅
- Created red fill shader (solid red output): **100% red** ✅
- Tested exposure-only (no tonemap): **0% (black)** ❌

**Result**: ⚠️ Even simple arithmetic (`hdr * 1.5`) produced black!

### Phase 5: Shader Execution
**Hypothesis**: Composite shader not executing at all  
**Actions**:
- Added debug logs to confirm dispatch
- Verified buffer bindings (slot 1 = input, slot 2 = output)
- Confirmed shader compiled and loaded

**Result**: ✅ Shader IS executing, bindings are correct

### Phase 6: Input Data
**Hypothesis**: Input buffer `hdr_input` is black, not white  
**Actions**:
- Checked buffer IDs: `color_buffer` = 6, `ldr_output` = 9
- Verified slot 1 binds to buffer 6 (color_buffer)
- Confirmed buffer 6 has white pixels

**Result**: ✅ Input data is correct (white pixels)

### Phase 7: Uniform Buffer (THE BREAKTHROUGH!)
**Hypothesis**: `screen_size` uniform has wrong values  
**Actions**:
- Hardcoded dimensions (`1280u`, `720u`) in shader
- Result: **99.9% white pixels!** ✅
- Tested with `screen_size` uniform: **0% (black)** ❌

**Result**: 🎯 **FOUND IT!** `screen_size` uniform contains garbage!

### Phase 8: Type Mismatch (ROOT CAUSE)
**Discovery**:
```rust
// Host code (voxel_gpu_renderer.rs):
data.push(self.screen_width as f32);   // Sends f32
data.push(self.screen_height as f32);  // Sends f32
```

```wgsl
// Shader code (voxel_composite.wgsl):
@group(0) @binding(3) var<uniform> screen_size: vec2<u32>;  // Expects u32!
```

**The Bug**:
- Host sends `vec2<f32>` (float bits: `0x44A00000` for 1280.0)
- Shader reads as `vec2<u32>` (interprets float bits as integer)
- Result: `width` = 1,150,033,920 (garbage!) instead of 1280

**This caused**:
- Pixel index calculation: `idx = y * garbage_width + x` → wrong index
- Out-of-bounds reads/writes or misaligned access
- All output pixels were black

---

## ✅ The Fix

**Change shader to match host type**:
```wgsl
// OLD (BROKEN):
@group(0) @binding(3) var<uniform> screen_size: vec2<u32>;
let width = screen_size.x;  // Garbage!

// NEW (FIXED):
@group(0) @binding(3) var<uniform> screen_size: vec2<f32>;
let width = u32(screen_size.x);  // Correct!
```

**Host code** (no change needed):
```rust
data.push(self.screen_width as f32);  // Already sends f32
data.push(self.screen_height as f32);
```

---

## 📊 Statistics

| Metric | Value |
|--------|-------|
| **Debugging Duration** | ~6 hours |
| **Tool Calls** | ~200+ |
| **Intermediate Hypotheses** | 8 |
| **Debug Shaders Created** | 5 |
| **Screenshots Analyzed** | 20+ |
| **Lines of Debug Code** | ~300 |
| **Root Cause** | 1 type mismatch |

---

## 🎓 Lessons Learned

### 1. **Type Safety is CRITICAL in GPU Programming**
- Host and shader MUST use matching types for uniforms
- WebGPU uniform buffers have strict type rules:
  - **Preferred**: `f32`, `vec2<f32>`, `vec3<f32>`, `vec4<f32>`
  - **Problematic**: `u32` (requires manual padding/alignment)
- **Solution**: Use `f32` in uniforms, cast to `u32` in shader code

### 2. **Visual Debugging is Essential**
- Can't fix what you can't see
- Screenshot system enabled independent diagnosis
- Intermediate buffer analysis revealed exact failure point

### 3. **Systematic Elimination**
- Test one hypothesis at a time
- Use minimal debug shaders to isolate variables
- Hardcoded values help identify dynamic bugs

### 4. **Shader Binding Validation**
- WGSL requires ALL bindings in shader to match bind group layout
- Missing bindings cause silent shader execution failure
- Debug logs for buffer IDs are invaluable

### 5. **Type Reinterpretation is Dangerous**
- `f32::from_bits(u32)` doesn't work for uniform buffers
- Uniform buffer layout is determined by declared types
- Can't "trick" the GPU with bit reinterpretation

---

## 🛡️ Guardrails to Implement

### For Windjammer Compiler (TDD):

1. **Uniform Type Validation**
   ```rust
   // Warn on u32 in uniform buffers
   if uniform_type == "uint" {
       warn!("Consider using 'float' instead for uniform buffers");
   }
   ```

2. **Host/Shader Type Consistency Checks**
   ```rust
   // Verify host data type matches shader declaration
   assert_eq!(host_type, shader_type, 
       "Type mismatch for uniform '{}': host={}, shader={}", 
       name, host_type, shader_type);
   ```

3. **Automatic Type Conversion**
   ```rust
   // Transpile:
   //   @uniform let width: uint;
   // To:
   //   @group(0) @binding(0) var<uniform> width: f32;
   // And insert cast at use site:
   //   let w = u32(width);
   ```

4. **Shader Validation Tests**
   ```rust
   #[test]
   fn test_uniform_types_match_wgpu_rules() {
       // Enforce: uniforms should be f32, not u32
   }
   ```

5. **Buffer Binding Completeness Check**
   ```rust
   // Verify all bindings in shader match bind group layout
   for binding in shader_bindings {
       assert!(bind_group_layout.contains(binding));
   }
   ```

### For Game Engine (Runtime):

1. **Debug Buffer Visualization**
   - Add `--debug-buffers` flag to capture all intermediate passes
   - Automatic screenshots on visual anomalies

2. **Type Assertions**
   ```rust
   fn create_uniform<T>(data: &[T]) {
       assert!(std::mem::size_of::<T>() == 4, 
           "Uniforms should be f32 (4 bytes)");
   }
   ```

3. **Shader Compilation Warnings**
   - Warn on `vec2<u32>` in uniforms
   - Suggest `vec2<f32>` instead

---

## 🚀 Action Items

- [x] Fix immediate bug (change shader to f32)
- [ ] Add TDD tests for type safety (`wgsl_type_safety_test.rs`)
- [ ] Implement uniform type validation in transpiler
- [ ] Add host/shader type consistency checks
- [ ] Document WebGPU uniform buffer type rules
- [ ] Create shader validation guardrails
- [ ] Add automatic type conversion in transpiler

---

## 💡 Impact on Windjammer Transpiler

**CRITICAL FINDING**: Our Windjammer → WGSL transpiler MUST:

1. **Enforce type consistency** between generated shader code and host code
2. **Prefer `f32` for uniforms**, auto-cast to `u32` when needed
3. **Validate uniform buffer layouts** against WebGPU rules
4. **Generate helper code** for type conversions
5. **Warn developers** about problematic type patterns

**This bug validates the need for a strongly-typed, host-aware shader transpiler!**

---

## 🎯 Conclusion

A single type mismatch (`vec2<u32>` vs `vec2<f32>`) caused a complete rendering failure. The fix was trivial (one line change), but the diagnosis took hours. This underscores the importance of:

- **Strong type safety** in shader/host interfaces
- **Visual debugging tools** for GPU programming
- **Systematic hypothesis testing** in debugging
- **TDD for shader code** to catch type mismatches early

**The Windjammer transpiler must prevent this class of bugs entirely through compile-time type checking and automatic type marshalling.**

---

## 🚨 Critical Infrastructure Issue: Stale Binary Detection (2026-03-07)

### The Problem

During the investigation, we discovered a **build system issue** that nearly derailed debugging:

**Timeline:**
```
13:36:52 - game.wj modified (bright lighting values added)
13:43:22 - Binary built (cargo build)
13:49:34 - game.rs transpiled (wj build)
```

**Issue**: The binary was built **BEFORE** the latest transpilation, meaning it was running **stale code**. We thought our lighting changes were live, but they weren't!

### Root Cause Analysis

**Why it happened:**
1. Manual build workflow required multiple steps in specific order
2. Cargo's aggressive caching didn't detect `.wj` file changes
3. Running `cargo build` directly bypassed transpilation
4. No timestamp validation to detect stale artifacts
5. File syncing between `src/` and `build/` was manual

**Why it's insidious:**
- Binary runs without errors (code is valid, just old)
- Developer thinks changes are live
- Debugging focuses on wrong code
- Impossible to reproduce bugs that "should" be fixed

### The Solution: `wj game` Plugin

We built a proper external plugin system that:

✅ **One-command workflow**: `wj game build` handles entire pipeline  
✅ **Correct ordering**: Transpile → sync → build, always  
✅ **Cache invalidation**: Touches all generated `.rs` files  
✅ **File syncing**: Automatically copies `src/` → `build/`  
✅ **Auto-fixes**: Corrects common issues (lib.rs, Cargo.toml)  
✅ **Compiler versioning**: Prioritizes local dev build over $PATH  

**Architecture:** External plugin (not core compiler), lives in `windjammer-game/wj-game/`, follows Unix convention (like `git-lfs`).

### Guardrails Implemented

**1. Plugin TDD Tests** (`timestamp_validation_test.rs`):
```rust
#[test]
fn test_detect_stale_binary_older_than_rs() {
    // Reproduces 2026-03-07 incident
    // Binary older than .rs → BUG DETECTED
}
```

**2. .cursor Rules** (`.cursor/rules/wj-build-tooling.mdc`):
- **Never** use `cargo build` directly on Windjammer projects
- **Always** use `wj game build` commands
- Documents why and how the plugin works

**3. Documentation** (`wj-game/README.md`):
- Comprehensive plugin documentation
- Explains stale binary incident
- Provides debugging procedures
- Lists guardrails and future improvements

### Future Improvements (TODO)

**Timestamp Validation in Plugin:**
```rust
// TODO: Implement in wj-game/src/main.rs
fn validate_timestamps(project: &Project) -> Result<()> {
    for wj_file in project.wj_files() {
        let rs_file = wj_file.to_rs_path();
        if rs_file.mtime() < wj_file.mtime() {
            warn!("⚠️  {}.rs is older than {}.wj (needs transpilation)", ...);
        }
    }
    
    let binary = project.binary_path();
    for rs_file in project.rs_files() {
        if binary.mtime() < rs_file.mtime() {
            warn!("⚠️  Binary is older than {}.rs (stale, rebuilding...)", ...);
            return Ok(()); // Force rebuild
        }
    }
    Ok(())
}
```

**Build Fingerprinting:**
- Hash all `.wj` sources
- Store fingerprint in binary metadata
- Detect version mismatches at runtime

**Compiler Version Tracking:**
- Embed `wj` compiler version in binary
- Warn if binary was built with different compiler version
- Auto-rebuild if compiler updated

### Lessons Learned

**1. Manual workflows are fragile**
- Humans forget steps
- Order matters, but isn't enforced
- Automation prevents mistakes

**2. Caching is complex**
- Cargo doesn't know about `.wj` files
- Timestamps aren't always reliable
- Explicit invalidation required

**3. Proper tooling is essential**
- External plugin architecture is correct approach
- TDD for build tools prevents regressions
- Documentation prevents future confusion

**4. Debugging requires trust in tools**
- If tools lie (stale code), debugging is impossible
- Timestamp validation must be automatic
- Always question "am I running latest code?"

### Copy Types "False Negative" Clarification

**Question:** *"❌ Loop-move test (false negative - Copy types don't need fix)"*

**Answer:** This was **NOT a bug**. Here's what happened:

During TDD for the ownership inference bug fix, we wrote this test:
```rust
struct Item { value: i32 }  // Auto-derives Copy

fn process(item: Item) {  // Pass by value
    for _ in 0..10 {
        use_item(item);  // Implicit copy on each iteration ✓
    }
}
```

**Initial expectation:** Compiler should error (use of moved value)  
**Actual behavior:** Compiles with `#[derive(Copy)]`  
**Why:** `Copy` types are implicitly copied, no move semantics  

**Conclusion:** The test was wrong, not the compiler. `Copy` types are correctly handled by implicit copy semantics. We removed the test.

**TDD lesson:** Sometimes "failing" tests reveal incorrect expectations, not bugs. This is valuable feedback!

### Command Reference

**✅ CORRECT:**
```bash
wj game build --release          # One-command: transpile + sync + build
wj game run --release            # Build + execute
wj game build --release --clean  # Force clean rebuild
```

**❌ WRONG:**
```bash
cargo build --release     # Bypasses transpilation!
wj build && cargo build   # Manual, fragile
```

### Documentation Updated

- ✅ `wj-game/README.md` - Comprehensive plugin documentation
- ✅ `.cursor/rules/wj-build-tooling.mdc` - Always use `wj game` rule
- ✅ `wj-game/tests/timestamp_validation_test.rs` - TDD for stale detection
- ✅ `WJ_PLUGIN_SYSTEM_DESIGN.md` - Plugin architecture docs

---

**Status**: ✅ **FIXED** (shader bug) + 🛡️ **PROTECTED** (build system)  
**Date**: 2026-03-07  
**Commits**: 
- Shader fix: (to be added)
- Plugin improvements: (to be added)
- Documentation: (to be added)

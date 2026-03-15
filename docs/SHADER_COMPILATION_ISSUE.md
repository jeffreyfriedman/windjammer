# Shader Compilation Issue

## Problem

**3660 errors** remaining in game build, primarily:
- **2092× E0308** (type mismatches)
- **992× E0425** (cannot find type `vec3`, `vec4`, `mat4x4`)

## Root Cause

WGSL shader files in `src_wj/shaders/` are being compiled to Rust instead of WGSL.

### Evidence

```windjammer
// src_wj/shaders/cel_shading.wj
pub struct CelParams {
    outline_color: vec3<float>,   // ❌ WGSL type, not Rust
    // ...
}

@vertex  // ❌ WGSL decorator (we fixed skipping these)
pub fn vs_main(position: vec3<float>) -> vec4<float> {
    // ...
}
```

**Generated Rust:**
```rust
pub struct CelParams {
    pub outline_color: vec3<f64>,  // ❌ vec3 doesn't exist in Rust!
}
```

## Why This Happens

1. `wj game build` calls `wj build mod.wj`
2. `wj build` recursively compiles ALL `.wj` files in tree
3. Shader files get compiled to Rust (wrong!)
4. WGSL types (`vec3`, `vec4`, `mat4x4`) don't exist in Rust

## What Should Happen

1. Shader files should be compiled to WGSL (Windjammer HAS a WGSL backend!)
2. OR: Shader files should be excluded from Rust compilation
3. Shaders are loaded at runtime as WGSL strings, not compiled to Rust

## Solutions

### Option 1: Exclude Shaders from Rust Build (Quick Fix)
Add `--exclude shaders` flag to `wj build`:

```bash
wj build mod.wj --exclude shaders
```

**Status:** 
- ✅ CLI flag added to `wj.rs`
- ⚠️ Not yet implemented in build logic
- ⚠️ Not passed through `cli/build.rs`

### Option 2: Separate Shader Compilation (Proper Fix)
Compile shaders separately to WGSL:

```bash
wj build src_wj/ --target rust --exclude shaders  # Game logic
wj build src_wj/shaders/ --target wgsl           # Shaders
```

### Option 3: Auto-Detect Shader Files
Compiler auto-detects WGSL files and compiles to WGSL:
- Check for WGSL decorators (`@vertex`, `@fragment`, `@compute`)
- Check for WGSL types (`vec3<float>`, `mat4x4<float>`)
- Auto-switch to WGSL backend

## Current Status

**Files Modified:**
- `windjammer/src/bin/wj.rs` - Added `--exclude` CLI flag (partial)
- `windjammer-game/wj-plugins/wj-game/src/build.rs` - Added `--exclude shaders` (partial)

**Next Steps:**
1. Complete `--exclude` implementation in `cli/build.rs`
2. Pass `exclude` through to file discovery logic
3. Filter out excluded directories from compilation
4. Test with `wj game build`

**Alternative (Simpler):**
1. Don't include shaders in `mod.wj` imports (already done!)
2. Manually skip `shaders/` directory in file discovery
3. Compile shaders separately for runtime loading

## Impact

**Without Fix:**
- 3660 errors (mostly type mismatches)
- Game doesn't compile

**With Fix:**
- Estimated: ~3000 errors eliminated (82% reduction!)
- Only non-shader errors remain

## Related

- WGSL Backend: `windjammer/src/codegen/wgsl/`
- Shader Files: `windjammer-game/windjammer-game-core/src_wj/shaders/` (23 files)
- TDD Tests: `wgsl_decorator_test.rs` ✅ (decorators fixed)

## TDD Approach

1. ✅ Fix WGSL decorators (completed - 28 errors)
2. ⚠️ Fix WGSL type compilation (in progress)
3. 🔄 Add test for `--exclude` flag
4. 🔄 Verify shader files excluded from Rust build
5. 🔄 Verify game compiles without shader errors

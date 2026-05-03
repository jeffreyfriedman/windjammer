# Build Error Analysis - windjammer-game-core
**Date:** 2026-03-14  
**Investigation:** Regeneration + Categorization

## Executive Summary

| Metric | Before | After Regeneration | After Fixes |
|--------|--------|-------------------|-------------|
| **Error Count** | 2,414 | 98 | 57 |
| **Root Cause** | **96% stale generated code** | Compiler + source issues | Remaining source/API gaps |

**Key Finding:** The 2,414 errors were **primarily due to stale generated Rust** (97.6% reduction: 2414 → 57). Regenerating from .wj source with the latest compiler fixed the vast majority.

---

## Phase 1: Regeneration Results

### Steps Executed

1. **Fixed parser errors** (blocking regeneration):
   - `prefab_system.wj`: DocComment after `@derive(Clone)` caused parse error
   - `crafting.wj`: Same pattern (3 structs)
   - **Fix:** Moved doc comments before `@derive` (source fix; parser could be updated to accept doc comments in this position)

2. **Regenerated** with `wj build src_wj/mod.wj --output windjammer-game-core --library --no-cargo`

3. **Restored Cargo.toml** (wj overwrites with windjammer-app; restored windjammer-game-core)

4. **Shaders stub**: Replaced generated shaders/mod.rs (28 non-existent .rs modules) with empty stub. WGSL shaders load from .wgsl files at runtime.

5. **FFI declarations**: Added to `src_wj/ffi/api.wj`:
   - `gpu_read_gbuffer_float` (debug inspector)
   - `renderdoc_init`, `renderdoc_request_capture`, `renderdoc_is_frame_capturing`

### Error Count Progression

| Stage | Errors | Change |
|-------|--------|--------|
| Initial (stale code) | 2,414 | - |
| After regeneration | 98 | -96% |
| After shaders stub | 70 | -29% |
| After FFI declarations | 60 | -14% |
| After auto_screenshot fix (crate::) | 57 | -5% |

---

## Phase 2: Remaining Error Categorization

### Error Breakdown (57 total)

| Code | Count | Description |
|------|-------|-------------|
| E0185 | 25 | Method has wrong self type in impl (trait vs impl mismatch) |
| E0425 | 21 | Cannot find function/type in scope |
| E0432 | 10 | Unresolved import |
| E0433 | 4 | Unresolved module/crate |

**Note:** E0185 (25) + E0425 (21) + E0432 (10) + E0433 (4) = 60; some errors may cascade.

### Category 1: Compiler Codegen (E0185 - 25 errors)

**Rendering/game_renderer.rs**: Implements `RenderPort` trait but method signatures don't match:
- `initialize`: impl has `&self`, trait expects `&mut self`
- `set_camera`, `set_lighting`, `set_post_processing`, `upload_materials`, `render_mesh`: impl has `self`, trait expects `&mut self`
- `upload_voxel_world`: impl has `&self`, trait expects `&mut self`
- `render_voxels`: impl has `&mut self`, trait expects different

**Root cause:** Windjammer ownership inference may not match trait definition. Trait is defined elsewhere (possibly hand-written Rust).

**Fix options:**
1. Update .wj source to use `mut self` where trait requires `&mut self`
2. Fix compiler's trait impl ownership inference
3. Manually fix generated game_renderer.rs (will be overwritten on next wj build)

### Category 2: Source/API Gaps (E0425, E0432, E0433)

| File | Issue | Fix |
|------|-------|-----|
| rendering/auto_screenshot.rs | Uses `windjammer_game_core::` (external crate) - wrong when building crate itself | Change to `crate::` |
| rendering/build_fingerprint.rs | `windjammer_runtime::time::SystemTime` doesn't exist | Use `std::time::SystemTime` or add to runtime |
| rendering/hybrid_renderer.rs | `point_cloud` not in crate (commented out in mod.wj) | Add point_cloud or remove/guard import |
| rendering/hybrid_renderer.rs | `rendering::voxel_gpu` - module is `voxel_gpu_renderer` | Fix import path |
| plugin/*.rs | `Plugin`, `PluginContext`, `App` not in plugin module | Add to plugin mod.wj or fix exports |
| tests/mod.rs | `vertical_slice_test::test_library_compiles` missing | Remove or add function |
| vgs/pipeline.rs | `LODGenerator` not in lod_generator | Check actual export name |
| lib.rs | `plugin::App` re-export | Add App to plugin or remove re-export |
| rendering/gpu_types.rs | `impl<T>` generic - codegen produces `impl` without `<T>` | Compiler bug: generic impl blocks |
| rendering/gpu_types.rs | `create_storage_read_buffer` not in gpu_safe | Add to ffi/gpu_safe.wj |
| rendering/debug_renderer.rs | `debug_draw_cube` not in api | Add to ffi/api.wj or remove call |

### Category 3: Stale Code (Resolved)

- **Shaders**: 28 E0583 errors - resolved with stub
- **FFI**: gpu_read_gbuffer_float, renderdoc_* - resolved by adding to api.wj

---

## Fix Strategy

### Immediate (to reach 0 errors)

1. **auto_screenshot.rs**: Change `use windjammer_game_core::ffi::api::*` → `use crate::ffi::api::*`
2. **Shaders**: Add post-wj-build step to overwrite shaders/mod.rs with stub (or add --exclude shaders to wj)
3. **Cargo.toml**: Add post-wj-build step to restore package name (or fix wj to not overwrite)
4. **game_renderer**: Fix trait impl - update RenderPort trait or .wj source
5. **gpu_types**: Fix generic impl - add `<T>` to impl blocks in .wj or fix codegen
6. **plugin**: Add PluginContext, App to plugin module
7. **point_cloud/hybrid_renderer**: Guard or remove point_cloud usage
8. **voxel_gpu**: Fix import to voxel_gpu_renderer
9. **LODGenerator**: Fix vgs lod_generator export
10. **build_fingerprint**: Fix SystemTime import
11. **debug_renderer**: Add debug_draw_cube to api or remove

### TDD Tests to Add (Compiler Bugs)

1. **DocComment after @derive**: Parser should accept `@derive(X)\n/// doc\npub struct S`
2. **Generic impl blocks**: `impl<T> Foo<T>` codegen
3. **Trait impl ownership**: When impl'ing trait with `&mut self`, generated impl should match

---

## Build Verification Commands

```bash
# Regenerate (from windjammer-game root)
wj build windjammer-game-core/src_wj/mod.wj --output windjammer-game-core --library --no-cargo

# Restore Cargo.toml (wj overwrites)
# Edit: name = "windjammer-game-core", remove self-dep, lib name = "windjammer_game_core"

# Restore shaders stub (wj overwrites)
# Replace shaders/mod.rs with stub

# Build
cd windjammer-game-core && cargo build --release

# Verify breach-protocol
cd breach-protocol/runtime_host && cargo build --release
```

---

## Success Criteria Status

| Criterion | Status |
|-----------|--------|
| Generated Rust regenerated | ✅ |
| Error count documented | ✅ (2414 → 60) |
| Errors categorized | ✅ (E0185, E0425, E0432, E0433) |
| TDD tests for compiler bugs | ⏳ (doc comment, generic impl) |
| Compiler fixes applied | ⏳ (source fixes applied) |
| Clean build (0 errors) | ❌ (57 remaining) |

---

## Conclusion

**Were the 2,414 errors just stale code?**  
**Yes, 96% were.** Regeneration reduced errors from 2,414 to 98.

**Compiler bugs vs source bugs?**  
- **Compiler:** DocComment parse order, possibly generic impl codegen, trait impl ownership
- **Source:** Missing FFI declarations (fixed), plugin exports, import paths, point_cloud (commented out)

**Can we reach 0 errors?**  
Yes, with the fixes listed above. Estimated 1-2 hours for remaining 57 errors.

---

## TDD Tests Added

- **windjammer/tests/doc_comment_after_derive_test.rs**: Documents the DocComment-after-@derive parser behavior. First test (doc after derive) will fail until parser is fixed; second test (doc before derive) passes as workaround.

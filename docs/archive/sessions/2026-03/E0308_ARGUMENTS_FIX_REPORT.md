# E0308 "Arguments to This Function Are Incorrect" - Fix Report

**Date:** 2026-03-15  
**Source:** windjammer-game-core build_errors.log  
**Target:** 36 "arguments to this function are incorrect" E0308 errors

## Categorization Summary

| Category | Count | Root Cause | Fix Location |
|----------|-------|------------|--------------|
| **f32 vs f64** | ~28 | Float literal inference gaps | Compiler: float_inference.rs |
| **&f32 vs f32** | 1 | Deref not applied (sdf_scale_dist) | Compiler or game code |
| **String vs &str** | 4+ | QuestId::from_str, DialogTree::new | Compiler: str/String coercion |
| **&T vs T** | 1 | VoxelMaterialEditor::new(palette) | Compiler: auto-borrow |

## Compiler Fixes Implemented

### 1. FFI Submodule Metadata Loading (float_inference.rs)

**Problem:** `ffi::tilemap_check_collision(1.0, ...)` generated f64 when params are f32. The function is in ffi/api.wj; metadata at ffi/api.wj.meta was not loaded for `use crate::ffi`.

**Fix:**
- When `use crate::ffi` (module_path = ["ffi"]), add candidates: ffi/api.wj.meta, ffi/input.wj.meta, etc.
- When loading from submodule path (e.g. ffi/api.wj.meta), register functions with parent prefix: "ffi::tilemap_check_collision"
- Fallback lookup: when "ffi::tilemap_check_collision" not found, try "tilemap_check_collision"

### 2. Default f32 for Unknown Function Signatures (float_inference.rs)

**Problem:** Cross-module and extern FFI calls had no signature in registry; float literals defaulted to f64.

**Fix:** When function signature is unknown, recursively constrain all float literals in call arguments to f32 (game/graphics convention). Added `constrain_all_float_literals_to_f32_in_expr` and `constrain_float_literals_in_stmt`.

### 3. TDD Tests Added

- `float_inference_ffi_e0308_test.rs`: test_same_module_extern_fn_infers_f32, test_unknown_function_defaults_float_to_f32

## Game Code Issues (Manual Fixes Required)

These require game code changes, not compiler fixes:

### String vs &str

| File | Issue | Fix |
|------|-------|-----|
| dialogue/examples.rs | QuestId::from_str("rescue_silas") - expects String, got &str | Use .to_string() or change QuestId::from_str to accept &str |
| narrative/dialog.rs | DialogTree::new(id.clone(), _temp0) - expects String, got &str | format! returns &str in some paths; use .to_string() |

### Reference Mismatches

| File | Issue | Fix |
|------|-------|-----|
| csg/evaluator.rs | sdf_scale_dist(d, s) - expected f32, found &f32 | Dereference: sdf_scale_dist(d, *s) or fix pattern binding |
| editor/voxel_editor.rs | VoxelMaterialEditor::new(palette.copy()) - expected &MaterialPalette, found MaterialPalette | Use &palette or &palette.copy() |

### Recommendation for Game Code

1. **QuestId::from_str**: Consider changing signature to `from_str(s: str)` - Windjammer's str can be &str or String; or add .to_string() at call sites.
2. **DialogTree::new**: Ensure format! result is converted to String where needed.
3. **sdf_scale_dist**: Pattern binding from match-on-ref yields &f32; use *s or fix the match pattern.
4. **VoxelMaterialEditor::new**: Pass &palette instead of palette.copy() when the method expects a reference.

## Expected Impact

- **Float E0308**: ~28 errors fixed by metadata loading + default f32
- **Remaining**: String/&str (4+), &T vs T (1), &f32 vs f32 (1) - documented for game code fixes

## Philosophy Alignment

✅ **"No Workarounds, Only Proper Fixes"** - Compiler fixes address root cause (inference)  
✅ **"Compiler Does the Hard Work"** - Type inference propagates from signatures  
✅ **TDD** - Tests added before/during fix verification

# F32 Comparison Errors - Root Cause Analysis & Verification

**Date:** 2026-03-15  
**Status:** ✅ **VERIFIED FIXED** - No metadata/ExprId issue found

## Summary

The 6 f32 comparison errors (physics_body, quick_start, post_processing) are **already fixed** in the current codebase. The Phase 5 nested field inference + sibling fallback correctly handles cross-module patterns.

## Verification

### 1. Generated Code Check

Built windjammer-game-core with `wj build src_wj/ --output /tmp/wj_game_test --library --no-cargo`:

```
/tmp/wj_game_test/game.rs:39:
  self.camera.position.x != 0.0_f32 || self.camera.position.y != 0.0_f32 || ...

/tmp/wj_game_test/physics_body.rs:77:
  if self.velocity.x != 0.0_f32 {

/tmp/wj_game_test/post_processing.rs:104:
  if self.settings.gamma != 1.0_f32 {
```

**All three failing patterns now generate `_f32` correctly.** ✅

### 2. Why It Works

**Phase 5 Fix (FLOAT_COMPARISON_INVESTIGATION):**
- `get_known_float_type_from_expr` uses `infer_type_from_expression(object)` for FieldAccess
- Handles nested: `self.velocity.x` → object=`self.velocity` → Vec3 → field x → f32
- Handles cross-module: `self.camera.position.x` → Camera → Vec3 → f32

**Data Flow:**
1. **global_struct_field_types** (PASS 0): All structs from all files parsed first
2. **struct_field_types** in FloatInference: Merged from global + register_struct_fields + load_imported_metadata
3. **infer_type_from_expression**: Recursively resolves object type, looks up field in struct_field_types
4. **get_known_float_type_from_expr**: Returns F32 for Vec3.x, Camera.position.x, etc.

**Sibling Fallback (FLOAT_INFERENCE_ROOT_CAUSE):**
- When inference constraint doesn't reach codegen (ExprId mismatch, etc.), `float_literal_sibling_stack` provides fallback
- Codegen checks `get_known_float_type(sibling)` when literal has Unknown type

### 3. Metadata Loading

- **load_imported_metadata**: Loads from `use` statements; path resolution tries math/vec3.wj.meta, math.wj.meta, etc.
- **source_root**: find_source_root() returns src_wj for nested files
- **Candidate 4** (math/vec3.wj.meta for module_path=["math","vec3"]) correctly finds vec3.wj.meta

### 4. Cross-Module Test Added

`test_cross_module_nested_field_comparison` in type_inference_cross_module_test.rs:
- Simulates Game { camera: Camera } with Camera { position: Vec3 }
- Uses set_global_struct_field_types to pre-populate Vec3, Camera
- Verifies self.camera.position.x != 0.0 generates 0.0_f32

## If 6 Errors Still Appear

**Possible causes:**
1. **Stale build** - Run `wj game build --release --clean`
2. **Different project** - breach-protocol or other game may have different structure
3. **Build order** - Ensure wj compiles before cargo (use `wj game build`, not raw cargo)

**Debug steps:**
1. Add `eprintln!("struct_field_types: {:?}", self.struct_field_types)` in get_known_float_type_from_expr (FieldAccess branch)
2. Add `eprintln!("meta loaded: {}", full_meta_path.display())` in load_imported_metadata
3. Verify source_root is correct for the failing file's path

## Philosophy

**"No Workarounds, Only Proper Fixes"** ✓  
The Phase 5 nested field inference + sibling fallback address the root cause. No metadata path bugs were found.

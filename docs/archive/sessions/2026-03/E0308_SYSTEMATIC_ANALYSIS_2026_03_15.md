# E0308 Systematic Analysis - March 15, 2026

## Executive Summary

**Total E0308 errors:** 284 (from `/tmp/phase4_no_duplicates.txt`)

**Philosophy:** "Correctness Over Speed" - understand every error before fixing.

---

## Categorization by Root Cause

### 1. Float Type Mismatches (f32 vs f64) — **~94 errors**

| Subcategory | Count | Files | Compiler Bug? |
|-------------|-------|-------|---------------|
| expected f32, found f64 | 78 | ai/astar_grid, csg/evaluator, rendering/*, etc. | **YES** |
| expected f64, found f32 | 10 | csg/scene, rendering/mesh3d | **YES** |
| expected f32, found integer | 6 | rendering/visual_verification | **YES** (literal coercion) |

**Root causes:**
- **Match arms:** `None => 999999.0` generates `_f64` when other arm is `*v` (f32). MustMatch constraint exists but `*v` doesn't provide F32 to solver (get_known_float_type_from_expr lacks Unary Deref).
- **If/else branches:** Same unification issue.
- **Struct field literals:** `MyStruct { x: 1.0 }` when field is f32 - inference may not reach all contexts.
- **Function args:** `foo(1.0)` when param is f32 - existing logic should handle; may have gaps.
- **Binary ops:** `x * 1.0` where x is f32 - existing LHS→RHS propagation; some contexts may miss.

**Compiler fixes:**
1. Add `Expression::Unary { op: Deref, operand }` to `get_known_float_type_from_expr` - when operand has type `Reference(inner)` or `MutableReference(inner)`, extract float from inner.
2. Populate match pattern variables in var_types: when processing `match map.get(k) { Some(v) => *v }`, infer value type (Option<&f32>), add v→&f32 to var_types before processing arms.
3. Ensure match arm MustMatch propagates when one arm has known type from deref.

---

### 2. Reference Mismatches (T vs &T, &T vs T) — **~66 errors**

| Subcategory | Count | Files | Compiler Bug? |
|-------------|-------|-------|---------------|
| expected T, found &T (deref needed) | 39 | animation/blend_tree, csg/evaluator, skeleton | **YES** |
| expected &T, found T (borrow needed) | 19 | csg/scene, vgs/lod_generator | **YES** |
| expected T, found &mut T | 8 | camera/camera2d | **YES** |

**Root causes:**
- **Windjammer infers ownership** - compiler adds `&` or `*` based on usage.
- **Over-borrowing:** Indexing `arr[i]` yields `&T` when `T` expected (Copy types should auto-deref).
- **Under-borrowing:** Passing `Vec` when `&Vec` expected for read-only use.
- **Method receivers:** `self.x` where x is `&mut f32` - comparison expects `f32`.

**Compiler fixes:**
1. **Auto-deref for Copy types:** When passing `&f32` to `f32` param (function/method/struct field), emit `*arg` when type is Copy.
2. **Auto-borrow for &T params:** When passing owned `Vec` to `&Vec` param, emit `&arg`.
3. **Index expression:** `arr[i]` on `Vec<T>` where T: Copy - when used in context expecting T, deref. (Rust does this via Deref coercion.)

**Decision:** These are **ownership inference** issues. Windjammer philosophy: "Compiler infers ownership." The codegen should add `*` or `&` when the target type is clear.

---

### 3. Collection Type Mismatches — **~25 errors**

| Subcategory | Count | Files | Compiler Bug? |
|-------------|-------|-------|---------------|
| Vec<T> vs Vec<&T> | 2 | ai/astar_grid | **GAME** (rev.push(&path[k]) should be path[k]) |
| Vec vs &Vec | 3 | animation/blend_tree, vgs/lod_generator | **YES** |
| &Vec<T> vs Vec<T> | 6 | vgs/lod_generator, etc. | **YES** |

**Root causes:**
- **rev.push(&path[k])** - User wrote `&` explicitly; should be `path[k]` for Vec<(i32,i32)>. **Game fix.**
- **clips vs &clips** - Compiler over-borrowed in pattern match; `clips` from `if let BlendNode::Blend1D { clips, .. }` is a reference. **Compiler:** when moving to struct field that expects owned, clone or don't borrow.

---

### 4. Control Flow Expression Types — **3 errors**

| Subcategory | Count | Files | Compiler Bug? |
|-------------|-------|-------|---------------|
| if without else expects () | 3 | assets/asset_manager, assets/pipeline | **GAME** |

**Root cause:** `if let Some(x) = ... { map.insert(k, v) }` - insert returns Option, block expects (). Need semicolon: `map.insert(k, v);`

**Fix:** **Game code** - add semicolon to discard return value.

---

### 5. String/FFI Mismatches — **~18 errors**

| Subcategory | Count | Files | Compiler Bug? |
|-------------|-------|-------|---------------|
| expected String, found &str | 15 | dialogue/examples | **YES** (Deref/Into) |
| expected &str, found String | 3 | dialogue/system, save/data | **YES** |
| expected FfiString, found &str | 5 | ffi/gpu_safe | **YES** (FFI wrapper) |

**Root cause:** String/&str conversion. Rust has Deref coercion (String → &str) and Into (&str → String). Windjammer should infer.

---

### 6. Enum Variant / Option Wrapping — **~10 errors**

| Subcategory | Count | Files | Compiler Bug? |
|-------------|-------|-------|---------------|
| Some(&mut T) vs Some(T) | 2 | ai/npc_behavior | **Compiler** (ownership) |
| Option<&T> vs Option<T> | 4 | ecs, components | **OTHER** (trait/API) |

---

### 7. Other / Trait / API Mismatches — **~86 errors**

Includes:
- `expected LightingData, found LightingConfig` - type alias/struct mismatch
- `expected type, found function` - E0432 (unresolved imports), not E0308
- `expected VoxelGrid, found &VoxelGrid` - reference mismatch
- `expected ShaderFile, found Option<_>` - Option unwrapping
- Various trait impl signature mismatches

---

## Compiler Bugs to Fix (Priority Order)

### P0: Float inference (78+ errors)
1. **Match arm literal inference** - Add Unary Deref to get_known_float_type_from_expr
2. **Match pattern variable population** - Add v→&f32 when processing match Some(v) over Option<&f32>
3. **If/else branch unification** - Same as match (MustMatch exists, need type source)

### P1: Reference coercion (66 errors)
1. **Auto-deref for Copy params** - When f32 expected, &f32 passed → emit *
2. **Auto-borrow for &T params** - When &Vec expected, Vec passed → emit &
3. **Index expression** - arr[i] yields &T; in f32 context emit *arr[i]

### P2: Control flow (3 errors - game fix)
- Add semicolons in assets/asset_manager.rs, assets/pipeline.rs

### P3: String conversion (18 errors)
- String/&str coercion in argument position

---

## Game Code Fixes (Non-Compiler)

1. **ai/astar_grid.rs:220** - `rev.push(&path[k as usize])` → `rev.push(path[k as usize])` (remove &)
2. **assets/asset_manager.rs:64,69** - Add `;` after insert
3. **assets/pipeline.rs:267** - Add `;` after add_texture

---

## Test Strategy (TDD)

For each compiler fix:
1. Create minimal .wj test case reproducing the pattern
2. Add to windjammer/tests/ (e.g., type_inference_match_arm_deref_test.rs)
3. Verify fix reduces E0308 count in windjammer-game build
4. Run full test suite

---

## Verification Commands

```bash
# Count E0308 before/after
cd windjammer-game && wj game build 2>&1 | grep -c "E0308"

# Run float inference tests
cd windjammer && cargo test float_inference

# Run match arm test
cd windjammer && cargo test float_inference_match_arms
```

---

## Files Modified

| File | Change | Status |
|------|--------|--------|
| windjammer/src/type_inference/float_inference.rs | Add Unary Deref to get_known_float_type_from_expr; Add infer_match_value_option_inner; Populate match pattern vars (Some(v) => v: &T) | **DONE** |
| windjammer-game/.../assets/asset_manager.wj | Add semicolons after insert() in if-let blocks | **DONE** |
| windjammer/tests/type_inference_match_arm_astar_pattern_test.rs | TDD test for match HashMap.get pattern | **DONE** |
| windjammer/src/codegen/rust/expression_generation.rs | Auto-deref for Copy args | Pending |
| windjammer-game/.../ai/astar_grid.wj | rev.push - compiler over-borrows index | Pending (compiler fix) |

---

## Success Criteria

- [ ] E0308 count reduced by 50%+ (target: <140)
- [ ] All new tests pass
- [ ] No regressions in existing tests
- [ ] Clear separation: compiler vs game fixes documented

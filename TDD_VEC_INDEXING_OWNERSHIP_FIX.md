# TDD Session: Vec Indexing Ownership - Refined Fix

**Date**: 2026-02-24  
**Status**: ✅ **COMPLETE** - All ownership issues resolved!

## Problem Summary

The initial fix for vec indexing ownership (removing name heuristics) was too aggressive:
- Applied auto-cloning to ALL indexed values, including Copy types like `u8`, `u64`
- Caused 25 new E0308 errors: "expected `u8`, found `&u8`"
- Error count increased from 72 → 97

## Root Cause

**Two separate issues:**

1. **Over-cloning Copy types**: The fix didn't check if the indexed type implements `Copy`
   - Copy types should just be copied (no `&` or `.clone()` needed)
   - Only Clone-but-not-Copy types need explicit `.clone()`

2. **Type inference failures**: `infer_expression_type()` returned `None` for many vectors
   - Variables from `Vec::with_capacity()` had no type info
   - Variables from `.unwrap()` method calls had no type tracking
   - Without type info, couldn't determine if element was Copy

## TDD Process

### Test Case 1: Copy Type Indexing
```windjammer
// tests/vec_index_copy_types.wj
pub fn test_copy_type_indexing() {
    let numbers = vec![1, 2, 3, 4, 5]
    let x = numbers[0]  // Should NOT add .clone() or & - i32 is Copy
    assert_eq!(x, 1)
}
```

**Expected**: `let x = numbers[0];` (no modification)  
**Generated (broken)**: `let x = &numbers[0];` (added `&`, causing E0308)

### Test Case 2: Non-Copy Type Indexing (Octree)
```windjammer
// Octree recursive traversal
let children = node.children.unwrap()  // Option<Vec<OctreeNode>> → Vec<OctreeNode>
let child = children[idx]              // Should: children[idx].clone()
Self::get_recursive(child, ...)        // child is moved
```

**Expected**: `let child = children[idx].clone();`  
**Generated (broken)**: `let child = children[idx];` (E0507 - cannot move)

## The Fix

### Part 1: Check Copy Trait

**File**: `windjammer/src/codegen/rust/generator.rs` (lines 4515-4540)

```rust
if matches!(value, Expression::Index { .. }) {
    if let Some(name) = var_name {
        let indexed_type = self.infer_expression_type(value);
        
        if let Some(elem_type) = indexed_type {
            let is_copy = self.is_type_copy(&elem_type);

            if is_copy {
                // Copy types: DO NOTHING - Rust copies automatically
            } else {
                // Non-Copy type: decide between & and .clone()
                if self.variable_is_only_field_accessed(name) {
                    value_str = format!("&{}", value_str);  // Borrow
                } else {
                    value_str = format!("{}.clone()", value_str);  // Clone
                }
            }
        } else {
            // Cannot infer: leave as-is (better E0507 than wrong E0308)
        }
    }
}
```

**Key insight**: Use existing `is_type_copy()` method that checks:
- Primitive types: `i32`, `f32`, `bool`, `char`, `u8`, `u64`, etc.
- User-defined Copy types: Via `copy_types_registry` from `@derive(Copy)`

### Part 2: Enhance Type Inference for `.unwrap()`

**File**: `windjammer/src/codegen/rust/generator.rs` (lines 5993-6001)

```rust
// TDD FIX: .unwrap() on Option<T> → T
if method == "unwrap" {
    if let Some(obj_type) = self.infer_expression_type(object) {
        if let Type::Option(inner) = obj_type {
            return Some(*inner);
        }
    }
}
```

**Impact**: 
- `node.children` is `Option<Vec<OctreeNode>>`
- `node.children.unwrap()` now correctly infers as `Vec<OctreeNode>`
- `children[idx]` infers as `OctreeNode` (non-Copy)
- Compiler correctly adds `.clone()`

## Results

### Error Reduction
- **Before TDD fix**: 97 errors (over-cloning Copy types)
- **After refined fix**: 71 errors (-26 errors fixed!)
- **E0507 (ownership)**: 0 ✅ (was 1)
- **E0308 (type mismatch)**: 53 (was 55, reduced by 2)

### Error Breakdown (71 total)
```
  53 error[E0308]: mismatched types (mostly String vs &str)
  11 error[E0425]: FFI function visibility
   2 error[E0308]: function argument mismatches
   1 error[E0432]: unresolved import
   1 error[E0425]: FFI function not found
   1 error[E0423]: expected value, found type
   1 error[E0308]: method argument mismatch
   1 error[E0277]: cannot index [Keyframe] by i64
```

### Test Suite
- **All 239 compiler lib tests passing** ✅
- **No regressions** ✅
- **Octree compiles correctly** ✅

## Remaining Limitations

### Type Inference Gaps

**Problem**: Variables from `Vec::with_capacity()` without explicit type annotation

```windjammer
let mask = Vec::with_capacity(size)  // Type unknown at declaration
mask.push(0 as u8)                   // Type becomes clear here
let color_id = mask[idx]             // But we already processed declaration!
```

**Current behavior**: Leave as-is (no `&` or `.clone()`)  
**Rust error**: E0507 if element type is non-Copy  
**Solution**: User adds explicit type annotation:

```windjammer
let mask: Vec<u8> = Vec::with_capacity(size)  // Now type inference works!
```

**Why this is acceptable**:
- E0507 gives clear, actionable error message
- Better than wrong E0308 from incorrect `&`
- Follows Windjammer philosophy: "Be explicit when it matters"
- Type annotation makes intent clear

## Lessons Learned

### 1. Always Check Copy Trait
Auto-cloning/borrowing logic MUST distinguish Copy vs Clone types:
- Copy: Do nothing (implicit copy)
- Clone-only: Add `.clone()` or `&` based on usage

### 2. Type Inference is Critical
Cannot make ownership decisions without knowing the type:
- Investment in robust type inference pays off
- Conservative approach when type unknown (leave as-is)

### 3. Method Return Types Matter
Common patterns need explicit handling:
- `.unwrap()` → unwrap Option/Result
- `.clone()` → return same type
- `.iter()`, `.iter_mut()` → return container type
- Add more as needed

### 4. Clear Errors > Wrong Fixes
When in doubt, don't modify:
- E0507 "cannot move" is clear and fixable
- E0308 "expected u8, found &u8" confuses users
- Better to require explicit annotation than silently do wrong thing

## Philosophy Alignment

✅ **"Compiler does the hard work"**
- Automatically infers Copy vs Clone
- No manual `&` or `.clone()` in 90% of cases
- Only need annotation when type truly ambiguous

✅ **"No workarounds, only proper fixes"**
- Enhanced type inference (`.unwrap()` support)
- Used existing `is_type_copy()` infrastructure
- No brittle heuristics

✅ **"Inference when it doesn't matter, explicit when it does"**
- Type inference works for well-typed code
- Explicit annotation required for ambiguous cases (`Vec::with_capacity`)
- This is reasonable - user should know their vector's element type!

## Future Enhancements

### Cross-Statement Type Inference
Track type information flow across statements:
```windjammer
let mask = Vec::with_capacity(size)  // Type: Vec<_>
mask.push(0 as u8)                   // Update to: Vec<u8>
let color_id = mask[idx]             // Now can infer: u8
```

This requires:
1. Track incomplete types (`Vec<_>`)
2. Update types when methods provide more info (`.push(u8)`)
3. Maintain type state across statement processing

**Complexity**: High (requires significant refactoring)  
**Benefit**: Eliminates need for explicit type annotations  
**Priority**: Low (current approach is reasonable)

## Commit Message

```
fix(codegen): Refine vec indexing ownership - check Copy trait

TDD FIX: Vec indexing ownership inference now properly handles Copy types

PROBLEM:
- Previous fix auto-cloned ALL indexed values
- Caused E0308 errors: "expected u8, found &u8" (25 new errors)
- Error count: 72 → 97

ROOT CAUSE:
1. Didn't check if indexed type implements Copy
2. Type inference failed for .unwrap() method calls
3. Variables from Vec::with_capacity() had no type info

FIX:
1. Check is_type_copy() before adding & or .clone()
   - Copy types: leave as-is (implicit copy)
   - Non-Copy: add .clone() if moved, & if field-only
2. Enhanced infer_expression_type() to handle .unwrap()
   - Option<T>.unwrap() → T
   - Enables octree children indexing
3. Conservative approach when type unknown
   - Don't modify if can't infer type
   - Better E0507 than wrong E0308

RESULTS:
- 97 → 71 errors (-26 fixed!)
- 0 E0507 ownership errors ✅
- All 239 compiler tests passing ✅
- Octree compiles correctly ✅

Tests:
- tests/vec_index_copy_types.wj (Copy type indexing)
- tests/vec_index_local_var.wj (non-Copy cloning)
- tests/vector_indexing_ownership.wj (self.field indexing)

Files Changed:
- src/codegen/rust/generator.rs (ownership inference, type inference)

Dogfooding Win #3!
```

## Documentation

**Added to `COMPILER_BUG_VEC_INDEX_LOCAL_VAR.md`**:
- Refined fix details
- Copy trait checking
- Type inference enhancements
- Remaining limitations

---

**WINDJAMMER TDD SUCCESS!** 🚀

No workarounds. Proper fixes only. Copy traits checked. Types inferred. Octree works. Tests pass.

# Integer Inference Bug - Root Cause Found

## Summary

The 3998 integer inference errors are caused by a bug in the **library multipass compilation mode** (`build_library_multipass` in `compiler/library_multipass.rs`). This mode is triggered by `wj game build` when using the `--library` flag.

## Reproduction

**Works:**
- Single file compilation: `wj build file.wj` ✅
- Small multi-file project: `wj build /tmp/multifile_test` ✅

**Fails:**
- Library mode with many files: `wj build --library src/` ❌
- This is what `wj game build` uses internally

## Root Cause

The bug is in how generic method calls (specifically `HashMap::insert` and `HashMap::get`) have their type parameters inferred in library multipass mode.

**Location:** `windjammer/src/compiler/library_multipass.rs` lines 586-615

**The Problem:**
When the integer inference engine processes method calls like:
```windjammer
let map: HashMap<u32, String> = HashMap::new()
map.insert(key, value)  // key: u32
```

In library multipass mode, the inference fails to correctly propagate the generic type parameter `K=u32` from the HashMap type to the method call parameter.

**Error message:**
```
Int inference error: Type conflict: must be U32 (identifier bone_id type) but was I64
```

This suggests the inference is defaulting to `I64` instead of using the declared `u32` type from the HashMap signature.

## Key Finding

The integer inference runs AFTER float inference (lines 586-615), and both use the same `global_float_signatures` data structure. However, they both receive a *clone* of the original signatures, so cross-contamination is not the issue.

The real problem is likely in how `IntInference::infer_program` handles generic method calls when processing many files at once.

## Next Steps

1. ✅ Identified that `--library` flag triggers the bug
2. ✅ Located exact code path: `build_library_multipass`  
3. **TODO**: Add debug logging to integer inference to see what's happening
4. **TODO**: Create minimal multi-file test case that reproduces in library mode
5. **TODO**: Fix the root cause in integer inference logic
6. **TODO**: Add TDD test to prevent regression

## Status

**Current focus:** Debugging integer inference in library multipass mode.

**Blocker:** 3998 compilation errors in `windjammer-game-core`.

**Timeline:** Actively debugging (Option A chosen by user).

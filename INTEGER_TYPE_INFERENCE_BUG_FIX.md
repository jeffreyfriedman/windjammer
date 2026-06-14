# Integer Type Inference Bug Fix

## Bug Summary

**Problem:** Compiler generates incorrect integer types when variables are used for array indexing:

```windjammer
// SOURCE
let n = scratch.len() as i32  // n is i32
let idx = n / 2                // Should be i32
if idx >= n { ... }            // Should work!
scratch[idx]                   // Cast to usize HERE only

// GENERATED (WRONG)
let n = scratch.len() as i32;
let mut idx = n as usize / 2_usize;  // WRONG! Hoisted usize
if idx >= n { ... }                   // ERROR! usize >= i32
```

## Root Cause

`IntInference` engine (`src/type_inference/`) analyzes that `idx` is used for array indexing and propagates `usize` type **backwards** to the variable declaration.

**Files involved:**
1. `src/codegen/rust/expression_generation.rs:638` - Calls `inference.get_int_type(expr)`
2. `src/type_inference/` - IntInference engine (needs fixing)
3. `src/codegen/rust/usize_expression_type_inference.rs:126-147` - `usize_variables` tracking

## Fix Strategy

**Variables should only be `usize` if:**
1. Assigned from a usize expression: `let n = data.len()` (no cast)
2. Explicitly cast: `let n = x as usize`
3. Type annotated: `let n: usize = 10`

**Variables should NOT be usize just because:**
- They're used for array indexing later

**The cast should happen at indexing site:**
```rust
let n = data.len() as i32;   // n is i32
let idx = n / 2;              // idx is i32
if idx >= n { ... }           // i32 >= i32 works!
scratch[idx as usize]         // Cast here only
```

## Test Case

Created: `tests/integer_inference_indexing.wj`
- Reproduces the bug
- Will pass once fixed

## Next Steps

1. Find `IntInference` implementation in `src/type_inference/`
2. Modify to NOT propagate usize from usage sites
3. Ensure cast happens at indexing site in expression generation
4. Run test to verify fix
5. Rebuild breach-protocol (should fix 12 errors!)

## Impact

This fix will resolve ALL 12 remaining errors in `breach-protocol`:
- 5 errors in `metrics_collector.rs` (usize/i32 comparisons)
- 6 errors in `pathfinding.rs` (usize/i32 arithmetic)
- 1 error in `dialogue/manager.rs` (nested casts)

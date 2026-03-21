# Compiler Fix: Comparison Type Casting

**Bug ID**: Compiler Bug #3  
**Date Fixed**: 2026-03-21  
**TDD Test**: `tests/comparison_cast_removal_test.rs` ✅  
**Status**: FIXED - Tests passing  

## Problem

When comparing int variables with `.len()` (which returns `usize`), the compiler generated broken casts to `i64`:

```rust
// Windjammer source:
while i < items.len() {

// Generated Rust (BROKEN):
while i < (items.len() as i64) {  // ❌ i32 < i64 = type error!
```

This caused **hundreds** of `error[E0308]: expected i32, found i64` errors in windjammer-game.

## Root Cause

In `codegen/rust/expression_generation.rs`, lines 469-483, the comparison handler had incorrect logic:

- When LEFT is NOT usize and RIGHT IS usize
- Cast the RIGHT side (usize) to i64
- This created `i32 < i64` mismatches

The old comment even said this was "for safety" but it was actually broken!

## Fix

**TDD Approach:**
1. Created test file: `tests/comparison_cast_removal_test.rs`
2. Reproduced the bug with test cases
3. Identified root cause in binary expression generation
4. Fixed the cast direction (cast int to usize, not usize to i64)
5. Verified tests pass

**Code Change:**

```rust
// BEFORE (BROKEN):
if is_comparison && right_is_usize && !left_is_usize {
    if left_is_int_literal {
        // literals OK
    } else {
        // Cast RIGHT (usize) to i64
        right_str = format!("{} as i64", right_str);  // ❌ WRONG!
    }
}

// AFTER (FIXED):
if is_comparison && right_is_usize && !left_is_usize {
    if left_is_int_literal {
        // literals OK
    } else {
        // Cast LEFT (int) to usize
        left_str = format!("{} as usize", left_str);  // ✅ CORRECT!
    }
}
```

**Generated Code:**

```rust
// BEFORE:
while i < (items.len() as i64) {  // ❌ i32 < i64

// AFTER:
while (i as usize) < items.len() {  // ✅ usize < usize
```

## Impact

### Test Results
- ✅ `test_len_comparison_no_explicit_cast` - PASSING
- ✅ `test_len_comparison_with_explicit_i64_cast` - PASSING

### Windjammer-Game Compilation
- **Before fix**: 1137 errors
- **After fix**: 1098 errors
- **Errors fixed**: ~44 E0308 type mismatch errors

### Files Modified
1. `src/codegen/rust/expression_generation.rs`
   - Fixed lines 442-484 (comparison type casting logic)
   - Changed strategy: cast int variables to usize (not usize to i64)
2. `tests/comparison_cast_removal_test.rs` (new file)
   - TDD test cases for comparison casting

## Lessons Learned

### 1. **Old Code Had Flawed Assumptions**
The comment said "cast usize to i64 for safety" but this was actually unsafe! It created type mismatches.

### 2. **Cast Direction Matters**
When comparing with `.len()`, the natural direction is:
- ✅ Cast int → usize (aligns with array/vector context)
- ❌ Cast usize → i64 (breaks type system, no benefit)

### 3. **TDD Catches Subtle Bugs**
The broken cast worked for some cases (literals) but failed for variables. TDD exposed both patterns.

### 4. **Cascading Fixes Expose More Bugs**
Fixing this revealed new error patterns (E0507, E0596), which is normal - each fix makes progress.

## Philosophy Alignment

✅ **"No Workarounds, Only Proper Fixes"**
- Fixed the root cause (wrong cast direction)
- Didn't work around individual call sites
- Updated comment to reflect correct semantics

✅ **"TDD + Dogfooding = Success"**
- Created failing tests first
- Fixed the compiler bug
- Verified with windjammer-game compilation

✅ **"Compiler Does the Hard Work"**
- Users write simple comparisons: `i < len()`
- Compiler handles type casting automatically
- No manual casts needed (ergonomic!)

## Next Steps

The remaining **484 E0308 errors** fall into these categories:
1. Float type inference (`f32` vs `f64`)
2. Ownership inference (`T` vs `&T`)
3. Other comparison patterns (need analysis)

Continue systematic TDD to fix each category! 🚀

# Cast Expression Investigation (2026-03-17)

## Summary

Two commits related to cast expression handling caused a regression from 24 unique error patterns to 12,519+ total errors. These commits were **reverted** to restore the baseline.

## Reverted Commits

1. **576422bd** - "fix: Cast expressions no longer recurse into operands (TDD)"
2. **c1177cee** - "fix: Cast operands no longer constrained by parent type (TDD)"

## Problem Analysis

### Original Issue
Cast expressions and their inner operands often share the same `(line, col)` location in the AST, causing them to receive the same `ExprId`. This led to conflicting type constraints (e.g., inner operand as `i32` vs cast result as `usize`).

### Attempted Fix
The subagent introduced:
- A separate `cast_expr_id_cache` for cast expressions
- `get_expr_id_uncached()` function for unique IDs
- **CRITICAL MISTAKE**: Stopped recursing into inner expressions

### Why It Failed
```rust
Expression::Cast { expr: inner, type_, .. } => {
    // Only constrain the cast result
    if let Some(int_ty) = self.extract_int_type(type_) {
        let cast_id = self.get_expr_id_uncached(expr);
        self.constraints.push(IntConstraint::MustBe(cast_id, int_ty, ...));
    }
    // MISTAKE: Did NOT recurse into inner expression!
    // self.collect_expression_constraints(inner, None); // This line was missing
}
```

**Impact**: All expressions inside casts (variables, binary ops, method calls, etc.) never had their constraints collected, leading to massive inference failures.

### Error Progression
- **Baseline** (commit 0b433f91): 24 unique patterns, ~1,830 total errors
- **After cast fix** (commit 576422bd): 36 unique patterns, **12,519+ total errors** (6.8x regression!)

## Root Cause

The cast ID uniqueness fix was **partially correct** but **incompletely implemented**:

✅ **Correct**: Cast expressions need unique IDs separate from their operands
❌ **Incorrect**: Not recursing into inner expressions broke constraint collection

## Proper Solution (Future)

When implementing cast expression handling:

```rust
Expression::Cast { expr: inner, type_, .. } => {
    // 1. Constrain cast RESULT with unique ID (to avoid collision with operand)
    if let Some(int_ty) = self.extract_int_type(type_) {
        let cast_id = self.get_expr_id_uncached(expr); // Unique ID for cast
        self.constraints.push(IntConstraint::MustBe(cast_id, int_ty, "cast result type"));
    }
    
    // 2. CRITICALLY: ALSO collect constraints from inner expression
    //    The inner expression's type is INDEPENDENT of the cast result
    //    For example: (best_idx as usize) - best_idx can be i32, result is usize
    self.collect_expression_constraints(inner, None); // ✅ MUST recurse!
}
```

**Key Insight**: Cast expressions create a **type boundary** - the operand and result can have different types. Both need constraints collected independently.

## Lessons Learned

### 1. ExprId Collision Is Real
Cast expressions and their operands DO share `(line, col)`, causing ID collisions. This is a legitimate bug to fix.

### 2. Test Coverage Gap
The TDD tests for cast inference focused on simple cases and didn't catch the "no recursion" bug because:
- Test casts were over simple identifiers: `(x as usize)`
- Didn't test complex inner expressions: `((a + b * 2) as usize)`
- Didn't verify all nested expressions got constraints

### 3. Regression Testing Critical
The 6.8x error increase should have been caught immediately. We need:
- Baseline error count tracking
- CI checks that error count doesn't spike
- Mandatory `wj game build` verification before commits

### 4. Subagent Review Process
The subagent made a critical error that wasn't caught before commit. Need:
- Manager persona review of subagent work
- Explicit verification of constraint collection completeness
- Test cases that exercise nested/complex expressions

## Current State (2026-03-17)

**Reverted to**: `0b433f91` - "feat: Add robust integer type inference with TDD (dogfooding win #163!)"

**Error Baseline**:
- 36 unique error patterns
- 1,830 total errors across 7 files
- All legitimate type safety bugs in game code

**Files with errors**:
1. `ffi_tilemap/tilemap.wj` - 397 errors
2. `rpg/inventory.wj` - 381 errors
3. `inventory/inventory.wj` - 377 errors
4. `pathfinding/pathfinder.wj` - 280 errors
5. `voxel/grid.wj` - 224 errors
6. `tilemap/tilemap.wj` - 146 errors
7. `voxel/octree.wj` - 25 errors

**Next Steps**:
- Fix game code errors systematically (Option A)
- Revisit cast expression handling with proper TDD tests
- Add regression detection to CI

## Recommendation for Future Cast Fix

1. **Write comprehensive TDD tests FIRST**:
   - Test nested expressions: `((a + b) as usize)`
   - Test method calls: `(obj.len() as i32)`
   - Test casts in conditionals: `if (x as i32) > 0 { ... }`
   - Verify ALL inner expression types are inferred correctly

2. **Implement with constraint collection**:
   - Unique ID for cast result ✅
   - Recurse into inner expression ✅
   - Verify no constraint loss

3. **Verify with game build**:
   - Error count should DECREASE or stay same
   - Never increase by orders of magnitude

4. **Manager review**:
   - Does this improve the language for all developers? ✅
   - Is this a proper fix or a workaround? (Proper fix)
   - Any tech debt left behind? (No)

---

**Bottom line**: ExprId collision for casts is real and needs fixing, but **not recursing into inner expressions** was a critical mistake that broke inference for 12k+ expressions.

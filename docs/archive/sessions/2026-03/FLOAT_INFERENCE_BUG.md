# Float Inference Bug: ExprId Collisions

## Problem

**2092×E0308 errors**: Float literals in game code generate `f64` instead of `f32`.

## Root Cause

The float inference uses `expr.location()` to create `ExprId`:

```rust
fn get_expr_id(&self, expr: &Expression) -> ExprId {
    let location = expr.location();
    if let Some(loc) = location {
        ExprId { line: loc.line, col: loc.column }
    } else {
        ExprId { line: 0, col: 0 }  // BUG: All expr without locations collide!
    }
}
```

**When expressions don't have locations** (or have duplicate locations):
- All map to `ExprId { line: 0, col: 0 }`
- HashMap collisions cause wrong type lookups
- Codegen defaults to `f64` when ExprId not found

## Evidence

### TDD Tests PASS (with float inference)
```rust
// tests/float_impl_method_test.rs
pub fn progress_percentage(self) -> f32 {
    if self.requirement == 0 {
        1.0  // Generates 1.0_f32 ✅
    } else {
        0.5  // Generates 0.5_f32 ✅
    }
}
```

### Game Code FAILS (same pattern!)
```windjammer
// src_wj/achievement/achievement.wj
pub fn progress_percentage(self) -> f32 {
    if self.requirement == 0 {
        1.0  // Generates 1.0_f64 ❌
    } else {
        (self.progress as f32) / (self.requirement as f32)
    }
}
```

**Why?** Test uses simple source → locations work. Game uses multi-file → locations lost/collide.

## TDD Fix Options

### Option 1: Fix Expression Locations (Proper)
Ensure all expressions get unique locations during parsing/analysis.

**Pros:** Fixes root cause, locations useful for error messages  
**Cons:** Requires parser/analyzer changes, might affect performance

### Option 2: Use Expression Pointer Identity
Use `expr as *const _ as usize` as ExprId instead of line/col.

**Pros:** Always unique, no location dependency  
**Cons:** Not stable across runs, can't serialize

### Option 3: Sequential ID Assignment
Assign sequential IDs during constraint collection.

**Pros:** Simple, stable, unique  
**Cons:** Loses location info for debugging

### Option 4: Fallback to Context-Sensitive (Temporary)
When ExprId lookup fails, use return type from parent context.

**Pros:** Quick fix, backwards compatible  
**Cons:** Band-aid, doesn't fix root cause

## Recommended Solution

**Option 1** (proper fix) + **Option 4** (temporary guardrail).

1. Fix parser to ensure all expressions have locations
2. Add context-sensitive fallback for safety
3. Add TDD test that detects ExprId collisions

## Next Steps

1. Debug: Print ExprIds during inference to see collisions
2. TDD: Create test that reproduces multi-file collision
3. Fix: Implement Option 1 or Option 3
4. Guardrail: Add assertion when ExprId collisions detected

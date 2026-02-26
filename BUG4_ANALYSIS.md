# Bug #4 Analysis - Array Index Expression Type Inference

## The Bug
**Error**: `error[E0277]: the type `[clip::Keyframe]` cannot be indexed by `i64`  
**Location**: animation/clip.rs:68: `self.keyframes[i + 1]`  
**Status**: REPRODUCED ✅

## Root Cause Analysis

### What's Happening
1. Variable `i` is correctly marked as `usize` (Bug #3 fix working!)
2. The while loop correctly uses `i < self.keyframes.len()` without cast
3. BUT: The expression `i + 1` is **NOT** being recognized as `usize`

### Generated Code (Current - BUGGY)
```rust
let mut i = 0;  // Defaults to i64!
while i < self.keyframes.len() {  // Bug #3 fix: no cast here ✅
    if self.keyframes[i + 1].time > time {  // Bug #4: i + 1 is i64 ❌
        after_idx = i + 1;  // This works because after_idx is usize
        break;
    }
    i += 1;
}
```

### The Problem
Even though `i` is in the `usize_variables` set:
- `i` alone works fine as array index
- `i < keyframes.len()` works fine (Bug #3 fix)
- BUT `i + 1` is treated as a NEW expression
- The `expression_produces_usize()` function doesn't check binary expressions!

### Why This Happens
Looking at `generator.rs` line ~6336-6378:
```rust
fn expression_produces_usize(&self, expr: &Expression) -> bool {
    match expr {
        Expression::Identifier(name) => {
            self.usize_variables.contains(name)  // ✅ Works
        }
        Expression::MethodCall { object, method, .. } => {
            method == "len" || method == "capacity" || ...  // ✅ Works
        }
        Expression::Binary { .. } => {
            // ❌ NOT HANDLED!
            false
        }
        _ => false
    }
}
```

**Binary expressions always return `false`!**

## The Fix Strategy

### Step 1: Handle Binary Expressions in `expression_produces_usize()`
```rust
Expression::Binary { op, left, right } => {
    // If it's an Add/Sub/Mul/Div operation and BOTH sides are usize, result is usize
    match op {
        BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
            let left_is_usize = self.expression_produces_usize(left);
            let right_is_usize = self.expression_produces_usize(right)
                || matches!(right, Expression::IntLiteral(_));  // Literals adapt
            left_is_usize && right_is_usize
        }
        _ => false
    }
}
```

### Step 2: Test Cases
1. `i + 1` where `i` is usize → should be usize ✅
2. `i + j` where both are usize → should be usize ✅  
3. `i + 5` where `i` is usize → should be usize ✅
4. `i * 2` where `i` is usize → should be usize ✅

### Step 3: Edge Cases to Consider
- What about `i - 1`? (could go negative for i64, but usize wraps)
- What about `i + j` where `i` is usize and `j` is i64? (need cast)
- What about nested expressions like `(i + 1) * 2`? (recursion handles it)

## Expected Generated Code (After Fix)
```rust
let mut i = 0;
while i < self.keyframes.len() {
    if self.keyframes[i + 1].time > time {  // ✅ i + 1 is usize!
        after_idx = i + 1;
        break;
    }
    i += 1;
}
```

## TDD Test Case
**File**: `tests/bug_array_index_expression_type.wj` ✅ CREATED

**Current Status**: Test transpiles successfully, but rustc would fail with E0277

**After Fix**: Test should compile cleanly with rustc

## Implementation Plan

1. ✅ Create TDD test (DONE)
2. ⏳ Modify `expression_produces_usize()` to handle Binary expressions
3. ⏳ Run TDD test - should PASS after fix
4. ⏳ Rebuild game library - Bug #4 error should disappear
5. ⏳ Run full test suite - should remain 239/239
6. ⏳ Commit with proper message
7. ⏳ Push to GitHub

## Estimated Time: 30-45 minutes

## Related Bugs
- Bug #3: While-loop index usize inference (FIXED) - prerequisite for Bug #4
- Both bugs are about integer type inference in loop contexts

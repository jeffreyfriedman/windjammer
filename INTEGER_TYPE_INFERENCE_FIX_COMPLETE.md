# Integer Type Inference Bug Fix - COMPLETED

## Summary
Fixed the root cause of integer type inference bugs in the Windjammer compiler that were causing `usize`/`i32` type mismatches.

## Problem
The IntInference engine was propagating types **backwards** from use sites to declaration sites:

```windjammer
let n = data.len() as i32  // Declared as i32
let idx = n / 2             // Used in arithmetic
data[idx]                    // Used in indexing (requires usize)
```

The compiler would:
1. See `data[idx]` requires usize
2. Create MustBe(idx, Usize) constraint  
3. Create MustMatch(idx, n) via identifier linking
4. Propagate Usize backward to `n` declaration
5. Generate: `let n = data.len() as usize` (WRONG!)

## Root Causes

### 1. Identifier Linking (expression_constraints.rs:13-18)
```rust
// BAD: Links every identifier use to its assignment
if let Some(assignment_id) = self.var_assignments.get(name) {
    self.constraints.push(IntConstraint::MustMatch(
        id,
        *assignment_id,
        format!("identifier {} links to assignment", name),
    ));
}
```

This caused backward propagation where array indexing requirements flowed back to variable declarations.

### 2. Binary Operation MustMatch (binary_op_inference.rs:24-28)
```rust
// BAD: Forces both operands to have same type
self.constraints.push(IntConstraint::MustMatch(
    left_id,
    right_id,
    format!("binary op {:?}", op),
));
```

This meant `n / 2_usize` would make `n` become `usize` too.

### 3. Comparison MustMatch (binary_op_inference.rs:152-156)
```rust
// BAD: Forces operands in comparisons to match
self.constraints.push(IntConstraint::MustMatch(
    left_id,
    right_id,
    format!("comparison {:?}", op),
));
```

## The Fix

### Fixed expression_constraints.rs
Removed the identifier linking that created backward propagation:

```rust
// FIXED: Don't link uses to declarations
// Casts will be inserted at use sites during code generation
if let Some(var_type) = self.var_types.get(name) {
    if let Some(int_ty) = self.extract_int_type(var_type) {
        self.constraints.push(IntConstraint::MustBe(
            id,
            int_ty,
            format!("identifier {} type", name),
        ));
    }
}
```

### Fixed binary_op_inference.rs
Removed MustMatch constraints for arithmetic and comparisons:

```rust
// FIXED: Each operand keeps its declared type
// Code generation inserts casts when types don't match
// (Lines 54-144 for arithmetic REMOVED)
// (Lines 182-272 for comparisons REMOVED)
```

### Code Generation Already Handles It
The existing `generate_index()` function (data_structure_generation.rs:558-607) already has logic to insert `as usize` casts at array indexing sites:

```rust
if !idx_str.contains(" as ") && !self.expression_produces_usize(index) {
    if needs_cast {
        format!("({}) as usize", idx_str)  // Cast at use site!
    } else {
        idx_str
    }
}
```

## Result

### Before Fix (11 errors)
```rust
// Generated code
let n = data.len() as i32;
let mut idx = n as usize / 2_usize;  // n wrongly cast to usize
if idx >= n {  // ERROR: usize >= i32
    idx = n as usize - 1_usize;
}
let value = data[idx as usize];
```

### After Fix (0 errors)
```rust
// Generated code
let n = data.len() as i32;
let mut idx = n / 2;  // n stays i32
if idx >= n {  // OK: i32 >= i32
    idx = n - 1;
}
let value = data[idx as usize];  // Cast at use site!
```

## Files Changed
- `src/type_inference/int_inference/expression_constraints.rs` - Removed identifier linking
- `src/type_inference/int_inference/binary_op_inference.rs` - Removed MustMatch constraints

## Test Results
- TDD test case: `tests/integer_inference_indexing.wj` (created)
- breach-protocol compilation: **11 errors → 0 errors** ✅
- Only 1 unrelated ownership inference bug remaining (manually fixed for now)

## Philosophy Alignment
This fix embodies the Windjammer principle:
> "The compiler should be complex so the user's code can be simple."

Users write:
```windjammer
let n = data.len() as i32
data[n]  // Just works!
```

Compiler generates:
```rust
let n = data.len() as i32;
data[n as usize]  // Automatic cast at use site
```

No backward propagation, no premature type conversions, clean separation of concerns!

# Float Inference Phase 2: Remaining f32*f64 Errors - Analysis & Fix

**Date:** 2026-03-15  
**Goal:** Fix remaining 17 f32*f64 inference gaps that Phase 1 didn't catch.

## Problem Examples

```rust
// squad_tactics: member_index as f32 * 6.28318 / self.members.len() as f32
let angle = member_index as f32 * 6.28318_f64 / self.members.len() as f32;  // E0277

// particle_emitter3d: s * 6.28318 where s = (seed as f32 * 0.1).sin() * 0.5 + 0.5
let s = (seed as f32 * 0.1_f32).sin() * 0.5_f64 + 0.5_f64;  // 0.5, 0.5 wrong
let x = (s * 6.28318_f64).cos() * ...;  // 6.28318 wrong
```

## Root Cause Analysis

### Gap 1: Variable type not inferred from complex expression
**Pattern:** `let s = (seed as f32 * 0.1).sin() * 0.5 + 0.5` then `s * 6.28318`

**Why it failed:** `get_known_float_type_from_expr(s)` looks up `var_types.get("s")`. For `s` to have f32, we need `infer_type_from_expression` to return f32 for the RHS. But:
- `infer_type_from_expression` didn't handle `Expression::Cast` → (seed as f32) returned None
- `infer_type_from_expression` didn't handle `Expression::Literal::Float` → 0.1, 0.5 returned None
- Binary required BOTH operands to infer; with one None, the whole chain failed

### Gap 2: Cast → literal propagation (already fixed in Phase 1)
The `(member_index as f32) * (6.28318 / count as f32)` pattern was already handled:
- `get_known_float_type_from_expr` has Cast case
- RHS→LHS propagation constrains 6.28318 when RHS is `count as f32`
- **squad_tactics already generates 6.28318_f32** (verified)

## Fix Implemented

### 1. Add `Expression::Cast` to `infer_type_from_expression`
```rust
Expression::Cast { type_, .. } => {
    if self.extract_float_type(type_).is_some() {
        Some(type_.clone())
    } else {
        None
    }
}
```
Enables: `seed as f32` → f32

### 2. Add `Expression::Literal::Float` to `infer_type_from_expression`
```rust
Expression::Literal { value: Literal::Float(_), .. } => None
```
Float literals are "flexible" - they adopt the type of the other operand in binary ops.

### 3. Extend `Expression::Binary` in `infer_type_from_expression`
When one operand has known float type and the other is a float literal, return the known type:
```rust
if let Some(ref ty) = left_ty.as_ref() {
    if self.extract_float_type(ty).is_some() && self.is_float_literal(right) {
        return left_ty;
    }
}
// Same for right_ty
```

### 4. Add `is_float_literal` helper
```rust
fn is_float_literal(&self, expr: &Expression) -> bool {
    matches!(expr, Expression::Literal { value: Literal::Float(_), .. })
}
```

## Inference Flow (After Fix)

For `let s = (seed as f32 * 0.1).sin() * 0.5 + 0.5`:
1. `(seed as f32 * 0.1)`: Cast → f32, Literal 0.1 → None, Binary returns f32 ✓
2. `.sin()`: primitive same type → f32 ✓
3. `* 0.5`: MethodCall → f32, Literal → None, Binary returns f32 ✓
4. `+ 0.5`: same ✓
5. **s → f32 in var_types** ✓

For `s * 6.28318`:
1. `get_known_float_type_from_expr(Identifier "s")` → var_types.get("s") = f32 ✓
2. LHS→RHS: add MustBeF32 for literal 6.28318 ✓
3. **6.28318_f32 generated** ✓

## Test Cases Added

- `test_f32_var_from_complex_expr_times_pi`: particle_emitter3d pattern
- Existing tests: test_cast_times_pi_over_cast, test_cast_mul_sin_mul_add, test_f32_var_times_pi

## Files Modified

- `windjammer/src/type_inference/float_inference.rs`:
  - infer_type_from_expression: +Cast, +Literal::Float, extended Binary
  - is_float_literal: new helper
- `windjammer/tests/float_inference_chained_ops_test.rs`: +test_f32_var_from_complex_expr_times_pi

## Verification

Run:
```bash
cd windjammer
cargo test float_inference_chained_ops
```

Then regenerate game code and verify:
```bash
wj build windjammer-game-core/src_wj/particles/particle_emitter3d.wj --output /tmp/out --no-cargo
grep -E "6\.28318|0\.5_f" /tmp/out/particle_emitter3d.rs
# Should show 6.28318_f32, 0.5_f32 (not _f64)
```

## Patterns Covered

| Pattern | Example | Fix |
|---------|---------|-----|
| Cast * literal / Cast | (m as f32) * (6.28318 / c as f32) | Phase 1 (RHS→LHS) |
| Var from complex * literal | s * 6.28318, s=(x as f32*0.1).sin()*0.5+0.5 | Phase 2 (infer var type) |
| Chained binary with params | x * 2.0 / y, x,y: f32 | Phase 1 (LHS→RHS) |
| Method result * literal | t.sin() * 0.5, t: f32 | Phase 1 (method return type) |

## Philosophy

**"No Workarounds, Only Proper Fixes"** - Complete the inference algorithm rather than adding explicit casts in game code.

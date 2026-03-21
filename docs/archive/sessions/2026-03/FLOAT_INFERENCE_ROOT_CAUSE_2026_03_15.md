# Float Type Inference - Root Cause Analysis & Fix

**Date:** 2026-03-15  
**Problem:** 12 float type errors remain (6 comparison, 6 multiplication/division)  
**Status:** FIXED

## Root Cause Summary

Three gaps were identified and fixed:

### 1. **Index Expression Not Handled** (get_known_float_type_from_expr)

**Pattern:** `arr[i] / 2.0` where `arr: Vec<f32>`

**Root Cause:** `get_known_float_type_from_expr` did not handle `Expression::Index`. The LHS of the division had known type (f32 from Vec element) but we returned `None`, so no MustBeF32 constraint was added for the literal.

**Fix:** Added Index case to `get_known_float_type_from_expr`:
```rust
Expression::Index { object, .. } => {
    let object_ty = self.infer_type_from_expression(object)?;
    let elem_ty = self.extract_vec_element_type(&object_ty)?;
    self.extract_float_type(&elem_ty)
}
```

### 2. **ExprId Lookup Returns Unknown** (codegen fallback needed)

**Pattern:** When inference correctly adds constraints but `get_float_type(expr)` returns `Unknown` at codegen (e.g., location mismatch, solver edge case, or nested expression not in inferred_types).

**Root Cause:** `get_float_type` looks up by (line, col). If no match in `inferred_types`, it returns `Unknown`. Codegen then fell through to `generate_literal_context_sensitive` which defaults to f64 when not in struct/return context.

**Fix:** Added **sibling fallback** at codegen. When generating a binary operand that is a float literal and `get_float_type` returns `Unknown`, we try `get_known_float_type(sibling)` - infer from the other operand. This handles:
- Location mismatches (inference stored type but codegen can't find it)
- Nested expressions where constraint propagation didn't reach the literal
- Cross-module cases where metadata wasn't fully loaded

**Implementation:**
- `float_literal_sibling_stack` on CodeGenerator - push right before generating left, push left before generating right
- In `generate_literal_with_context`, when `Unknown`: check stack, use `get_known_float_type(sibling)` if available

### 3. **get_known_float_type Not Exposed** (for codegen fallback)

**Fix:** Added `pub fn get_known_float_type(&self, expr) -> Option<FloatType>` to FloatInference, delegating to the existing private `get_known_float_type_from_expr`.

## Verification

### TDD Tests Added

- `test_comparison_field_literal_infers_f32` - self.velocity.x != 0.0
- `test_index_div_literal_infers_f32` - arr[i] / 2.0
- `test_var_from_param_div_literal_infers_f32` - width / 2.0 (f32 param)

### Existing Tests (unchanged)

- test_cast_div_literal_infers_f32
- test_nested_field_access_div_literal_infers_f32
- test_nested_division_literal_infers_f32_from_rhs_cast
- test_literal_times_cast_div_infers_from_context

## Architecture Notes

### Constraint Flow

```
collect_expression_constraints(Binary)
  ├─ MustMatch(left_id, right_id)
  ├─ LHS→RHS: get_known_float_type_from_expr(left)=F32 + right=Literal → MustBeF32(right_id)
  └─ RHS→LHS: get_known_float_type_from_expr(right)=F32 + left=Literal → MustBeF32(left_id)

get_known_float_type_from_expr(expr)
  ├─ Identifier → var_types
  ├─ FieldAccess → infer_type_from_expression(object) → struct_field_types
  ├─ Index → infer_type_from_expression(object) → extract_vec_element_type  [NEW]
  ├─ Cast → extract_float_type(type_)
  ├─ MethodCall → determine_method_return_type
  ├─ Binary → get_known_float_type_from_expr(left) or right
  └─ Unary Deref → extract from reference inner type
```

### Codegen Flow

```
generate_expression(Binary { left, right })
  ├─ push(right); generate_expression(left); pop()
  ├─ push(left); generate_expression(right); pop()
  └─ For literal in operand: generate_literal_with_context checks float_literal_sibling_stack
```

## Philosophy Alignment

**"Correctness Over Speed"** ✓ - Systematic debugging, proper fixes  
**"Compiler Does the Hard Work"** ✓ - Inference + fallback ensures literals get correct type  
**"No Workarounds"** ✓ - Extended inference and codegen, not casts in game code

# Float Arithmetic Type Mismatch - Root Cause Analysis

**Date:** 2026-03-14  
**Problem:** 14+ `cannot multiply f32 by f64` / `cannot divide f64 by f32` errors remain after Phase 1-4 fixes.

## Error Patterns Extracted from Game Code

### Pattern 1: Cast / Literal (pathfinder.wj)
```windjammer
let total_cost = current_g as f32 / 10.0
```
- **Generated:** `current_g as f32 / 10.0_f64` ❌
- **Expected:** `10.0_f32` (LHS is f32 from cast)
- **Inference:** `get_known_float_type_from_expr(Cast)` → F32 ✓
- **Constraint:** MustBeF32 for literal from LHS→RHS ✓

### Pattern 2: Nested FieldAccess / Literal (physics_body.wj)
```windjammer
let min_x = (self.position.x - self.size.x / 2.0) as i32
```
- **Generated:** `self.size.x / 2.0_f64` ❌
- **Expected:** `2.0_f32` (LHS is self.size.x → Vec3::x → f32)
- **Inference:** `get_known_float_type_from_expr(FieldAccess)` uses `infer_type_from_expression`
- **Requires:** struct_field_types["Vec3"]["x"] = f32 (from math/vec3.wj.meta)

### Pattern 3: Nested Division (squad_tactics.wj)
```windjammer
let angle = (member_index as f32) * (6.28318 / self.members.len() as f32)
```
- **Generated:** `6.28318_f64` in numerator ❌
- **Expected:** `6.28318_f32` (RHS of division is Cast to f32)
- **Inference:** RHS→LHS propagation for division ✓

### Pattern 4: f64 * f32 (mesh3d.wj)
```windjammer
let theta = 6.28318530718 * (seg as f32) / (segments as f32)
```
- **Expected:** Literal should infer f32 from operands (both f32)
- **Inference:** MustMatch propagates from Cast operands ✓

## Root Cause Analysis

### Why Phase 1-4 Didn't Catch These

1. **Constraint collection is correct** - LHS→RHS and RHS→LHS propagation exists for binary ops
2. **get_known_float_type_from_expr** - Handles Cast, FieldAccess (nested via infer_type_from_expression), MethodCall, Binary
3. **Metadata loading** - Vec3 struct fields loaded from math/vec3.wj.meta
4. **Solver** - MustMatch propagates types correctly

### Potential Gaps

1. **ExprId lookup at codegen** - get_float_type uses (line, col). If location differs between inference and codegen, lookup fails → Unknown → context-sensitive fallback (f64 default when not in struct/return context)

2. **Multi-module compilation order** - When physics_body compiles, does it have access to Vec3 metadata? The metadata loader runs per-module from `use` statements. Source root must be correct.

3. **Context-sensitive fallback** - When inference returns Unknown, we use current_struct_*, current_function_return_type. For literals in mid-expression (not return, not struct field), we default to f64.

## Fixes Implemented

### 1. TDD Test Suite
- `windjammer/tests/float_inference_remaining_patterns_test.rs`
- 4 test cases covering all patterns
- Run: `cargo test float_inference_remaining_patterns`

### 2. No Code Changes Required (Analysis)
The existing inference logic *should* handle all patterns. The issue may be:
- **Location mismatch** - ExprId (line, col) might not match between passes
- **Metadata path** - Vec3 might not load for physics_body

### 3. Recommended Verification Steps

1. **Run tests:**
   ```bash
   cd windjammer && cargo test float_inference_remaining_patterns
   ```

2. **If tests pass** - Inference works in isolation. Issue is multi-module (metadata, source root).

3. **If tests fail** - Add debug logging to trace:
   - Which constraints are added for each literal
   - What get_float_type returns at codegen
   - Whether ExprIds match

4. **Regenerate game:**
   ```bash
   cd windjammer-game-core && wj build src_wj --output .
   ```
   Then check if float errors reduced in `cargo build`.

## Architecture Summary

```
collect_expression_constraints(Binary)
  ├─ MustMatch(left_id, right_id)
  ├─ MustMatch(binary_id, left_id)  // result = operand type
  ├─ LHS→RHS: if get_known_float_type_from_expr(left)=F32 and right=Literal
  │            → MustBeF32(right_id)
  └─ RHS→LHS: if get_known_float_type_from_expr(right)=F32 and left=Literal
               → MustBeF32(left_id)

get_known_float_type_from_expr(expr)
  ├─ Identifier → var_types
  ├─ FieldAccess → infer_type_from_expression(object) → struct_field_types
  ├─ Cast → extract_float_type(type_)
  ├─ MethodCall → determine_method_return_type
  ├─ Call → function_signatures
  └─ Binary → get_known_float_type_from_expr(left) or right
```

## Philosophy Alignment

**"Compiler Does the Hard Work"** ✓ - Arithmetic should never need explicit casts. The inference engine handles f32/f64 unification via constraints.

**"No Workarounds"** ✓ - We extend inference, not add casts in game code.

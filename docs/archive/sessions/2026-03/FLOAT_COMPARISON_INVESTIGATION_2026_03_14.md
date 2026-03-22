# Float Comparison Investigation: Test Pass vs Game Fail

**Date:** 2026-03-14  
**Mystery:** `cargo test float_inference_comparison_test` → 10/10 passing ✅, but game build → 6 "can't compare f32 with f64" errors ❌

## Root Cause

**`get_known_float_type_from_expr` did not handle nested FieldAccess.**

The comparison constraint logic (lines 917-935 in float_inference.rs) calls:
```rust
if let Some(lhs_float_ty) = self.get_known_float_type_from_expr(left) {
    if let Expression::Literal { value: Literal::Float(_), .. } = right {
        // Add MustBeF32 for right_id
    }
}
```

For `self.velocity.x != 0.0`:
- **left** = `FieldAccess { object: FieldAccess { object: Identifier("self"), field: "velocity" }, field: "x" }`
- **right** = `Literal::Float(0.0)`

The OLD `get_known_float_type_from_expr` for FieldAccess only handled `object` when it was `Expression::Identifier`:
```rust
let struct_name: Option<String> = if let Expression::Identifier { name, .. } = object {
    if name == "self" { self.current_impl_type.clone() }
    else { self.var_types.get(name)... }
} else {
    None  // ← self.velocity falls through here! object is FieldAccess, not Identifier
};
```

So `get_known_float_type_from_expr(self.velocity.x)` returned **None**, no MustBeF32 constraint was added, and the literal defaulted to f64.

## Failing Patterns (6 errors)

| File | Line | Pattern | LHS Type |
|------|------|---------|----------|
| physics/physics_body.rs | 81, 99 | `self.velocity.x != 0.0`, `self.velocity.z != 0.0` | Vec3.x, Vec3.z (f32) |
| quick_start/game.rs | 42 | `self.camera.position.x/y/z != 0.0` (×3) | Vec3.x/y/z (f32) |
| rendering/post_processing.rs | 108 | `self.settings.gamma != 1.0` | ColorGradingSettings.gamma (f32) |

All involve **FieldAccess** (single or nested). Test patterns used `x: f32` (Identifier) and `self.value` (one-level FieldAccess) — both worked. The game uses **nested** FieldAccess (`self.velocity.x`, `self.camera.position.x`, `self.settings.gamma`).

## Fix

Replace the Identifier-only logic with recursive type resolution:

```rust
Expression::FieldAccess { object, field, .. } => {
    let object_type = self.infer_type_from_expression(object)?;  // Handles nested!
    let struct_name = match &object_type {
        Type::Custom(name) => name.clone(),
        _ => return None,
    };
    self.struct_field_types
        .get(&struct_name)
        .and_then(|fields| fields.get(field))
        .and_then(|ty| self.extract_float_type(ty))
}
```

`infer_type_from_expression` already handles:
- `Identifier("self")` → current_impl_type
- `FieldAccess(object, field)` → recursively resolves object, then looks up field

So `self.velocity.x` → object=`self.velocity` → Vec3 → field x → f32 ✓

## Test Coverage

- **test_nested_field_access_comparison** (float_inference_struct_fields_test.rs): `self.velocity.x != 0.0` with PhysicsBody { velocity: Vec3 }
- **test_nested_field_access_comparison** (float_inference_comparison_test.rs): Same pattern via wj build (full pipeline)

## Verification

```bash
# Run internal API test (fast)
cargo test test_nested_field_access_comparison --test float_inference_struct_fields_test

# Rebuild game and verify no f32/f64 comparison errors
cd windjammer-game
wj game build --release 2>&1 | grep "can't compare f32 with f64"
# Should output nothing
```

## Philosophy

**"If tests pass but game fails, the tests are incomplete."** — The tests covered Identifier and single FieldAccess but not nested FieldAccess. The fix extends inference to cover all comparison contexts.

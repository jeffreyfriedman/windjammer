# Float Literal Codegen Fix

## Problem

Compiler generated f64 literals by default, causing type mismatches when variables/params/fields expect f32.

**Example Errors (from Breach Protocol):**
```rust
let x: f32 = 2.0;  // ERROR: f64 cannot coerce to f32
let y = player_pos.x + 10.0;  // ERROR: cannot add f32 and f64
```

**Root Cause:** Windjammer AST stores float literals without type suffix. Rust codegen emitted bare `2.0` (defaults to f64 in Rust) or hardcoded `_f64` suffix.

## Approach Chosen: Strategy A + B

### Strategy A: Infer from Context (Primary)

The existing `FloatInference` engine already propagates constraints from:
- Variable declaration: `let x: f32 = 1.0`
- Function parameter: `fn foo(x: f32) → foo(1.0)`
- Struct field: `Vec3 { x: 1.0 }` where `x: f32`
- Binary operation: `f32_var + 2.0` (MustMatch constraint)
- Return type: `fn bar() -> f32 { 1.0 }`

**Fix 1:** `get_float_type()` in `float_inference.rs` now returns `FloatType::Unknown` when no location match is found (instead of `FloatType::F64`). This allows the caller to fall through to context-sensitive logic for edge cases (folded literals, expressions with (0,0) location).

### Strategy B: Default to f32 (Fallback)

When inference returns Unknown and no struct/function context is available, default to f32.

**Fix 2:** `generate_literal_context_sensitive()` in `expression_generation.rs` now defaults to `"f32"` instead of `"f64"` when:
- No struct field context
- No function return type context

**Rationale:** Game engines (Breach Protocol, rendering, physics) predominantly use f32. Defaulting to f32 eliminates ~150+ manual fixes after regeneration.

## Files Changed

| File | Change |
|------|--------|
| `src/type_inference/float_inference.rs` | `get_float_type()` returns `Unknown` instead of `F64` when no match |
| `src/codegen/rust/expression_generation.rs` | Default float suffix changed from `"f64"` to `"f32"` |
| `tests/float_literal_codegen_test.rs` | **New** - TDD tests for f32/f64 literal codegen |
| `tests/float_literal_inference_test.rs` | `test_float_default_is_f64` → `test_float_default_is_f32` |
| `tests/float_inference_local_var_test.rs` | `test_unannotated_let_defaults_to_f64` → `test_unannotated_let_defaults_to_f32` |
| `tests/float_inference_array_vec_test.rs` | `test_no_annotation_remains_f64` → `test_no_annotation_defaults_f32` |
| `tests/float_inference_field_initializer_test.rs` | `test_unconstrained_defaults_to_f64` → `test_f64_return_constrains_literal` |
| `tests/float_inference_return_test.rs` | `test_no_return_type_defaults_f64` → `test_no_return_type_defaults_f32` |

## Test Coverage

- `test_f32_literal_in_variable_init` - `let x: f32 = 2.0` generates `2.0_f32`
- `test_f32_literal_in_binary_op` - `pos + 10.0` where `pos: f32` generates `10.0_f32`
- `test_f32_literal_in_function_call` - `scale(1.5)` where param is f32 generates `1.5_f32`
- `test_f64_literal_explicit` - `let x: f64 = 2.0` generates `2.0_f64`
- `test_unconstrained_defaults_to_f32` - `let x = 1.0` (no annotation) generates `1.0_f32`

## Verification

```bash
# Run float literal tests
cd windjammer && cargo test float_literal

# Regenerate Breach Protocol (no manual f32 fixes needed)
cd breach-protocol && wj game build --release
```

## Strategy C (Not Implemented)

Adding type annotation in parser (`Literal::Float { value, inferred_type }`) would require AST changes and parser modifications. Strategy A+B achieves the goal without parser changes.

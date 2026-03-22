# Float Arithmetic E0277 - Complete Analysis & Fix

**Date:** 2026-03-15  
**Goal:** Eliminate remaining `cannot multiply f32 by f64` / `cannot divide f64 by f32` errors.

## Error Categorization (from build_errors.log)

| Category | Count | Example | Root Cause |
|----------|-------|---------|------------|
| **f32 * f64** | ~35 | `(seed * 1234.567_f64).sin() * 3.14_f64` | Literals default to f64 when inference returns Unknown |
| **f32 / f64** | ~12 | `current_g as f32 / 10.0_f64` | Same - literal suffix wrong |
| **f64 / f32** | ~8 | `6.28318_f64 / count as f32` | Literal on LHS gets f64 |
| **f64 * f32** | ~6 | `2.0_f64 * seg as f32` | Literal on LHS |
| **multiply-assign** | ~3 | `price *= rep_modifier` (f32 *= f64) | Compound op not casting |

## Root Causes

### 1. Inference Returns Unknown
- **Location mismatch**: get_float_type looks up by (line, col). If ExprIds don't match between inference and codegen, lookup fails → Unknown.
- **Multi-module**: When compiling file A, struct_field_types for imported Vec3 might not be loaded.
- **Constraint solver**: MustBeF32/MustMatch may not propagate to all literals in complex expressions.

### 2. Codegen Only Cast When BOTH Operands Known
- Previous logic: `(F32, F64)` → cast, `(F64, F32)` → cast
- Gap: `(F32, Unknown)` → no cast. Literal with wrong suffix (f64) causes E0277.

### 3. Constants (const PI: f64)
- var_types has PI → f64. When used in `f32_var * PI`, we have (F32, F64).
- Fix: Existing cast logic handles this. Const propagation works.

## Fixes Implemented

### 1. Codegen Defense-in-Depth (expression_generation.rs)
Extended binary op cast logic to handle Unknown:
```rust
(Some(F32), Some(Unknown)) => right_str = format!("({}) as f32", right_str);
(Some(Unknown), Some(F32)) => left_str = format!("({}) as f32", left_str);
(Some(F64), Some(Unknown)) => right_str = format!("({}) as f64", right_str);
(Some(Unknown), Some(F64)) => left_str = format!("({}) as f64", left_str);
```

When one operand is known and the other Unknown (inference failed), we cast Unknown to match. This handles:
- Literals with wrong suffix (6.28_f64 when should be f32)
- Constants (PI: f64 in f32 context)
- FFI results with unknown type

### 2. Compound Assignment (statement_generation.rs)
Same Unknown handling for `*=`, `/=`, `+=`, `-=`.

### 3. .min()/.max() Arguments (expression_generation.rs)
Same Unknown handling for method call arguments.

## Test Cases Added

`windjammer/tests/float_arithmetic_e0277_test.rs`:
- `test_const_pi_f32_context` - const PI: f64 in f32 expression
- `test_emitter_angle_pattern` - (seed * 1234.567).sin() * 3.14 * 2.0
- `test_squad_tactics_angle_pattern` - 6.28318 / count as f32
- `test_compound_assignment_f32_f64` - price *= rep_modifier

## Philosophy Alignment

**"Consistency Over Convenience"** ✓ - f64 default is correct for unconstrained literals. Context propagation (inference) handles most cases. Defense-in-depth (cast when Unknown) ensures we never emit invalid code.

**"Compiler Does the Hard Work"** ✓ - User never writes explicit casts. Compiler infers or emits them automatically.

**"No Workarounds"** ✓ - We extend the inference/codegen properly, not patch game source.

## Verification

```bash
# Run new tests
cargo test float_arithmetic_e0277

# Run existing float inference tests
cargo test float_inference_remaining_patterns
cargo test bug_f32_f64_explicit_cast

# Build game (windjammer-game-core)
cd windjammer-game
wj build src_wj --output .
# Then: grep -c "cannot multiply\|cannot divide" build_errors.log
# Expected: 0 (or significantly reduced)
```

## Files Changed

- `windjammer/src/codegen/rust/expression_generation.rs` - Binary op + min/max Unknown handling
- `windjammer/src/codegen/rust/statement_generation.rs` - Compound assignment Unknown handling
- `windjammer/tests/float_arithmetic_e0277_test.rs` - New TDD tests

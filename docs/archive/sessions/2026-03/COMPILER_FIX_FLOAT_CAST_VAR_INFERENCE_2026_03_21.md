# Float cast → local type inference (f32/f64 literal unification)

## Symptom

Generated Rust: `survival_rate < 0.3_f64` while `survival_rate` is f32 (dogfooding: `ai/squad_tactics.wj`).

Rustc: `expected f32, found f64` on the literal.

## Root cause

For `let survival_rate = (alive as f32) / (total as f32)` with no explicit type, `collect_statement_constraints` uses `infer_type_from_expression(value)` to populate `var_types` so comparisons can add `MustBeF32` on the literal.

`infer_type_from_expression` handled `Binary` (same-type operands) but had **no `Cast` arm**. Operands `(alive as f32)` therefore did not contribute `Type::Custom("f32")`, the division inferred nothing, `var_types` omitted the local, and `survival_rate < 0.3` fell through to f64 defaults.

## Fix

In `float_inference.rs`, extend `infer_type_from_expression` with:

- `Expression::Cast { type_, .. }` → map `extract_float_type(type_)` to `Type::Custom("f32")` / `Type::Custom("f64")`.

## Tests

- `tests/float_type_unification_test.rs` — three cases: cast div + comparison, `n as f32` + compare, `n as f64` + compare.
- Tests use **in-process** lexer/parser/codegen (same pattern as `f32_f64_codegen_e0308_test.rs`) so `cargo test` does not depend on a stale `target/release/wj` binary.

## Verification

- `cargo test --release -p windjammer --features cli --test float_type_unification_test --test float_inference_comparison_test --test float_comparison_final_test --test f32_f64_codegen_e0308_test`
- `cargo test --release -p windjammer --lib` (252 tests)

Note: Full `cargo test --features cli` may still report unrelated failures (e.g. `build_system_ffi_deps_test`, some `pattern_matching_tests`) in this checkout.

# src/type_inference/

## Purpose
Legacy expression-level numeric inference: float (f32 vs f64) and integer width inference with implicit-cast rules. Slated to be replaced by `ir::numeric_solver` (WP 8).

## Inputs/Outputs
In: `Program` + struct-field metadata. Out: per-expression numeric types and implicit-cast decisions used by codegen.

## Invariants
- One of ~11 places that decide numeric width; do not add more.
- Constraint-based: collect, unify, report conflicts (f32 vs f64) as errors.

## Owns
`float_inference/`, `int_inference/`, `int_implicit_casts`, `struct_field_registry`.

## Never decides
Ownership, text emission.

## Allowed imports
Layer `types`. May import: every module in a lower layer (support < syntax < resolve) and peers `type_classification`, `type_display`. Known violations (tracked in docs/architecture-allowlist.toml, may only shrink): `analyzer`, `codegen`, `metadata`.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`FloatInference::new` / `IntInference::new` (their `mod.rs`), `int_implicit_casts::is_safe_implicit_cast`.

## Tests
tests/type_inference_*_test.rs, tests/float_*_test.rs, tests/int_*_test.rs.
Run: `cargo test --release --test all <filter>`; architecture rules: `cargo test --release --test all architecture`.

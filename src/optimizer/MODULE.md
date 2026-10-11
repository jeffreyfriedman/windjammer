# src/optimizer/

## Purpose
Optimization phases over the AST (string interning, dead-code elimination, loop optimization, escape analysis, SIMD). Currently disabled in the pipeline.

## Inputs/Outputs
In: `Program`. Out: transformed `Program`.

## Invariants
- Disabled: nothing in `compiler` calls it today. WP plan: wire a safe subset or delete.
- Phases are independent and composable.

## Owns
Optimization pass implementations.

## Never decides
Ownership inference, code emission.

## Allowed imports
Layer `backend`. May import: every module in a lower layer (support < syntax < resolve < types < ownership < ir). No known violations.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`optimize_*` functions per phase; `OptimizerConfig` in mod.rs.

## Tests
Inline unit tests only; no tests/ files because the optimizer is disabled.
Run: `cargo test --release --test all <filter>`; architecture rules: `cargo test --release --test all architecture`.

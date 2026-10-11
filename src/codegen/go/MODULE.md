# src/codegen/go/

## Purpose
Experimental Go backend (`ir_lowering.rs`, stdlib method lowering, per-construct generators under `generator/`).

## Inputs/Outputs
In: AST `Program`. Out: Go source text.

## Invariants
- Experimental: does not consume the shared IR.
- Stdlib method mapping lives in `stdlib_method_lowering.rs` only.

## Owns
Go text emission.

## Never decides
Ownership and Rust semantics.

## Allowed imports
Layer `backend`. May import: every module in a lower layer (support < syntax < resolve < types < ownership < ir). Known violations (tracked in docs/architecture-allowlist.toml, may only shrink): `compiler`, `linter`.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`GoBackend` (generator/mod.rs), `mod.rs`, and `generator/` (expression/statement/type generation).

## Tests
tests/go_backend_test.rs, go_backend_extended_test.rs, go_enum_pattern_extraction_test.rs.
Run: `cargo test --release --test all <filter>`; architecture rules: `cargo test --release --test all architecture`.

# src/codegen/wgsl/

## Purpose
WGSL emission and validation for compute/vertex/fragment shaders produced from the shader front ends.

## Inputs/Outputs
In: shader AST/metadata. Out: WGSL source plus `ShaderMetadata`.

## Invariants
- One of three shader stacks (`wjsl/`, `shader/`, here); WP 12 merges them.
- `validation.rs` runs before text is returned.

## Owns
WGSL type mapping, struct layout and emission.

## Never decides
Host-side Rust code, ownership.

## Allowed imports
Layer `backend`. May import: every module in a lower layer (support < syntax < resolve < types < ownership < ir). Known violations (tracked in docs/architecture-allowlist.toml, may only shrink): `compiler`, `linter`.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`WgslBackend` (src/codegen/wgsl/mod.rs); `codegen.rs`, `validation.rs`.

## Tests
tests/wgsl_*_test.rs.
Run: `cargo test --release --test all <filter>`; architecture rules: `cargo test --release --test all architecture`.

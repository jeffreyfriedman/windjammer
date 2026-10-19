# src/package/

## Purpose
Package manifest parsing (`wj.toml`). Resolver and registry client are design-only (ADR-0010) and not in the tree.

## Inputs/Outputs
In: manifests. Out: resolved dependency descriptions.

## Invariants
- No registry or resolver code until ADR-0010 is accepted.

## Owns
Manifest types.

## Never decides
Build orchestration.

## Allowed imports
Layer `tooling`. May import: every module in a lower layer (support < syntax < resolve < types < ownership < ir < backend). No known violations.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`package/manifest.rs`.

## Tests
Inline unit tests; tests/package_flat_lib_wj_test_exports_test.rs covers library exports.
Run: `cargo test --release --test all <filter>`; architecture rules: `cargo test --release --test all architecture`.

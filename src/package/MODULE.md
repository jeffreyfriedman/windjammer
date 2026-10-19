# src/package/

## Purpose
Package management foundation: manifest parsing, dependency resolution stubs, registry client interface.

## Inputs/Outputs
In: manifests. Out: resolved dependency descriptions.

## Invariants
- Mostly stubs; no real registry yet.

## Owns
Manifest and resolver types.

## Never decides
Build orchestration.

## Allowed imports
Layer `tooling`. May import: every module in a lower layer (support < syntax < resolve < types < ownership < ir < backend). No known violations.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`package/manifest.rs`, `resolver.rs`, `registry.rs`. `graph::resolve`: the real dependency graph (path + git,
git pinned in `wj.lock`, ADR-0010 option A), used by the interpreter and the host bridge.

## Tests
Inline unit tests; tests/package_flat_lib_wj_test_exports_test.rs covers library exports.
Run: `cargo test --release --test all <filter>`; architecture rules: `cargo test --release --test all architecture`.

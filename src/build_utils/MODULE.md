# src/build_utils/

## Purpose
Post-build helpers: generate `mod.rs`, strip `main` from library modules, clean stale module files, path utilities.

## Inputs/Outputs
In: output directory. Out: edited generated files.

## Invariants
- Idempotent; only touches generated files.

## Owns
Generated-module-tree manipulation.

## Never decides
Compilation.

## Allowed imports
Layer `driver`. May import: every module in a lower layer (support < syntax < resolve < types < ownership < ir < backend < tooling) and peers `test_runner`. No known violations.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`generate_mod_file`, `strip_main_functions`, `cleanup_stale_module_files` (src/build_utils/mod.rs).

## Tests
Inline unit tests in src/build_utils; exercised by every library build test.
Run: `cargo test --release --test all <filter>`; architecture rules: `cargo test --release --test all architecture`.

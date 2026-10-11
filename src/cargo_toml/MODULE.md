# src/cargo_toml/

## Purpose
Generation of `Cargo.toml` for single-file and multi-file builds, WASM crates, dependency and feature sections.

## Inputs/Outputs
In: source dir, project name, deps. Out: Cargo.toml text/files.

## Invariants
- Process-global `skip` flag exists (ratcheted state).

## Owns
Manifest generation.

## Never decides
Compilation.

## Allowed imports
Layer `driver`. May import: every module in a lower layer (support < syntax < resolve < types < ownership < ir < backend < tooling) and peers `compiler`. No known violations.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`generate_single_file_cargo_toml`, `generate_wasm_cargo_toml`, `infer_project_name_from` (src/cargo_toml/mod.rs).

## Tests
Inline unit tests; exercised by build tests in tests/compiler_*.
Run: `cargo test --release --test all <filter>`; architecture rules: `cargo test --release --test all architecture`.

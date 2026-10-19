# src/compiler/

## Purpose
Driver-level orchestration of a build: parse, multi-pass analysis, codegen, cargo manifest generation, dependency resolution, incremental cache, library multipass.

## Inputs/Outputs
In: source paths, target, flags. Out: generated Rust/Go/JS files, `.wj.meta` metadata, Cargo manifests.

## Invariants
- Owns the multi-pass loop; later passes re-run analysis with global signatures.
- `compilation_pipeline.rs` and `library_multipass.rs` are large; function-length ratchets apply.
- `std_stubs.rs`: the `std/<module>.wj` stubs of a build (always re-analysed, never overriding declared runtime formals).
- `multipass_bare_pass_demotion.rs` contains ownership post-fixes that belong to the solver (WP 9). A standard-library function's formals are the runtime's: bare-pass demotion never touches them (`server_serve(address: String, ..)`).

## Owns
Pipeline sequencing, file/dependency discovery, incremental cache, metadata writing.

## Never decides
Per-expression semantics; text emission.

## Allowed imports
Layer `driver`. May import: every module in a lower layer (support < syntax < resolve < types < ownership < ir < backend < tooling) and peers `build_utils`, `cargo_toml`, `compiler_database`. No known violations.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`build_project`, `build_project_ext` (src/compiler/compilation_pipeline.rs, re-exported from lib.rs).

## Tests
tests/compiler_test.rs, tests/compiler_tests.rs, tests/compiler_improvements_integration_test.rs and every end-to-end test using `build_project`.
Run: `cargo test --release --test all <filter>`; architecture rules: `cargo test --release --test all architecture`.

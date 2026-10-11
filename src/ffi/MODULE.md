# src/ffi/

## Purpose
C FFI binding generation (IDL and C header) from module metadata.

## Inputs/Outputs
In: `ModuleMetadata`. Out: IDL structures and C headers.

## Invariants
- Experimental (WJ-IMPL-02).

## Owns
IDL and header text.

## Never decides
Compilation.

## Allowed imports
Layer `support`. May import: nothing. Known violations (tracked in docs/architecture-allowlist.toml, may only shrink): `codegen`, `metadata`, `parser`.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`generate_c_header`, `wj_type_name_from_metadata` (src/ffi/).

## Tests
tests/ffi_*_test.rs.
Run: `cargo test --release --test all <filter>`; architecture rules: `cargo test --release --test all architecture`.

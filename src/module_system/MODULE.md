# src/module_system/

## Purpose
Module discovery and import resolution: builds the module tree from directories and `mod.wj`, resolves imports, generates `lib.rs`/`mod.rs`.

## Inputs/Outputs
In: project root path. Out: `ModuleTree`, generated module files.

## Invariants
- Directory structure is the source of truth; explicit `pub mod`/`pub use` are respected.

## Owns
Module graph and resolution.

## Never decides
Type or ownership analysis.

## Allowed imports
Layer `resolve`. May import: every module in a lower layer (support < syntax). No known violations.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`discover_nested_modules`, `generate_lib_rs`, `generate_mod_rs_for_submodule` (src/module_system/mod.rs).

## Tests
tests/module_declaration*_test.rs and inline tests.
Run: `cargo test --release --test all <filter>`; architecture rules: `cargo test --release --test all architecture`.

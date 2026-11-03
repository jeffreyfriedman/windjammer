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
`package/manifest.rs`. `rust_surface::load`: the Rust crate dependencies of `wj.toml` and their signature surfaces `rust/<crate>.wji` (ADR-0026), consumed by `check` (call checks, effects) and `compiler` (crate metadata). A hand-written surface wins over the generated cache copy `.wj-cache/rust-surfaces/<crate>/surface.wji`. `rust_surface_gen`: the `wj rust-surface` generator (crate source parsed with `syn`; `locate` finds the locked version in the registry/vendor/path, `scan` + `cfg` walk the public free functions and re-exports, `map` maps types or gives the skip reason, `pure` + `pure_crates.toml` the curated `@pure` list, `cache::ensure` is what `wj build` calls). Deterministic; only `locate::resolve_with_cargo` downloads. `graph::resolve`: the real dependency graph (path + git, git pinned in `wj.lock`, ADR-0010 option A), used by the interpreter, the host bridge and the build-order step of `wj build` / `wj test`. A path dependency named `../pkg/build` (the generated output of a Windjammer package) resolves to the package `../pkg` (`package_of_build_output`).

## Tests
Inline unit tests (`rust_surface_gen/tests.rs`: fixture crate); tests/rust_surface_generator.rs (real crates, cache, precedence, staleness, gate); tests/package_flat_lib_wj_test_exports_test.rs covers library exports.
Run: `cargo test --release --test all <filter>`; architecture rules: `cargo test --release --test all architecture`.

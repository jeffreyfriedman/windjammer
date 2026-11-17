# src/package/

## Purpose
Package manifest parsing (`wj.toml`), the path/git dependency graph and `wj.lock`. The registry client is design-only (ADR-0010) and not in the tree.

## Inputs/Outputs
In: manifests. Out: resolved dependency descriptions.

## Invariants
- No registry code until a registry exists (ADR-0010).
- Two builds from the same `wj.lock` are byte-identical; the lock names no absolute path and no time.

## Owns
Manifest types.

## Never decides
Build orchestration.

## Allowed imports
Layer `tooling`. May import: every module in a lower layer (support < syntax < resolve < types < ownership < ir < backend). No known violations.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`package/manifest.rs`. `rust_surface::load`: the Rust crate dependencies of `wj.toml` and their signature surfaces `rust/<crate>.wji` (ADR-0026), consumed by `check` (call checks, effects) and `compiler` (crate metadata). A hand-written surface wins over the generated cache copy `.wj-cache/rust-surfaces/<crate>/surface.wji`. `rust_surface_gen`: the `wj rust-surface` generator (crate source parsed with `syn`; `locate` finds the locked version in the registry/vendor/path, `scan` + `cfg` walk the public free functions and re-exports, `map` maps types or gives the skip reason, `pure` + `pure_crates.toml` the curated `@pure` list, `cache::ensure` is what `wj build` calls). Deterministic; only `locate::resolve_with_cargo` downloads. `graph::is_windjammer_path_dep`: the one rule for whether a `path` dependency (including `../pkg/build`) is a Windjammer package, shared by `rust_surface` and the generator. `graph::resolve`: the real dependency graph (path + git, git pinned in `wj.lock`, ADR-0010 option A), used by the interpreter, the host bridge and the build-order step of `wj build` / `wj test`. A path dependency named `../pkg/build` (the generated output of a Windjammer package) resolves to the package `../pkg` (`package_of_build_output`).

`lock.rs` (ADR-0010 option A): `wj.lock` v2, one entry per package of the whole graph (dependencies first; path: `sha256` of its files, git: commit and tree), `plan`/`update`/`sync`/`diff`/`enforce`; `graph::resolve_in` is the resolution both use. `wj lock update|check` (`src/cli/lock.rs`) and `wj build --locked` are the commands; a plain build follows the lock (a manifest that disagrees is an error, a changed path dependency a warning). Gate: scripts/wj/lock_gate.wj.

## Tests
Inline unit tests (`rust_surface_gen/tests.rs`: fixture crate); tests/rust_surface_generator.rs (real crates, cache, precedence, staleness, gate); tests/package_flat_lib_wj_test_exports_test.rs covers library exports.
Run: `cargo test --release --test all <filter>`; architecture rules: `cargo test --release --test all architecture`.

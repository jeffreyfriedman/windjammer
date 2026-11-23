# src/compiler/

## Purpose
Driver-level orchestration of a build: parse, multi-pass analysis, codegen, cargo manifest generation, dependency resolution, incremental cache, library multipass.

## Inputs/Outputs
In: source paths, target, flags. Out: generated Rust/Go/JS files, `.wj.meta` metadata (in the analysis cache, not beside sources), Cargo manifests.

## Invariants
- Owns the multi-pass loop; later passes re-run analysis with global signatures.
- `compilation_pipeline.rs` and `library_multipass.rs` are large; function-length ratchets apply.
- `std_stubs.rs`: the `std/<module>.wj` stubs of a build (always re-analysed, never overriding declared runtime formals).
- `rust_surface_metadata.rs`: Rust crate signature surfaces (`rust/<crate>.wji`, ADR-0026) become `metadata.json` under `.wj-cache/rust-surfaces/<crate>/` and join the external-metadata paths, so every build mode borrows what the crate borrows.
- `path_dep_metadata.rs`: a `wj.toml` path dependency with Windjammer sources and no `metadata.json` is analyzed as a
  library into `<cache root>/path-deps/<crate>-<fingerprint>/` (compiler identity, location, sources), and that
  metadata is used; a dependency that cannot be analyzed is named in an error, never given guessed formals. A
  `.wj-deriving` marker in the output directory guards against dependency cycles (no thread-local state).
- `lone_script_modules.rs`: the file a single-file build is given (a lone script, or a project's `main.wj`) is the
  crate root of that build; it also generates the modules it declares (`mod dep` -> `dep.wj` / `dep/mod.wj`), as a
  project build does, `mod.rs` files with the `pub use self::child::*` re-exports. The entry's `use crate::name::...` paths
  also name sibling modules (`name.wj` / `name/mod.wj` beside the entry): they are generated and declared in `main.rs`;
  one that exists nowhere is an error naming both candidate files.
- `value_formal_fixed_point.rs` makes callers and callees agree on value-formal modes (convergence O4): after each IR
  lowering round the solved modes are published to the registry and lowering repeats until none changes. Single-file
  builds run the same rounds over their one program (`attach_ir_module_if_missing`).
- `multipass_bare_pass_demotion.rs` contains ownership post-fixes that belong to the solver (WP 9). A standard-library function's formals are the runtime's: bare-pass demotion never touches them (`server_serve(address: String, ..)`).

## Owns
Pipeline sequencing, file/dependency discovery, incremental cache, metadata writing.

## Never decides
Per-expression semantics; text emission.

## Allowed imports
Layer `driver`. May import: every module in a lower layer (support < syntax < resolve < types < ownership < ir < backend < tooling) and peers `build_utils`, `cargo_cache`, `cargo_toml`, `compiler_database`. No known violations.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`build_project`, `build_project_ext` (src/compiler/compilation_pipeline.rs, re-exported from lib.rs).

## Tests
tests/compiler_test.rs, tests/compiler_tests.rs, tests/compiler_improvements_integration_test.rs and every end-to-end test using `build_project`.
Run: `cargo test --release --test all <filter>`; architecture rules: `cargo test --release --test all architecture`.

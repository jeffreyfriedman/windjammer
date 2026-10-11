# src/metadata/

## Purpose
Cross-file/cross-crate metadata (`.wj.meta`): serialized function signatures, struct fields and trait impls so later compilations can infer across module boundaries.

## Inputs/Outputs
In: analyzed items. Out/In: JSON metadata files and loaders (`load_struct_field_types_from_file`, `meta_cache_path`).

## Invariants
- Format changes must stay backward readable by `compiler`.
- Loaders tolerate missing files.

## Owns
Metadata schema, cache paths, signature filters.

## Never decides
Analysis itself; codegen.

## Allowed imports
Layer `tooling`. May import: every module in a lower layer (support < syntax < resolve < types < ownership < ir < backend). Known violations (tracked in docs/architecture-allowlist.toml, may only shrink): `compiler`.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`extract_struct_field_types`, `load_merged_external_struct_fields`, `meta_cache_path` (src/metadata/mod.rs).

## Tests
src/metadata/tests.rs; cross-crate behavior in tests/compiler_*.
Run: `cargo test --release --test all <filter>`; architecture rules: `cargo test --release --test all architecture`.

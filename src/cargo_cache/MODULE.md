# src/cargo_cache/

## Purpose
Central Cargo target directory outside project repos so artifacts are shared across projects.

## Inputs/Outputs
In: target kind. Out: a path and configured `Command`s.

## Invariants
- Reads the cache-location env vars; keep such reads here rather than in core code.

## Owns
Cache root and cargo command setup.

## Never decides
Compilation.

## Allowed imports
Layer `driver`. May import: every module in a lower layer (support < syntax < resolve < types < ownership < ir < backend < tooling). No known violations.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`cache_root`, `target_dir(kind)`, `configure_cargo_command`, `apply_lean_build_env` (src/cargo_cache/mod.rs).

## Tests
Inline unit tests in src/cargo_cache/mod.rs.
Run: `cargo test --release --test all <filter>`; architecture rules: `cargo test --release --test all architecture`.

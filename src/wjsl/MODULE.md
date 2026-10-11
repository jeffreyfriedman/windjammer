# src/wjsl/

## Purpose
WJSL (Windjammer Shader Language) front end: lexer, parser, type checker, includes, diagnostics and WGSL transpilation.

## Inputs/Outputs
In: `.wjsl` source text. Out: WGSL source or diagnostics.

## Invariants
- Self-contained: imports nothing else from the crate.
- Overlaps with `shader/` and `codegen/wgsl` (three stacks; WP 12 merges).

## Owns
WJSL syntax, shader type rules, include resolution.

## Never decides
Host-side code generation.

## Allowed imports
Nothing from the rest of the crate (self-contained). No known violations.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`transpile_wjsl`, `transpile_wjsl_with_includes`, `type_check_wjsl` (src/wjsl/mod.rs).

## Tests
tests/wjsl_*_test.rs (~30 files), fixtures in tests/common/wjsl_shader_fixtures.rs.
Run: `cargo test --release --test all <filter>`; architecture rules: `cargo test --release --test all architecture`.

# src/shader/

## Purpose
Older shader stack (AST, parser, type checker, WGSL codegen). Superseded by `wjsl/`; slated for deletion (WP 5/12).

## Inputs/Outputs
In: shader source. Out: WGSL text.

## Invariants
- Do not extend; fix bugs in `wjsl/` instead.

## Owns
Legacy shader code.

## Never decides
Anything new.

## Allowed imports
Layer `backend`. May import: every module in a lower layer (support < syntax < resolve < types < ownership < ir). No known violations.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`mod.rs`.

## Tests
tests/shader_exclusion_test.rs, shader_file_detection_test.rs, shader_wjsl_test.rs.
Run: `cargo test --release --test all <filter>`; architecture rules: `cargo test --release --test all architecture`.

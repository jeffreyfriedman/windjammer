# src/codegen/javascript/

## Purpose
JavaScript/TypeScript backend with optional tree-shaking, minification, source maps, polyfills and web-worker support.

## Inputs/Outputs
In: AST `Program`. Out: JS source (and optional `.d.ts`, source maps).

## Invariants
- Works from the AST (`ir_lowering.rs` is a local lowering), not the shared IR.
- Optimization files (minifier, tree_shaker, code_splitter, v8_optimizer) are independent post-passes.

## Owns
JS text emission and JS-specific post-processing.

## Never decides
Ownership, borrows, numeric widths (JS has none), Rust-specific concerns.

## Allowed imports
Layer `backend`. May import: every module in a lower layer (support < syntax < resolve < types < ownership < ir). Known violations (tracked in docs/architecture-allowlist.toml, may only shrink): `compiler`, `linter`.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`JavaScriptGenerator` (generator.rs); `mod.rs` wires the backend.

## Tests
tests/js_backend_test.rs.
Run: `cargo test --release --test all <filter>`; architecture rules: `cargo test --release --test all architecture`.

# src/agent_index/

## Purpose
Generates agent-facing index artifacts for MCP resources and tools, the offline docs index behind `wj docs <query>` (`docs_search.rs`) and the verified-example manifest (`verified.rs`).

## Inputs/Outputs
In: output path, `std/`, `agent_index/builtins.txt`, `docs/agent/*.md`, `examples/verified/*.wj`. Out: `errors.json`, `stdlib.json`, `spec.json`, `docs_index.json`, `examples/verified/manifest.json`.

## Invariants
- Deterministic output (docs index is sorted; no timestamps).
- `agent_index/docs_index.json` is embedded in the binary (`include_str!`); `tests/verified_examples_test.rs` fails when it or the manifest is stale: run `wj agent-index`.

## Owns
Index generation and search ranking (`docs_search::search`), example header parsing (`verified::parse_header`).

## Never decides
Compilation.

## Allowed imports
Layer `tooling`. May import: every module in a lower layer (support < syntax < resolve < types < ownership < ir < backend). No known violations.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`generate_agent_index` (src/agent_index/mod.rs).

## Tests
`tests/verified_examples_test.rs` (corpus oracle, lint, compiled path, freshness), `tests/agent_docs_test.rs` (doc code blocks, `wj docs`, `wj check --json`), MCP `tests/agent_tools_test.rs`.
Run: `cargo test --release --test all <filter>`; architecture rules: `cargo test --release --test all architecture`.

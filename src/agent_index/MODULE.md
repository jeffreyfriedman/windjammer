# src/agent_index/

## Purpose
Generates agent-facing index artifacts for MCP resources and tools.

## Inputs/Outputs
In: output path. Out: index files.

## Invariants
- Deterministic output.

## Owns
Index generation.

## Never decides
Compilation.

## Allowed imports
Layer `tooling`. May import: every module in a lower layer (support < syntax < resolve < types < ownership < ir < backend). No known violations.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`generate_agent_index` (src/agent_index/mod.rs).

## Tests
No dedicated tests/ file; covered via MCP crate tests and `wj` CLI runs.
Run: `cargo test --release --test all <filter>`; architecture rules: `cargo test --release --test all architecture`.

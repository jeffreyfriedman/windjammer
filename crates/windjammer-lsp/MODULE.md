# crates/windjammer-lsp

## Purpose
Language Server Protocol implementation for Windjammer: diagnostics, hover, completion, semantic tokens, inlay hints, refactorings, a debug adapter, and a Salsa-based incremental database.

## Inputs/Outputs
In: LSP JSON-RPC over stdio from an editor; document text. Out: diagnostics, hover/completion items, edits, code actions.

## Invariants
- Reuses the compiler through the `windjammer` library (parse, analyze, type inference); it must not fork compiler logic.
- Responses never invoke codegen or cargo; analysis stays in-process (see `windjammer::ide_analysis`).
- `lib.rs` exposes the Salsa `database` for benchmarks and tests.

## Owns
Editor protocol handling, document cache, IDE-oriented queries (`ide_queries.rs`), refactorings (`refactoring/`), debug adapter.

## Never decides
Ownership, type or numeric semantics (delegated to the compiler library).

## Allowed imports
Depends on `windjammer` (path dependency) only; consumed by `windjammer-mcp`. Never depends on the CLI/driver modules.

## Entry points
`src/main.rs` (binary `windjammer-lsp`), `WindjammerLanguageServer` in `src/server.rs`, `database.rs`, `refactoring/mod.rs`.

## Tests
`cargo test -p windjammer-lsp` (crates/windjammer-lsp/tests); benches in crates/windjammer-lsp/benches.

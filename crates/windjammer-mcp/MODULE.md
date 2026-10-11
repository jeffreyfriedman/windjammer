# crates/windjammer-mcp

## Purpose
Model Context Protocol server that lets AI assistants parse, analyze, explain, refactor and generate Windjammer code through typed tools.

## Inputs/Outputs
In: MCP JSON-RPC (stdio or HTTP with OAuth). Out: tool results (diagnostics, definitions, types, effects, code, patches).

## Invariants
- Each tool lives in `src/tools/<name>.rs` and is registered in `tools/registry.rs` / `catalog.rs`; adding a tool touches only those.
- Tools call the `windjammer` library and `windjammer-lsp` queries; they do not reimplement analysis.
- Tool inputs are validated before use; errors map to `error.rs` types.

## Owns
Agent-loop tools (`check_code`, `run_code`, `test_code`, `format_code`, `docs_search`, plus `explain_error`; shared plumbing in `tools/agent_loop.rs`; `run_code`/`test_code` spawn `wj`, located via `WJ_BIN`, a sibling binary or `PATH`). Protocol handling (`protocol.rs`, `server.rs`, `http_server.rs`, `oauth.rs`), tool registry, agent index serving (`agent_index.rs`).

## Never decides
Language semantics; that is the compiler library's job.

## Allowed imports
Depends on `windjammer` and `windjammer-lsp`; nothing depends on this crate.

## Entry points
`src/main.rs` (binary `windjammer-mcp`), `McpServer` in `src/server.rs`, `tools/registry.rs`.

## Tests
`cargo test -p windjammer-mcp` (crates/windjammer-mcp/tests: `agent_tools_test.rs` needs a built `wj`, `manifest_test.rs` keeps `tools/manifest.json` equal to the catalog); design notes in DESIGN.md.

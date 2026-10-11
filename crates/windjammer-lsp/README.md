# windjammer-lsp

Language Server Protocol server for Windjammer (diagnostics, hover, completion, semantic tokens, inlay hints,
refactorings, a debug adapter). It calls the compiler as a library and keeps its analysis in a Salsa database.

Architecture, invariants and entry points: [MODULE.md](MODULE.md). Behaviour is defined by the code and its tests,
not by this file.

```bash
cargo build --release -p windjammer-lsp     # binary: windjammer-lsp (stdio)
cargo test -p windjammer-lsp
```

Editor setup lives with each editor integration: [vscode-extension](../../vscode-extension/README.md),
[editors/vim](../../editors/vim/README.md), [editors/intellij](../../editors/intellij/README.md).
The [MCP server](../windjammer-mcp/README.md) shares this crate's database.

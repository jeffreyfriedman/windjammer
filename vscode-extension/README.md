# Windjammer for VS Code

Registers the `windjammer` language for `.wj` files with a TextMate grammar and language configuration, and starts the
[windjammer-lsp](../crates/windjammer-lsp/README.md) language server (`src/extension.ts`). All language intelligence comes from that server.

The commands and settings below are generated from `package.json`:

<!-- wj:gen name=vscode_package -->
| Command | Title |
|---|---|
| `windjammer.restartServer` | Restart Windjammer Language Server |
| `windjammer.explainError` | Explain Error Code |
| `windjammer.showErrorCatalog` | Show Error Catalog |

| Setting | Default | Description |
|---|---|---|
| `windjammer.autoFix.enable` | `true` | Enable auto-fix suggestions |
| `windjammer.diagnostics.enable` | `true` | Enable enhanced diagnostics with error codes |
| `windjammer.inlayHints.enable` | `true` | Enable inlay hints for ownership modes |
| `windjammer.lsp.path` | `"windjammer-lsp"` | Path to the windjammer-lsp executable |
| `windjammer.lsp.trace.server` | `"off"` | Traces the communication between VS Code and the language server |
<!-- wj:end -->

Build the extension from this directory with `npm run compile` (TypeScript) and `npm run package` (needs `vsce`).

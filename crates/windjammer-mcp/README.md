# windjammer-mcp

Model Context Protocol server exposing Windjammer tools (check, run, test, format, docs search, explain error, and
code-intelligence tools) to AI assistants over stdio or HTTP (OAuth 2.0, see `src/oauth.rs`).

Architecture, invariants and entry points: [MODULE.md](MODULE.md). The tool list is `src/tools/catalog.rs`
(mirrored by `tools/manifest.json`, which a test keeps equal to the catalog). Behaviour is defined by the code and its tests,
not by this file.

```bash
cargo build --release -p windjammer-mcp     # binary: windjammer-mcp (stdio by default)
cargo test -p windjammer-mcp
```

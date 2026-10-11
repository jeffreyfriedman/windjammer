---
status: current
last_verified: 203e43d114c9
---
# Repository metadata for the owner to apply

An agent cannot set these; they live in GitHub settings.

## Description (About box, under 350 characters)
```
Windjammer: a high-level language that compiles to Rust with inferred ownership and explainable decisions. Run the same source instantly in an interpreter, or ship it as Rust, Go, JavaScript or WebAssembly. Built for humans and coding agents (JSON diagnostics, MCP server, llms.txt).
```

## Website
Leave empty until a docs site exists; do not point it at the repository.

## Topics
```
programming-language, compiler, transpiler, rust, ownership, borrow-checker, interpreter, webassembly, javascript, golang, language-server, mcp, model-context-protocol, llm, ai-agents, developer-tools
```

## Social preview image (1280x640)
Text, in this order, left aligned, monospace code on the right:
- Title: `Windjammer`
- Subtitle: `Ownership inferred. Explained. Runs anywhere.`
- Right panel: the 6-line `add_song` function from README.md next to the same function in Rust (the side-by-side sample).
- Footer: `compiles to Rust | runs in an interpreter | made for agents`
Do not put a speed claim or a version number on the image.

## Settings
- Enable Discussions (a question channel keeps issues for compiler bugs; ADR-0013 says bugs are tracked elsewhere).
- Pin `docs/agent/WINDJAMMER_FOR_AGENTS.md` link in the README (already there); consider a "good first issue" label
  mapped to `docs/known-gaps.md` rows.
- crates.io: after publishing, the `Cargo.toml` description, keywords and categories in this repo are the listing.

## Checked-in metadata (done in this change)
- `Cargo.toml` description, keywords, categories.
- `vscode-extension/package.json` description and keywords.
- `llms.txt`, `llms-full.txt` (generated), `AGENTS.md` pointer, `docs/index.md`.
No docs site exists in the repository, so there is no structured data (JSON-LD) to add; when one is created, serve
`llms.txt` at its root and add `SoftwareSourceCode` JSON-LD with `programmingLanguage`, `codeRepository` and `license`.

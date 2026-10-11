---
status: current
last_verified: 203e43d114c9
---
# Windjammer documentation

Windjammer is a high-level language that compiles to Rust with inferred ownership. Pick your path.

## I want to try it
- [README quickstart](../README.md#quickstart): build, run a file, see the generated Rust.
- [Why Windjammer](WHY.md): honest comparisons and non-goals.
- [Coming from Rust, Go, Python or TypeScript](agent/cheatsheet_from_rust.md) (the other cheatsheets sit next to it).
- [Keywords](reference/keywords.md), [standard library](reference/stdlib.md), [error codes](reference/errors.md), [CLI](reference/cli.md).

## I am an AI agent (or I configure one)
- [Windjammer for agents](agent/WINDJAMMER_FOR_AGENTS.md): the check, run and test loop; every example is executed in CI.
- [llms.txt](../llms.txt) (index) and [llms-full.txt](../llms-full.txt) (everything inline), generated.
- MCP server: [crates/windjammer-mcp](../crates/windjammer-mcp/README.md). JSON diagnostics: `wj check --json`.
- Benchmark: [bench/agent-dx](../bench/agent-dx/README.md).

## I want to understand or change the compiler
- [Architecture](ARCHITECTURE.md), [ADRs](adr/README.md), [known gaps](known-gaps.md), [roadmap](../ROADMAP.md).
- [Repository instructions for agents and contributors](../AGENTS.md), [CONTRIBUTING](../CONTRIBUTING.md).
- [Full documentation index](README.md) with the truth status of every document.

## How these docs stay true
Generated references, executed examples and a claims lint: [CONTRIBUTING-docs.md](CONTRIBUTING-docs.md).

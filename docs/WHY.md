# Why Windjammer

Windjammer is a clean, expressive language that compiles to Rust. You write functions and structs without `&`,
`&mut`, lifetimes or `.clone()`, and the compiler infers them. The same source runs instantly in an interpreter while
you iterate, and compiles to native Rust when you ship, with Go, JavaScript and WebAssembly backends alongside.

## What sets it apart

| Property | Try it |
|---|---|
| Ownership of parameters and `self` is inferred, and you can always see the result | `wj build --no-cargo FILE`, then read the `.rs` |
| Instant runs with no build step | `wj run --interpret FILE` |
| One source, several backends | [reference/targets.md](reference/targets.md) |
| Machine-readable diagnostics | `wj check --json FILE` |
| First-class support for coding agents: MCP server, offline docs search, agent guide | `wj docs <terms>`, `crates/windjammer-mcp` |

## How it compares

### Rust
Windjammer output *is* Rust: it links against Cargo crates and runs on the same optimizer. What changes is how you
write it. Ownership annotations, `.to_string()`, `format!` and `.clone()` disappear from everyday code, `|>` pipelines
keep data flow readable, and the interpreter gives you a fast edit-run loop. When you want to see exactly what the
compiler decided, the generated Rust is right there.

### Go
Windjammer shares Go's appetite for simplicity and fast feedback, and adds sum types with exhaustive `match`,
generics with traits, and a native Rust compile path. A Go backend lets the same code target Go projects.

### Python
Python-style iteration (`wj run --interpret`) with static types, and a path to a native binary without a rewrite.

### Zig
Zig gives you explicit control over memory. Windjammer takes the opposite approach: the compiler makes the memory
decisions and shows you what it chose, so you can focus on the program.

### Mojo
Mojo focuses on Python-compatible, accelerator-heavy workloads. Windjammer targets general application and systems
code, with GPU work expressed as WGSL shaders through `.wjsl` files.

### Gleam
Gleam and Windjammer share a friendly, compact surface and the pipe operator. Gleam targets the BEAM; Windjammer
targets Rust and native code, with inferred ownership.

## Built for agents

Windjammer is designed so coding agents can write it well from day one, even without much training data:

- a compact [agent guide](agent/WINDJAMMER_FOR_AGENTS.md) whose examples run in CI, plus `llms.txt` and `llms-full.txt`
- structured JSON diagnostics with stable error codes
- an offline docs index (`wj docs`) and an MCP server
- a cross-language benchmark harness ([bench/agent-dx](../bench/agent-dx/README.md)) for measuring agent results

## Where it's headed

Windjammer is pre-1.0 and evolving quickly. Upcoming work includes a package registry, the async surface
([ADR-0004](adr/0004-concurrency-surface.md)), an ownership-explanation command, first-class arenas for high-performance code,
and published benchmarks against hand-written Rust and real agent trials. See the [roadmap](../ROADMAP.md) and the
[ADRs](adr/README.md).

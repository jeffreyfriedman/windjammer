# Why Windjammer

Windjammer is a high-level language that compiles to Rust. You write functions and structs without `&`, `&mut`,
lifetimes or `.clone()`; the compiler infers them. The same source runs instantly in an interpreter and can be emitted
as Rust, Go, JavaScript or WebAssembly.

Every claim below is either something you can check with a command (shown) or is labelled **Goal**. Maturity is
summarised in the [README](../README.md#status); the ranked list of gaps is [known-gaps.md](known-gaps.md).

## What is different today

| Property | Check it |
|---|---|
| Ownership of parameters and `self` is inferred; the generated Rust shows the decision | `wj build --no-cargo FILE`, read the `.rs` |
| One source, instant run, no build step | `wj run --interpret FILE` |
| One source, several backends | [reference/targets.md](reference/targets.md) (generated, with known-unsupported cases) |
| Machine-readable diagnostics | `wj check --json FILE` |
| An MCP server and an offline docs search for agents | `wj docs <terms>`, `crates/windjammer-mcp` |
| A benchmark harness that grades agents on the same tasks in six languages | `bench/agent-dx/README.md` |

## Compared with

### Rust
- Windjammer wins today: less ceremony in code that is mostly data and function calls (see the side-by-side sample
  in the README); an interpreter for instant feedback; pipelines with `|>`.
- Rust wins today: everything else that matters for production. Mature diagnostics (Windjammer type and borrow errors
  still come from rustc and are translated, known-gaps rank 2), a formatter, a package registry, complete async,
  decades of ecosystem, and full control over ownership when inference is not what you want.
- Relationship: Windjammer output is Rust, so it links against Cargo crates and inherits Rust's speed ceiling.
  **Goal:** inference plus explanations good enough that most application code never needs hand-written ownership.
  There is no published speed comparison; see the README performance note.

### Go
- Windjammer wins today: sum types with exhaustive `match`, generics and traits, an interpreter.
- Go wins today: toolchain maturity, compile speed, goroutines and channels, a large standard library, stability.
  Windjammer's Go backend is experimental and its concurrency surface is undecided
  ([ADR-0004](adr/0004-concurrency-surface.md)).

### Python
- Windjammer wins today: static types and a native compile path from the same source.
- Python wins today: ecosystem, notebooks, every library an agent has seen. Windjammer's interpreter is a subset of
  the compiled language ([INTERPRETER.md](INTERPRETER.md) lists what it rejects).
- **Goal:** a Python-like feedback loop (`wj run --interpret`) that graduates to a native binary without a rewrite.

### Zig
- Zig wins today: explicit control, comptime, cross-compilation, a tiny toolchain.
- Windjammer is the opposite bet: hide memory management decisions and explain them. Compile-time evaluation is
  an RFC, not a feature (see the roadmap).

### Mojo
- Mojo targets Python-compatible high-performance and accelerator code. Windjammer does not target ML accelerators;
  its GPU story is WGSL shaders through `.wjsl` files. They solve different problems; there is no benchmark between
  them and none is claimed.

### Gleam
- Gleam shares the pipe operator and a friendly, small surface, and is a mature, statically typed language with a
  package ecosystem. Windjammer has no registry yet ([known-gaps.md](known-gaps.md) rank 4). Windjammer's difference
  is its Rust target and inferred ownership rather than the BEAM.

## Built for agents: what exists and what does not
- Exists: [agent guide](agent/WINDJAMMER_FOR_AGENTS.md) whose code blocks run in CI, `llms.txt`, JSON diagnostics,
  an offline docs index (`wj docs`), an MCP server, and the `bench/agent-dx` harness.
- Does not exist yet: a published result showing that agents succeed more often in Windjammer than in other
  languages. The checked-in report grades hand-written reference solutions and scripted transcripts, not LLM runs, and
  says so. **Goal:** run real agent trials and publish them, including the cases where Windjammer loses.
- Training-data disadvantage is real: models have seen far more Rust, Go, Python and TypeScript than Windjammer.
  The bet is that a small surface, inferred ownership, structured diagnostics and good documentation offset it.
  That bet is unproven.

## When not to use Windjammer today
- You need a stable language: it is pre-1.0 and behaviour changes between minor versions.
- You need a package registry, a `.wj` formatter or an async story: none exists yet.
- You need the fastest possible code with hand-tuned ownership: write Rust.

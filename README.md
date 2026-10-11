# Windjammer

**A high-level language that compiles to Rust, with ownership and borrowing inferred by the compiler.**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

Windjammer source (`.wj`) is parsed, analysed and translated to Rust, which `cargo` then builds. You write plain
function signatures and bodies; the compiler decides where `&`, `&mut`, moves and clones go.

This README describes only what exists in the repository today. Items that are being built or only proposed are in
separate sections. The honest, ranked list of gaps is in [docs/known-gaps.md](docs/known-gaps.md); open design
decisions are in [docs/adr/](docs/adr/README.md).

**Version:** 0.50.0 (`Cargo.toml`). Pre-1.0: language and compiler behaviour can change between minor versions.

---

## Today

### Language
- Functions, structs, enums, traits, `impl` blocks, generics, closures, pattern matching with guards.
- Ownership of function parameters and `self` inferred (borrow, mutable borrow, move); clones inserted by the compiler.
- Local `let` bindings that are mutated are emitted as `let mut` even if you omit `mut` (see
  [ADR-0002](docs/adr/0002-local-mutability.md) for the open decision on whether this is the intended rule).
- String interpolation (`"${name}"`), pipe operator (`|>`: `x |> f(a)`, `x |> .method()`, `x |> f(a, _)`, closures, `?` per stage, multi-line chains; see [docs/design/pipe-operator.md](docs/design/pipe-operator.md)), `defer`, `if`/`else` as expressions, `?` error propagation,
  decorators (`@name`), `mod`, `extern`, `unsafe`, `static`, `where` clauses.
- Concurrency keywords that exist in the lexer: `thread { }`, `async`/`await`, `async fn`. Channel send/receive operators (`ch <- v`, `<-ch`) are parsed. There is no `go` keyword,
  no `chan::new()` and no `select`; see [ADR-0004](docs/adr/0004-concurrency-surface.md).
- Effect checking against an `[app_capabilities]` section in `wj.toml` (fails the build on violations).

Formal grammar: [docs/windjammer.ebnf](docs/windjammer.ebnf). Module system: [docs/MODULE_SYSTEM.md](docs/MODULE_SYSTEM.md).

### Tooling (the `wj` binary)
| Command | What it does |
|---------|--------------|
| `wj new`, `wj build`, `wj run`, `wj test` | Create, transpile + build, build and run, run tests |
| `wj lint` | Rust-leakage lint rules (W0001-W0004) on `.wj` files |
| `wj add` / `wj remove` / `wj update` | Edit `wj.toml` dependencies / update the toolchain |
| `wj run --interpret`, `wj test --interpret`, `wj repl` | Interpreted dev loop, no cargo (experimental subset; see [docs/INTERPRETER.md](docs/INTERPRETER.md)) |
| `wj explain`, `wj errors`, `wj stats`, `wj docs` | Error catalogue and diagnostics browsing |
| `wj shader-compile`, `wj validate-wjsl` | WJSL shader front end to WGSL |
| `wj cache`, `wj clean`, `wj self-install`, `wj agent-index` | Shared cargo cache, cleanup, install, agent artifacts |

Caveats, each tracked in [docs/known-gaps.md](docs/known-gaps.md):
- `wj fmt` runs `cargo fmt --all`; it does not format `.wj` files.
- `wj check` checks `.wj` sources with the front end only (no cargo/rustc). It does not yet catch moves, borrow conflicts, trait bounds, unknown methods, match exhaustiveness or operand types; see [docs/DIAGNOSTICS.md](docs/DIAGNOSTICS.md).
- `wj eject` does not exist. The older `windjammer` binary has an `eject` subcommand that writes a standalone Rust project.
- A language server (`crates/windjammer-lsp`) and an MCP server (`crates/windjammer-mcp`) exist as workspace crates.

### Targets
| Target | Status |
|--------|--------|
| Rust (`--target rust`, default) | Primary; the only backend that consumes the ownership IR solver |
| WebAssembly (`--target wasm`), JavaScript (`--target javascript`) | Implemented; not held to the same conformance bar |
| Go (legacy `windjammer` binary, "experimental") | Implemented as source translation; does not use the IR |
| WGSL | Via WJSL (`.wjsl`) shaders |

Tiers and the conformance contract are proposed in [ADR-0007](docs/adr/0007-target-tiers-conformance.md).

---

## Install and build

Requirements: Rust 1.90 or newer (`rust-version` in `Cargo.toml`).

```bash
git clone https://github.com/jeffreyfriedman/windjammer.git
cd windjammer
cargo build --release
./target/release/wj --version        # wj 0.50.0
```

Optional, to keep build artifacts outside repositories:

```bash
./target/release/wj cache setup      # writes env.sh / env.ps1 under the platform cache directory
wj cache status | wj cache path | wj cache prune
```

Override the cache root with `WJ_CARGO_TARGET_ROOT`; set `WJ_USE_LOCAL_TARGET=1` to force `./target`.
Design notes: [docs/superpowers/specs/2026-09-04-shared-cargo-cache-design.md](docs/superpowers/specs/2026-09-04-shared-cargo-cache-design.md).

Package-manager installs (`brew`, `cargo install windjammer`) are not verified by this repository's CI; build from source.

### Hello world

```windjammer
fn main() {
    let name = "World"
    println("Hello, ${name}!")
}
```

```bash
wj run hello.wj
```

### Ownership inference

```windjammer
fn longest(s1: string, s2: string) -> string {
    if s1.len() > s2.len() { s1 } else { s2 }
}
```

You do not write `&` or lifetimes; the generated Rust carries whatever the solver decides. The specification of what is
inferred is in [docs/language-design/ownership-inference.md](docs/language-design/ownership-inference.md).

## Test

```bash
cargo test --release --test all --no-fail-fast      # about 5,500 tests; 15-20 minutes
cargo test --release --test all <name-filter>       # targeted
```

Known failing tests are listed in `.config/known-test-failures.txt` (a ratchet: it may only shrink).
Backend-neutral behaviour tests live in [tests/conformance/](tests/conformance/README.md). Strategy:
[docs/TESTING_STRATEGY.md](docs/TESTING_STRATEGY.md).

---

## In progress

Work described in [docs/ARCHITECTURE_IMPROVEMENT_PLAN.md](docs/ARCHITECTURE_IMPROVEMENT_PLAN.md):
- Single ownership/clone/numeric-width owner (the IR solver) replacing the legacy analyzer patches.
- Name resolution to stable identities instead of string lookups.
- Layering and fitness tests, per-module `MODULE.md` documents, removal of global state and environment toggles.
- A `.wj` formatter and a real `wj check`.

## Planned (not implemented)

Status per RFC, with evidence, is in [docs/proposals/rfcs/README.md](docs/proposals/rfcs/README.md).
- Package registry and dependency resolver (today: path/git/version entries in `wj.toml` are forwarded to Cargo; `src/package/` is a stub).
- Caller-controlled `async expr` / `spawn expr` syntax (AST nodes exist, the parser does not produce them).
- Comptime evaluation, const generics, `owned` argument convention, map type hierarchy.
- `wj doc`, `wj watch`, `wj publish`, `wj optimize`, `wj economics`.
- Capability lock file enforcement, taint errors failing the build, capability profiles.
- An optimizer. The optimizer module exists but is disabled in the pipeline ([ADR-0006](docs/adr/0006-optimizer-fate.md)).

Performance claims (speed relative to Rust, incremental-build multipliers) are intentionally absent until they are
produced by a benchmark that runs in CI.

---

## Repository map

| Path | Contents |
|------|----------|
| `src/` | Compiler: lexer, parser, analyzer, `ir/` (solver), `codegen/` (Rust, Go, JS, WASM, WGSL), CLI |
| `crates/` | `windjammer-runtime`, `windjammer-lsp`, `windjammer-mcp` |
| `std/` | Standard library written in `.wj` (wrappers over the runtime crate) |
| `tests/`, `tests_wj/` | Rust integration tests, `.wj` fixtures, conformance suite |
| `docs/` | Architecture, RFCs, ADRs, language design, known gaps |
| `editors/`, `vscode-extension/` | Editor support |
| `examples/` | Example programs (not all are maintained; see `examples/README.md`) |

## Documentation
- [Architecture](docs/ARCHITECTURE.md) and [improvement plan](docs/ARCHITECTURE_IMPROVEMENT_PLAN.md)
- [Architecture decision records](docs/adr/README.md) and [RFCs](docs/proposals/rfcs/README.md)
- [Known gaps](docs/known-gaps.md), [Roadmap](ROADMAP.md), [Changelog](CHANGELOG.md)
- [API reference](docs/API_REFERENCE.md), [Comparison with Rust and Go](docs/COMPARISON.md)
- [Contributing](CONTRIBUTING.md), [Security policy](SECURITY.md), [Agent instructions](AGENTS.md)
- [Documentation index](docs/INDEX.md)

## Related projects
- [windjammer-ui](https://github.com/jeffreyfriedman/windjammer-ui) - UI framework written in Windjammer

## License

Dual-licensed under MIT OR Apache-2.0.

Created by [Jeffrey Friedman](https://github.com/jeffreyfriedman) and contributors.

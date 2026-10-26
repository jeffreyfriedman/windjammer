# Windjammer Compiler Architecture

Status: describes the code as of 0.50.0 (2026-10) and the target layering from
[ARCHITECTURE_IMPROVEMENT_PLAN.md](ARCHITECTURE_IMPROVEMENT_PLAN.md). Every statement under "Today" was checked against
`src/`; every statement under "Target" is a plan, not a fact. Open decisions are in [adr/](adr/README.md).

## 1. What the compiler is

`.wj` source is lexed, parsed, analysed, lowered to a safety-typed IR and printed as Rust (or Go, JavaScript, WASM,
WGSL). For the Rust target the output is then built by `cargo`. Windjammer has no code generator of its own below Rust:
rustc is the type checker of last resort (see [ADR-0005](adr/0005-error-strategy.md)).

Workspace crates (`Cargo.toml`): `windjammer` (compiler library plus the `wj`, `windjammer` and `wj-lint` binaries),
`windjammer-runtime`, `windjammer-lsp`, `windjammer-mcp`. Rust 1.90+.

## 2. Target layering (what the plan enforces)

Dependencies point downward only. Each layer owns specific decisions; no other layer may re-decide them.

```
 driver        one compile() entry: config, ports (source, sink, cargo, cache)       src/bin, src/cli*, src/compiler
 backends      printers over the IR: Rust, Go, JS, WASM, WGSL                         src/codegen
 IR            lowering + solvers: ownership, clones, numeric width, effects, taint   src/ir
 types         type facts, numeric inference                                          src/type_inference (to merge into ir)
 resolve       names -> DefIds                                                        (not yet present)
 syntax        lexer, parser, AST                                                     src/lexer, src/parser
```

| Decision | Owner (target) | Today |
|----------|----------------|-------|
| Ownership of params/`self`, clone insertion | `src/ir` solver | Split between `src/analyzer`, `src/ir`, and post-IR patches in `src/codegen/rust` and `src/compiler` |
| Integer/float width | `src/ir/numeric_solver` | Spread over `type_inference`, `analyzer`, `ir`, `codegen` (11 places) |
| Call-site coercion | `src/ir/coercion` | Rust backend uses it; other backends do not |
| Name resolution | resolver (`DefId`) | Bare-string registries (`get_signature(&str)`) |
| Configuration | one `CompilerConfig` | `IrCutoverConfig::from_env()` plus about 8 env toggles |

Principles and fitness tests: plan sections 2 and 4. The layering is encoded in `docs/MODULE_MAP.toml` and checked by
`tests/architecture.rs` once those land (work packages Q1 to Q3).

## 3. Today: pipeline

```
 .wj ──> lexer ──> parser ──> analyzer ──> IR (constraint_gen -> solver) ──> codegen ──> target source ──> cargo (Rust)
          src/lexer  src/parser  src/analyzer   src/ir                       src/codegen
```

- **Entry points:** `wj build/run/test/...` (`src/bin/wj.rs`, `wj_cli_args.rs`, `wj_cli_cmds.rs`) and the older `windjammer`
  binary (`src/main.rs`, `cli_args.rs`, `cli_commands.rs`). `wj eject` (`src/ejector.rs`) runs the build pipeline and vendors
  path dependencies into a standalone Cargo project. Single-file and multipass project
  builds are in `src/compiler/compilation_pipeline.rs`; Salsa-backed library builds in `src/compiler/salsa_library_build.rs`
  and `src/compiler_database.rs`.
- **Lexer/parser:** `src/lexer`, `src/parser`. Keywords include `thread`, `async`, `await`, `defer`, `extern`, `unsafe`,
  `mod`, `where`; pipe operator `|>`; string interpolation `${...}`. Ternary `?:` is not parsed (removed; use `if` expressions).
  The grammar is in [windjammer.ebnf](windjammer.ebnf).
- **Analyzer (`src/analyzer`):** per-function analysis (parameter and `self` usage, mutation, borrow, passthrough),
  signature registry, trait analysis. Still produces results that codegen consumes and patches.
- **IR (`src/ir`):** `constraint_gen` walks the AST and emits constraints; `solver` (union-find) produces a `SafetyType` per node;
  `numeric_solver`, `effects`, `taint`, `coercion`, `target_encodings`, `shadow` (parity check with the analyzer). The IR pipeline
  (`pipeline.rs`) runs the Effect, Taint and Execution solvers. Details and status: [IR_SOLVER_CODEGEN_ARCHITECTURE.md](IR_SOLVER_CODEGEN_ARCHITECTURE.md).
- **Backends (`src/codegen`):** `rust/` (about 90 files, production, consumes the IR via `set_ir_module`), `go/` and
  `javascript/` (compute an IR context and discard it), `wasm.rs` (through the Rust generator), `wgsl/`. A second shader stack
  (`src/shader`) and the WJSL front end (`src/wjsl`) coexist; consolidation is plan WP 12.
- **Optimizer (`src/optimizer`):** compiled but disabled in the pipeline (`optimize_program` in `src/compiler_database.rs`). See [ADR-0006](adr/0006-optimizer-fate.md).
- **Diagnostics:** parser/lint/IR errors in `.wj` terms; Rust type/borrow errors are produced by cargo and mapped back by
  `src/error_mapper.rs` with `src/source_map.rs`. Catalogue: `src/error_catalog.rs`, [errors/](errors/errors.md).
- **Modules and packages:** `src/module_system`, `wj.toml` handling in `src/config.rs`, `src/cargo_toml`, `src/package` (stub). See [MODULE_SYSTEM.md](MODULE_SYSTEM.md).
- **Tooling:** `src/linter`, `src/interpreter` (REPL), `src/ide_analysis` plus `crates/windjammer-lsp`, `crates/windjammer-mcp`, `src/agent_index`.
- **Standard library:** `std/*.wj` over `crates/windjammer-runtime` ([STDLIB_ARCHITECTURE.md](STDLIB_ARCHITECTURE.md), [ADR-0008](adr/0008-stdlib-portability.md)).

## 4. Module documents

Each top-level module is meant to carry a `MODULE.md` (purpose, inputs/outputs, invariants, owns/never-decides, allowed imports,
entry points, how to test) next to its code. They are added by the module-documentation work package; where one exists it is
authoritative for that module:

| Module | Document |
|--------|----------|
| Lexer | `src/lexer/MODULE.md` |
| Parser | `src/parser/MODULE.md` |
| Analyzer | `src/analyzer/MODULE.md` |
| IR / solvers | `src/ir/MODULE.md` |
| Backends | `src/codegen/MODULE.md` |
| Compiler driver | `src/compiler/MODULE.md` |
| Type inference | `src/type_inference/MODULE.md` |
| Module system / packages | `src/module_system/MODULE.md`, `src/package/MODULE.md` |

Links in this table may be missing on branches where that work has not merged yet.

## 5. Key design decisions (all in `docs/adr/`, all Proposed)

| Topic | ADR |
|-------|-----|
| Single ownership truth; end state of `src/analyzer` | [0001](adr/0001-ownership-truth.md) |
| Local mutability inferred vs explicit | [0002](adr/0002-local-mutability.md) |
| `owned` keyword | [0003](adr/0003-owned-keyword.md) |
| Concurrency surface | [0004](adr/0004-concurrency-surface.md) |
| Error strategy | [0005](adr/0005-error-strategy.md) |
| Optimizer | [0006](adr/0006-optimizer-fate.md) |
| Target tiers and conformance | [0007](adr/0007-target-tiers-conformance.md) |
| Stdlib portability | [0008](adr/0008-stdlib-portability.md) |
| Security enforcement | [0009](adr/0009-security-enforcement.md) |
| Packages | [0010](adr/0010-package-source-of-truth.md) |
| Releases | [0011](adr/0011-release-version-policy.md) |
| Rust interop | [0012](adr/0012-rust-interop-boundary.md) |
| Bug triage | [0013](adr/0013-bug-triage-tracker.md) |

## 6. Related documents
- [ARCHITECTURE_IMPROVEMENT_PLAN.md](ARCHITECTURE_IMPROVEMENT_PLAN.md): work packages and fitness tests
- [known-gaps.md](known-gaps.md): ranked gaps against the docs and RFCs
- [IR_SOLVER_CODEGEN_ARCHITECTURE.md](IR_SOLVER_CODEGEN_ARCHITECTURE.md), [SALSA_ARCHITECTURE.md](SALSA_ARCHITECTURE.md) (LSP incremental queries)
- [proposals/rfcs/README.md](proposals/rfcs/README.md): proposals and their implementation status

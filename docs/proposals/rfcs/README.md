# Windjammer RFCs (Request for Comments)

This directory contains formal proposals for significant changes to the Windjammer language, standard library, and tooling.

## Overview Documents

- **[Security Framework Overview](./SECURITY-FRAMEWORK-OVERVIEW.md)** - Complete guide to all security RFCs

## Active RFCs

### Security Framework

- **[WJ-SEC-01: Inferred Effect Capabilities](./WJ-SEC-01-effect-capabilities.md)** - Compile-time permission system to prevent supply chain attacks
- **[WJ-SEC-02: Taint Tracking](./WJ-SEC-02-taint-tracking.md)** - Type-level injection prevention (SQL injection, XSS, command injection)
- **[WJ-SEC-03: Capability Lock File](./WJ-SEC-03-capability-lock-file.md)** - Per-dependency capability sandboxing and escalation detection
- **[WJ-SEC-04: Capability Profiles](./WJ-SEC-04-capability-profiles.md)** - First-import security via profile-based analysis and community trust signals

### Syntax Improvements

- **[WJ-SYN-01: Pipe Operator](./WJ-SYN-01-pipe-operator.md)** - Optional ergonomic improvement for chaining functions
- **[WJ-SYN-02: Syntax Improvements](./WJ-SYN-02-syntax-improvements.md)** - Ternary operator, auto-derive, impl keyword (partly implemented: ternary was removed)

### Language Features

- **[WJ-LANG-01: Shader Language (WJSL)](./WJ-LANG-01-shader-language.md)** - Type-safe shader language that compiles to WGSL
- **[WJ-LANG-02: Mojo Feature Analysis](./WJ-LANG-02-mojo-feature-analysis.md)** - Analysis of Mojo features for potential adoption: compile-time metaprogramming, SIMD abstractions, const generics, ASAP destruction
- **[WJ-LANG-03: Map Types](./WJ-LANG-03-map-types.md)** - Backend-agnostic map type hierarchy: Map, SlotMap, OrderedMap, ConcurrentMap (research: slotmap, dashmap, papaya, flashmap, crossbeam)
- **[WJ-CONC-01: Async/Await & Concurrency](./WJ-CONC-01-async-concurrency.md)** - Caller-controlled execution (v2.0): eliminates function coloring, enables single-version libraries

### Package Management

- **[WJ-PKG-01: Package Management & Registry](./WJ-PKG-01-package-management.md)** - Package manifest, dependency resolution, registry protocol, security/economics integration

### Performance & Economics

- **[WJ-PERF-01: Economic Efficiency Framework](./WJ-PERF-01-economic-efficiency.md)** - Optimization strategy for AI-agent-scale deployments (compilation speed, runtime performance, memory efficiency, binary size, energy efficiency)

### Implementation

- **[WJ-IMPL-01: Compiler Refactoring](./WJ-IMPL-01-compiler-refactoring.md)** - Modular compiler architecture for maintainability
- **[WJ-IMPL-02: FFI Generation](./WJ-IMPL-02-ffi-generation.md)** - Automatic C FFI binding generation from IDL

## RFC Process

### Lifecycle

1. **Draft** - Initial proposal, open for discussion
2. **Review** - Community feedback and iteration
3. **Accepted** - Approved for implementation
4. **Implemented** - Feature is live in specified version
5. **Rejected** - Not proceeding with this proposal

### Status Legend

- 🟡 **Draft** - Under initial development
- 🔵 **Review** - Open for feedback
- 🟢 **Accepted** - Approved, ready for implementation
- ⚫ **Implemented** - Feature is live
- 🔴 **Rejected** - Not proceeding

## Current Status

Verified against the code on 2026-10-10 (branch `next/v0.50.0`, workspace version 0.50.0). "Document status" is what the RFC file
itself says; "Implementation" is what the code does. An RFC is only **Accepted** when the project owner says so; none of the
statuses below imply acceptance of the open questions in the matching ADR.

Implementation legend: **Implemented** (works end to end on the main path), **Partial** (some of it works; gap noted),
**Not started** (no code on any user path), **Superseded** (replaced; see note).

| RFC | Document status | Implementation | Evidence | Gap / next step |
|-----|-----------------|----------------|----------|-----------------|
| WJ-SEC-01 Effect capabilities | Draft | Partial | `src/ir/effects.rs`, `EffectSolver` in `src/ir/pipeline.rs`; `[app_capabilities]` enforced in `src/output_generation.rs`; `src/config.rs` | Only enforced when `[app_capabilities]` exists; app-level only, not per dependency; unknown externals unclassified. [ADR-0009](../../adr/0009-security-enforcement.md) |
| WJ-SEC-02 Taint tracking | Draft | Partial | `src/ir/taint.rs`, `TaintSolver` stage in `src/ir/pipeline.rs`; read by `src/cli/json_diagnostics.rs` | Errors do not fail `wj build`; sources/sinks only for annotated stdlib. ADR-0009 |
| WJ-SEC-03 Capability lock file | Draft | Partial | `src/ir/capability_lock.rs` (format, escalation detection) | Not referenced outside `src/ir/mod.rs`; no `.wj-capabilities.lock` is read or written by the CLI. ADR-0009, ADR-0010 |
| WJ-SEC-04 Capability profiles | Draft | Partial | `src/ir/capability_profiles.rs` (5 built-in profiles) | Not wired to any command or check; no community trust signals. ADR-0009 |
| WJ-SYN-01 Pipe operator | Draft (optional) | Implemented | `Token::PipeOp`, `src/parser/binary_expression_parser.rs`; `tests/pipe_operator_multi_arg_test.rs` | Document status should become Implemented; RFC text still debates adoption |
| WJ-SYN-02 Syntax improvements | Implemented (v0.46.0) | Partial | Ternary was removed: `src/parser/expression_parser.rs` ("Ternary operator removed"), `tests/fixtures/ternary_operator.wj` now tests `if`/`else`; auto-derive: `tests/auto_derive_comprehensive_tests.rs` | RFC claims ternary shipped; it does not parse. Update the RFC or restore the operator |
| WJ-LANG-01 Shader language (WJSL) | Draft | Partial | `src/wjsl/`, `wj shader-compile`, `wj validate-wjsl`, `tests/shader_wjsl_test.rs` | Three shader stacks (`src/wjsl`, `src/shader`, `src/codegen/wgsl`); plan WP 12 |
| WJ-LANG-02 Mojo feature analysis | Draft (analysis) | Not started | `ConstEval::Comptime` IR type extension only (CHANGELOG 0.50.0); no `owned`/comptime/const-generic syntax in lexer or parser | Needs accepted ADRs first: [ADR-0003](../../adr/0003-owned-keyword.md) |
| WJ-LANG-03 Map types | Draft | Partial | `std/map.wj` defines `Map`, `OrderedMap`, `SlotMap` (SlotMap is a HashMap stub); no `ConcurrentMap` | Backend-agnostic story and `ConcurrentMap` missing; [ADR-0008](../../adr/0008-stdlib-portability.md) |
| WJ-LANG-04 Universal string conversion | Draft | Implemented | CHANGELOG 0.50.0; `src/codegen/rust/method_call_expression_generation/mod.rs`; `tests/string_method_alias_test.rs` | Non-Rust backends unverified; RFC status stale |
| WJ-CONC-01 Async/concurrency | Draft | Partial | `ExecutionConstraint`/`ExecutionValidator` in `src/ir/`; `tokio::spawn` emission in `src/codegen/rust/async_generation.rs`, `block_generation.rs` | `AsyncCall`/`SpawnCall` are never constructed by the parser, so `async expr`/`spawn expr` source does not parse; `async fn` is the working form. [ADR-0004](../../adr/0004-concurrency-surface.md). Design doc `design/ASYNC_EXECUTION_MODEL.md` is superseded |
| WJ-PKG-01 Package management | Draft | Partial (stubs) | `src/package/{manifest,resolver,registry}.rs`; `DependencySpec` in `src/config.rs`; `wj add/remove/update` | Resolver accepts versions as-is, registry client makes no network calls, no lock file, no `wj publish`. [ADR-0010](../../adr/0010-package-source-of-truth.md) |
| WJ-PERF-01 Economic efficiency | Draft | Partial | `CompilationCostTracker` in `src/ir/cost_model.rs`; IR pass scaffolding in `src/ir/optimizations.rs`; `--defer-drop` build flag | No `wj economics`/`wj optimize`; the 15-phase optimizer is disabled (`src/compiler_database.rs`). [ADR-0006](../../adr/0006-optimizer-fate.md) |
| WJ-IMPL-01 Compiler refactoring | Review | Partial (in progress) | Module splits under `src/codegen/rust/`, `src/ir/`; superseded in scope by [ARCHITECTURE_IMPROVEMENT_PLAN.md](../../ARCHITECTURE_IMPROVEMENT_PLAN.md) | The plan is the live work list; this RFC is background |
| WJ-IMPL-02 FFI generation | Draft | Partial | `src/ffi/{idl,c_bindings}.rs` | No `wj` subcommand exposes it; C only. [ADR-0012](../../adr/0012-rust-interop-boundary.md) |
| SECURITY-FRAMEWORK-OVERVIEW | Draft (overview) | n/a | Summarises SEC-01..04 | Keep in sync with the four rows above |

The previous table listed WJ-LANG-02 and WJ-LANG-04 nowhere (they were missing) and showed every RFC as Draft even though the
CHANGELOG lists them as added in 0.50.0; the statuses and target versions that used to be here were not tied to evidence and were removed.

### Superseded documents
- `design/ASYNC_EXECUTION_MODEL.md` (deleted; see git history) is superseded by WJ-CONC-01 and ADR-0004 (its "APPROVED" status is historical).
- [archive/design/auto-reference.md](../../archive/design/auto-reference.md) is superseded by the IR solver ([ADR-0001](../../adr/0001-ownership-truth.md)).

### Decisions that gate RFC work
Accepting or rejecting the remaining RFC features needs the Proposed ADRs in [docs/adr/](../../adr/README.md), in particular
ADR-0004 (concurrency), ADR-0009 (security), ADR-0010 (packages) and ADR-0003 (`owned`).

## Contributing

To propose a new RFC:

1. Create a new file `WJ-[CATEGORY]-[NUMBER]-[short-name].md`
2. Follow the existing RFC format (see WJ-SEC-01 as template)
3. Submit for review
4. Address feedback and iterate

**Categories:**
- **SEC** - Security features
- **SYN** - Syntax improvements
- **STD** - Standard library additions
- **TOOL** - Tooling improvements
- **LANG** - Core language features
- **CONC** - Concurrency and async
- **PKG** - Package management
- **PERF** - Performance and economics
- **IMPL** - Implementation/architecture

## Philosophy Alignment

All RFCs must align with the Windjammer philosophy (see [AGENTS.md](../../../AGENTS.md)):

- **No Rust Leakage** - Examples must be idiomatic Windjammer (no `&`, `&mut`, `.as_str()`)
- **Compiler Does the Hard Work** - Automatic inference over manual annotations
- **80/20 Rule** - 80% of power with 20% of complexity
- **Backend-Agnostic** - Features work across Rust, Go, JavaScript, Interpreter backends
- **TDD + Dogfooding** - Tests first, real-world validation

## See Also

- [WJ-LANG-01](./WJ-LANG-01-shader-language.md) - Shader language proposal
- [AGENTS.md](../../../AGENTS.md) - Core development principles

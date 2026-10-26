# Architecture Improvement Plan

Status: active (2026-10). Source: architecture audit, RFC/ADR gap analysis, test-discovery audit.
Goal: a foundation where ownership, clones and numeric widths have **one owner**, names resolve to
**identities, not strings**, layers only depend downward, every test runs automatically, and an engineer
(or agent) can work on one module with only that module's context.

## 1. What the audits found

### Layering and ownership of decisions
- `analyzer`, `ir` and `codegen` import each other (analyzer→codegen 27, ir→codegen 51,
  type_inference→codegen/analyzer 14, codegen→compiler 6). Lexer/parser/`type_classification` are clean.
- Ownership of a parameter is decided in ≥5 places, clones in ≥3 (including string-level `.clone()` surgery),
  integer/float width in 11 places (~17k LOC), call-site coercion in 6.
- Registries are keyed by bare strings (`get_signature(&str)` ×469, suffix searches ×~100). A free function
  sharing a name with a method was silently dropped / inherited the method's `&Self` receiver.
- Hidden state: 7 `thread_local!`, 2 `static AtomicBool`, process-global `OnceLock` registries, ~8
  behaviour-changing env toggles (tests and production ran different code paths).
- Hot spots: 3 functions >2,000 lines, 57 >300 lines, ~20 files >2,000 lines, crate-wide
  `#![allow(dead_code)]`, 3 shader stacks, 2 `stdlib_method_traits`.

### Promise vs reality (RFC/design/README)
Implemented: pipe operator, ternary/auto-derive, WJSL front end, shared cargo cache, LSP/MCP breadth, effects
solver (partial). Partial: ownership inference (analyzer still patches the IR), capability/taint/lock/profile
enforcement, async per WJ-CONC-01, package management (stubs), multi-target (Go/JS discard the IR),
diagnostics (rustc errors translated). Not started: comptime/const generics, `wj economics`,
`wj optimize` (not started; optimizer disabled), `wj fmt` for `.wj` (runs `cargo fmt`), `wj watch/doc/publish`.

### Test discovery
Compiler `tests/*.rs` are auto-discovered by `build.rs` (top level only). See §5 for the orphan audit.

## 2. Principles (enforced by fitness tests, see §4)
1. One owner per decision: ownership/borrow, clone insertion, numeric width, call-site coercion, resolution.
2. Resolve once: names become `DefId`s before analysis; no suffix/bare-name lookups after that.
3. Layers depend downward only: syntax → resolve → types → ownership → IR → backends → driver.
4. Backends are printers: they never decide ownership, clones or widths.
5. No ambient configuration: one `CompilerConfig` built by the driver; core code never reads env vars.
6. No hidden state: caches live on a `CompilationSession`.
7. Every module has a `MODULE.md` (≤60 lines): purpose, inputs/outputs, invariants, owns/never-decides,
   allowed imports, entry points, how to test it.
8. A test is one file in the right place; CI fails on orphaned tests.

## 3. Work packages (execute in order; each ships alone and is gated by the suite + fitness tests)

| WP | Goal | Exit criteria |
|----|------|---------------|
| 0 | Safety net: golden generated-Rust corpus over `tests/**/*.wj` + `examples/` | diff tool + committed baseline; CI job |
| Q1 | Fitness tests (`tests/architecture.rs`): forbidden-import edges, function/file length ratchets, env/thread_local ratchet, dead-code allow ratchet, MODULE.md presence | all ratchets pass at today's numbers and only move down |
| Q2 | Orphan-test detector + recursive test discovery (§5) | adding a file anywhere under `tests/` runs; CI fails on unreachable test files |
| Q3 | Seed `MODULE.md` for top-level modules; `docs/MODULE_MAP.toml`; generated dependency graph | files exist; fitness test reads the map |
| 1 | `CompilerConfig` replaces `IrCutoverConfig::from_env()` and env toggles; defaults == production | 0 behaviour-changing env reads outside the driver; tests use production config |
| 2 | Retire cutover flags; delete legacy analyzer fields/paths superseded by the IR | 0 `ir_cutover` references; corpus unchanged |
| 3 | Remove global state into `CompilationSession` | no `thread_local!`/`static` flags outside driver; determinism test passes |
| 4 | Break import cycles (neutral `ir_contract`; move registry helpers out of codegen; no nested `Analyzer::new` in codegen) | forbidden-edge allow-list empty |
| 5 | Dead-code hygiene: remove crate-wide `allow(dead_code)`, delete dead modules (`shader/` stack, stubs, `safety_typed_ir` feature) | clippy clean with 0 crate-level allows |
| 6 | `DefId` resolver in shadow mode | 100% calls in corpus resolve or produce a tagged diagnostic |
| 7 | Cut registries over to `DefId`; delete suffix search and collision sets | 0 bare-name lookups for free functions |
| 8 | One numeric solver (`ir/numeric_solver`) authoritative; delete legacy inference | `type_inference/` removed; codegen only prints casts |
| 9 | One ownership/clone owner (solver); delete demotion/promotion heuristics and string-level clone surgery | single `OwnershipSolver::solve` writer; `ir_call_site.rs` <2k LOC |
| 10 | Decompose giant functions into named steps over a context struct (parallelisable with 7–9 only on disjoint files) | no function >300 lines; no file >2,000 |
| 11 | `Backend` trait over `IrModule`; Go/JS/WASM consume the IR | adding a backend touches only its directory |
| 12 | Merge shader stacks | one WGSL emitter |
| 13 | One `compile()` driver with ports (source, sink, cargo, cache) | ≤1 analyze→codegen driver |
| 14 | (optional, last) crate split | cargo enforces layering |

Order: 0, Q1–Q3, 1–5 first (cheap, de-risk), then 6, then 7→8→9 serially (they touch the same files), 10 in
parallel on disjoint files, then 11–14.

## 4. Fitness tests (WP Q1)
`tests/architecture.rs` reads `docs/MODULE_MAP.toml` and asserts, with ratchets that may only shrink:
- forbidden edges (`analyzer|ir|type_inference|parser` must not import `codegen`; `codegen` must not import
  `compiler`; `lexer|parser` import nothing above them);
- max function length and max file length (allow-lists shrink to 300 / 2,000, then 800);
- `std::env::var`, `thread_local!`, `static` atomics only in the driver allow-list;
- count of `#[allow(dead_code)]` never rises; `get_signature(&str)` call count never rises;
- every top-level module directory has a `MODULE.md` ≤ 60 lines.

## 5. Test discovery and orphan detection (WP Q2)
Findings and decisions are recorded in `docs/TESTING_STRATEGY.md` (rewritten). Mechanism: `build.rs` generates
the module tree recursively (every `*.rs` under `tests/` except `common/`, `fixtures/`), and a CI step fails
if a `*_test.rs`/`*_test.wj` file exists that is not reachable from a runner target.

## 6. Language-level gaps to close (from the RFC/ADR audit), ranked
1. Solver-owned ownership/clone/width correctness (WP 6–9) — blocks every dogfood project.
2. Native diagnostics: a front-end check that proves the common errors without invoking cargo; stop leaking rustc.
3. `.wj` formatter and a real `wj check`; `wj watch`, `wj doc`.
4. Path/git dependencies that build in dependency order; lock file; defer the registry.
5. Async per WJ-CONC-01 across backends; decide the `thread`/`go`/`spawn`/`async` surface.
6. Security: taint errors fail the build; wire `.wj-capabilities.lock`; classify unknown external calls.
7. Backend parity: Go/JS consume the IR; grow `tests/conformance/`.
8. Optimizer/economics: wire a safe subset or delete and fix the README.
9. RFC features (comptime, const generics, `owned`) only after an accepted ADR.

## 7. ADRs to write (decisions implied but unrecorded)
`docs/adr/NNNN-*.md`: ownership truth (IR solver vs analyzer), local mutability (inferred vs explicit),
`owned` keyword, concurrency surface, error strategy, optimizer fate, target tiers + conformance contract,
stdlib portability, security enforcement level, package source of truth, release/version policy,
rust-interop boundary, bug-triage tracker.

## 8. What not to do
No rewrite; fixtures are the asset. No crate split before cycles are broken. No new heuristic "demotion/
promotion" passes. No mechanical splitting of giant functions without a context struct and tests. No new
env-var toggles. No fourth shader stack. Do not run WP 2/7/8/9 concurrently on the same files.

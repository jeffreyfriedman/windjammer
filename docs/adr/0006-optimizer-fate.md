# ADR-0006: Optimizer fate

- **Status:** Proposed
- **Date:** 2026-10-10
- **Related:** [OPTIMIZATION_ARCHITECTURE.md](../OPTIMIZATION_ARCHITECTURE.md), [COMPILER_OPTIMIZATIONS.md](../COMPILER_OPTIMIZATIONS.md), [WJ-PERF-01](../proposals/rfcs/WJ-PERF-01-economic-efficiency.md), `src/compiler_database.rs`, `src/optimizer/`, `src/ir/optimizations.rs`, `src/ir/cost_model.rs`

## Context
`src/compiler_database.rs` (`optimize_program`) states the optimizer is "INTENTIONALLY DISABLED": `src/optimizer/` has
about 150 arena-lifetime errors against the Salsa `Program<'db>` types. The README, ROADMAP and several design docs
advertised a "15-phase optimization pipeline", "99%+ of Rust's performance" and "276x faster" builds. None of these are
measured by anything in the repository today. Meanwhile `src/ir/optimizations.rs` and `cost_model.rs` hold IR-level
pass scaffolding (DeferDrop, SmallVec, Cow stubs) and `--defer-drop` is a real build flag.

## Options
### A. Delete `src/optimizer/` (and its docs); keep IR-level passes; reintroduce only with benchmarks
The generated Rust is compiled by rustc/LLVM, which already performs most of these optimisations.
- Pros: removes dead code and false claims. Cons: loses design history (kept in `docs/archive`).

### B. Fix the lifetime architecture and re-enable
Pros: restores the advertised feature. Cons: months of work for unmeasured gain; conflicts with "no new heuristic passes" until the solver owns ownership.

### C. Leave disabled, fix only the docs
Pros: no code churn. Cons: dead module and `allow(dead_code)` debt persists.

## Recommendation
Option A. Keep Windjammer-specific wins that rustc cannot see (defer-drop, clone elimination decided by the solver)
as IR passes with a benchmark each (`benches/`, `benchmarks/`), and publish numbers only from CI. The 15-phase text moves to
the archive or is rewritten as "history".

## Consequences
- README/ROADMAP drop performance claims (done in this change); OPTIMIZATION_ARCHITECTURE.md and COMPILER_OPTIMIZATIONS.md need a "not active" banner or a rewrite.
- `wj optimize` / `wj economics` (WJ-PERF-01) remain "Not started".
- Plan WP 5 (dead-code hygiene) can delete `src/optimizer/`.

## Decision needed from: project owner
Choose A, B or C.

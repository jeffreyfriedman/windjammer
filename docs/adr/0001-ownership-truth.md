# ADR-0001: Ownership truth: IR solver vs analyzer

- **Status:** Proposed
- **Date:** 2026-10-10
- **Related:** [IR_SOLVER_CODEGEN_ARCHITECTURE.md](../IR_SOLVER_CODEGEN_ARCHITECTURE.md), [ARCHITECTURE_IMPROVEMENT_PLAN.md](../ARCHITECTURE_IMPROVEMENT_PLAN.md) WP 2, 8, 9; `src/ir/solver.rs`, `src/ir/constraint_gen.rs`, `src/analyzer/`

## Context
Windjammer's central promise is that the compiler infers ownership (`&`, `&mut`, owned) and clones. Today two
engines claim that job:

- `src/analyzer/` (about 35 files: `parameter_analysis`, `self_analysis`, `borrow_analysis`, `passthrough_inference`,
  `mutation_detection`, ...) produces `AnalyzedFunction`s and signatures.
- `src/ir/` (`constraint_gen`, `solver`, `coercion`, `target_encodings`) is a union-find constraint solver. The
  CHANGELOG for 0.50.0 says all cutover flags default to `true`, and `IrCutoverConfig::from_env()` always sets `call_sites: true`.

Despite the cutover, the analyzer still runs first and codegen/`compiler` still patch results afterwards
(for example `multipass_bare_pass_demotion.rs`, post-IR `reconcile_post_ir_*` helpers, string-level `.clone()` fixes).
The architecture audit found ownership decided in at least five places and clones in at least three. The IR doc's
claim "the IR constraint solver is the single source of truth" is therefore true for call-site coercion on the Rust
backend only. Go and JavaScript call `prepare_ir_context` and discard the result (`let _ir = ...` in
`src/codegen/go/generator/mod.rs` and `src/codegen/javascript/mod.rs`).

## Options
### A. Solver is the single writer (end state: `src/analyzer` is facts only)
The analyzer is reduced to collecting facts (mutation, moves, field access) that feed `constraint_gen`. Only
`OwnershipSolver::solve` writes ownership and clone decisions. Codegen prints. Demotion/promotion passes are deleted.
- Pros: one place to debug; backends can share it; matches plan principles 1 and 4.
- Cons: long serial migration (WP 7, 8, 9) touching the same files; risk to dogfood output; needs the golden corpus (WP 0).

### B. Analyzer stays authoritative, IR is a validator
Keep `src/ir/shadow.rs` as a parity checker and treat the analyzer as truth.
- Pros: least churn.
- Cons: preserves the two-engine problem and every Rust-only heuristic; Go/JS never get correct ownership.

### C. Two engines by backend
Solver for Rust, analyzer for the rest. Rejected: it freezes the divergence the conformance suite is meant to catch.

## Recommendation
Option A. Define the end state explicitly: `src/analyzer/` keeps only fact collection; its ownership-deciding files
are deleted behind golden-corpus checks; `src/ir/` owns ownership, clone insertion and numeric width (ADR-0001 covers
the first two; numeric width follows plan WP 8). Reject any new heuristic "demotion/promotion" pass.

## Consequences
- Fitness tests forbid new writers of parameter ownership outside `src/ir`.
- Docs: IR_SOLVER_CODEGEN_ARCHITECTURE.md must say which backends consume the IR (only Rust today).
- Go/JS backends either consume the IR (ADR-0007) or are tiered down.
- Large but mechanical deletions in `src/analyzer`, `src/type_inference`, `src/codegen/rust/*call_site*`.

## Decision needed from: project owner
Confirm Option A, and confirm that the analyzer may be reduced to fact collection (not merely "frozen").

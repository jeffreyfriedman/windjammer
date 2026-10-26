# src/ir/

## Purpose
Safety-typed intermediate representation between analyzer and codegen: `IrModule`/`IrNode` with `SafetyType` (ownership, effects, taint, execution mode), plus the unified constraint solver and the numeric solver.

## Inputs/Outputs
In: analyzed program (`analyzer`) and `parser` AST. Out: `IrModule` with solved ownership/effect/taint/execution data and `IrDiagnostic`s.

## Invariants
- Every compilation runs the pipeline; solvers must be deterministic and idempotent.
- The IR is meant to be the single owner of ownership/clone/numeric decisions (`solver.rs`; the unused `numeric_solver.rs` was deleted in C4, widths now flow through the unified solver and the legacy collectors until WP 8 finishes), but legacy analyzer/codegen paths still decide too.
- The stdlib tables (effect declarations, taint sources/sinks) depend on the stdlib registry only: a session computes them once (`stdlib_tables::StdlibTables`) and passes them to its pipelines (`IrPipeline::with_stdlib_tables`); no process-global memo. Gate: tests/stdlib_tables_test.rs.
- `ir` must not depend on `codegen`; the current imports are tracked debt (WP 4: neutral `ir_contract`).

## Owns
IR node types, lowering (`lower.rs`), constraints/solver, effects, taint, capability profiles/lock, execution mode, numeric bridge/solver, coercion encoding.

## Semantic facts (ADR-0017)
`liveness.rs` (`last_uses` / `last_moves`, `dying_in`, `read_after`, `captures`, by AST node identity; the Rust backend's reuse clones and closure capture mode obey them, C6 MOVE-LAST-USE / CLONE-USE-AFTER-MOVE / CLOS-MOVE-CAPTURE) and `numeric_defaults.rs` (NUM-DEFAULT-INT, by the literal's source position) are target-independent facts stored on `IrFunction`; backends consume them and never re-decide. Both are deliberately conservative: a use they do not model opens the class / blocks the claim.

## Effect facts (ADR-0021)
`effect_facts/`: the capability model (`model` = `crates/windjammer-runtime/src/caps_core`, ONE source compiled into both the compiler and the runtime: vocabulary, path/host/name matching, grants), THE sink table (`sinks`; the interpreter gate reads it, nothing else classifies effects), `wj.toml` parsing (`manifest`), per-package grants and the lock file (`packages`, `lock`), the pass over `resolve` output (`analysis`: per-function effect sets with provenance, latent parameters, open calls), `[policy]` (`policy`), findings (`violations`), what a compiled program needs (`install`: proven vs run-time guard) and the `wj explain-effects` views (`report`). Tests: unit tests inline; `tests/effects_{escape,soundness,packages,compiled,vectors,runtime_audit}_test.rs`, `tests/spec/effects/`.

## Reporting
`explain/` (`wj explain-ownership`) only reports: it re-reads the constraint set (with provenance rule IDs, `ConstraintOrigin`), the IR result and the emitted Rust, and shows where analyzer / solver / IR / codegen disagree. It never feeds a decision back. Rule IDs: `explain/rules.rs` <-> `docs/spec/inference.md`.

## Never decides
Text emission for a target, parsing, file IO.

## Allowed imports
Layer `ir`. May import: every module in a lower layer (support < syntax < resolve < types < ownership). Known violations (tracked in docs/architecture-allowlist.toml, may only shrink): `codegen`, `compiler`.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`IrPipeline::new`/`with_config`, `lower_to_ir`, `lower_multi_file_to_ir` (src/ir/pipeline.rs); `IrNode` types in node.rs.

## Tests
tests/ir_*_test.rs (call-site coercion, cutover) and inline solver tests.
Run: `cargo test --release --test all <filter>`; architecture rules: `cargo test --release --test all architecture`.

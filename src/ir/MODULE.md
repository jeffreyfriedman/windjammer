# src/ir/

## Purpose
Safety-typed intermediate representation between analyzer and codegen: `IrModule`/`IrNode` with `SafetyType` (ownership, effects, taint, execution mode), plus the unified constraint solver and the numeric solver.

## Inputs/Outputs
In: analyzed program (`analyzer`) and `parser` AST. Out: `IrModule` with solved ownership/effect/taint/execution data and `IrDiagnostic`s.

## Invariants
- Every compilation runs the pipeline; solvers must be deterministic and idempotent.
- The IR is meant to be the single owner of ownership/clone/numeric decisions (`solver.rs`, `numeric_solver.rs`), but legacy analyzer/codegen paths still decide too.
- `ir` must not depend on `codegen`; the current imports are tracked debt (WP 4: neutral `ir_contract`).

## Owns
IR node types, lowering (`lower.rs`), constraints/solver, effects, taint, capability profiles/lock, execution mode, numeric bridge/solver, coercion encoding.

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

---
status: proposed
last_verified: b77ea8a63
---
# ADR-0029: One type owner for widths; `usize` stays a language type

- **Status:** Proposed
- **Date:** 2026-11-03 (revised the same day after the owner's decision)
- **Supersedes (if accepted):** nothing; it extends [ADR-0028](0028-usize-class-in-the-numeric-solver.md), which stays in force
- **Related:** [ADR-0015](0015-language-consistency-proposals.md) section 1, [ADR-0005](0005-error-strategy.md) (rustc is the
  type checker of last resort), [ADR-0017](0017-semantic-ir-for-all-backends.md), [ADR-0026](0026-rust-crate-dependencies.md)
  (`.wji` surfaces), `docs/design/numeric-width-redesign.md` (evidence, options, section 7 the staged plan),
  `docs/spec/convergence-burndown.md` (W2-W12)

## Context

The numeric solver decides integer widths but has no type facts (closure parameters, collection element types, std
argument and result types, receiver types, cross-module fields), so legacy codegen deciders W2-W12 keep deciding next to
it and `D-WIDTH-OWNERS` stays at 58. The first draft of this ADR proposed removing `usize` from the surface (option B)
and letting rustc type unsuffixed literals (D2). The owner rejected removing or hiding a language feature to avoid
inference: `usize` stays a first-class, explicit type. The D2 experiment (`wt/c4-exp`) did not meet its bar.

## Decision proposed

1. **`usize` stays.** Declared `usize` is an exact type; ADR-0028's index class (a literal-only class that meets an
   index slot or a length is `usize`) stays. Whether unconstrained integers default to `int` remains an open question.
2. **One type owner** (option C, `src/typing/`). Static types come from declarations only: signatures, struct and enum
   fields, the Windjammer std (`std/**/*.wj`), Rust crate surfaces (`rust/<crate>.wji`) and a typed surface of the Rust
   core types (`std/rust_core.wji`). A bidirectional checker computes the type of every expression; the numeric
   solver, codegen and ownership read those types instead of re-deriving them.
3. **Staged, each stage deletes** (design review section 7): C1 typed core surface feeding the solver; C2 checker in
   report-only mode with an untyped-expression ratchet and a rustc agreement oracle; C3 the solver reads it and the
   legacy width deciders are deleted one per step; C4 codegen reads it (`infer_expression_type` and the type-guessing
   files go); C5 ownership reads receiver types.

## Gates at every stage

Golden rustc rejects may only fall; narrowing casts may only fall; `D-WIDTH-OWNERS` and the width-ratcheted tests may
only fall; `scripts/ecosystem_width_gate.wj` and the compiled health sweep stay green; every core-surface entry agrees
with rustc (`tests/core_surface_rustc_test.rs`).

## Kill criterion

Stop and redesign a stage that cannot delete what it lists without a new golden reject or a red ecosystem target, or
(from C2 on) whose checker disagrees with rustc on more than 1% of corpus expressions.

## Consequences

- Programs: no source change. Emitted Rust converges on the widths the program declared and the index class.
- Diagnostics: type errors can be reported in Windjammer terms before rustc (C2 onward).
- The interpreter's resolved engine and the Go/JS backends can read the same types (ADR-0017).
- Cost: 8-15k lines added over C1-C5 against 6-8k deleted beyond the deciders; C1 is small (a surface file, a module,
  a solver hook).

## Decision needed from: project owner

Accept option C as the width path, staged C1-C5 with the gates and kill criterion above.

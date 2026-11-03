---
status: proposed
last_verified: e0ec926dc
---
# ADR-0029: `int` at the surface; rustc types the literals

- **Status:** Proposed
- **Date:** 2026-11-03
- **Supersedes (if accepted):** [ADR-0028](0028-usize-class-in-the-numeric-solver.md) sections 1-4 (the inferred `usize` class)
- **Related:** [ADR-0015](0015-language-consistency-proposals.md) section 1 (int = i64; indexing and `len()` take and give `int`
  through a checked conversion), [ADR-0005](0005-error-strategy.md) (rustc is the type checker of last resort),
  [ADR-0017](0017-semantic-ir-for-all-backends.md), [ADR-0026](0026-rust-crate-dependencies.md) (`.wji` surfaces),
  `docs/design/numeric-width-redesign.md` (evidence and options), `docs/spec/convergence-burndown.md` (W2-W12)

## Context

The numeric solver decides integer widths, including an inferred `usize` class (ADR-0028). To decide a width it needs
facts it does not have (closure parameters, collection element types, std argument and result types, receiver types,
cross-module fields), so legacy codegen deciders W2-W12 keep deciding next to it. Publishing widths for classes the
solver cannot see regressed real code twice (nine ecosystem targets; 31 new rustc rejects in the publish-defaulted
experiment) and the C4 kill criterion fired with `D-WIDTH-OWNERS` stuck at 58. The design review measured: 18 golden
programs rustc rejects for a numeric width; 359 `_i32` suffixes emitted in a language whose default is i64 (29 goldens
emit `i32` without ever mentioning it); stripping every literal suffix by regex makes rustc accept 7 more goldens and
breaks 6, all in two syntactic shapes; LLVM emits identical code for `usize` and `i64`-plus-checked-conversion index
loops.

## Decision proposed

1. **No `usize` in the language surface.** `len()`, `count()`, `capacity()`, `position()`, `enumerate()` indices and
   every std or crate result that Rust spells `usize` are `int`. Indexing, slicing and every std or crate formal that
   Rust spells `usize` accept `int`, converting with a checked conversion; a negative value is the defined panic
   `index -1 is negative` in every engine. Declared `usize` in user code stays an exact type (FFI, layout) without
   implicit conversion. In a `.wji` surface, `usize` denotes an index slot (`int` to the caller).
2. **Boundary rule in the Rust backend, not inference.** Sources print `(x.len() as i64)` (or a prelude trait such
   as `WjLen` when the receiver is not statically a std collection); sinks print `wj_ix(e)`, a generic checked helper
   over any integer type, so a sink needs no width knowledge. Literals at a sink print unsuffixed.
3. **Literal verdicts.** The solver classifies each literal's class as *exact* (a declared or suffixed width meets
   it), *closed* (every use is modelled and none has a width: printed `_i64`) or *open* (some use the solver cannot
   see: printed **unsuffixed**, so rustc infers the width from that use). A literal-only class used as a method
   receiver prints the default suffix. Float literals print unsuffixed (rustc's fallback is f64, Windjammer's
   default). No codegen decider may choose a literal width.
4. **Interpreter.** `len()` and the other sources return `int`; usize arithmetic leaves the interpreter's slow path.
   The ADR-0028 underflow divergence (`v.len() - 1` on an empty vector) disappears: both engines compute `-1`.

## Migration (each step deletes a decider; see the design review, section 4)

S1 floats unsuffixed (delete `type_inference/float_inference/`), S2 integer verdicts (delete W2, W3 peer logic, W4,
W6 paints, `int_inference/`, fold W9), S3 usize boundary (delete W5, the ADR-0028 solver machinery,
`int_implicit_casts.rs`, usize branches of W3/W7/W12), S4 `D-WIDTH-OWNERS` ratcheted to 0. Gates at every step:
rustc rejects may only fall, narrowing casts may only fall, `scripts/ecosystem_width_gate.wj` 9/9, drift differential
corpus green.

## Kill criterion

Stop and fall back (open classes keep the legacy decision; option C of the review, one type owner, becomes the path)
if the S0 experiment or any step produces more than 3 newly rejected goldens that need a type fact the two syntactic
rules cannot supply, or an interpreter/compiled overflow divergence caused by rustc's `i32` fallback in an open class.

## Consequences

- Programs: no source change except code that relied on `usize` wrap-around; `std/strings.wj` and `std/json.wj`
  `len` become `-> int`. Emitted Rust loses most suffixes and every invented `i32`; conversions appear only as
  `wj_ix(..)` at sinks and `as i64` at sources.
- Diagnostics: width errors name only widths the program declared; an ambiguous literal becomes "annotate this
  binding" with a machine-applicable fix.
- Go and JS backends already treat lengths as `int`; the portable lowering stops needing index widths.
- `D-WIDTH-OWNERS` is redefined: an open verdict that the backend prints unsuffixed counts as agreement.

## Decision needed from: project owner

1. Accept section 1 (no `usize` at the surface, ADR-0015 section 1 applied literally) and supersede ADR-0028?
2. Approve the S0 experiment (one release build, env-gated, no deletions) as the go/no-go for S1-S4?

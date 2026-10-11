# ADR-0016: Pins: the surface for overriding inference

- **Status:** Proposed
- **Date:** 2026-10-11
- **Related:** [ADR-0001](0001-ownership-truth.md), [ADR-0002](0002-local-mutability.md), [ADR-0003](0003-owned-keyword.md), [ADR-0015](0015-language-consistency-proposals.md), `docs/spec/inference.md`, `docs/spec/inference-divergences.md`

## Context
Inference is the language's promise, but an agent or user sometimes needs control: a public API whose signature must
not change when a body changes (D-OWN-PUB shows the compiler already guesses this from `pub`), an FFI boundary, a
hot path where a clone is unacceptable, a width that must be exact. Today the controls are: `&T` / `&mut T` /
`&self` / `&mut self` on a signature (parsed into `OwnershipHint`; honored only as a solver seed, and buggy:
D-PIN-DOUBLE-REF, D-PIN-NOT-HONORED), declared numeric types and literal suffixes (exact, NUM-DECLARED). There is no way
to demand ownership (ADR-0003 is Proposed, `owned` is not a token), to forbid a clone, or to assert the *inferred* result.

## Decision proposed
Three pins, all constraints on the solver (never a second decision path), all reported by `wj explain-ownership`:

1. **Mode pins.** `&T`, `&mut T` stay as they are. Add the decorator form `@owned` on a parameter
   (`fn put(@owned key: Vec<u8>)`), which is ADR-0003 option C without a new keyword. Semantics: the pin is a *lower
   bound* in the lattice (OWN-PIN-*). If the body needs more, it is an error at the annotation; if the body needs less,
   the pin wins (this is the point of a pin). Pins on `pub` signatures replace the visibility heuristic of D-OWN-PUB.
2. **Assertions.** `@expect(ref)` / `@expect(mut)` / `@expect(owned)` on a parameter: no effect on inference, but the
   build fails with the explain output if the inferred mode differs. This gives agents a way to lock an API without
   changing it, and gives CI a check that a refactor did not change the contract.
3. **No-clone assertion.** `@no_clone` on a function: the build fails (listing the sites) if codegen would insert any
   clone in that function. Width pins need nothing new: declared types and suffixes are exact (ADR-0015).

## Consequences
- Parser: one decorator argument form; `OwnershipHint::Owned` already exists. Checker: pin vs inferred comparison in the
  IR (single place), reported with rule IDs.
- Must follow, not precede: D-PIN-DOUBLE-REF and D-PIN-NOT-HONORED fixed (a pin that miscompiles is worse than none),
  and ADR-0001 (the solver as sole owner), otherwise `@owned` is one more of five deciders.
- Not implemented in this change: it needs the owner's decision on ADR-0003 first.

## Decision needed from: project owner
Accept 1-3, or restrict to `@expect` (assertions only), which needs no solver change and is the smallest useful step.

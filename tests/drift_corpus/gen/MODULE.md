# tests/drift_corpus/gen

## Purpose
The fixed prelude of the generated value-kind x position programs of the drift gate
(`scripts/wj/drift_gen.wj`, `docs/testing/DRIFT.md`).

## Inputs / outputs
- In: `prelude.wj`, the enum and `show_*` functions every generated program starts with.
- Out: nothing; the generator writes programs to a scratch directory.

## Layout
`prelude.wj` only. Kinds, positions and the seeded choice live in the generator.

## Invariants
- Every integer literal in a generated cell goes through `num(..)`.
- A kind provides: its type, a hit value, a miss value, a rebuild expression that matches the value and rebuilds it,
  and a `show_*` function. A position wraps those four.

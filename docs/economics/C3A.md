---
status: proposal
---
# C3a: read-only aggregate parameters become borrows (design note for the ownership/solver stream)

> **Proposal: not implemented.** A design note for the ownership and solver stream; the compiler does not do this yet.

- **Related:** [ADVANTAGES.md](ADVANTAGES.md) C3, [ADR-0001](../adr/0001-ownership-truth.md) (the solver decides
  ownership), `src/ir/constraint_gen.rs` (`OWN-ANALYZER-SEED`), [PASSES.md](../optimization/PASSES.md) (soundness rules)

## The gap, measured

The generated `dx` code has 156 `.clone()` against 12 in the Rust twins. `wj explain-ownership` over the 78 suite
programs: 298 parameters, 150 emitted owned; 40 owned non-Copy parameters have no consuming use, and every one is
`OWN-ANALYZER-SEED` ("analyzer pre-decided ... ADR-0001 says the solver should decide"). The callers then clone:
`skip_level(people.clone(), ..)` copies a whole `HashMap` per query (t033); `less(self.data[i].clone(), ..)` clones two
entries per heap comparison (t012); `pair_before(v[j].clone(), v[j - 1].clone())` per sort step (t018, t055).

## Rule

A parameter `p: T`, `T` not Copy, is solved `Ref` when every use of `p` in the body is a read:

- field or index read (`p.f`, `p[i]`, `p.0`), comparison, formatting;
- an argument to a callee whose formal for it is solved `Ref` (fixed point over the call graph);
- a method call whose receiver mode is `&self` (from the signature registry);

and none of: a move (returned, stored in a struct/collection/closure, passed to an owned formal, matched by value with
a binding that moves a non-Copy field), a mutation, `&mut`, or an escape into a thread/closure.

Codegen then emits `p: &T` and the call sites pass `&x` instead of `x.clone()` (the existing call-site coercion of the
solver: `CoercionKind::Borrow`).

## Soundness

- Accepted by rustc whenever the owned version was: the body only reads through `p`, and every read is valid through a
  shared reference; callees that read keep their `Ref` formals (fixed point).
- Behaviour: the only observable difference is that the clone is no longer made and dropped. That is invisible unless
  `T` (or a type it contains) has a user `impl Drop` or a side-effecting `impl Clone`: the rule is refused for such
  types (the type registry knows user `impl Drop`/`impl Clone`).
- Interpreter agreement: the interpreter already passes values without copying semantics visible to the program.

## Where it goes

In the solver, as a constraint instead of the analyzer seed: `constraint_gen` emits `OwnershipIs(p, Ref)` with origin
`OWN-READ-ONLY-AGGREGATE` when the use facts say read-only, and the `OWN-ANALYZER-SEED` constraint for that parameter is
not emitted. No codegen heuristic, no text pass.

## Gates

- Clone ratchet: `tests/optimizer_ratchet_test.rs` corpus plus the `dx` generated clone count from `economics.wj`
  (156 today; target at most 20).
- Fixtures: a read-only `HashMap` parameter (borrowed, caller does not clone); a parameter returned (stays owned); a
  parameter stored into a `Vec` (stays owned); a type with `impl Drop` (stays owned).
- Mutation: a variant that borrows a stored parameter must fail to compile in the fixture suite.
- D5/D4 rows of `economics.wj` on `dx` with realistic inputs (the sample inputs are too small to show the clones).

## Effort

About 3 agent-days in the solver stream: the use-fact collection exists (`OWN-READ-FIELD`, `OWN-CALL-FORMAL`), the work
is the fixed point over callees, the Drop/Clone refusal, and replacing the seed.

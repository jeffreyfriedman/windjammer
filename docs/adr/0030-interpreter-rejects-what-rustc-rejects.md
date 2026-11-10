---
status: proposed
last_verified: 132cc5e76
---
# ADR-0030: The interpreter refuses what the compiled build refuses, from the generated Rust

- **Status:** Proposed (first slice landed: uses after a by-value receiver, WJ0007)
- **Date:** 2026-11-03
- **Related:** [ADR-0020](0020-interpreter-execution-engine.md) (interpreted-first), [ADR-0005](0005-error-strategy.md) (rustc is the
  type checker of last resort), `docs/drift/findings.md`, `docs/drift/pilot4-findings.md`, `src/ir/explain/` (`wj explain-ownership`)

## Context

`wj run --interpret` runs only programs `wj check` accepts. Even so, the interpreter is more permissive than the compiled
build in one systematic way: it has no ownership. Values are shared trees, so a program that moves a value and uses it
again prints an answer, and the same program fails to build (rustc E0382). The drift ledger and agent pilot 4 show this
is the case that holds interpreted-first back. An agent iterates happily in the interpreter and meets the failure only
at the final `wj build`, far from the edit that caused it.

The cases where the interpreter accepts and rustc rejects fall into three classes:

| Class | Example | Who is wrong | Right fix |
|---|---|---|---|
| Ownership: use after move (E0382), including a `self`-by-value receiver used again | `let m = n.advance(); n.peek()` | the program | refuse before running (this ADR) |
| Borrow conflicts (E0499/E0502/E0506) | mutating a vector while a reference into it is live | the program | refuse before running (later slices) |
| Code-generation defects (integer literal widths, `collect` into `Vec<string>`, `&str` vs `String`, `&mut &mut`) | `docs/drift/findings.md` A, B, D | the compiler | fix the backend; the interpreter is right |

Only the first two are language rules the interpreter should enforce. The third must not be imitated.

Ownership in Windjammer is inferred. Whether a call moves its argument, whether a receiver is taken by value, where a
clone is inserted automatically, and which types are `Copy` are all compiler decisions. They are made in stages
(analyzer seed, IR solver, pipeline adjustments, and the code generator's own overrides; `wj explain-ownership` shows
that the stages disagree). A checker that re-derives them would drift from the compiler the same way the interpreter
does now.

## Decision proposed

1. **Predict rustc's ownership errors from the generated Rust, not from a parallel analysis.**
   - The prediction runs the code generator in-process (no cargo, no rustc; about 20 ms for a small file) and reads the
     result with the same reader `explain-ownership` uses (`ir::explain::emitted`).
   - It reports only what the generated program would fail on: a move that the compiler did not turn into a clone,
     followed by a use, of a type the generated code does not derive `Copy` for.
   - This shares every compiler decision by construction, including the ones that are not in any facts table.
2. **Report it as the existing WJ0007** ("Value used after it was moved", today mapped from rustc E0382 after a build).
   - The report carries the move's location, the callee's by-value receiver, and a fix: use the result, reorder, or clone.
3. **Where it runs.**
   - First slice: in the interpreted-run gate (`cli::check::require_clean`), for single-file programs, only when `wj check`
     is otherwise clean.
   - Next: `wj check` itself, cached with the check cache, so agents see it without running.
   - Projects: generate the project the way `wj build --no-cargo` does.
4. **Precision over recall.** A wrong refusal stops a correct program, so every class lands only with a pinned
   zero-false-positive scan (`tests/move_prediction_test.rs`). Ambiguous shapes stay unreported: a later reassignment
   re-arms the variable, and a receiver held by reference counts as a reborrow.
5. **Classes, in order.** Each is one slice with its own kill criterion:
   - a. by-value receiver used again (landed);
   - b. a bare variable passed to an owned formal and used again (`CALL-MOVE` with `used_later`, which
     `explain-ownership` already computes);
   - c. a move inside a loop of a variable declared outside it;
   - d. partial moves (a field moved out, then the whole value used);
   - e. borrow conflicts, which need lifetimes of references in the generated code. This one may not fit the approach:
     the alternative is item 6.
6. **Exact fallback (measured, not adopted):** `rustc --emit=metadata` on the generated crate. It is exact for every
   class, but takes about 1 s or more per program. It may suit a planned deep mode of `wj check` or a background check, not every run.

## First slice (landed)

- `ir/explain/emitted.rs` now records a method call's bare receiver, whole reassignments, and the types derived `Copy`.
- `ir/explain/moves.rs` (`receiver_moves`) finds `v.m()` where the emitted `Type::m` takes `self` by value, `v` is an
  owned non-`Copy` local, and `v` is read again before any reassignment.
- `check/moves.rs` turns each hit into WJ0007 with a span on `v.m(`.
- The interpreted-run gate refuses the program. the `self_by_value_reuse` case of `tests/move_prediction_test.rs` now stops before
  running:

  ```
  error[WJ0007]: `n` is used after `n.advance()` moved it (`Node::advance` takes `self` by value)
  ```

- Precision: across every `.wj` file under `examples/` and `tests/` (2184 files), the only prediction is that repro.
  The pinned list in `tests/move_prediction_test.rs` fails on any new one.
- Mutation check: the "read again" condition replaced by `true` fails the scan.
- Cost: the generator runs on every interpreted single-file run that passes `wj check`. On `bench/interp/structs.wj` it
  adds about 20 ms of CPU (0.01 to 0.03 s user; 0.02 to 0.05 s wall).

## Second slice (landed): classes b, c and d, in `wj check`

- **What is predicted:**
  - b: a bare variable passed to a parameter the emitted callee takes by value;
  - b': `let w = v`;
  - d: `let w = v.f` followed by a use of `v` as a whole;
  - c: any of these inside a loop that does not declare `v`.
  - Each needs a type the emitted code spells and that is known to move (`String`, collections, `Box`/`Rc`/`Arc`,
    non-`Copy` structs of the file, `Option` of those).
- **Where it runs:** in `wj check`'s per-file analysis phase, so the check memo keeps it. The interpreted-run gate
  reports it as one more check error.
- **What the scan found first, and the rules that removed it:**
  - A method name was matched to a user method of the same name (`HashMap::get` taken for a user `get`). Method
    callees are now resolved by the receiver's spelled type.
  - A use on another arm of an `if`/`match`, or behind a `return`, `break` or `continue`, was counted as a later use
    (`return parse_alias(&trimmed, expr)` followed by more code). The emitted reader now records branch paths and
    exits.
  - Uses inside a closure were counted as uses inside the loop around it.
- **Precision:**
  - The scan over `examples/` and `tests/` reports the three fixtures `tests/diagnostics_oracle/check/wj0007_*.wj` (annotated for the span oracle).
  - It also reports two programs rustc rejects with E0382 on their generated code: `tests/error_system/test_ownership.wj`,
    an ownership-error fixture, and `tests/inference_corpus/string_literal_move_then_use.wj`, the compiler defect
    D-CLONE-STRLIT, where the compiler emits no clone where one is needed.
  - The ecosystem files of the drift corpus have no prediction.
  - `tests/engine_parity/map_get_match.wj` copied a map with `let snapshot = m` and used `m` afterwards. Its compiled
    build already failed on that line; it now copies explicitly.
- **Recall check:** the auto-cloned shapes (`tests/move_prediction/accepted/`) build, and nothing is reported for them.

## Kill criteria

- **Any false positive** (a program the compiled build accepts that the prediction refuses) that one revision of the
  class cannot remove means that class is dropped. The interpreter then keeps running such programs, and the drift
  ledger keeps them.
- **Cost:** more than 50 ms added to the start-up of a small single-file run, or more than 10% of `wj check` time on the
  ecosystem, means the prediction moves behind the check cache, or behind a planned deep mode of `wj check` only.
- **Coverage:** if after classes a-d the drift ledger's ownership rows and pilot 4's ownership-shaped failures are not
  at least halved, stop extending this approach and evaluate the rustc metadata check (item 6) as the default instead.

## Consequences

- Interpreted-first gets a refusal at the line of the move, instead of a build failure later. The compiled-only
  failures that remain are code-generation defects, which the drift gate lists for the backend to fix.
- A program the interpreter used to run may now be refused. It is exactly one the compiled build refuses too, which is
  the interpreter's contract (`require_clean`: "the interpreter runs only programs the compiler accepts").
- The prediction depends on the emitted Rust reader. Changes to the generated code's shape (receiver spelling, clone
  placement) are visible to it at once, and the precision scan catches the ones that change its answers.

## Decision needed from: project owner

1. Adopt the approach (predict from the generated Rust, refuse with WJ0007) for classes a-d.
2. Should the prediction run in `wj check` too, so a check without a run reports it? This is proposed as the next
   slice, cached.

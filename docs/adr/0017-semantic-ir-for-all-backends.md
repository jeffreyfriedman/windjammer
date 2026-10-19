---
status: proposed
last_verified: 2006232625ab
---
# ADR-0017: One semantic layer, consumed by every backend

- **Status:** Proposed
- **Date:** 2026-10-11
- **Related:** [ADR-0001](0001-ownership-truth.md), [ADR-0002](0002-local-mutability.md), [ADR-0007](0007-target-tiers-conformance.md), [ADR-0015](0015-language-consistency-proposals.md), [ADR-0016](0016-ownership-pins.md), `docs/spec/inference.md`, `docs/spec/convergence-plan.md`, `src/ir/`, `src/codegen/portable/`

## Context
ADR-0001 makes the IR solver the only writer of ownership decisions. The convergence plan (C0-C7) executes that
for the Rust backend. Two observations widen the scope:

1. The facts the solver needs to decide Rust ownership well (who mutates a value, whether a value escapes or is
   aliased, where a variable is used for the last time, which numeric width a value has) are not Rust facts. They are
   properties of the Windjammer program.
2. The Go and JS/TS emitters (`src/codegen/portable`) re-derive a subset of the same facts on their own: parameter
   modes come from the analyzer registry, every owned use of an aggregate place is wrapped in a defensive `PK::Copy`,
   and integer widths are inferred by the portable lowering. When the Rust path gets smarter, they do not.

Today one parameter's mode is decided in four places (D-OWNERS-OWN) and the clone decision of the solver is never
read (D-CLONE-OWNERS). Fixing that for Rust only would leave the same duplication between Rust and the portable layer.

## Decision proposed
A single, target-independent **semantic layer** (the IR solver plus the fact passes it reads) produces a typed
**semantic facts** interface. Backends consume it; no backend re-decides a fact the interface carries.

### The facts (the backend contract)
Keyed by stable identity (function `DefId` / parameter index / binding id / expression node id; the current
(line, column) keys are a transition state, see convergence plan C2/C3), never by bare names:

| Fact | Meaning | Producer |
|---|---|---|
| `param_mode(fn, i)` | `Ref` / `MutRef` / `Owned` (OWN-* rules, pins as lower bounds) | IR solver |
| `receiver_mode(fn)` | same for `self` (SELF-* rules) | IR solver |
| `mutated(binding)` | the binding (or a path rooted at it) is written | fact pass |
| `escapes(binding)` | the value is stored, returned or captured by an escaping closure | fact pass |
| `lent_mut(binding)` | the binding is passed to a `&mut` formal or receiver (GC targets need a cell/pointer representation for scalars, e.g. `&mut int` in Go/JS) | IR solver (call-site formals) |
| `consumed(binding, site)` | the use moves the value (owned formal, a `self`-by-value method including generic trait methods such as `Into::into`) | IR solver |
| `last_use(binding, site)` | no later read of the binding on any path after `site` | liveness pass |
| `needs_copy(site)` | the use at `site` must observe an independent value (CLONE-* rules) | IR solver |
| `width(expr)` | resolved numeric type (ADR-0015 defaults: `int`=i64, `float`=f64; explicit widths exact) | numeric solver |
| `conversions(expr)` | checked conversions the solver placed (ADR-0015 #2) | numeric solver |

Invariant (money-type correctness): no backend inserts a narrowing numeric conversion that is not in `conversions`.
If a backend would need one, the semantic layer reports a diagnostic (NUM-MIXED) instead; silent truncation of an
`int` is never an acceptable realisation.

Requests for new facts from a backend stream (for example the portable-gaps stream asking for `lent_mut`) are
added to this table and produced by the semantic layer; the backend does not derive them locally.

### What each backend may and must do
* **Must** read every decision it emits from the facts: Rust prints `&`/`&mut`/owned and `.clone()` exactly where the
  facts say; Go/JS/TS print a copy exactly where `needs_copy` holds.
* **May** use a fact to omit work that is unobservable under value semantics: no defensive copy when the value is
  unaliased at that point (`last_use` or `!escapes`), `const`/`Object.freeze`/read-only views when `!mutated`,
  native numbers instead of `BigInt` when `width` proves the range, skipping a bounds check when an index is proven
  in range.
* **Must not** contradict a fact: a backend that cannot honor a fact (for example, a target with no borrows) must
  choose a realisation with the same observable behavior, never a different decision.
* **Must not** add a decision path of its own (no `promote_*` / `demote_*` passes, no string-level `.clone()` surgery,
  no width heuristics). A missing fact is added to the semantic layer, with a spec rule and a test.

### Conformance
`tests/backend_conformance_test.rs` and the convergence gates (`tests/convergence_gates_test.rs`) are the contract:
for every conformance program, every backend's output equals the interpreter's (independent oracle), and the facts
each backend consumed are the same object. A backend optimisation that relies on a fact must have a conformance
program where the fact is false (so the conservative path is exercised too).

## Aim, not promise
This ADR records an internal aim: improvements made for world-class Rust output should reach the other targets for
free (alias/escape freedom -> fewer copies, immutability -> `const`/frozen, last use -> move instead of copy, width
facts -> correct overflow and `BigInt` choices). It is not an external commitment about Go/JS/TS output quality.

## Sequencing
After convergence-plan C5/C6 (the solver is the single owner for Rust). First portable consumers: `needs_copy` /
`last_use` (copies), `mutated` (immutability), `width` (integer representation). Each change ships with conformance
programs that prove no behavior change and a measurement (copies emitted over the conformance corpus).

## Status of the implementation (2026-10-12)
* `last_use` exists (`src/ir/liveness.rs`, keyed by AST node identity) and is stored on `IrFunction::last_uses`.
  The portable lowering consumes it: a `let`-bound or by-value local is moved, not copied, at its last use
  (`src/codegen/portable/lower/mod.rs::own`). Measured by `tests/semantic_facts_portable_test.rs`
  (ratchet `tests/golden/portable_copies.txt`); behavior checked by the backend conformance suite.
* `width` for unconstrained integer literals exists (`src/ir/numeric_defaults.rs`, NUM-DEFAULT-INT) and the Rust
  backend consumes it; the IR solver's integer default is `int` (i64).
* Not yet: `param_mode`/`receiver_mode` from the solver (the portable layer still reads the analyzer registry until
  convergence-plan C5), `mutated`, `escapes`, `lent_mut`, `consumed`, `needs_copy` per site (C6).

## Consequences
* `src/codegen/portable/env.rs` stops reading parameter modes from the analyzer registry and reads the facts.
* The analyzer becomes fact collection only (ADR-0001 option A); fitness tests forbid decision writers outside `src/ir`.
* The facts interface is versioned with the IR; changing a fact's meaning is an ADR-level change.

## Decision needed from: project owner
Accept the contract (facts + may/must/must-not rules), or limit it to the Rust backend (then the portable layer stays
on the analyzer registry and keeps its own copies).

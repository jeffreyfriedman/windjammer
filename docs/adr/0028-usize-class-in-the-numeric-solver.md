---
status: accepted
last_verified: e5f389872
---
# ADR-0028: An index class (`usize`) in the numeric solver

- **Status:** Accepted (implementation in progress, convergence step C4)
- **Date:** 2026-11-02
- **Related:** [ADR-0015](0015-language-consistency-proposals.md) (int = i64, checked conversions), ADR-0001 (one owner per decision),
  `docs/spec/convergence-plan.md` (C4), `docs/spec/convergence-burndown.md` (W2-W6, kill criterion),
  `docs/spec/inference.md` section 7, `src/ir/solver.rs`, `src/ir/constraint_gen.rs`, `src/ir/pipeline.rs`,
  `src/codegen/rust/usize_expression_type_inference.rs`, `src/codegen/rust/binary_expression_generation.rs`,
  `src/codegen/rust/int_promotion_type_inference.rs`, `src/codegen/rust/let_statement_generation.rs`,
  `tests/interpreter_corpus/len_index_arith.wj`

## Context

The IR numeric solver (`src/ir/solver.rs`) decides integer widths by union-find over the classes that `let`,
assignment, arithmetic, comparison, returns and call formals connect. It had no notion of `usize`:
`parser_type_to_base_type` mapped a declared `usize` to `U64`, `len()`/`count()` results were untyped fresh variables,
and an index operand only received `IsNumeric`. Every class that contained a length or an index therefore defaulted
to `int` (i64) in the solver, while the Rust backend, which must emit `usize` for those values, decided the width
with four legacy deciders that disagree with each other and with the solver:

| decider | what it decides |
|---|---|
| W3 `binary_expression_generation.rs` peer logic | operand widths of `a < b`, `a + b` (`prefer_i32`, `comparison_should_prefer_*`) |
| W4 `int_promotion_type_inference.rs` | whether a compare promotes to i32 or u32 |
| W5 `usize_expression_type_inference.rs` | whether an expression is `usize`, when to cast it to int |
| W6 `let_statement_generation.rs` paints | width of an untyped `let` from peers, counters and `while` bounds |

`D-WIDTH-OWNERS` (the explain meter, gate 3) counts locals whose solver width differs from the emitted width. At
`e5f389872` it is 81 after the spelling fix; the two dominant shapes are `u64 -> usize` (no usize class) and
`i64 -> i32` (a legacy paint). Publishing the solver's literal widths for every literal was tried and made rustc
reject a number of golden programs, because the classes that contain `len()` were wrongly `int`.

## Decision

### 1. The lattice

`BaseType` gains `Usize` (a declared `usize`, the `usize` literal suffix) and `Isize`. They are distinct from
`U64`/`I64`: the backend spells them differently and Rust does not unify them.

An integer class (union-find class with an integer constraint) resolves to the first that applies:

1. **Hard width.** Any member has an exact type: declared (`NUM-DECLARED`), literal suffix, `as T` (`NUM-CAST`),
   field type (`NUM-FIELD-DECLARED`), callee formal or result (`TY-CALL-RESULT`). Two different hard widths in one
   class are a type error (unchanged).
2. **Index class (`NUM-USIZE`).** Otherwise, if any member is an *index source* or an *index use*, the class is
   `usize`. This is a solved width, not a default: it is published as a fact.
3. **Default (`NUM-DEFAULT-INT`).** Otherwise `int` (i64), marked defaulted.

Index sources: the result of `len()`, `count()`, `capacity()` called on a receiver for which no user signature
resolves (a user method named `len` keeps its declared return type). Index uses: the operand of `v[i]`. The new
constraint is `PrefersUsize(var)`; it is soft, so it never conflicts with a hard width.

Consequences of the order: `let mut i = 0; while i < v.len() { ... i = i + 1 }` puts `i`, `0`, `1`, `i + 1` and
`v.len()` in one class with no hard width, so all are `usize`. `let n: int = v.len()` puts `n` and `v.len()` in one
class whose hard width is i64, so the class is i64 and the `len()` result is converted.

### 2. Where conversions are inserted (`NUM-USIZE-TO-INT`)

A conversion is inserted **at the site where a `usize` value enters a slot of another width**, and the solver is
the only one who decides it:

- **Source conversion.** An index source whose class resolved to a width `W != usize` is converted where it is
  produced: `v.len()` emits `(v.len() as W)`. Fact: `IrFunction::usize_conversions` (source position -> `W`).
- **Use conversion.** An index use whose class resolved to `W != usize` is converted at the index: `v[i as usize]`.
  The Rust backend already does this for every non-`usize` index; no new fact is needed until W5 is deleted.
- **Store conversion.** A `usize` class stored into a collection whose element type is a different integer width
  (`out.push(i + 1)` with `out: Vec<int>`), where the store does not unify the classes (the solver does not model
  generic element types), is converted at the argument. Fact: same map, keyed by the argument's position.

A `usize -> i64` conversion is spelled `as i64` by the Rust backend (lossless for any length a 64-bit host can
hold). A `usize -> i32` or narrower conversion counts in gate 4 (compiler-inserted narrowing casts); the solver only
produces one when the program declared that narrow width.

### 3. How codegen consumes facts

The solver publishes, per function, keyed by source position (`numeric_defaults::literal_key`; the AST may be
re-allocated between analysis and codegen, so arena addresses are not stable keys):

| fact | content | replaces |
|---|---|---|
| `literal_widths` | every integer literal whose class is not defaulted, any width incl. `usize` | W2 legacy collector, W3 literal peer logic |
| `solver_local_widths` | single-binding locals whose class is not defaulted, any width | `solver_i64_locals` (kept as a view), W6 paints |
| `usize_conversions` | positions of index sources / stored values that need `as W` | W5 `maybe_cast_usize_to_int_target` |

Rules for the backend: a fact, when present, wins and is printed verbatim; a legacy decider may run only where no
fact exists. Every legacy decider is gated behind "no fact" first, then deleted once the corpus shows it never
fires.

### 4. Interaction with `int` = i64 (ADR-0015)

ADR-0015 says `len()` returns `int` through a checked conversion. This ADR keeps that as the *language* rule and
treats `usize` as the backend representation of a value that never met an `int`: a class made only of lengths,
indices, counters and literals has no observable width (its values are non-negative lengths), so it stays in the
target's native index type and no conversion is printed. The moment such a class meets a declared `int` (a
parameter, a field, a return type, a `Vec<int>`), the class is `int` and the conversion sits at the `len()` or the
store, exactly as ADR-0015 says. Declared `usize` is unchanged (FFI and layout).

### 5. Interpreter agreement

The interpreter tags every integer with its width (`IntTy`, value held as i128); `len()` returns `Usize`, an
unsuffixed literal is `Lit` and adopts the other operand's width, and overflow is checked against the tagged width.

**Observable agreement rule.** For a program in which every integer intermediate fits both the interpreter's tagged
width and the width the solver chose, the interpreter and the compiled program print the same output. The class
rule above is chosen so that the solver's width equals the interpreter's tag for the shapes it models (`len()`
classes are `usize` in both; a class with a declared `int` is i64 in both, the interpreter converting at the
operation and the compiled program at the site). Known divergence, out of scope here: a `usize` underflow
(`v.len() - 1` on an empty vector) is an overflow error in the interpreter and wraps in a compiled release build
(the generated crate does not enable `overflow-checks`).

Gate: `tests/interpreter_corpus/len_index_arith.wj` (lengths, index arithmetic, a counter pushed into `Vec<int>`,
a length stored in an `int` field), checked against an independent reference model and against the compiled
program by `tests/interpreter_differential_test.rs`.

### 6. Migration order (each step deletes code or lowers `D-WIDTH-OWNERS`)

1. `BaseType::Usize`/`Isize`, declared `usize` maps to `Usize`; `len()`/`count()`/indexing typed (`PrefersUsize`);
   class resolution of section 1; `width_label` prints `usize`. Lowers `D-WIDTH-OWNERS` (the `u64 -> usize` shape).
2. Publish `literal_widths` for every non-defaulted width and `solver_local_widths`; codegen reads them first.
   Goldens may change; rustc rejects may only fall. Lowers `D-WIDTH-OWNERS` (the `i64 -> i32` paints lose).
3. `usize_conversions` (source and store); gate `maybe_cast_usize_to_int_target` and its helpers behind the fact,
   then delete W5's decision helpers.
4. Gate the `prefer_i32` / `comparison_should_prefer_*` thicket in `binary_expression_generation.rs` (W3) behind
   operand facts; delete.
5. Gate and delete the W6 let paints, then W4 (`int_promotion_type_inference.rs`).

### 7. Kill criterion

Hard stop: two consecutive steps that neither delete code nor lower the corrected `D-WIDTH-OWNERS`, or any step that
raises `tests/golden/rustc_rejects.txt` or the gate 4 narrowing-cast count. Additionally, if after step 2 more than
10% of the corpus locals still disagree for reasons that are not legacy bugs, the class rules above are wrong and the
stream stops for re-derivation (convergence plan C4 kill criterion).

## Consequences

- The solver's report (`wj explain-ownership`) shows `usize` for length and index classes and names `NUM-USIZE`.
- Codegen loses the right to decide widths wherever a fact exists; the legacy files shrink step by step.
- User programs: no source change. Emitted Rust loses some casts (classes that are really `usize`) and gains
  `as i64` exactly where a length meets a declared `int`.

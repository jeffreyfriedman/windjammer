# Spec-oracle findings

The language spec (`docs/spec/*.md`, executable examples in `tests/spec/**`) is an oracle for the three execution
modes: `scripts/wj/spec_oracle.wj` runs every spec case that has an expected output under the tree walker, the resolved
engine and the compiled backend and judges each run against the case's own `// expect:` lines (stdout and exit status;
for `mode: panic`, exit status and the `expect-error:` substring). Compiled results are recorded in
`tests/spec_oracle/compiled_baseline.tsv` (re-record with `--compiled live --record`; one `cargo build --keep-going`
of a scratch workspace with one binary per case, dev profile, so overflow checks are on as in `wj run --compiled`).
The shrink-only ledger of cases that diverge is `tests/spec_oracle/known_divergences.txt`; the drift gate runs the
oracle as its `spec` step (`scripts/wj/drift_gate.wj`).

Direction of every divergence below: the walker and the resolved engine agree with the spec; the compiled backend
does not build the program (the spec cases are short, so each case file is its own minimal repro). Fixing one turns
its ledger line stale: delete the line.

## Fixed by this work

| Case | Finding |
|---|---|
| `SPEC-EXEC-010` | Struct literal fields were evaluated in the struct's declaration order by both interpreter engines; Rust (and the spec) evaluates them in the order written (`S { b: f(), a: g() }` runs `f` first). The walker also re-evaluated a `..base` expression once per missing field. Fixed in `eval_struct_literal_in` and the resolved engine's struct-literal plan. |

## Coverage

`wj run --interpret scripts/wj/spec_oracle.wj wj out.tsv --report` prints the table below (cases per spec area, cases with
an expected output that the oracle compares, and how many of those ran in all three modes). 293 spec cases in 27 areas;
228 are comparable (the others check parse errors, `wj check` results, effect reports, simulations and replays, which
have no stdout to compare across engines and stay with `tests/spec_conformance_test.rs`); all 228 ran in all three
modes (100%). Walker and resolved engine agree with the spec on all 228. The compiled backend builds and agrees on 197;
31 do not build (listed below, one ledger line each).

| Area | Cases | Compared | Compiled builds |
|---|---|---|---|
| arguments | 9 | 3 | 3 |
| attributes | 4 | 4 | 2 |
| closures | 9 | 5 | 3 |
| collections | 12 | 12 | 7 |
| concurrency | 5 | 5 | 1 |
| doctests | 3 | 0 | |
| effects | 39 | 1 | 1 |
| errors | 7 | 7 | 7 |
| execution | 11 | 11 | 11 |
| expressions | 19 | 18 | 17 |
| grammar | 8 | 7 | 6 |
| guard-match | 4 | 2 | 2 |
| intrinsics | 9 | 9 | 8 |
| items | 11 | 10 | 7 |
| lexical | 16 | 15 | 15 |
| modules | 6 | 5 | 4 |
| numeric | 27 | 26 | 26 |
| optionals | 6 | 5 | 3 |
| ownership | 11 | 11 | 11 |
| patterns | 12 | 12 | 11 |
| pipe | 7 | 6 | 6 |
| statements | 11 | 10 | 9 |
| stdlib | 5 | 5 | 3 |
| strings | 17 | 17 | 17 |
| trailing-blocks | 4 | 3 | 3 |
| traits | 11 | 11 | 7 |
| types | 10 | 8 | 7 |

Spec areas the oracle added cases for (the spec had none, or none that ran compiled): integer wrapping/saturating/checked
operations, casts, shifts and bit operations, shift-amount and division overflow panics, integer division and Euclidean
helpers, float display and rounding, number parsing, string byte offsets and boundary panics, non-ASCII string
operations, format specifiers, Debug and pretty-Debug output, evaluation order, shadowing, and drop order (scopes,
shadowing, assignment, fields, early return, loops, `Option::take`, collections, closures). Of the 23 added cases 19
agree in all three modes and four (`NUM-025`, `NUM-026`, `EXEC-012`, `STR-017`) expose compiled-backend build failures.

## Closed since the first record

After the typed integer and float core surface landed (`wt/c4-optc`), `SPEC-NUM-001` (negated minimum literal),
`SPEC-NUM-025` and `SPEC-NUM-026` (numeric methods on literal receivers, `min`/`max`/`clamp`, `rotate_left`) and
`SPEC-STD-005` build and agree compiled; their ledger lines are gone (38 -> 34). `SPEC-NUM-017` now fails at the class-B
overflow lint instead of the literal width.

## Closed since the second record

With the generated crates allowing the constant-overflow lints (`arithmetic_overflow`, `unconditional_panic`), class B is
closed: `SPEC-NUM-002`, `003`, `004`, `006` and `017` build and panic at run time as specified. `SPEC-ERR-003`,
`SPEC-ERR-005`, `SPEC-EXEC-012` and `SPEC-STR-017` also build now. For the last two the expected output was taken from an
equivalent Rust program while the backend could not build them; the compiled output now matches it byte for byte.
Ledger: 34 -> 25, then +6 new second-round cases -> 31.

## Compiled backend does not build these spec cases

### A. Integer literal and method-argument widths (rustc sees `i32` where the spec type is `int`)

| Case | rustc says |
|---|---|
| `collections/SPEC-COLL-004` | `unique_by(\|x\| x % 2)`: `i32` element against `i64` literal |
| `stdlib/SPEC-STD-003` | `i32` against `i64` in arithmetic |
| `statements/SPEC-STMT-009`, `items/SPEC-ITEM-006`, `items/SPEC-ITEM-011`, `expressions/SPEC-EXPR-011` | `i32` / `i64` mixed in arithmetic or arguments |
| `intrinsics/SPEC-INTR-007` | `usize` expected, `i32` found |
| `grammar/SPEC-GRAM-006` | `Vec<i64>` collected from an iterator of `usize` |

### B. A statically known panic is a compile error in rustc (closed)

`numeric/SPEC-NUM-002` (`i64::MAX + 1`), `SPEC-NUM-017` (`-m` of the minimum value), `SPEC-NUM-003` (`u8::MAX + 1`), `SPEC-NUM-004` (`3u32 - 5`), `SPEC-NUM-006`
(`10 % 0`): rustc's deny-by-default `arithmetic_overflow` and `unconditional_panic` lints reject the generated
program ("this arithmetic operation will overflow"). The spec says these panic at run time with exit status 101.
The new `SPEC-NUM-021` and `SPEC-NUM-023` hide the operand behind a `parse` call and pass in all three modes, which
shows that the run-time behaviour is right and the lint is the whole difference. Suggested direction: emit
`#![allow(arithmetic_overflow, unconditional_panic)]` in generated crates (the panic is the specified behaviour).

### C. Emitted Rust is ill-typed or not borrow-clean for an ordinary program

| Case | rustc says |
|---|---|
| `traits/SPEC-TRT-001` | expected `String`, found `&str` |
| `items/SPEC-ITEM-010` | expected type parameter `T`, found `&T` |
| `traits/SPEC-TRT-004` | `V + V` in a generic function without an `Add` bound |
| `collections/SPEC-COLL-005`, `SPEC-COLL-009` | lifetime may not live long enough |
| `closures/SPEC-CLOS-001` | `Vec<User>` has no `filter` (an iterator adaptor on a list; the call needs `.iter()`) |
| `types/SPEC-TYP-006` | `Ok([1])` against `Result<Vec<i64>, _>`: array where a `Vec` is expected |
| `patterns/SPEC-PAT-009` | non-exhaustive integer range patterns: rustc does not see that the ranges and the wildcard cover `i64` |
| `stdlib/SPEC-STD-002` | `wj build` refuses the program (W0010, `str` in a signature) while `wj run --interpret --no-lint` accepts it |

### D. Optional chaining `?.` is refused by the Rust backend

`optionals/SPEC-OPT-005`, `SPEC-OPT-006`: "optional chaining `?.` is not supported by the Rust backend yet (it needs the
IR solver's receiver type)".

### E. Attributes and concurrency forms the compiled build does not link

`attributes/SPEC-ATTR-001`, `SPEC-ATTR-004` (unknown attributes `custom_tag`, `route`), `concurrency/SPEC-CONC-001`
to `SPEC-CONC-003` (no `tokio` crate in the generated manifest; `__wj_scope` helper missing; `wait` on an async block),
`SPEC-CONC-004` (a `recv` Result printed with `{}`).

## Observation outside the oracle

`wj check` rejects `i64::checked_abs` (WJ0015, unknown method) although it exists in Rust's std: the surface table
(`stdlib_surface::known_methods`) lacks it. The spec case `SPEC-NUM-018` uses `checked_neg` instead.

## Second round: thin areas (traits, patterns, closures, errors, modules, slices, iterators)

Nine cases added (`TRT-010`, `TRT-011`, `PAT-011`, `PAT-012`, `CLOS-009`, `ERR-007`, `MOD-006`, `COLL-011`, `COLL-012`).
`ERR-007`, `PAT-011` and `PAT-012` build compiled and agree in all three modes (expected output from the compiled
program). The other six do not build compiled; their expected output is that of an equivalent hand-written Rust
program (checked with rustc), and the walker and resolved engine print exactly that output:

| Case | rustc says |
|---|---|
| `traits/SPEC-TRT-010` | default method `fn name(self) -> string { "shape" }` is emitted returning `&str` against `-> String` (E0308) |
| `traits/SPEC-TRT-011` | `impl Describe for string`: `"hi".describe()` finds no method on `&'static str` (the impl is for `String`); `impl ... for int` with a `self` receiver mismatches |
| `closures/SPEC-CLOS-009` | `Box<dyn Fn(int) -> int>` is emitted with a zero-argument closure type (E0593, the known `dyn_fn_box_return` finding); `let f = \|x: int\| x + base` with a shadowed `base` mixes `i32` and `i64` |
| `modules/SPEC-MOD-006` | a `pub const LIMIT` inside `mod util` is not emitted (E0425 for `LIMIT` and `util::LIMIT`) |
| `collections/SPEC-COLL-011` | integer widths (`Vec<i64>` collected from `i32` and `usize` iterators; `&i32 as i32`) |
| `collections/SPEC-COLL-012` | integer widths in closures (`cannot multiply i32 by i64`, `&i64 + i32`) |

Language-surface gaps met while writing the cases (these are limits of the language today, not oracle findings): no
`..=` in index expressions or after `MIN` in patterns, no slice patterns (`[first, .., last]`), no `x @ pat` bindings,
and a one-line `if c { CONST } else { y }` parses `{ CONST }` as a struct literal. `wj check` also rejects (WJ0002,
"cannot find function `g`") a closure that returns a closure over its own parameter,
`let compose = |f: fn(int) -> int, g: fn(int) -> int| move |x: int| g(f(x))`, which is valid.

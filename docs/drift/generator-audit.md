# Differential generator audit: value positions

Audit date 2026-11-02 against `next/v0.50.0`. Question: which *kinds of values in which positions* do the
differential corpora and generators exercise? The bare-`None` arm bug (`match m.get(k) { Some(v) => Some(v),
None => None }` evaluated to `()` on a miss in the resolved engine) passed the parity corpus because every
`None` arm there produced a literal, never the value `None`.

## What exists

| Source | Programs | Value kinds it produces |
|---|---|---|
| `tests/interpreter_jit_loops_test.rs`, `interpreter_jit_differential_test.rs` (generated) | seeded | integers of every width, `f64`, `bool`, `Vec` elements, struct fields; loop control (`break`/`continue`/nested), overflow, zero divisors, out-of-bounds. No Option, Result, enum, String, closure or map value anywhere. |
| `tests/interpreter_packed_vec_test.rs` (generated) | seeded | packed vs boxed vectors of scalars |
| `tests/interpreter_corpus` (74 files, seeded) | hand-written | 2 of them mention `Some`/`None`; none has an `=> None` arm |
| `tests/conformance_programs` + `examples/verified` (370 files) | hand-written | `=> None` arm in 1 file, `Err(e) => Err(e)` in none, `return Some/None/Ok/Err` in 37, `break <value>` in 14, labelled loops in 1 |
| `tests/engine_parity` (34 files before this change) | hand-written | `map_get_match`, `option_arm_values`, `match_arms`, `condvar_guard_match` only |

## Gaps (before)

| Value position | Covered? |
|---|---|
| Option as an arm result (`None => None`, `Some(v) => Some(v)`) | one program (added with the fix) |
| Result as an arm result, `Err(e) => Err(e)` | none |
| user enum (unit, tuple, struct variant) as arm result / return / struct field / Vec element | partly (`13_enum_unit_match`, `14_enum_data_match`), never with bare variant names as patterns in value position except the fix's program |
| nested matches (match as arm result, as `let` initialiser, as call argument, inside a binary expression, `Option<Option<_>>`) | none |
| lookup misses (map get/remove, `first`/`last`/`pop` on empty, `find`/`position` without a match, string `find`, parse errors) used as values | only the fix's program (map get) |
| shadowing (parameter, loop variable, match binding, retyping) | `33_shadow_and_blocks`, `77_shadowing_blocks` (scalar) |
| early `return` of Option/Result/enum values from inside nested loops | partly (`return Some/None` in 37 programs, none nested in `while` inside `for`) |
| captures of Option/enum/Vec values by closures; closures returning Option; closures in a Vec | `16`, `17`, `63`, `174` (scalar captures only) |
| Option-typed struct fields read, rewritten and matched in loops | none |

## Cases added (`tests/engine_parity/vp_*.wj`)

Each program prints its results with `println` only, runs under the walker, the resolved engine and the compiled
backend (`scripts/wj/drift_corpus.wj`), and also under the existing `interpreter_engine_parity_test` (walker vs
resolved, with and without eager machine-code loops) because it lives in `tests/engine_parity`.

| Program | Positions |
|---|---|
| `vp_result_values` | `Ok`/`Err` as arm results, `Err(e) => Err(e)`, `?` inside loops, `return Err` from `while`, Results in a `Vec` and a struct field |
| `vp_enum_values` | unit/tuple/struct variants as arm results, returns from `for`, struct fields, `Vec` elements, closure captures, bare variant names as patterns |
| `vp_nested_matches` | match as arm result, as `let` initialiser, as an operand of `+`, as a call argument; `Option<Option<Option<_>>>`; tuple scrutinees |
| `vp_collection_misses` | map `get`/`remove`/`contains_key` misses, `first`/`last`/`pop` on empty, `find`/`position` misses, string `find`, `parse` failures, `and_then`/`map`/`unwrap_or` on them, lookups in a loop |
| `vp_shadowing` | parameter, loop variable (nested), match binding, retyping, block value, closure parameter |
| `vp_loop_exits` | `return` of enum/Option/Result from nested `while`/`for`/`loop`, labelled `break`/`continue`, `break <value>` |
| `vp_closure_captures` | captured `Option`/enum/`Vec`, closures returning Option, closures called through their bindings, a captured name shadowed later |
| `vp_option_fields` | Option fields read in methods, rewritten each iteration, matched inside loops, `ok_or`, `map` |

The generator (`scripts/wj/drift_gen.wj`, docs/testing/DRIFT.md) is that rung: the cross product of five value kinds
and ten positions, hit and miss, seeded; the walker is the oracle for the resolved engine and rustc for the compiled
backend. Seeds 1..200 found no walker/resolved divergence on the current engine; rustc flagged one compiled-side defect
(a bare unit-variant pattern in a value-position match, now `repros/bare_unit_pattern_in_let_match.wj`).

## Mutation checks

| Gate | Mutant | Result |
|---|---|---|
| differential corpus, walker vs resolved | remove the fix of the bare-`None` bug (`names.retain(..)` in `src/interpreter/resolved/matching.rs`) and rebuild | the ecosystem tests of `wj-config`, `wj-cookie`, `wj-dotenv` and `wj-toml` (12 tests, 24 rows with their panic-location rows) diverge: walker `pass`, resolved `fail` at the package's `get` helper. `vp_collection_misses` diverges too: walker `exit0`, resolved `exit101` |
| differential corpus, compiled arm | none needed: the 14 repros of `docs/drift/findings.md` and 61 compiled-red ecosystem tests are real divergences the corpus reports until they are ledgered | each ledger line is `known`, not hidden |
| drift budget (ledger) | `scripts/wj/drift_selftest.wj` plants a new, changed, vanished, grown and vacuous result | 11 of 11 checks, each fails the ledger comparer for its own reason |
| `#runs` guard | a program that does not parse (an `option_binding_plus_literal.wj` with its first line joined to a comment) agreed with itself in all three modes and passed the corpus | found while recording; `<program>#runs` rows now fail such a program |
| generator (`drift_gen.wj`), walker vs resolved | the bare-`None` fix removed again (kind `Option<int>` from a map lookup) | 32 of the 40 default programs diverge (walker exit 0, resolved exit 101 at the lookup), 64 rows; with the fix, 0 of 200 seeds |

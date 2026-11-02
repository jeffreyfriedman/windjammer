# Drift findings

Divergences the drift gate found between the interpreter (walker, resolved engine) and the compiled backend.
Each has a minimal program under `tests/drift_corpus/repros/` (hand-minimised against the compiled output with
`wj build --no-cargo` plus `rustc --emit=metadata`; `wj reduce` attempts are noted below), runs in the differential
corpus as a case, and is listed in `tests/drift_corpus/known_divergences.txt`. Fixing one turns its ledger line
stale: the gate then fails until the line is deleted.

Direction of every finding below: the interpreter accepts and runs the program; the compiled backend's generated
Rust is rejected by rustc (a compile-time failure users would see as a Windjammer diagnostic, `WJ0003`), so
"interpreted green, compiled red" for programs of this shape.

## A. Integer literal typing (compiled backend infers `i32`, the declared type is `int` = `i64`)

One family, nine sites. The interpreter has one integer type; the compiled backend picks `i32` for an
unannotated literal in positions where the surrounding type is not yet known, and rustc rejects the mix.

| Repro | Site |
|---|---|
| `option_binding_plus_literal` | `match local_opt { Some(b) => Some(b + 1) ... }` where `local_opt: Option<int>` is a local variable |
| `shadow_match_binding_literal` | `let item = item + 1` in the arm that binds `item` |
| `shadowed_option_literal_arms` | `let item = Some(5)`; later `match item { Some(item) => item * 3, None => 0 }` (arms get `i32` and `i64`) |
| `literal_in_match_operand` | `(match inner(Some(7)) { Some(v) => v, None => 0 }) + 1`: literal in `Some(..)` and arm literal |
| `literal_in_option_field_arm` | `let nx = match self.next { Some(v) => if v > 2 { Some(v - 1) } else { None }, ... }` in a method |
| `literal_option_through_closure` | `let maybe = Some(7)`, `let get = \|\| maybe`, `show(get())` with `show(x: Option<int>)` |
| `closure_returns_literal_option` | closure `\|flag\| if flag { Some(1) } else { None }` passed to an `Option<int>` parameter |
| `enum_variant_literal_payload` | `let t = Tag::B(4)` where `B(int)`. **Fixed** by the usize-class merge (tuple-variant payloads have their declared types): the case now agrees in all three modes and stays as a regression case |
| `let_literal_captured_by_closure` | `let five = 5` used in `\|x: int\| x + five` |

Suggested fix direction: literals take the type of the expected/unified type, defaulting to the language's `int`
(`i64`) rather than Rust's `i32`, at every site that today leaves the literal unconstrained.

## B. Other compiled-backend findings

| Repro | Finding |
|---|---|
| `dyn_fn_box_return` | `fn adder(n: int) -> Box<dyn Fn(int) -> int>` with `Box::new(move \|x\| x + n)`: the generated callable type takes zero arguments (E0593) |
| `nested_vec_index_by_value` | `let row = grid[r]` on a `Vec<Vec<int>>` parameter: generated `let row: Vec<i64> = &grid[r]` (E0308); needs a clone or a borrowed binding |
| `map_get_loop_str` | `m.get(k)` with `k` the loop variable over a list of string literals: `get(&k)` is `&&str` against `HashMap<String, _>` (E0277) |
| `closure_string_param_literal` | `\|who: string\| ...` called with a string literal: parameter `String`, argument `&str` (E0308) |
| `self_by_value_reuse` | a `self`-by-value method that moves fields, then the value is used again: rustc E0382. The interpreter copies and prints; the compiled backend moves. A use-after-move check in the front end (or an auto-clone) would close it |

## C. Regression at the integrated head: the usize class (ADR-0028)

Found by the first gate run after merging `next/v0.50.0` at `fec181b1a`. Before that merge the compiled side of
`packages/wj-compress`, `wj-table`, `wj-timefmt`, `wj-toml`, `wj-uuid` and `apps/wj-auth-api`, `wj-chat`,
`wj-notes-api`, `wj-proxy` was green; afterwards each fails to compile (263 test cases). The interpreter is right and
unchanged.

| Repro | Site |
|---|---|
| `len_minus_literal_returned_as_int` | `parts.len() - 1` returned from `-> int`: generated `parts.len() - 1_i64` (E0308, `usize - i64`); `wj-uuid` `hyphen_count` |
| `int_compared_with_len_and_indexed` | `int` loop variable in `c >= 0 && c < out.len()` then `out[c]`: generated `c >= 0_usize` (E0308); `wj-table` `right_align` |
| (not reduced) | `-1` as the tail of an `int` function that returns a loop index: generated `-1_usize` (E0600); `wj-toml`, `wj-compress`, `wj-timefmt` |

Fixing the compiler turns those 263 ledger lines stale at once; delete them with the fix.

## Tooling notes

- `wj reduce --oracle 'rustc-error E0308:<regex>'` reports "does not hold on the input project" for programs whose
  generated Rust does contain that rustc error; plain `rustc-error E0382` reduced `vp_option_fields` to a different
  E0382 shape than the original. The repros above were therefore minimised by hand with the cheap oracle
  (`wj build --no-cargo` then `rustc --emit=metadata`).
- The value-position programs in `tests/engine_parity/vp_*.wj` avoid the constructs above (annotated literals,
  no `Box<dyn Fn>`) so that they gate walker == resolved == compiled strictly; the constructs live in the repros.

## Interpreter-side divergences

None at `next/v0.50.0`: walker and resolved engine agree on every case of the corpus. Mutation check
(`docs/drift/generator-audit.md`): with the bare-`None` fix removed, the corpus reports the divergence.

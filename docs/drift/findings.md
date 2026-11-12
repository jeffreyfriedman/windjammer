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
| `literal_in_option_field_arm` | `let nx = match self.next { Some(v) => if v > 2 { Some(v - 1) } else { None }, ... }` in a method. **Fixed** on the ownership branch (the arm literal takes the field's `int` width): the case now agrees in all three modes and stays as a regression case |
| `literal_option_through_closure` | `let maybe = Some(7)`, `let get = \|\| maybe`, `show(get())` with `show(x: Option<int>)` |
| `closure_returns_literal_option` | closure `\|flag\| if flag { Some(1) } else { None }` passed to an `Option<int>` parameter |
| `enum_variant_literal_payload` | `let t = Tag::B(4)` where `B(int)`. **Fixed** by the usize-class merge (tuple-variant payloads have their declared types): the case now agrees in all three modes and stays as a regression case |
| `let_literal_captured_by_closure` | `let five = 5` used in `\|x: int\| x + five` |

Suggested fix direction: literals take the type of the expected/unified type, defaulting to the language's `int`
(`i64`) rather than Rust's `i32`, at every site that today leaves the literal unconstrained.

Until then `wj check` reports the shapes whose widths are certain from the Rust the build emits (WJ0003, ADR-0033),
and an interpreted run warns about them before the program's output: `let_literal_captured_by_closure`,
`shadow_match_binding_literal`, `shadowed_option_literal_arms`,
`tests/conformance_programs/62_vec_alias_semantics.wj` and three value-position programs of `tests/engine_parity`.
Their ledger lines stay: the interpreter still runs them, and the backend defect is still there. Families B (a reference
expected for a lookup) and C (`String` against `&str`) are predicted the same way since ADR-0033's third slice
(`explicit_ref_literal_arg`, `closure_string_param_literal`, `examples/verified/hashset_basics.wj`, ...).

## B. Other compiled-backend findings

| Repro | Finding |
|---|---|
| `dyn_fn_box_return` | `fn adder(n: int) -> Box<dyn Fn(int) -> int>` with `Box::new(move \|x\| x + n)`: the generated callable type takes zero arguments (E0593) |
| `nested_vec_index_by_value` | `let row = grid[r]` on a `Vec<Vec<int>>` parameter: generated `let row: Vec<i64> = &grid[r]` (E0308); needs a clone or a borrowed binding |
| `map_get_loop_str` | `m.get(k)` with `k` the loop variable over a list of string literals: `get(&k)` is `&&str` against `HashMap<String, _>` (E0277) |
| `closure_string_param_literal` | `\|who: string\| ...` called with a string literal: parameter `String`, argument `&str` (E0308) |
| `self_by_value_reuse` | a `self`-by-value method that moves fields, then the value is used again: rustc E0382. The interpreter copies and prints; the compiled backend moves. **Closed**: the interpreted run now refuses it before running with WJ0007, predicted from the generated Rust (ADR-0030); the repro moved to the annotated check corpus (`tests/diagnostics_oracle/check/wj0007_*.wj`) |

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

**Fixed** on `wt/c4-regress`: the numeric solver no longer publishes a width for a class it cannot decide (two exact
widths met, a negated literal in an unsigned class, a loop binding or a field read or a `len()` receiver whose type the
walk cannot see), and a local bound to a call or a module function (`let parts = strings.split(..)`) takes that
callee's declared type. The 263 ledger lines are deleted, the compiled baseline for the nine targets is re-recorded
(all pass), the three shapes are golden-corpus programs (`tests/inference_corpus/usize_*.wj`), and
`scripts/ecosystem_width_gate.wj` runs the nine targets compiled before merge.

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

## F. Regression from the collect fixes (found by the live record on 1dbf700b3)

`wj-chat`, `wj-find`, `wj-taskflow` and `wj-gitignore` (and so 74 ecosystem test cases) fail to compile since the
`collect::<Vec<string>>()` fix landed: `let words: Vec<string> = line.split(" ").collect()` is emitted as
`collect::<Vec<_>>()` into a `Vec<String>` binding (rustc: expected `Vec<String>`, found `Vec<&str>`).
Repro: `collect_annotated_let_split` (test `annotated_collect_of_split_into_vec_string_typechecks`). The interpreter is right.

Also on this head: `nth_cast` is fixed (its ledger line is gone, the test passes); `vp_loop_exits` had lost its
`.clone()` workaround (a lint cleanup elsewhere) and hit `nested_vec_index_by_value`, so it now indexes `grid[r][c]` directly.

## G. Found by the wider corpus, probes and fuzz run (wt/drift4)

| Where | Finding | Direction |
|---|---|---|
| `repros/i64_min_literal.wj` | `let b: i64 = -9223372036854775808`: the interpreter evaluates `9223372036854775808` (it does not fit i64) and panics on the negation; Rust accepts the literal | interpreter bug |
| `repros/u64_literal_above_i64_max.wj` | a `u64` literal above `i64::MAX` is emitted as `-9223372036854775808_u64` (rustc E0600) | compiled-backend bug |
| `repros/param_used_only_in_break_value.wj` | a parameter whose only use is inside the value of a `break` out of `loop` is renamed `_h` and the use then fails to resolve (E0425) | compiled-backend bug |
| `repros/thread_join_coalesce.wj` | `handle.join() ?? 0` emits a zero-argument closure where the Result default takes the error (E0593) | compiled-backend bug |
| `repros/pow_exponent_literal_width.wj`, `shift_count_method_literal_width.wj` | `pow` and `wrapping_shl` take a `u32` count; the literal gets the receiver's width | compiled-backend bug |
| `repros/division_by_known_zero.wj` | a division whose divisor rustc can see is zero stops the compiled build (deny-by-default `unconditional_panic`); the interpreter runs it and panics | policy difference, ledgered |
| `tests/conformance/types/enums_and_matching.wj` | the interpreter refuses a non-exhaustive match (WJ0010); the compiled build accepts the program | decide which is right |
| the other programs of `examples/verified`, `tests/conformance*`, `tests/engine_parity` | the interpreter runs them; the compiled build rejects them (mostly WJ0003 `uint`/`int` width mismatches, e.g. `examples/verified/algo_lcs.wj`) | compiled-backend regressions in the verified corpus; not reduced |

Fixed in this branch: struct literal fields ran in declaration order (rustc: as written; next had the same fix, its version was kept),
`env::current_dir` returned a bare string, `env::current_exe` and `json::number_f64` were missing in the interpreter, and
`strings.split_lines` disagreed between the runtime (a final newline added an empty last element, every `\r` was dropped) and
the interpreter; both now follow `str::lines()` (SPEC-STR-018).

## H. Ownership and `main` results (wt/drift4, later passes)

| Where | Finding | Direction |
|---|---|---|
| `tests/move_prediction/receiver_non_clone.wj` | the interpreter runs a by-value receiver called on a value that cannot be copied and then used again; rustc rejects it (E0382). Its sibling fixtures (argument, let alias, loop, partial move) are refused by both; `move_prediction_test::moves_of_values_that_cannot_be_copied_are_reported_per_kind` fails on it at this head | interpreter does not refuse (ADR-0030 receiver slice) |
| `repros/main_returns_err.wj` | `fn main() -> Result<(), string>` returning `Err` exited 0 silently when interpreted; compiled it prints `Error: "negative"` and exits 1. Fixed in the interpreter (`Error: {Debug of the error}` on stderr, exit 1); the corpus now has an `#mainerr` row comparing that line in every mode | fixed |

## I. Reductions and mutation checks (wt/drift5)

- `repros/explicit_ref_literal_arg.wj`: the explicit borrow of an integer literal (`set.contains(&9)`, `map.get(&3)`) is dropped by the
  compiled backend (rustc: expected `&i64`, found `i64`). Reduced from `hashset_basics`, `map_get_match` and `match_value`.
- `repros/i64_min_literal.wj` is fixed: the parser keeps an integer literal as `i64` bits, so `9223372036854775808` reaches the
  interpreter as `i64::MIN`; the written literal `-9223372036854775808` now evaluates to itself in both engines, and a computed
  `-i64::MIN` still panics (SPEC-NUM-017). The same representation explains `u64_literal_above_i64_max.wj`: the compiled
  backend prints a `u64` literal above `i64::MAX` as a negative number.
- The other interpreted-ok / compiled-rejected programs fall into a few families (rustc's first error): the integer literal
  typed `i32` next to `int`, a `&T` expected where an `int` is passed to a collection lookup or an iterator adapter, `String`
  against `&str` at call boundaries, and iterator chains collected into the wrong element type. Programs that reach std modules
  (`math`, `process`) cannot be checked by rustc alone and need a cargo-backed run.
- `apps/wj-webhook` (23 cases) is the one ecosystem target still red compiled: its generated test crate fails with rustc E0596
  (`cannot borrow `app` as mutable`): a binding that is mutated through a method call is emitted without `mut`. The HMAC
  addition the ledger used to name has landed; `apps/wj-grep` now compiles and its 38 lines are gone.

## L. Ecosystem findings 9-14 and the `mut` receiver (wt/drift7)

Reproduced on `next` (`wj` built from the merge of green7, c2, ownership O4 and the book registry). Each has a neutral repro in
`tests/drift_corpus/repros` (a corpus row, so the ledger names it) or, when it needs a package, in
`tests/drift_corpus/repro_packages`. In each case the interpreter runs the program and the compiled build does not.

| Ecosystem item | Repro | Compiled result | Owner |
|---|---|---|---|
| 9 `picked.contains(&i)` loses the `&` | `repros/vec_contains_local_ref.wj` (+ test `contains_with_a_borrowed_local_typechecks`) | fixed (wt/ownership-7): a formal the declaration spells `&T` keeps the borrow for a Copy `T` | done |
| 10 `env.temp_dir()` missing from the compiled runtime | `repros/env_temp_dir_call.wj` | rustc E0425, no `temp_dir` in the runtime `env` module | runtime agent (add `temp_dir` to the runtime `env`) |
| 11 function-typed parameter calls mis-adapt arguments | `repros/fn_typed_param_call.wj` (+ test) | fixed (wt/ownership-7): a parameter named like a function is the callee, so only its own type adapts the arguments (the repro declares its mutated local `let mut`, ADR-0002) | done |
| 12 db connect under explicit capabilities refused | `repro_packages/db_sqlite_url_capability` | the compiled run cannot get past 13 first; interpreted passes | runtime / effects owner |
| 13 runtime feature groups from library output only | `repro_packages/runtime_feature_from_tests_only` | rustc E0432, `windjammer_runtime::db` configured out | build-pipeline owner (decide features from every compiled crate, tests included) |
| 14 `items(n)?.len()` in a struct literal | `repros/try_then_method_chain.wj` | the lexer reads `?.` as optional chaining: the interpreter reports WJ0015 (`a Result has no method __opt_chain`), the Rust backend refuses `?.` | parser/lexer owner: a `?` followed by `.method()` must be postfix try, then a method call |

Item 12 stays a package repro: the capability check needs a `[capabilities]` grant and the compiled test crate, and item 13
hides it. Re-check it once 13 is fixed.

### The `mut` receiver (wj-prop E0596, reproduced)

The ecosystem report was that the interpreter accepts a mutating method call through a `let` that is not `let mut`. On `next`:

- The interpreter accepting it is the documented state (Q-007, ADR-0002): the language infers `mut` for locals, and the Rust backend
  does that (`let c = Counter {..}; c.bump()` compiles). Making the interpreter refuse it would disagree with the compiler, so that
  is not the fix.
- The real drift is the compiler's inference missing two shapes, both rustc E0596 (`WJ0008` through `wj run --compiled`), both running
  interpreted: a receiver whose method mutates only through another mutating call inside an expression
  (`repros/mut_receiver_transitive_in_expr.wj`: `1 + self.step()`), and a binding taken from a tuple pattern
  (`repros/mut_receiver_tuple_binding.wj`). wj-prop itself compiles and passes on current `next`.
- Decision: fix the inference (`src/codegen/rust/variable_analysis/mut_inference.rs`, the signature registry already knows each
  method's receiver mode from its declaration) so the generated `let` carries `mut`; no warning is needed because the compiled
  program is then correct. `wj check` does not see the problem today (it reports 0 errors); a certain check is possible only if
  inference is kept as is and the declaration rule of ADR-0002 is enforced (Q-007). That is a language decision for the owner: it
  turns every program that relies on inference into an error until `wj fix` inserts the `mut`. If the owner chooses it, the wording
  follows ADR-0033 (error in `wj check`, warning in `wj run --interpret`, stdout and exit status unchanged).
- Owner: ownership agent (a7a9f9cae99e4cd3a), the same hand-off as section J's `test_crate_mut_receiver`.

## M. Ledger burn-down and owners (wt/interp-i7)

The last full gate had 90 divergent corpus cases and 16 spec-oracle divergences, all ledgered, none with an owner. Every
line now carries `| owner: STREAM` (streams in `docs/testing/DRIFT.md`); the comparer and `tests/drift_contract_test.rs`
fail on a line without one. Counts by stream at this change:

| Stream | Corpus lines | Spec lines |
|---|---|---|
| `numeric-width` | see the ledger (`grep 'owner: numeric-width'`) | 1 |
| `codegen-refs`, `codegen-types`, `codegen-ownership` | see the ledger | 12 |
| `runtime-surface`, `parser-lexer`, `language-decision`, `gate-harness`, `interp-literal-typing` | see the ledger | 4 |

Fixed here (interpreter or checker side; compiled behaviour is the reference unless the spec says otherwise):

- `tests/conformance/types/enums_and_matching.wj` and its `#accepted` row: `wj check` (and so the interpreter) reported
  WJ0010 for `match r { Result::Ok(Maybe::Some(v)) => .., Result::Ok(Maybe::None) => .., Result::Err(_) => .. }` over a
  user-defined `Result`, because a variant matched only through refutable payload patterns counted as uncovered. The match
  is exhaustive (SPEC-EXPR-010 asks for exhaustive enum matches; this one is), and the compiled build was right to accept
  it. `rules::gaps` now treats a variant as covered when its payload patterns are variants of one enum that together cover
  it (`tests/check_nested_variant_exhaustive_test.rs`). Both ledger lines are deleted.

New in the gate: the seeded fuzz step (empty ledger `tests/drift_corpus/fuzz_divergences.txt`) and mutation checks for the
integer, float, format, string, sequence, map, option, iterator and JSON builtin classes
(`tests/interpreter_builtin_mutants_test.rs`).

Baselines re-recorded and repros adopted (Linux run, values equal the Mac interpreter signatures):

- Twelve recorded compiled baselines were stale: the compiled output of `literal_in_option_field_arm`, `shadow_match_binding_literal`,
  `shadowed_option_literal_arms`, `map_get_in_sort_by_closure`, `map_get_unwrap_or_float`, `algo_matrix_multiply`,
  `prog_game_of_life`, `result_and_option`, `closures_and_iteration`, `177_opt_chain`, `jit_packed_store` and `packed_vecs` now equals the
  interpreter's, so their ledger lines are gone.
- Six pilot-6 repros joined the corpus (`docs/drift/pilot6-repros/README.md` describes each). `noise_warnings_in_build_output` matches
  and needs no ledger line. The other five are compile failures of the generated Rust (WJ0003) and are ledgered with owners:
  `adapter_into_vec_annotation`, `closure_if_else_ordering` and `closure_map_get_if_else_ordering` (codegen-types); `map_insert_indexed_vec_string`
  and `nested_map_unwrap_or_new` (codegen-refs).

## N. Interpreter correctness fixes (wt/interp-correct)

Compiled behaviour is the reference unless the spec says otherwise; each fix has a test that fails without it.

- **`DirEntry.name` field read.** `std/fs.wj` declares `DirEntry { name, path, is_file, is_dir }` and `Metadata { size, is_file, is_dir,
  is_readonly }`; the interpreter held them as host objects, so a field read panicked (`cannot project Field(name)`) while `.name()`
  worked. `Native::field` now answers `name` and `path` of a `DirEntry` and every `Metadata` field. `DirEntry.is_file` / `is_dir` stay
  methods (they look at the file system again and go through the effect port). The compiled runtime's `DirEntry` keeps its fields private
  (`crates/windjammer-runtime/src/fs.rs`), so a field read is a rustc error there (E0616/E0609): that is a rejection the interpreter
  does not refuse, and the accessor methods remain the portable spelling. Test:
  `tests/interpreter_correctness_test.rs::dir_entry_name_field_reads_like_the_name_method`.
- **`json::parse_string::<T>` and unknown fields.** ADR-0027 denies unknown fields for structs. The compiled derive does that with
  `#[serde(deny_unknown_fields)]` on a struct that has named fields (`item_generation.rs`) and nowhere else. The interpreter denied them
  everywhere, so it rejected input the compiled build accepts: an unknown field inside an enum struct variant, and any field given to a
  struct with no fields. Both now ignore unknown fields (the ADR does not cover variants, so the compiled build decides). A struct
  variant also left the value of an ignored key unread, which would have corrupted the parse; it is skipped now. Test:
  `tests/interpreter_correctness_test.rs::json_unknown_fields_follow_the_compiled_derive`. Open question for the owner: should the
  compiled enum carry `deny_unknown_fields` too (then both engines would deny in variants)? That is a codegen change.
- **`Err` returned from `main`.** The `WJ0171` report had no span and no locals. `main` now notes, as it returns an `Err`, the
  location of the expression that ran last (the `Err(..)`, `return` or `?` that produced it) and the bindings of its frame
  (walker and closure engine); `--error-format json` puts them in `primary_span`, `stack[0].span` and `locals`. Text output is
  unchanged. Tests in `tests/runtime_error_report_test.rs` (`main_err_from_try_*`, `main_err_from_a_tail_err_*`).
- **Reported locals after a machine-code bail-out.** ADR-0036 listed that a block-local binding that already ended may show
  in the locals of a function the JIT bailed out of and the closure engine re-ran. On the current engine it does not: the
  names a compiled function keeps are those of its own scope and a finished block releases its slots. No difference to the
  walker reproduced for blocks, `if`, `for`, `while` and `match` arms; the ADR sentence is now a tested guarantee
  (`runtime_error_report_test.rs::locals_match_the_walkers_across_engines_and_the_jit`).
- **`items(n)?.len()` (finding 14).** On a `Result` receiver the interpreter now reads `?.member` as `?` followed by `.member`, the
  way the compiled build treats it (`Ok(v)` continues with `v.member`, not lifted into an option; `Err(e)` returns). `?.` on an
  `Option` is unchanged (ADR-0018 optional chaining). The lexer/parser still desugars both to `__opt_chain`; a parser-level
  decision between the two needs the receiver type, which the parser does not have, so the interpreter decides at run time.
  The Rust backend now emits the same for a `Result` receiver (`{ let _oc = items(n)?; _oc.len() }`,
  `optional_chain_lowering.rs`); other non-`Option` receivers are still reported. Both engines print `x 3`; the ledger line is
  deleted and the compiled baseline re-recorded (`exit0:4:19307120`).

### N.5 `literal_out_of_range` (design note, ledger line stays)

`fn small() -> u8 { let x = 255; let y = x + 1; y }` panics in the compiled build (`x` is a `u8` because `y` is returned as `u8`;
`255 + 1` overflows, exit 101). The interpreter runs `x + 1` on two untyped literals, gets `256`, and only the return coercion
(`coerce.rs`, `literal out of range for u8`) rejects it as a semantic error, exit 1. The difference is the width of a
literal that is only fixed by a later use. A local fix is not clear: `coerce` cannot tell a source literal (`let a: u8 = 300`,
rejected by rustc too) from a computed one (`x + 1`), because both are `Value::Int(_, Lit)`. The sound fix is a pre-pass that
infers a width for each unannotated integer `let` from its later uses (the same unification the compiler's solver does,
`solver-undecidable-width-classes`), stored in the resolved form so both engines start `x` as `u8`. Smaller step to try first:
tag a `Lit` produced by arithmetic (`Int(_, LitComputed)`) so that `coerce` reports an overflow panic (exit 101,
`attempt to add with overflow`) instead of `literal out of range`; this fixes the exit status but not the case where the sum
fits in the wrong width and silently continues, so it is not a replacement for the pre-pass.

### N.6 Loop variable write-through

`for mut p in ps { p.x = .. }` and `for (id, mut qty) in self.items { qty = .. }` change the collection element in the compiled
build (it iterates by `&mut` when the body changes the variable). The interpreter bound a copy. The parser drops the `mut`
marker of a loop variable (ADR-0002 keeps it for `wj check`), so the interpreter finds the loops by analysis: a `for` over a
place (a variable or field path) whose body assigns, assigns through or mutates a loop variable writes the variable (rebuilt from
its pattern: identifier or tuple) back to its element after each iteration, including on `break`; `self` / a parameter looped
that way counts as mutated (`analysis::loop_var_mutated`). Ranges, temporaries and unchanged variables are untouched, and
the resolved engine runs such a function in the walker so all three engines agree
(`tests/interpreter_correctness_test.rs`, drift case `repros/for_mut_variable_write_through.wj`). Two compiled-side gaps found
on the way (the interpreter now follows the language, the compiled backend does not): `for mut q in qs { q.xs[0] = .. }` is emitted as
`for q in &qs` (rustc E0596) and `for mut row in grid { row[0] = .. }` as `grid.iter().copied()` (E0277 on `Vec`); owner `codegen-ownership`.

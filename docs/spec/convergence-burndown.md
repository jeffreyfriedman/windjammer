# Convergence burn-down: remaining decider sites

Owner rule: every step reports LOC deleted and the ratchet delta. Kill criterion: two consecutive steps that do not
lower `D-WIDTH-OWNERS` stop the stream until the cause is understood. Baseline (tests/golden/explain_counts.txt):
`D-WIDTH-OWNERS` 191 (81 after the 2026-11-02 measurement fix), `D-CLONE-OUTSIDE-SOLVER` 25, `D-OWN-STAGES` 21, `D-CLONE-LASTUSE` 1, `EXPLAIN-ERROR` 5.

LOC figures are file sizes (wc -l) for whole-file deletions and function spans for partial ones; function spans were
measured from `fn` to the next `fn` and over-count inside `impl` blocks, so treat them as upper bounds. "Status":
deleted / replaced-but-still-present (the solver fact exists and wins, the legacy path still runs behind it) /
untouched.

## Width deciders (lowers D-WIDTH-OWNERS)

**Update (ADR-0028, branch wt/c4-usize).** The solver now has a `usize` class (NUM-USIZE: `len()`/`count()`/indexing and
std sequence index arguments) and publishes `literal_widths` for every width, `solver_local_widths`, `index_source_widths`
and `usize_conversions` (NUM-USIZE-TO-INT) from `ir/numeric_facts.rs`; codegen reads them through
`codegen/rust/solver_widths.rs`, and W3 (binary peer logic), W5 (block-tail casts) and W6 (let paints) are gated behind
them. Classes with an index *use* of another width or a shadowed local are still held for the legacy path: shadowed
names share one constraint variable (scoping is the next solver gap).

Solver facts available before ADR-0028: `default_int_literals` (NUM-DEFAULT-INT), `literal_widths` (NUM-LITERAL-WIDTH, literals
beside a `for a..b` variable only), range loop variable typing (TY-UNIFY), solver int local pins used by
`solver_pins_int_literal_init` / `stamp_solver_int_binding`, `usize -> int` conversion at declared-int stores.
Missing facts that block most deletions: solver models for `len()`/`count()` (usize), indexing (usize), `as` casts,
method-call result widths, struct field widths, closure parameters.

| # | site (file:line) | LOC | replaced by | status | lowers |
|---|---|---|---|---|---|
| W1 | `src/ir/pipeline.rs` post-passes for numeric widths (7 passes + while promote) | -- | solver | deleted (earlier C4/C5 steps) | D-WIDTH-OWNERS |
| W2 | `codegen/rust/expression_generation.rs:~960` Int literal arm: legacy collector fallback after the two facts | ~120 | NUM-LITERAL-WIDTH for every literal (needs solver `len()`/index/field/cast models) | replaced-but-still-present (facts win for default-int and range-loop literals) | width owners per literal |
| W3 | `codegen/rust/binary_expression_generation.rs` (peer/promotion thicket: `prefer_i32`, `comparison_should_prefer_i32_over_i64`, `signed_peer_for_zero_sentinel`, usize/i32 mixed arithmetic casts, lines ~320-830) | 1573 file; ~900 in the decision region | solver operand unification + `as` casts at the few real mismatches | untouched | D-WIDTH-OWNERS (largest single source) |
| W4 | `codegen/rust/int_promotion_type_inference.rs` (`expression_promotes_to_i32_in_compare` 185, `expression_promotes_to_u32_in_compare` 375, `scan_while_cond_for_compare_int_peer` 88, `u32_width_from_expr_for_while_peer` 75, `expression_has_i32_width_in_tree` 133) | 2183 | solver unification of comparison operands | untouched | D-WIDTH-OWNERS |
| W5 | `codegen/rust/usize_expression_type_inference.rs` (`index_expr_needs_u32_or_i32_usize_cast` 411, `type_is_signed_int_for_len_usize_comparison` 239, `method_call_rust_emits_usize` 99, `maybe_cast_usize_to_int_target`) | 1159 | solver usize class for `len()`/index + one conversion rule (NUM-USIZE-TO-INT) | replaced-but-still-present: `convert_usize_into_declared_int` now covers declared-int lets and struct fields at stores; return/assign/block already used the same helper | D-WIDTH-OWNERS |
| W6 | `codegen/rust/let_statement_generation.rs` untyped-let width paints (peer scan, counter promotion, `while`-bound scans; ~600 of 1696) | ~600 | solver local pin (`stamp_solver_int_binding` already authoritative when the solver pins) | replaced-but-still-present | D-WIDTH-OWNERS |
| W7 | `codegen/rust/type_casting.rs` (`coerce_arg_str_for_i64_formal`, `coerce_arg_str_for_u32_formal`, `coerce_struct_field_numeric`) | ~330 | call-site formal types from the solver | untouched | narrowing-cast gate |
| W8 | `type_inference/int_inference/mod.rs` + `float_inference/mod.rs` (legacy (line,col) collectors) | 1112 + 750 | solver facts keyed by node identity | untouched; still the fallback behind W2 | D-WIDTH-OWNERS |
| W9 | `ir/numeric_defaults.rs` class analysis (duplicates the solver's IsInteger defaulting) | 561 | solver `defaulted` set (added in this step; `SolverResult::defaulted`) | untouched; fold into solver once literal facts cover every context | none until merged |
| W10 | `type_inference/int_implicit_casts.rs`, `codegen/rust/type_balancing*.rs`, `int_formal_scan_extra.rs`, `std_idiom_fixups.rs`, `closure_unary_constraints.rs`, `program_facts.rs` (ownership-gaps helpers) | ~3000 | solver closure/unary/formal constraints | untouched | D-WIDTH-OWNERS |
| W11 | `codegen/rust/function_formal_parameter_generation.rs` int-formal scans (part of 4913) | ~800 | callee formal types from DefId registry (C3) | untouched | D-WIDTH-OWNERS |
| W12 | `codegen/rust/ir_call_site.rs` arg coercions to int formals (part of 10444) | ~1500 | same as W11 | untouched | D-WIDTH-OWNERS |

## Ownership and clone deciders

| # | site | LOC | replaced by | status | lowers |
|---|---|---|---|---|---|
| O1 | `analyzer/parameter_analysis.rs` precedence chain (steps 0a-9) | ~1500 | OWN-* solver rules (OWN-CONSUMING-METHOD, OWN-RETURNED, OWN-STORED, OWN-READ-FIELD, OWN-CALL-FORMAL exist) | replaced-but-still-present as OWN-ANALYZER-SEED | D-OWN-STAGES |
| O2 | `analyzer/string_optimization.rs` + `generator.rs::str_ref_optimizable_param` (text demotion) | ~1500 | OWN-STR-REF decided by use (solver-owned only when the trail shows the solver moved it; pub free-fn lock excluded) | replaced-but-still-present | D-OWN-STAGES |
| O3 | `compiler/multipass_bare_pass_demotion.rs`, `codegen/rust/signature_promotion.rs` (registry `restore_*`/`promote_*` passes) | 4674 + 4519 | C3 DefId registry + solver fixed point | untouched here; owned by the C3 agent (wt/convergence-c3) | D-OWN-STAGES |
| O4 | `codegen/rust/function_formal_parameter_generation.rs` body-walking demotion heuristics | ~3000 | solver param modes | untouched | D-OWN-STAGES |
| O5 | `auto_clone.rs` + codegen `.clone()` surgery (`maybe_auto_clone`, ~147 sites in ir_call_site, 46 generator) | 2211 + sites | C6 liveness clone facts (owned by the C6 agent, wt/convergence-c6) | in progress elsewhere | D-CLONE-OUTSIDE-SOLVER, D-CLONE-LASTUSE |
| O6 | `find_signature_ending_with` callers (~13 sites) and bare-name fallbacks | -- | module-qualified lookup (C3) | one site fixed (`global_signature_for_function` tries module path first); rest owned by C3 agent | D-OWN-STAGES |

## Deletion order

1. Stop adding paints: each new width bug gets a solver fact first (rule in `docs/spec/inference.md`), then the legacy path
   is gated behind it. (No ratchet movement; prevents regrowth.)
2. Teach the solver `len()`/`count()`/indexing (usize class) and struct field + formal types. This unblocks W2-W7 and W11-W12.
3. Delete W4 (int_promotion_type_inference) once comparisons are solver-unified; expect the largest D-WIDTH-OWNERS drop.
4. Delete W3 peer thicket, then W5.
5. Delete W6 let paints, W7 coercions, W10 helpers.
6. Delete W8 collectors and fold W9 (numeric_defaults) into the solver.
7. Ownership: O4 after O3 (C3), O1/O2 last (they are the seed the solver still reads).

Each deletion is a separate commit that reports `LOC deleted` and the `explain_counts.txt` delta.

## Pace log

| step | commit | LOC deleted | LOC added | D-WIDTH-OWNERS delta |
|---|---|---|---|---|
| C5 cat 3-6 | see git log | ~45 (bare-name lookup order, str-ref re-decisions narrowed) | ~230 | 0 (not a width step) |
| C4 loop var + literal fact | 6559e2ff7 | 0 | ~120 | not measured (gate 3 passed: counts did not rise) |
| C4 len() into declared int | af60eb7fa+ | 0 | ~30 | not measured |

| C4 cross-module field widths (P0) | 72e..cb634d32b | 0 | ~130 | 0 |
| C4 delete return-width counter ascription | this branch | 112 | 1 | 0 |
| explain: compare types, not spellings | this branch | 1 | 14 | 191 -> 81 (measurement fix, see below) |
| restore homonym-safe lookups (cherry-pick artifact) | 759d64583 | 4 | 4 | 69 -> 65 (measured with step 1) |
| ADR-0028 step 1: usize class (NUM-USIZE) | 49bce0147 | 0 | ~120 | 81 -> 69 |
| ADR-0028 step 2: width facts read by codegen | 911b99fe8 | ~20 | ~300 | 65 -> 60; narrowing casts 14 -> 12 |
| ADR-0028 step 3: NUM-USIZE-TO-INT, variant payloads, tail-cast helper | aa0bbcebe | 50 | ~150 | 60 -> 58; narrowing casts 12 -> 11; rustc rejects 93 -> 91 |
| ADR-0028: lexical scoping (per-binding vars) | c1df05cfb | 0 | ~70 | 58 -> 58 (no corpus program shadows a width-carrying name) |
| ADR-0028: delete five never-firing W5 sites | 3c6a01667 | 106 | 1 | 58 -> 58 |

### Regression fixed: the usize class broke nine ecosystem targets (wt/c4-regress)

The 321-program golden corpus did not contain `len() - 1` returned as `int`, an `int` loop variable compared with
`len()`, or a counter passed to a std `usize` formal and returned as `int` with a `-1` tail. The solver published a
width for classes it could not decide. Rule added to `ir/numeric_facts.rs` (held classes): a class is not published
when the solver recorded a type conflict in it, when a negated literal in it is unsigned, or when it contains a loop
binding of unknown element type, a field read of unknown type or a `len()` on a receiver of unknown type. Receivers
bound to calls, module functions and struct literals are typed. Gates: three golden-corpus programs
(`tests/inference_corpus/usize_*.wj`) and `scripts/ecosystem_width_gate.wj` (nine targets compiled, all must pass).

### Kill criterion reached (2026-11-02, wt/c4-usize2)

Two consecutive steps did not lower D-WIDTH-OWNERS (58). The 58 findings are: 25 `i64 -> i32` locals in
*defaulted* classes (the solver says `int`, the legacy paint says i32), 3 `f64 -> f32` (float, out of C4 scope) and about
29 compound types the meter still compares by spelling (`Custom("Stack")` vs `Stack<i64>`, `Vec(Tuple(..))` vs
`Vec<(..)>`, `HashMap` vs `BTreeMap`, a type alias). Publishing the defaulted classes (experiment, env-gated, not
committed) makes rustc reject 31 more goldens: the solver does not yet model closure parameters, collection element
types or std method arguments, so the legacy paint still has information the solver lacks. Next: model those three
uses, then publish defaulted classes; separately, normalise generic arguments in the meter.

### Why the width-deletion step did not move D-WIDTH-OWNERS (kill criterion, 2026-11-02)

`D-WIDTH-OWNERS` was mostly measuring spelling. The explain report compared the solver's `Debug` label (`Vec(I64)`,
`HashMap(String, I64)`, `Option(I64)`) with the emitted Rust spelling (`Vec < i64 >`), so every compound-typed local was
a finding even when both sides agree. 110 of the 191 findings were this artifact; comparing labels by type (not by
spelling) leaves 81 real disagreements. That fix is a measurement correction, not a deletion, and is reported
separately from the deletion.

The deleted branch (`mut_let_rhs_is_return_width_counter` ascription in untyped `let mut`, 112 lines) only printed a
type annotation that the literal suffix already carried (`let mut i: usize = 0_usize` became `let mut i = 0_usize`);
the golden corpus changed in 13 files, all redundant annotations, and rustc rejects did not change. It removed a decider
of the *annotation*, not of the *width*, so it could not move the counter.

What the 81 real findings are (sampled over examples/verified): `i64 -> i32` (solver says int, codegen paints i32;
the W3/W4 peer thicket and W6 let paints decide them) and `u64 -> usize` (the solver has no usize class for `len()` and
indexing). Neither moves until the solver models usize (`len()`, `count()`, indexing) and codegen reads
`solver_i64_locals` for *all* inits, not just literals. That is the next step that can lower the counter; deleting
further codegen paints first would only change spellings.

Under the kill criterion the stream stops here for the owner's decision. New baseline: `D-WIDTH-OWNERS` 81.

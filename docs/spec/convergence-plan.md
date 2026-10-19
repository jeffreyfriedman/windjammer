# Convergence plan: one owner for ownership, clones and widths (WP 7-9)

Audience: the agent who executes ADR-0001 and ARCHITECTURE_IMPROVEMENT_PLAN WP 7, 8, 9. Written 2026-10-11 from the
measurements in [inference-divergences.md](inference-divergences.md). The WP numbering is the plan's; the step IDs
(C0...) are this document's. Do the steps in order; each ships alone and is gated by the checks in C0.

## Where we are (measured)
* Ownership of a parameter: 4 stages of deciders (D-OWNERS-OWN). `wj explain-ownership` shows them per parameter.
* The solver loses ownership across unions (D-SOLVER-UNION), so even the constraints it receives are only partly used.
* Clones: solver result unread; `auto_clone.rs` (2.2k lines) + ~2000 codegen `.clone()` sites (D-CLONE-OWNERS).
* Widths: four deciders; ADR-0015 defaults not implemented (D-NUM-DEFAULT, D-NUM-OWNERS).
* Registries keyed by bare string: 470 `get_signature(` call sites, suffix search fallbacks. The resolver
  (`src/resolve`, shadow mode) exists.
* Giant hosts of the heuristics: `codegen/rust/ir_call_site.rs` 10.4k lines, `function_generation_prepare.rs` 13.5k,
  `compiler/multipass_bare_pass_demotion.rs` 4.7k, `codegen/rust/signature_promotion.rs` 4.5k, `type_inference/` 7k.

## C0. Gates (do first; nothing below merges without them)
1. **Golden emitted-Rust corpus (WP 0).** Script `scripts/golden-rust.sh`: `wj build --no-cargo` every `tests/**/*.wj`
   in the corpus list of `tests/wj_corpus_test.rs` plus `examples/`, store `tests/golden/rust/<path>.rs`. Test fails on any
   diff; updating is `UPDATE_GOLDEN=1` and the diff is reviewed. Expected: every step below changes goldens; each
   change must be explained by a divergence ID in the commit message.
2. **rustc-accepts corpus.** For every golden file run `rustc --edition 2021 --emit=metadata` (no cargo). Ratchet the
   failing set in `tests/wj_corpus_known_failures.txt`-style file; it may only shrink. This is the independent oracle.
3. **Differential harness at depth.** `WJ_INFER_SEEDS=300 cargo test --release --test all inference_differential`
   (seeded, shrinking; `tests/inference_differential_test.rs`). Run per step; seeds must stay green. Add templates
   to the generator for each rule you touch (the generator is the executable form of `inference.md`).
4. **Explain ratchet.** Add `tests/explain_ratchet_test.rs`: run `explain_source` (with emitted Rust) over the corpus and
   count findings by ID (`D-OWN-STAGES`, `D-CLONE-OUTSIDE-SOLVER`, `D-WIDTH-OWNERS`, `D-CLONE-LASTUSE`, `D-SOLVER-UNION`,
   `D-PIN-NOT-HONORED`). Commit the counts; the test fails if any count rises. Each step below names which count must fall.
   This is the progress meter; "0 findings" is the exit of WP 9.
5. Run tests with nextest (one process per test): process-global registries make in-process runs order dependent
   (ratchet residue section 5). Do WP 3 (state into `CompilationSession`) before C5 if flakiness blocks the gates.
Kill criterion: if C0.1 cannot be made deterministic (diff noise between runs) stop and fix determinism first; every later
step depends on it.

**Done (2026-10-11).** `tests/convergence_gates_test.rs` (standalone: `cargo test --release --test convergence_gates
-- --test-threads 1`): gate 1 goldens in `tests/golden/rust/` (304 programs: examples/verified, conformance,
inference corpus, auto_clone, top-level `tests/*.wj`); gate 2 `tests/golden/rustc_rejects.txt` (`rustc --emit=metadata`,
lib crate type when there is no `main`; programs needing `windjammer_runtime` are recorded, not checked: a known blind
spot); gate 3 `tests/golden/explain_counts.txt`; determinism opt-in `WJ_GATE_DETERMINISM=N`. Determinism was broken
(registry homonym order); fixed before the baseline. Differential harness: `cargo test --release --test
inference_differential` (`WJ_INFER_SEEDS=300`: green at baseline).

## C1. Make the solver correct on its own terms (small, local; no behavior change expected beyond D-SOLVER-UNION)
Tests first, all in `src/ir/solver.rs`:
1. Un-ignore `ownership_propagates_across_type_equals` (written, `#[ignore]`d).
2. Add: join is commutative (generate random constraint orders, same result), Region identity does not create
   spurious `OwnershipConflict` diagnostics (`Ref(r1)` vs `Ref(r2)` currently differ), clones read at class root.
3. Fix: on `TypeEquals`, merge the two classes' ownership with the lattice join and clone flags with OR; expose
   `Solver::result_for(var)` that resolves roots; change `pipeline.rs` write-back (and the local-types loop) to use it.
Expected goldens: many functions change mode in the direction of the callee formal (OWN-CALL-FORMAL becomes live).
Review every diff; anything that makes rustc fail is a bug in a later pass that was compensating for the loss: list it
in the step's PR and fix it in C5, not by weakening C1.
Explain counts: `D-SOLVER-UNION` -> 0.
Kill/rollback: if more than ~5% of goldens change in a way that rustc rejects, land C1's tests and the merge behind
`IrPipelineConfig::merge_ownership_on_union` (default off) and turn it on in C5 when the post-passes are gone.

**Done (2026-10-11, commit "C1: solver keeps ownership per variable").** Design change from the text above:
ownership is *not* merged on `TypeEquals` (that would couple `a == b` parameter modes). Ownership is a
per-variable requirement; a directional constraint `UsedAs(value, site)` carries a position's requirement (callee
formal, return slot, `let` alias) to the value used there, propagated to a fixed point. `TypeEquals` is type-only.
Goldens did not change (the seven post-passes still override the solver), so no flag was needed.

## C2. Provenance and the explain tool as the debugger
1. Remove every `UNTAGGED` constraint: `cs.add(` without `add_because(` (grep `src/ir/constraint_gen.rs`; ~60 sites,
   numeric ones first). Test: `explain_source` on the corpus reports 0 `UNTAGGED` reasons.
2. Make `lower_to_ir` record, per parameter, the stage trail (`after_solver`, `after_<pass name>`) in `IrFunction` (data only).
   Explain then names the pass that changed the mode instead of only saying the stages differ. This is also the
   to-do list for C5: each named pass is a deletion target.

## C3. DefId cutover (WP 6/7)
Prerequisite: `src/resolve` shadow report shows 100% calls in the corpus resolve or carry a tagged diagnostic (WP 6 exit).
1. `FunctionSignature` gains `def_id: DefId` (registry keyed by `DefId`; `name` kept for display). Test: two functions with
   the same name in different modules (and a free function sharing a name with a method) keep distinct signatures
   (the historical bug in the plan, section 1).
2. `constraint_gen::resolve_callee_signature` and `ir_call_site` take `Res` from `resolve` (a `CallResolution` table in
   `CompilationSession`), not names. `Unresolved`/`Ambiguous` become diagnostics, not fallbacks.
3. Ratchet `get_signature(` call count (470) down per file; delete `find_signature_ending_with` and
   `find_signature_by_name_and_arg_count` last (their callers show where resolution was missing).
4. Cross-file: the registry is built once, before codegen, from the converged analysis (ratchet residue section 4); call
   sites read only it. Delete the per-file refresh (`emitted_rust_ref_params` write-back) and `global_signature_registry` snapshot.
Explain/ratchet: cross-file corpus (`tests/fixtures/library_multipass`, the `cross_crate_dogfooding_*` entries in
`.config/known-test-failures.txt`) must turn green here; remove them from the ratchet file.
Do not run in parallel with C4/C5 on `ir_call_site.rs`, `signature_registry.rs` (plan section 8).

## C4. One numeric solver (WP 8)
Decide first (already decided by ADR-0015 #1/#2): default `int` = i64, `float` = f64, explicit widths exact, mixed widths
are errors, indexing and `len()` convert through checked conversions.
1. **Shadow.** Run `ir::numeric_solver` over each function with the full constraint vocabulary of the legacy collectors
   (`type_inference/{int,float}_inference/*constraints*.rs`); compare per expression with what codegen emits; report as
   `D-WIDTH-OWNERS`. Exit: the disagreement list is understood and each item is either a legacy bug or a missing constraint.
2. **Constraints.** Add the missing ones with tests from the corpus: `PreferIfUnconstrained(id, T)` (soft defaults), for-loop
   element/accumulator unification (D-NUM-FOR-ACC), closure-body unification (D-NUM-CLOSURE-LEN), index use
   (ratchet residue 1), struct-field and callee-formal propagation (`NUM-PROPAGATE`).
3. **Defaults.** Switch defaults to ADR-0015 (i64/f64) in the numeric solver and delete the hard-coded I32 in
   `constraint_gen::literal_to_base_type` and `solver::resolve_numeric_classes`. Expected goldens: widespread `i32`->`i64`,
   `f32`->`f64`. Separate commit from 4.4; ship a `wj fix` note for users.
4. **Authority.** Codegen reads widths from the solver result only: delete `let_statement_generation.rs`
   (`prefer_usize_over_i32`, `while_i32_peer`, `codegen_i32_binding_names`, `sync_i32_coord_binding_after_let`),
   `int_promotion_type_inference.rs`, `type_balancing.rs` promotions, and finally `src/type_inference/` (WP 8 exit).
   Codegen then only prints the checked conversions the solver placed.
5. ADR-0015 #2 (`int(x)`, `float(x)`, `.truncate()`) lands after 4.4 so the compiler has a place to put conversions.
Kill criterion: if after step 2 more than 10% of corpus expressions still disagree for reasons that are not legacy bugs,
the constraint vocabulary is wrong; stop and re-derive from `inference.md` section 7 before deleting anything.
Explain counts: `D-WIDTH-OWNERS` -> 0. Corpus: `tests/inference_corpus/{for_accumulator_width,closure_len_width,default_int_literal}.wj`
pass and leave the ratchet file.

**C4 priority input (2026-10-12, dogfooding, P0 silent miscompile).** `let mut total = 0; for line in &lines { total = total + line.amount }`
with `amount: int` was emitted `total: i32` plus `line.amount as i32` (values above 2^31 truncated, no diagnostic). Cause: the
solver (legacy collector) said i64 but `let_statement_generation.rs` painted bare literal inits i32 in functions whose return type
does not mention `int` (`function_prefers_i32_coord_locals`) - a D-WIDTH-OWNERS case. Fix direction implemented first: a literal
init takes the solver's width (and the semantic NUM-DEFAULT-INT fact) before any paint. Acceptance:
`tests/inference_corpus/int_accumulator_field_width.wj` (`corpus_int_accumulator_over_int_fields_is_not_truncated_to_i32`).
**Invariant to add (money-type correctness):** the solver never lets the compiler introduce a narrowing conversion (`as i32` of an
`int` operand, `i64 -> i32`) without an explicit user conversion (ADR-0015 #2); when a backend would need one, the semantic layer
reports a diagnostic with the expression and rule NUM-MIXED instead of casting. Gate: grep of goldens for `as i32` applied to a
`.amount`-style `int` field must stay empty; count `as i32|as u32|as i16|as u8` narrowing casts in goldens as a ratchet.

## C5. One ownership owner (WP 9), parameters and receivers
Method: replace the analyzer's decisions with solver-native constraints one *rule category* at a time, never all at once.
The analyzer's precedence chain (`parameter_analysis.rs` steps 0a-9) maps onto spec rules:

| analyzer step | spec rule | solver-native constraint to add |
|---|---|---|
| returned (1), field-move return (2.5) | OWN-RETURNED | `OwnershipIs(p, Owned)` at `return`/tail and at `let x = p.field` moves |
| mutated (2) | OWN-MUT-* | exists (`detect_mutation_target`, `emit_mutref_on_receiver_root`); delete `OWN-MUT-ANALYZER` |
| stored (3) | OWN-STORED | exists for literals/tuples/payload calls; add field-assign (decide D-OWN-STORE-ASSIGN first) |
| arithmetic/concat (4), pattern with fields (5) | OWN-READONLY exceptions | add as explicit rules or delete (several are Copy-only artifacts) |
| for-loop (6) | OWN-READONLY | consuming iteration vs shared scan: liveness-based |
| consuming method (6b), passthrough (6d, 7) | OWN-CALL-FORMAL | exists, live after C1 |
| text params (0b, 9) | OWN-STR-REF | decided by use, not by `pub` (D-OWN-PUB) |

Per category: (a) add the constraint with provenance; (b) add differential-generator templates and a corpus program;
(c) turn off the matching analyzer branch (return a neutral value) and run C0; (d) delete the branch. After the table is
empty: delete `OWN-ANALYZER-SEED`, then the seven post-passes in `lower_to_ir` (the C2 trail tells you which are
no-ops first), then `param_ownership_for_formal_demotion` and `function_generation_prepare::promote_*`, then
`compiler/multipass_bare_pass_demotion.rs`, `signature_promotion.rs`. Receivers (`self_*`) follow the same table with
SELF-* rules; decide D-SELF-VS-PARAM first (recommended: SELF-GETTER wins for both).
End state: `src/analyzer/` keeps only facts (mutations, moves, field accesses), as ADR-0001 requires; add the fitness
test "no writer of parameter ownership outside `src/ir`" (ADR-0001 consequences).
Explain counts: `D-OWN-STAGES` -> 0, `D-OWNERS-OWN` closed.
Iteration note: ownership is a fixed point across functions (call graph). Solve per SCC of the call graph in reverse
topological order, start from `&`, only rise; this replaces the multipass convergence loop and makes the result independent
of file order.

## C6. One clone owner
1. Facts: a liveness pass (per function, over the AST/IR) producing `last_use(var, site)` and `used_in_loop(var)` (data only).
2. Solver: `NeedsClone(site)` from "owned formal/store receives a variable that is live afterwards or in a loop"
   (CLONE-USE-AFTER-MOVE, CLONE-LOOP), and "borrowed value flows to an owned position" (CLONE-BORROWED-TO-OWNED).
   Per-*site*, not per variable (the current var-level flag cannot express `f(x.clone()); g(x)`).
3. Codegen prints exactly the solver's sites. Delete `auto_clone.rs`, `CLONE-ANALYZER`/`CLONE-MULTI-USE`, and the string
   level `.clone()` surgery in `src/codegen` (ratchet the 2065-site grep count). Fix D-CLONE-LASTUSE, D-CLONE-STRLIT,
   D-CLONE-REDUNDANT by construction; corpus `string_literal_move_then_use.wj` must pass.
Explain counts: `D-CLONE-OUTSIDE-SOLVER`, `D-CLONE-LASTUSE` -> 0.

## C5/C6 acceptance inputs from other streams (2026-10-11)
* Consuming trait methods on generic parameters: `fn label<T: Into<string>>(t: T) -> string { t.into() }` must make
  `t` owned (category "consuming method" / OWN-CALL-FORMAL with `self`-by-value receivers). Ratchet test from parser-gaps2.
* Loop borrow mutability is decided twice (`iteration_borrow_dispatch.rs`, `self_dispatch_for_loops.rs`): one owner.
* ownership-gaps helpers to fold into the single owner or delete (keep their tests, `tests/codegen_correctness_test.rs`
  class gates + `tests/codegen_correctness_corpus`): `closure_escape.rs`, `type_balancing_fixups.rs`,
  `std_idiom_fixups.rs`, `int_formal_scan_extra.rs`, `program_facts.rs`, `analyzer/spawn_capture.rs`,
  `closure_unary_constraints.rs`. C4 acceptance: `codegen_correctness_test::literal_default_through_adapters`.
* Expression identity: numeric facts are keyed by the (line, column) *after* an expression, so an expression and its
  enclosing closure can share a key. Key facts by AST node identity (C2/C4 prerequisite for authoritative widths).
* `src/check/semantic` (wj check) must consume the semantic layer's facts (types, widths, ownership) instead of
  re-deriving them, and gain borrow/move-conflict diagnostics from the solver's liveness (C6). Acceptance: the three
  entries of `tests/check_negative/known_false_negatives.txt` (two borrow conflicts, one use after move).
* Portable backends request facts through ADR-0017 (first request: `lent_mut` for `&mut int` lowering in Go/JS).

## C8. ADR-0002 migration (explicit `let mut`), owned by the semantic layer
Mutability of locals is a fact (`mutated(binding)`) of the single layer. Steps: (1) diagnostic "`x` is assigned but not
declared `mut`" computed from that fact, with a `wj fix` edit that inserts `mut`; (2) run `wj fix` over the corpus,
examples and std; (3) delete `auto_needs_mut` and the codegen `let mut` inference
(`let_statement_generation.rs`, `variable_analysis/mut_inference.rs`, `function_formal_parameter_generation.rs`).
Gate: goldens change only by `mut` placement in sources, never in emitted Rust.

## C7. Decisions that need the owner before the matching step
* D-OWN-STORE-ASSIGN: is a parameter that is stored borrowed-and-cloned-inside, or owned-and-moved? (recommend owned:
  one allocation, matches `push`). Needed before C5 "stored".
* D-SELF-VS-PARAM (recommend SELF-GETTER for both), D-OWN-PUB (recommend: remove the visibility dependence; use pins),
  closure capture mode (CLOS-MOVE-CAPTURE: recommend read-only captures borrow), ADR-0016 pins, ADR-0003.
* ADR-0002 enforcement (D-MUT-LOCAL): diagnostic "x is assigned but not declared mut" with `wj fix`; remove `auto_needs_mut`.
  Independent of C3-C6; can ship any time after the `wj fix` migration exists. Do the migration first or the corpus breaks.

## Ordering and parallelism
C0 -> C1 -> C2 -> {C3} -> {C4, then C5 (+C6)}. C3 before C5 because call-site ownership needs identity-based
signatures. C4 and C5 touch different decisions but the same hosts (`let_statement_generation.rs`, `ir_call_site.rs`):
serialize them. C7 decisions can be taken in parallel and do not block C0-C4.

## Definition of done
`wj explain-ownership` over the corpus prints zero `D-*` findings; `inference.md` has no `Diverges` status; the
differential harness is green at 1000 seeds; `src/analyzer` has no `inferred_ownership` writer; `src/type_inference/`
and `auto_clone.rs` are gone; the architecture fitness tests enforce all three.

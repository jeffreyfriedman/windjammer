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
| O4 | `codegen/rust/function_formal_parameter_generation.rs` body-walking demotion heuristics | ~3000 | solver param modes | receivers: replaced (`self_receiver` spells the IR mode; `self` arms deleted); plain value formals (non-Copy user types and collections, private and public) spelled from the IR mode by `ir_value_formal`, the public keep-owned walks deleted; field-move walk deleted (analyzer fact `field_moved_parameters`). Left: text, Copy, generic, pattern and trait-impl formals, and the prepare-phase registry predictions | D-OWN-STAGES |
| O5 | `auto_clone.rs` + codegen `.clone()` surgery (`maybe_auto_clone`, ~147 sites in ir_call_site, 46 generator) | 2211 + sites | C6 liveness clone facts (owned by the C6 agent, wt/convergence-c6) | in progress elsewhere | D-CLONE-OUTSIDE-SOLVER, D-CLONE-LASTUSE |
| O6 | `find_signature_ending_with` callers (~13 sites) and bare-name fallbacks | -- | module-qualified lookup (C3) | one site fixed (`global_signature_for_function` tries module path first); rest owned by C3 agent | D-OWN-STAGES |

### Where receiver and parameter ownership is decided today (wt/ownership-recv, 2026-11-04)

Every stage below writes a mode for `self` or a parameter; later stages override earlier ones. `wj explain-ownership`
shows the stages as `analyzer= solver= ir= emitted=`.

| stage | file / function | decides | shapes it got wrong |
|---|---|---|---|
| A1 analyzer, receivers | `analyzer/function_analysis.rs::infer_self_ownership_*` + `resolve_impl_self_receiver_ownership`, fed by `self_return_and_consumption.rs` (`function_matches_on_self`, `match_arms_consume_bound_values`, `function_body_moves_non_copy_self_fields`), `self_mutating_calls.rs`, `self_field_mutation.rs` | `self` seed (OWN-ANALYZER-SEED) | read-only `match self` with bindings made Owned (text concat counted as a move) |
| A2 analyzer, parameters | `analyzer/parameter_analysis.rs::infer_parameter_ownership` steps 0a-9, `passthrough_inference.rs` (2.5b `param_projects_non_copy_field_into_call_arg`, step 7 passthrough) | parameter seed | Copy field projected into a call; read-only param of a type from another module |
| A3 analyzer, traits | `analyzer/trait_analysis.rs::infer_trait_self_receiver`, `upgrade_trait_method_self_receiver` | join of impl receivers for `analyzed_trait_methods` (only impls seen in the same compilation) | impl in another module mutating through a field |
| S1 IR constraints | `ir/constraint_gen.rs` (OWN-MUT-ASSIGN `detect_mutation_target`, OWN-MUT-ANALYZER, OWN-RETURNED, seeds) and `constraint_gen/numeric.rs` (method calls: OWN-MUT-RECEIVER via `ir/receiver_type.rs`) | solver input | `self.items[n].step(x)`: receiver type of an indexed element was unknown, so the `&mut self` callee was never resolved |
| S2 solver | `ir/solver.rs` (join, only rises) + `ir/pipeline.rs::lower_to_ir` (`retain_mutated_mutref` post-step) | IR mode | - |
| C1 codegen, explicit `self` | `codegen/rust/function_formal_parameter_generation.rs::collect_additional_formal_parameter_strings` (Owned/Ref/Mut/Inferred hint arms, ~600 lines of `self` branches), helpers in `function_self_helpers.rs` (`owned_self_receiver`, `self_field_partial_move_needs_owned_receiver`, `self_receiver_upgrades`) and `codegen/rust/self_analysis.rs` (`function_consumes_self`, `function_modifies_self`, a 2000-line copy of the analyzer's walks without the registry) | emitted receiver | `lb(self.items, ..)` into a *borrowed* formal counted as a field move, overriding IR `&mut` with owned `self` |
| C2 codegen, implicit `self` | `codegen/rust/function_implicit_self_generation.rs::extend_implicit_self_parameters` | emitted receiver | same overrides as C1 |
| C3 codegen, traits | `codegen/rust/item_generation.rs::generate_trait_with_analysis` (analyzer join overrode even a declared `&mut self`; undeclared defaulted to `&self`), `function_self_helpers.rs::get_effective_self_ownership` (impl follows the trait) | trait + impl receivers | undeclared bodyless receiver guessed before the impls were seen |
| C4 codegen, trait impl post | `function_formal_parameter_post.rs` (std traits `Iterator::next` etc. -> `&mut self`) | emitted receiver | - |
| R registry/metadata | `codegen/rust/function_self_helpers.rs::record_self_receiver_upgrade`, `function_generation_prepare.rs` (`promote_*`, `get_param_ownership_for_registration`), `compiler/multipass_bare_pass_demotion.rs` (O3) | what callers see | - |

### C5 receiver step (wt/ownership-recv): what moved and what is left

- Receivers outside trait impls are spelled from the IR mode (`function_self_helpers.rs::ir_self_receiver`); the
  analyzer seed and the solver decide. Goldens changed in 8 programs: read-only or snapshot methods that codegen had
  made owned (`with_port(self) -> Config`, `multiply(self, ..) -> Mat4`) now borrow (`&self`, field clones where a
  non-Copy field is copied into the result). This is the plan's D-SELF-VS-PARAM choice (SELF-GETTER wins); it raised
  `D-CLONE-OUTSIDE-SOLVER` by the clones of those snapshot methods, which C6 should own as CLONE-BORROWED-TO-OWNED.
- Trait receivers: a declared `&self`/`&mut self` is the receiver of the trait and every impl; an undeclared
  bodyless one is W0016 (warning; 67 such declarations in this repo's corpora and more downstream, so making it an
  error is an owner decision with a `wj fix` migration). Undeclared ones still take the join of the impls the
  compilation sees, never a name.
- Next deletions: the `param.name == "self"` arms of `collect_additional_formal_parameter_strings` and
  `extend_implicit_self_parameters` that only run without IR, then `codegen/rust/self_analysis.rs` copies of the
  analyzer walks; analyzer A1 rules move to SELF-* solver constraints (consumes/moves/returns self) so the seed can go.
- Open cases filed as project_corpus programs: `vec_element_moved_then_reused` (cross-module method call does not
  convert a literal for an owned `String` formal; listed in `project_rejects.txt`), `param_rebound_then_compared`
  and `use_imported_homonym_by_value` (pass here; the larger-project failures they stand for did not reproduce).

## Name-based guesses (decide from declarations instead)

The compiler decides from declarations, never from what a program happens to call things. Each row below guesses a
fact from an identifier's spelling; each is a deletion target once the declaration-based fact exists.

| site | guess | replacement |
|---|---|---|
| `codegen/rust/string_utilities.rs` `is_windjammer_string_const_name` | a constant named `SCOPE_*`, `AUDIT_*`, `PERIOD_STATUS_*` is a string | **deleted** (wt/c4-optc): `module_string_consts`, built from `const …: string` declarations in this and imported modules, already held every such constant; test `tests/string_const_by_declaration_test.rs` |
| `module_system/mod.rs` (5 sites), `build_utils/module_generation.rs` (3 sites) | a module named `desktop_*` or `app_*` (except `app_reactive`) gets a desktop feature gate | **open, needs a declaration first**: a `wj.toml` table (`[module_features]` `desktop = ["desktop_window", ...]`) read where `lib.rs`/`mod.rs` are generated (`generate_lib_rs` has the project root; `generate_mod_file` and `generate_mod_rs_for_submodule` need it passed); the gate is emitted only for listed modules. A downstream project that relies on the prefix must add its entries in the same change, then the 8 prefix checks go |
| `codegen/rust/stdlib_method_traits.rs:810`, `ir_call_site.rs:436/448`, `method_call_expression_generation/arguments.rs:914/986` | a receiver type named `SharedMap` / `SharedMapSI` (or ending so) is a lock-guarded map | **deleted** (wt/names): the lock guard is typed from the declared field (`inner: Arc<Mutex<HashMap<..>>>`); `tests/locked_map_handle_by_declaration_test.rs` uses a neutral name, and the compiled ecosystem sweep covers the cross-package case |
| `codegen/rust/function_formal_parameter_generation.rs:1466` | a debug trace filters on one function name | **deleted** (wt/names): the trace fires for every function under the env flag |
| `codegen/rust/function_generation_prepare.rs:10735` | a `Custom("Vec<…>")` type spelled with `string` inside is a `Vec<string>` | **done** (wt/names): `type_classification_utilities::type_is_string_element_vec` reads the element (`Vec<Vec<string>>` is not a string vector) |
| `codegen/rust/program_generation.rs:830` | a `@game` decorator selects a code path | **code path deleted** (wt/names): its `impl Default` conflicted with the derived `Default` every all-default struct gets (E0119); `@game` stays a registered framework decorator, documented in `docs/spec/attributes.md`, tested by `tests/framework_decorator_game_test.rs` |
| `cargo_integration.rs:345` (and `config.game_path`), `plugin.rs:29` | importing one of three hard-coded engine crate names adds a path dependency to a local engine checkout; plugins are searched in a hard-coded sibling engine directory | a declared dependency in `wj.toml` (path or registry), like every other crate |

Language-level conventions that stay: test modules and files named `*_test` / `*_tests` (the test runner's documented
rule) and std module names (`db`, `config`, `strings`).

## Heuristic surface (gate: `tests/heuristic_surface_test.rs`)

Convergence is converging when the deciders shrink, not only when the solver grows. The heuristic-surface ratchet
counts, per file in `src/codegen`, `src/analyzer`, `src/ir`, `src/typing` and `src/type_inference` (test modules and
test files excluded):

- `H-NAME-MATCHES`: a string literal that spells a name (identifier, type, method, module or path fragment such as
  `"Vec<"` or `"_i32"`) compared with `==` / `!=` or searched with `starts_with` / `ends_with` / `contains` /
  `strip_prefix` / `strip_suffix`;
- `H-MATCH-ARM-NAMES`: such a literal used as a `match` arm pattern (`"Foo" =>`, `"a" | "b" =>`, `Some("x") =>`);
- `H-FALLBACK-LEGACY`: identifiers containing `fallback` or `legacy` (each definition and each use).

Literals that spell a language keyword, a primitive type, a Rust std module or a Windjammer std module (`std/`) are
exempt; any other exemption is a `[conventions]` entry in `docs/heuristic-allowlist.toml` that must point at the doc
defining it. Every per-file number may only fall; a new file may not appear; the allow-list may not be higher than
reality (lower it with `cargo test --test all heuristic_surface_test::regenerate_heuristic_baseline -- --ignored`); a
mutation check adds one guess of each kind to a real file and requires all three sections to fail. The gate also
fails when the line below does not match the allow-list.

Current totals (2026-11-09, after merging wt/c2 and the ownership O4 steps including public formals; seeded at 1925 on 2026-11-04, the prelude constructors `Some` `None` `Ok` `Err` are a documented convention since stage C2):
`H-NAME-MATCHES` 528, `H-MATCH-ARM-NAMES` 1128, `H-FALLBACK-LEGACY` 128 (total 1784)

Breakdown at seeding (2026-11-04):

| area | name matches | match-arm names | fallback/legacy |
|---|---|---|---|
| `codegen/rust` | 467 | 109 | 76 |
| other backends (`codegen/portable`, `javascript`, `go`, shared) | 46 | 957 | 0 |
| `analyzer` | 56 | 9 | 48 |
| `ir` | 38 | 46 | 4 |
| `type_inference` | 17 | 28 | 0 |
| `typing` | 1 | 1 | 0 |

Largest files: name matches `ir_call_site.rs` 32, `program_generation.rs` 24, `match_statement_generation.rs` 23,
`statement_generation.rs` 22; match-arm names are dominated by the other backends' builtin tables
(`portable/builtins.rs` 259, `javascript/emit/builtin.rs` 244, `go/emit/builtin.rs` 237), which map std function names
to target spellings and should come from the declared std surface; fallback/legacy `analyzer/signature_registry.rs` 48
(the C3 bare-name searches), `signature_promotion.rs` 17, `ir_call_site.rs` 16.

Solver meters at the same time (next, 2026-11-04): `D-WIDTH-OWNERS` 51, `D-OWN-STAGES` 9, `D-CLONE-OUTSIDE-SOLVER` 19,
golden rustc rejects 86. After stage C2 (wt/c2, 2026-11-09): `D-WIDTH-OWNERS` 35, golden rustc
rejects 5 fewer, compiler-inserted narrowing casts 11 -> 6; the 8 spec class-A cases (COLL-004, STD-003, STMT-009,
ITEM-006, ITEM-011, EXPR-011, INTR-007, GRAM-006) compile.

Net growth of the deciding code: `src/codegen` + `src/analyzer`, 2026-11-01 (9c14e598e, last first-parent commit
before 2026-11-02) to wt/names 585b70904: 1728 lines added, 1289 deleted, net +439
(`git diff --shortstat 9c14e598e HEAD -- src/codegen src/analyzer`). Net growth is the number to drive below zero:
a convergence step deletes more decider code than the solver rule it adds.

### Hand-back rules: where the solver returns a decision to the legacy deciders

Each rule below is heuristic surface: the solver solved the class but does not publish it, so the legacy decider
(`codegen/rust` W3-W12, the analyzer O1-O5) still decides. Each one is retired by the stage named, and its deletion is
measured by `D-WIDTH-OWNERS` / `D-OWN-STAGES` and this ratchet.

| rule (site) | the solver hands back when ... | retired by |
|---|---|---|
| index-use hold (`ir/numeric_facts.rs` `held_classes`, `index_uses`) | an index use sits in a class that resolved to a width other than `usize` (needs a conversion at the index) | C4: NUM-USIZE-TO-INT at the index, as already done at index sources |
| unbound-name hold (`held_classes`, `var_map.locals` not in `binding_vars`) | a class contains a name the walk met without a binding: a closure parameter or a global | **retired for closure parameters (C2, wt/c2)**: closure parameters are bindings (NUM-CLOSURE); a typed module constant of any file of a library is modelled (NUM-DECLARED); globals remain |
| width-conflict hold (`result.width_conflicts`, c4-regress) | two exact integer widths meet in one class | C4: a conversion fact at the meeting point, or a diagnostic when the program is wrong |
| negated-unsigned hold (`unmodelled.negated_literals`, c4-regress) | a negated literal sits in an unsigned class | C4: a diagnostic (the program is wrong), never a silent legacy width |
| loop-binding hold (`unmodelled.loop_bindings`, c4-regress) | a `for` binding's element width is unknown | **narrowed (C2, wt/c2)**: a loop binding is the iterable's element (NUM-ELEMENT); held only when the iterable is a call result the surface did not type |
| unknown-length hold (`unmodelled.unknown_lengths`, c4-regress) | `len()` on a receiver of unknown type | **C2** type aliases, then C3 cross-module receiver types |
| unknown-field-read hold (`unmodelled.unknown_fields`, c4-regress) | a field read whose declared type the walk cannot see (another module's struct, a generic) | C3 (cross-module declared field types by DefId) and **C2** (generic arguments through aliases) |
| lexical-scope publish-if-agree (`binding_widths`, NUM-LOCAL-WIDTH) | bindings of one name carry different widths: no name-keyed fact, the classes stay legacy | C4: facts keyed by binding node identity instead of name (replaces the W8 (line,col) collectors) |
| borrowed-binding skip (`unmodelled.borrowed_bindings`) | a local is bound to a borrow: no width fact for the name | C5: reference types in the ownership solver |
| defaulted-class withhold (`View::published`, `defaulted`) | the class has no use that fixes a width (`int` default); the legacy paint may still pick i32 | **retired (C2, wt/c2)**: defaulted classes are published at `int`; replaced by the opaque-use hold below, which holds only classes that reach a use the walk cannot type |
| defaulted-float withhold (`View::published_float`) | a float class has no `f32`/`f64` use | **retired (C2, wt/c2)**: defaulted float classes are published at `float` |
| opaque-use hold (`unmodelled.opaque`, C2) | a value is passed to a call with no signature, to a generic or named formal, or to an unknown method; the hold covers the parts of its class (an opaque `Vec` holds its elements) | C3 (callee identity for every call), then the remaining unknown methods become surface entries |
| crate-surface formals in the numeric solver (`rust/<crate>.wji`) | the build metadata says `half(x: f32)`, but the registry entry the IR resolves for `shapes::half` carries `float`, so the literal is published f64 (test `float_literal_to_f32_crate_parameter`, ledgered) | C3: one signature per callee identity, read from the surface |
| codegen operand hand-back (`codegen/rust/solver_widths.rs` `solver_operand_width` / `solver_agrees` answering `None`/false; the `!solver_agreed` branches of `binary_expression_generation.rs`) | any hold above applies to an operand | follows the holds: each retired hold deletes the `!solver_agreed` branch it feeds (W3/W4) |
| analyzer seeds (`OWN-ANALYZER-SEED`, `OWN-MUT-ANALYZER`, `OWN-COPY-SEED`, `TY-ANALYZER-SEED` in `ir/constraint_gen.rs`) | the solver starts from the analyzer's pre-decision instead of deriving it | C5 (ownership from uses; O1/O2), C3 for `TY-ANALYZER-SEED` |
| `CLONE-ANALYZER` (auto-clone pass sites) | clone sites come from `auto_clone.rs` | C6 liveness clone facts |
| `generator.rs` `get_param_ownership` falls back to `AnalyzedFunction.inferred_ownership` | the IR function for the current function is not found | C3: one function identity (DefId), so the IR function is always found |

C2's share is the rows marked **C2**. Its acceptance list stays the 8 spec class-A cases and the ratcheted width tests,
and each retired hold must lower `H-NAME-MATCHES` or `H-FALLBACK-LEGACY` in the file whose branch it deletes.

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
| C5 receivers from the IR (`ir_self_receiver`), indexed receiver types, W0016 | cbe50d47c | 0 | ~160 | D-OWN-STAGES 21 -> 9; D-CLONE-OUTSIDE-SOLVER 14 -> 18 (snapshot methods clone fields) |
| C5 field-move fact replaces the codegen walk | cbe50d47c, 24fe5083b | ~180 | ~70 | (same measurement) |
| C5 self owned only when self is consumed (seed rule; field reads clone) | af2a31ce9 + next | 6 | 4 | ecosystem width gate 6/9 -> 9/9; ratchet: 3 batch3 ownership tests leave (48 -> 45 entries owned by C3/C5/C6) |
| C5 receiver arms deleted: one `self_receiver` spelling rule (IR mode; trait mode in trait impls) | d76a87198 | ~620 | ~60 | D-OWN-STAGES 9 -> 9 (the deleted arms ran only where codegen and the IR already agreed); flagship-app api-check ownership errors 3 -> 1; ratchet 44 -> 43 (one stale entry) |
| C5: delete name-gated debug traces (formal parameters `grid`/`do_work`, passthrough `clear`/`place`/`Cache`); one CLONE-USE-AFTER-MOVE decision with a `MoveSite` | this branch | ~70 | ~15 | H-NAME-MATCHES 647 -> 642 |
| O4: the `@string_ref` / `@str_ref` pins are one AST fact (`Parameter::pins_string_ref` / `pins_str_ref`), not 17 decorator-name comparisons in the formal-parameter and analyzer code | this branch | ~25 | ~16 | H-NAME-MATCHES 642 -> 625 |
| O4: private plain value formals (non-Copy user types and collections, declared by value, no pattern, pin or `mut`) are spelled from the IR mode (`ir_value_formal`); public formals keep the owned-API rules until cross-crate call sites read the IR mode | this branch | 0 | ~45 | D-OWN-STAGES 9 -> 8 |
| O4: public formals join the IR rule (other crates' call sites already read the emitted formals from `metadata.json`); the keep-owned walks are deleted (`is_public_owned_non_copy_formal_api` and its 25 call sites, `pub_vec_non_copy_custom_indexed_api`, `vec_param_index_is_readonly_scan`, `vec_formal_only_forwards_as_call_arg` and its 5 call sites); external metadata roots load in name order with explicitly named crates last, so a transitive dependency's homonym cannot take a bare key | this branch | 409 | 33 | project rejects -1 (`bare_pass_vec_reader`); H-NAME-MATCHES 530 -> 529, H-MATCH-ARM-NAMES 1130 -> 1129 |
| O4 follow-up (flagship regressions 3 -> 0): bare-pass demotion (O3) no longer guesses `&mut T` / `&Vec` for collections and user types, which the solver read through OWN-CALL-FORMAL; OWN-STORED reaches a parameter through a `let` alias; a loop over a borrowed map binds `(&K, &V)` and an owned payload clones a `&T` field; maps and sets keep their legacy spelling; an owned value formal the body writes through is `mut` | this branch | 21 | ~60 | project corpus +3 cases (one, `stored_through_let_alias`, ratcheted: a caller lowered before its callee's file reads the pre-IR mode) |
| O4 fixed point (wt/ownership-6): after each IR lowering round the solved value-formal and receiver modes are published to the registry (and every file's analysis registry) under the keys that name their function; lowering repeats until none changes, so callers read callees' solved modes (`compiler/value_formal_fixed_point.rs`). Also: IR and codegen identity fixes (a `Type::method` IR before a same-named trait declaration; a bare call never names a method; a local shadows a function), owned-field clones (a borrowed root's field bound by `let`, a field path moved and read again), match on a borrowed parameter binds references, tuple-pattern `let` bindings get `mut` | wt/ownership-6 | 49 | 297 | project rejects -1 (`stored_through_let_alias`); corpus +6 neutral database-project repros (F32 F33 F34 F39 F40, all passing) |
| C2 structured types (HasPart, surface binding, closures, patterns, aliases, opaque hold, published defaults) | wt/c2 | 233 (C1 surface binder, push element-store name check, old pattern binder) | 1236 (src; codegen+analyzer +80 / -9) | 51 -> 35; rustc rejects -5; narrowing casts 11 -> 6 |

### Regression fixed: the usize class broke nine ecosystem targets (wt/c4-regress)

The golden corpus did not contain `len() - 1` returned as `int`, an `int` loop variable compared with
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

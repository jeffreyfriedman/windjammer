# C3 inventory: name-keyed signature lookups

Input for step C3 of [convergence-plan.md](convergence-plan.md) (DefId / module-qualified cutover). Measured on
`next/v0.50.0` at `9c14e598e` (2026-11-01). Ratchets that keep these numbers from rising:
`tests/architecture.rs` sections `get_signature_calls` (total outside the registry), `get_signature_calls_by_file`
and `signature_fallback_calls` (both per file; baselines in `docs/architecture-allowlist.toml`).

## Totals

| What | Sites |
|---|---|
| `get_signature(` raw grep over `src/` | 470 |
| `get_signature(` in code (comments/strings stripped), outside `signature_registry.rs` | 434 |
| `get_signature_with_global(` (codegen wrapper: local layer, then crate-global snapshot) | 65 |
| Suffix / bare-name / arity searches (`signature_fallback_calls`, 11 APIs; code only, definitions excluded) | 125 |
| `get_signature(` per file (`get_signature_calls_by_file`, code only, registry included) | 463 |

Fallback APIs and their callers (calls, not definitions):

| API | What it guesses | Calls | Callers |
|---|---|---|---|
| `find_signature_ending_with` | first homonym by key order | 14 | function_generation_prepare (6), auto_clone (2), registry (2), constraint_gen, library_multipass, receiver, storage_tracking |
| `find_unique_signature_ending_with` | the only `*::name` | 28 | ir_call_site (11), signature_promotion (4), generator, function_generation_prepare, registry (2 each), 7 others |
| `find_signature_by_name_and_arg_count` | unique `*::name` with that arity | 7 | generator, call_signature_resolution (2 each), constraint_gen, storage_tracking, registry |
| `find_own_signature_by_name_and_arg_count` | same, local layer only | 2 | method_call_expression_generation/signature_resolution |
| `find_signature_by_name_and_arg_count_with_global` | same, both layers | 3 | function_generation_prepare (2), statement_mut_binding_scan |
| `lookup_method` | exact key, else unique suffix | 46 | ir_call_site (18), function_generation_prepare, self_return_and_consumption, passthrough_inference (4 each), 11 others |
| `signatures_matching_suffix` / `signatures_for_method_name` / `method_keys_for` | all homonyms of a name | 22 | stdlib_method_traits (analyzer 5, codegen 2), call_signature_resolution, signature_promotion, storage_tracking, collection_element_inference, auto_clone |
| `all_signatures_for_suffix_search` | whole registry scan | 5 | ir/taint (2), ir/effects, type_inference/int_inference, one test |
| `callee_lookup_keys` | exact + every suffix match | 1 | multipass_bare_pass_demotion |

## Classification: can the resolver supply the identity at the site?

The resolver (`src/resolve`) gives every call expression a `Res`, keyed by `ExprKey` (the expression's arena
address, which the analyzer and the Rust backend share: `FunctionDecl::body` holds the arena references). What it can
give depends on the call shape:

| Class | Argument shape | Sites | Resolver answer | Cutover |
|---|---|---|---|---|
| **C** call-site callee | `callee_name`, `name`, `method`, `simple` | 153 | free / path call: `Def(DefId)` (path `a::b::f`); method call: `MethodCandidates` until the receiver type is known; std/runtime/dependency: `Unresolved(External(path))` | look up by `Def::path`; external by its full path; `Ambiguous` / several candidates: diagnostic |
| **Q** constructed qualified key | `format!("{ty}::{m}")`, `&key`, `&qualified`, `lookup` | 187 | method on a known receiver type: the impl's `DefId` (the resolver links impls to types); module-qualified keys: `Def` | replace the string building with `(type DefId, method name)` → `DefId`; key already qualified: keep, it is the identity once the registry uses resolver paths |
| **D** definition side | `&func.name`, `&decl.name`, `fn_name`, `func_name` | 38 | the definition's own `DefId` (`Collected::decls`) | the function being generated knows its identity; never look it up by name |
| **E** literal std/runtime key | `"strings::contains"`, `"HashMap::get"` | 77 (most in `#[cfg(test)]` modules) | `External` | keep: these name a runtime API, not a program definition |
| **T** tests | `#[test]` bodies, `*_test.rs` | 9 | n/a | follow the API |

Per file (top of the list; C/Q/D/E as above, T+other last). Every file in the list is in the per-file ratchet.

| File | All | C | Q | D | E | T |
|---|---|---|---|---|---|---|
| `src/codegen/rust/ir_call_site.rs` | 125 | 75 | 47 | 0 | 3 | 0 |
| `src/analyzer/signature_registry.rs` | 28 | 8 | 9 | 0 | 10 | 1 |
| `src/stdlib_scanner.rs` | 26 | 4 | 1 | 0 | 20 | 1 |
| `src/codegen/rust/signature_promotion.rs` | 26 | 7 | 9 | 0 | 9 | 1 |
| `src/codegen/rust/function_generation_prepare.rs` | 26 | 9 | 15 | 2 | 0 | 0 |
| `src/codegen/rust/generator.rs` | 23 | 9 | 6 | 8 | 0 | 0 |
| `src/codegen/rust/call_signature_resolution.rs` | 23 | 1 | 6 | 2 | 13 | 1 |
| `src/codegen/rust/stdlib_method_traits.rs` | 21 | 3 | 8 | 0 | 8 | 2 |
| `src/compiler/multipass_bare_pass_demotion.rs` | 17 | 0 | 15 | 0 | 2 | 0 |
| `src/codegen/rust/call_expression_generation/function_call_generation.rs` | 14 | 2 | 5 | 7 | 0 | 0 |
| `src/codegen/rust/call_expression_generation/argument_generation/regular_call_arguments.rs` | 10 | 0 | 3 | 7 | 0 | 0 |
| `src/compiler/compilation_pipeline.rs` | 8 | 2 | 0 | 0 | 6 | 0 |
| `src/analyzer/self_return_and_consumption.rs` | 7 | 2 | 5 | 0 | 0 | 0 |
| `src/analyzer/module_analysis.rs` | 7 | 0 | 5 | 2 | 0 | 0 |
| `src/compiler/library_multipass.rs` | 6 | 4 | 2 | 0 | 0 | 0 |
| `src/analyzer/string_optimization.rs` | 6 | 1 | 4 | 1 | 0 | 0 |
| `src/analyzer/stdlib_method_traits.rs` | 6 | 4 | 2 | 0 | 0 | 0 |
| `src/codegen/rust/call_site_borrow.rs` | 5 | 2 | 2 | 0 | 1 | 0 |
| `src/analyzer/section_render_ownership_test.rs` | 5 | 0 | 0 | 0 | 0 | 5 |
| `src/analyzer/passthrough_inference.rs` | 5 | 3 | 1 | 1 | 0 | 0 |
| `src/ir/taint.rs` | 4 | 0 | 4 | 0 | 0 | 0 |
| `src/codegen/rust/variable_analysis/collection_element_inference.rs` | 4 | 0 | 2 | 2 | 0 | 0 |
| `src/codegen/rust/string_utilities.rs` | 4 | 0 | 3 | 0 | 1 | 0 |
| `src/codegen/rust/method_call_expression_generation/receiver.rs` | 4 | 0 | 4 | 0 | 0 | 0 |
| `src/output_generation.rs` | 3 | 0 | 2 | 1 | 0 | 0 |
| `src/ir/effects.rs` | 3 | 0 | 3 | 0 | 0 | 0 |
| `src/compiler/declaration_stub_registry.rs` | 3 | 0 | 0 | 0 | 3 | 0 |
| `src/codegen/rust/self_analysis.rs` | 3 | 1 | 2 | 0 | 0 | 0 |
| `src/codegen/rust/method_call_expression_generation/signature_resolution.rs` | 3 | 1 | 2 | 0 | 0 | 0 |
| `src/codegen/rust/int_promotion_type_inference.rs` | 3 | 1 | 2 | 0 | 0 | 0 |
| `src/codegen/rust/call_expression_generation/argument_generation/field_access_method_args.rs` | 3 | 1 | 2 | 0 | 0 | 0 |
| `src/auto_clone.rs` | 3 | 3 | 0 | 0 | 0 | 0 |
| `src/analyzer/usage_tracking/storage_tracking.rs` | 3 | 2 | 1 | 0 | 0 | 0 |
| `src/analyzer/borrow_analysis.rs` | 3 | 2 | 0 | 0 | 1 | 0 |
| `src/metadata/tests.rs` | 2 | 0 | 0 | 0 | 0 | 2 |
| `src/metadata/mod.rs` | 2 | 0 | 0 | 2 | 0 | 0 |
| `src/ir/signature_bridge.rs` | 2 | 1 | 1 | 0 | 0 | 0 |
| `src/codegen/rust/string_analysis.rs` | 2 | 1 | 1 | 0 | 0 | 0 |
| `src/codegen/rust/program_generation.rs` | 2 | 0 | 2 | 0 | 0 | 0 |
| `src/codegen/rust/method_call_expression_generation/arguments.rs` | 2 | 0 | 2 | 0 | 0 | 0 |
| `src/codegen/rust/collection_detection.rs` | 2 | 0 | 2 | 0 | 0 | 0 |
| `src/analyzer/parameter_analysis.rs` | 2 | 1 | 1 | 0 | 0 | 0 |
| `src/metadata/crate_metadata.rs` | 1 | 0 | 1 | 0 | 0 | 0 |
| `src/ir/constraint_gen.rs` | 1 | 1 | 0 | 0 | 0 | 0 |
| `src/codegen/rust/usize_expression_type_inference.rs` | 1 | 0 | 1 | 0 | 0 | 0 |
| `src/codegen/rust/statement_mut_binding_scan.rs` | 1 | 1 | 0 | 0 | 0 | 0 |
| `src/codegen/rust/runtime_signatures.rs` | 1 | 0 | 1 | 0 | 0 | 0 |
| `src/codegen/rust/method_call_expression_generation/finalize.rs` | 1 | 0 | 1 | 0 | 0 | 0 |
| `src/codegen/rust/intrinsic_lowering.rs` | 1 | 0 | 0 | 1 | 0 | 0 |
| `src/codegen/rust/expression_generation.rs` | 1 | 0 | 1 | 0 | 0 | 0 |
| `src/codegen/rust/call_expression_generation/stdlib_call_generation.rs` | 1 | 0 | 0 | 1 | 0 | 0 |
| `src/codegen/portable/env.rs` | 1 | 0 | 1 | 0 | 0 | 0 |
| `src/analyzer/self_mutating_calls.rs` | 1 | 0 | 0 | 1 | 0 | 0 |
| `src/analyzer/mutation_detection.rs` | 1 | 1 | 0 | 0 | 0 | 0 |

## Identity bugs found (identity corpus)

`tests/identity_corpus/*.wj` (gate: `cargo test --release --test identity_corpus`) compiles every program and runs it
in the interpreter; both must print the hand-computed `// expect:` lines. At the base commit the interpreter passed all
eight, the compiled form failed six (rustc rejected the emitted Rust):

| Program | Compiled failure at base | Cause (all bare-name identity) |
|---|---|---|
| `free_fn_homonyms_across_modules` | `b::take` emitted with `a::take`'s body (E0277) | codegen picked the `AnalyzedFunction` by name for inline-module items |
| `enum_homonyms_across_modules` | `b::area` with `a::area`'s body (`Shape::Dot` in `b`, E0599/E0308) | same |
| `struct_homonyms_across_modules` | `b::make` with `a::make`'s body (E0560) | same |
| `impl_homonyms_across_modules` | `b::Counter` methods with `a`'s bodies, and emitted a second time at file level (E0428) | impl methods matched by name; impls inside modules not recognised as methods |
| `bare_calls_bind_to_own_module` | `a::twice` passes `xs` to `keep` with `b::keep`'s borrowed formal (E0382) | registry key `keep` is last-writer-wins; no lexical scope at the call |
| `module_free_fn_and_method_share_name` | free fn `store::put` emitted with a `self` receiver | bare key `put` holds the method's signature |
| `method_homonyms_across_types`, `free_fn_shares_name_with_method` | pass | (regression guards) |

## Cutover order (burn-down)

| # | Decider / lookup | Replacement | Status |
|---|---|---|---|
| 1 | inline-module item ↔ analysis by name (`generate_inline_module_item`, `analyzed_matches_impl_ast`) | `AnalyzedFunction::is_analysis_of` (shared arena body references = AST identity) | replaced |
| 2 | `inline_mod_only_function_names` (name filter for top-level emission) | `AnalyzedFunction::module_path` (empty = file level) | deleted |
| 3 | inline-module signatures only under the bare key | also registered under the resolver path (`a::take`, `a::Counter::new`) | added |
| 4 | bare call inside `mod m` | `SignatureRegistry::scoped_key` (lexical rule) in `signature_lookup_callee_name` and the IR free-call constraint | added |
| 5 | IR function selection by name (`select_ir_function_for`) | name + `IrFunction::module_path` | replaced |
| 6 | registry refresh of an inline-module function under the bare key | its module-qualified key | replaced |
| 7 | `find_signature_ending_with` (first homonym) | `find_unique_signature_ending_with`; homonyms stay unresolved | 12 of 14 callers cut over; `auto_clone.rs` (2) belongs to the C6 stream; delete the API when those go |
| 8 | `find_delegation_callee` (first homonym with the same arity) | none (no caller) | deleted |
| 9 | `resolve_callee_signature` first-match branch (`constraint_gen`, `storage_tracking`) | none | deleted |
| 10 | `get_signature(name)` for **C** sites | resolver table (callee expression -> `Def::path`), the registry key | started: single-file programs resolve every plain call (`same_crate_callee.rs`, `CallResolution::callee_key`), goldens unchanged. Next: the multipass driver installs the project-wide table, then the name-based steps of `resolve_call_signature` are deleted |
| 11 | **Q** sites building `"{Type}::{method}"` | `(impl DefId, method)` | not started; blocked on receiver types (the resolver has impls, not expression types) |
| 12 | **D** sites (definition looks itself up by name) | the definition's own identity | started: #6, module metadata (`collect_analyzed_module_metadata`), the multipass per-file overlay (`own_definition_keys`, `keep_own_definitions`) |
| 13 | `multipass_bare_pass_demotion.rs` (one promote pass, six `restore_*` passes) | solver-native cross-file ownership per call-graph SCC (C5 iteration note) | gate in place (`tests/project_gates_test.rs`); 4 of 6 restores deleted, see "Step 3 progress" |

## Step 3 inventory: `multipass_bare_pass_demotion.rs`

Runs twice in `library_multipass::build_library_multipass` (before IR lowering and after IR sync). Each pass rewrites
`param_ownership` in the merged global registry by bare-name or suffix key (`callee_lookup_keys`), after the solver.

| Pass | What it decides | Guarded by |
|---|---|---|
| `promote_callees_from_bare_pass_callers` | a callee whose sibling-module callers pass bare bindings is demoted to `&` / `&mut` | 18 unit tests in the file, `tests/*cross_crate*`, `tests/*multipass*` |
| `restore_owned_field_forward_formals` | undo the demotion when the formal only forwards its fields into callees (moves need ownership) | unit tests; `http_adapter_owned_reply` style fixtures |
| `restore_owned_string_formals_stored_in_payload` | undo when a `string` formal is stored in a constructed payload | unit tests |
| `restore_owned_returned_formals` | undo when the whole binding is returned (`col_string(row, name) -> (Row, string)`) | unit tests |
| `restore_pub_owned_non_copy_api_formals` | keep `pub` non-Copy API formals owned (OWN-PUB, D-OWN-PUB) | unit tests |
| `restore_owned_formals_for_producer_only_call_sites` | keep owned `Vec` formals when every caller passes a fresh producer | unit tests |
| `restore_owned_formals_forwarded_to_owned_callees` | keep owned when forwarded bare into an owned (FFI, `Vec`) callee | unit tests |

The promote/restore pairs are one rule expressed as an override plus six exceptions; in solver terms they are
OWN-CALL-FORMAL (callee formal flows to the argument) solved across files, OWN-STORED, OWN-RETURNED and OWN-PUB. They
disappear together when ownership is solved per SCC over the whole crate.

## Ratchet deltas (step 2, first cut)

| Ratchet | Before | After |
|---|---|---|
| identity corpus, compiled programs matching the oracle | 2 / 8 | 8 / 8 |
| `get_signature_calls` (outside the registry) | 434 | 430 |
| `get_signature_calls_by_file` (sum) | 463 | 459 |
| `signature_fallback_calls` (sum) | 125 | 123 |
| `find_signature_ending_with` callers | 14 | 2 (`auto_clone.rs`, C6 stream) |
| explain `D-WIDTH-OWNERS` (gate 3) | 191 | 189 (with eight more programs in the corpus) |
| explain `D-OWN-STAGES` | 21 | 21 |
| `rustc_rejects.txt` | 95 | 95 |
| long files: `signature_registry.rs` / `function_generation_prepare.rs` / `generator.rs` | 2539 / 13323 / 5059 | 2526 / 13275 / 5049 |
| long functions: `analyze_program_pass` / `analyze_function` / `generate_program` | 477 / 682 / 1093 | 322 / 653 / 1064 |

Goldens that changed: `examples/verified/module_inline.wj` and `module_nested_pub_mod.wj` (functions of nested inline
modules were emitted a second time at file level; the name filter only saw the first module level) and
`tests/inline_mod_extern_fn_test.wj` (module-qualified calls now resolve, so literal arguments take the formal's width:
`3_i64`).

Found on the way, not identity: a `pub fn f(v: Vec<int>)` that only reads `v` keeps an owned formal (D-OWN-PUB) and the
caller does not clone a binding it uses again, so `let n = f(xs); let m = f(xs)` is E0382 (single file, no modules).
Owner: C5 (OWN-PUB) with C6 (clone at the reusing call).

## Homonyms across files (library multipass): fixed

A three-file project (`main.wj` with `mod a` / `mod b`; `a.wj` and `b.wj` each define `pub enum Shape` and
`pub fn take`) ran correctly in the interpreter and failed compiled: `b::take` got `a::take`'s return type and
`b::Shape::Circle(2)` got `2_i32`. The trace (`a`'s signature under `b::take` only at codegen time) showed the cause:
the per-file codegen overlay let a "fresher" global entry of another file's same-name function beat the file's own
analysis, and the global promotion copied it back over. Fixed by identity (own definitions keep their own analysis
unless the global entry has the same declared shape), plus module-keyed numeric return types and module-qualified
metadata lookups. Both are gate cases now (`fn_homonyms_across_files`, `enum_payload_homonyms_across_files`); the
second also fixed the literal width, so nothing is left for C4 there.

## Step 3 progress (project gate, differential per pass)

Gate: `cargo test --release --test project_gates` runs over the projects in `tests/project_corpus/`. Each project gets a
golden of the generated Rust, rustc on the generated crate, and a check that the compiled output matches the
hand-computed expectation and the interpreter. Failures are ratcheted in `tests/golden/project_rejects.txt`.

Method, per pass: disable it, then compare four things:
- the project gate;
- the golden corpus;
- the 465 multi-file tests of the `all` binary (names matching `multipass|cross_crate|cross_module|cross_file|bare_pass|returned|library|dogfood`);
- the 18 unit tests of the module.

| Pass | With it disabled | Decision |
|---|---|---|
| `restore_owned_returned_formals` | nothing changed (same 14 failures, all already failing on the merged head; unit tests pass) | deleted. `bare_pass_hint_should_skip` and OWN-RETURNED already keep returned params owned. It also wrote back under the bare name |
| `restore_owned_string_formals_stored_in_payload` | nothing changed | deleted |
| `restore_owned_formals_for_producer_only_call_sites` (and its producer-only scan) | nothing changed | deleted (-345 lines together with the previous row) |
| `restore_owned_field_forward_formals` | projects unchanged (incl. `field_forward_owned`, `field_forward_into_owned_struct`, `let_mut_rebind_consume`); the known failure `dogfood_engine_get_delegation_passes_borrowed_key` passes; only its 2 unit tests fail, and both start from a hand-made MutBorrowed formal | deleted with those 2 tests; the test leaves `.config/known-test-failures.txt` |
| `restore_pub_owned_non_copy_api_formals`, `restore_owned_formals_forwarded_to_owned_callees` | disabled together with the field-forward restore: `bug_p3781_cross_module_aging_borrows_wrong_vec_test` fails, and the known failure `dogfood_engine_get_delegation_passes_borrowed_key` passes | keep for now. The solver must express the p3781 behaviour (move the `Vec`s, borrow the demoted strs) before these can go |

`bare_pass_vec_reader` stays listed. The defining module emits `pub fn total(xs: Vec<int>)` owned (D-OWN-PUB), while
the bare-pass demotion makes the caller pass `&xs`. Deciding D-OWN-PUB (C7) comes first. Deleting the promote pass alone
would turn this into E0382 (see the single-file `pub fn` shape above).

# Failing-test triage, 2026-11-10

Source: the full `cargo test --release --test all` run on integrated head `ca7993c0a` (next/v0.50.0 at that time; the head has since moved to `726c0edc8` and further). The run is recorded in the ledger below, test by test.

Method: the 123 names were re-run with `--exact --test-threads=2` (a) at the last public-push base `4108e5df5` and (b) again at `ca7993c0a`. `4108e5df5` is not an ancestor of the current history (the history was rewritten); the equivalent commit in the current history is `20f859b1a` (trees differ in 9 files, 34 lines). The first-parent range from it to `ca7993c0a` is 112 commits, 77 of them merges.

## Counts

| Class | Tests |
|---|---|
| PRE-EXISTING (fail at base and at head) | 47 |
| REGRESSION since base (pass at base, fail at head) | 51 |
| NEW tests (do not exist at base), failing at head | 6 (5 ledgered, 1 unclassified) |
| ENVIRONMENT / load flake (failed in the full run, pass when re-run at head) | 17 |
| FIXED since base (fail at base, pass at head) | 2 |
| Stray line from a generated fixture (`test_split_lines_crlf_and_trailing_newline`) | counted under `windjammer_fixtures_integration::stdlib_strings_parse` |

Regressions by owner stream: ownership (value formals / IR) 30, C4 width solver 16, interpreter / stdlib runtime 5, goldens-gates 2 (families below; the stream assignment is by test family, not by bisect, except where noted).

## Bisect evidence (budget stopped after 1 completed step)

Representatives run at `895fcf1fa` (2026-11-09, after the green6 merge), a mid-point of the first-parent commits after the base-equivalent: most of the representatives already fail there (so their cause lies between `20f859b1a` and `895fcf1fa`): bug_mutating_method_needs_mut_self, bug_module_file_ctor_must_move_self_field, codegen_fixed_array_in_struct, ecosystem_findings2_interp::path_functions_follow_the_runtime, int_float_arithmetic_complete (f32 *= i32), interpreter_slot_gate, self_consuming_ownership, vec_index_context, vec_repeat_syntax, windjammer_fixtures_integration::stdlib_strings_parse. 5 pass there (so their cause is later than `895fcf1fa`): dogfood_lsm_engine_put_owned_key_after_pending_push, interpreter_conformance_oracle::compiled_panic_formats_its_arguments, mixed_int_float_arithmetic, ownership_integration_test::test_parameter_owned_vec, library_multipass_map_key (hashmap_with_capacity). A second step at `1ac4c7edc` was cancelled. The introducing merge per test is therefore NOT determined; candidate merges are in `git log --first-parent 20f859b1a..895fcf1fa` (ownership-recv "IR-decided value formals", "public formals follow the IR rule", ownership-6 value-formal fixed point, c2/C4 width merges).

## Family verdicts

- REAL REGRESSION, width (C4): `int_float_arithmetic_complete_test::test_compound_assignment_f32_times_int` emits `scale *= count` for f32 *= i32, invalid Rust. Same family: the other int_float_*, mixed_int_float.
- REAL REGRESSION, ownership: `bug_mutating_method_needs_mut_self_test` emits `fn append_edge(self, ..)` that mutates self without `mut` (same family as the "implicit self" ownership merges).
- REAL REGRESSION (design guard), ownership: `bug_module_file_ctor_must_move_self_field_test` now emits `&mut self` plus `self.scene.clone()`; rustc accepts it but the stated guard (no demotion to `&mut self`, move the field) is broken.
- STALE EXPECTATION, width suffixes: `vec_repeat_syntax_test` (4 tests) emits valid `vec![0_i32; 5_usize]`; the assertion expects `vec![0; 5]`. Proposed action: update the assertion to accept the suffixed spelling (owner: C4 width).
- NOT YET CLASSIFIED (no rustc run, no bisect): every other regression row marked UNCLASSIFIED below, notably build_output_golden and convergence_gates_test::gate1 (golden diffs: re-record only after the ownership and width fixes land), interpreter_slot_gate_test, windjammer_fixtures_integration::stdlib_strings_parse, ecosystem_findings2_interp_test, codegen_fixed_array_in_struct_test, vec_index_context_test, toolchain_stale_test::standalone_binary_warns_after_sixty_days_once_per_day, and `tmp_hygiene_test::temp_dir_uses_outside_the_helper_match_the_shrink_only_allowlist` (new test, failing, no ledger entry: treat as a regression).

## Regressions (51)

| Test | Class | Introduced by | Owner stream | Reason | Proposed action |
|---|---|---|---|---|---|
| `bug_demoted_vec_param_into_owned_vec_callee_must_clone_test::reused_vec_second_arg_into_owned_callee_must_clone_not_reborrow` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | ownership (value formals / IR) | not yet run through rustc | classify with rustc, then fix or re-record |
| `bug_engine_i32_range_literal_must_not_emit_i64_suffix_test::engine_i32_range_literal_must_not_emit_i64_suffix` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | ownership (value formals / IR) | not yet run through rustc | classify with rustc, then fix or re-record |
| `bug_from_chars_borrow_call_site_test::from_chars_owned_vec_must_borrow_at_call_site` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | ownership (value formals / IR) | not yet run through rustc | classify with rustc, then fix or re-record |
| `bug_i32_coord_literal_peers_must_not_emit_i64_test::i32_coord_literal_peers_must_not_emit_i64` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | ownership (value formals / IR) | not yet run through rustc | classify with rustc, then fix or re-record |
| `bug_index_owned_method_receiver_test::test_index_owned_method_receiver_clones` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | ownership (value formals / IR) | not yet run through rustc | classify with rustc, then fix or re-record |
| `bug_library_index_owned_method_receiver_test::test_library_module_index_owned_method_receiver_clones` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | ownership (value formals / IR) | not yet run through rustc | classify with rustc, then fix or re-record |
| `bug_migrate_copy_elem_vec_for_in_must_borrow_at_loop_call_sites_test::copy_elem_vec_for_in_callee_must_borrow_at_loop_call_sites` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | ownership (value formals / IR) | not yet run through rustc | classify with rustc, then fix or re-record |
| `bug_module_file_ctor_must_move_self_field_test::module_file_ctor_after_mut_helpers_must_move_self_field` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | ownership (value formals / IR) | not yet run through rustc | classify with rustc, then fix or re-record |
| `bug_module_file_ctor_must_move_self_field_test::module_file_ctor_must_move_self_field` | REAL REGRESSION | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | ownership (value formals / IR) | emits `&mut self` + `self.scene.clone()`; test requires a move (rustc accepts, guard broken) | fix in the compiler; do not add to the known list |
| `bug_module_file_demoted_vec_args_into_owned_cli_must_clone_test::module_file_owned_cli_vec_must_not_receive_ref_args` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | ownership (value formals / IR) | not yet run through rustc | classify with rustc, then fix or re-record |
| `bug_module_file_owned_copy_into_new_must_not_borrow_test::module_file_associated_new_owned_palette_must_not_demote` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | ownership (value formals / IR) | not yet run through rustc | classify with rustc, then fix or re-record |
| `bug_module_file_owned_enum_must_not_clone_before_as_float_test::module_file_noncopy_enum_match_must_not_demote_before_as_float` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | ownership (value formals / IR) | not yet run through rustc | classify with rustc, then fix or re-record |
| `bug_module_file_owned_self_wither_must_move_fields_test::module_file_owned_self_wither_must_move_fields` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | ownership (value formals / IR) | not yet run through rustc | classify with rustc, then fix or re-record |
| `bug_module_file_owned_self_wither_must_move_fields_test::module_file_struct_lit_only_wither_must_move_fields` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | ownership (value formals / IR) | not yet run through rustc | classify with rustc, then fix or re-record |
| `bug_module_file_owned_state_call_must_not_over_borrow_test::module_file_owned_state_call_must_not_over_borrow` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | ownership (value formals / IR) | not yet run through rustc | classify with rustc, then fix or re-record |
| `bug_mutating_method_needs_mut_self_test::field_mutating_method_must_emit_mut_self` | REAL REGRESSION | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | ownership (value formals / IR) | emitted `fn append_edge(self, ..)` mutates self without `mut` | fix in the compiler; do not add to the known list |
| `bug_owned_vec_custom_filter_helper_must_not_demote_and_clone_test::owned_vec_custom_filter_helper_must_not_demote_and_clone` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | ownership (value formals / IR) | not yet run through rustc | classify with rustc, then fix or re-record |
| `bug_owned_vec_formal_beats_forwarding_borrow_product_blend_tree_test::owned_vec_formal_with_forwarding_borrow_must_not_reborrow` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | ownership (value formals / IR) | not yet run through rustc | classify with rustc, then fix or re-record |
| `bug_p3773_replace_demoted_str_beside_owned_vec_test::replace_demoted_str_beside_owned_vec_must_borrow_strings_only` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | ownership (value formals / IR) | not yet run through rustc | classify with rustc, then fix or re-record |
| `bug_passthrough_method_call_test::test_passthrough_to_self_field_method` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | ownership (value formals / IR) | not yet run through rustc | classify with rustc, then fix or re-record |
| `bug_querystring_same_crate_owned_parse_must_not_over_borrow_test::same_crate_owned_parse_must_not_over_borrow` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | ownership (value formals / IR) | not yet run through rustc | classify with rustc, then fix or re-record |
| `bug_std_csv_write_owned_rows_auto_borrow_test::package_csv_write_homonym_owned_rows_must_auto_borrow` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | ownership (value formals / IR) | not yet run through rustc | classify with rustc, then fix or re-record |
| `build_output_golden::sampled_examples_generate_the_recorded_rust` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | goldens-gates | not yet run through rustc | classify with rustc, then fix or re-record |
| `codegen_fixed_array_in_struct_test::test_standalone_array_uses_fixed_syntax` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | C4 width solver | not yet run through rustc | classify with rustc, then fix or re-record |
| `convergence_gates_test::gate1_golden_emitted_rust_and_rustc_accepts` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | goldens-gates | not yet run through rustc | classify with rustc, then fix or re-record |
| `cross_crate_dogfooding_ownership_test::dogfood_lsm_engine_put_owned_key_after_pending_push` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | ownership (value formals / IR) | not yet run through rustc | classify with rustc, then fix or re-record |
| `cross_crate_dogfooding_ownership_test::dogfood_lsm_engine_recover_seed_write_owned_key` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | ownership (value formals / IR) | not yet run through rustc | classify with rustc, then fix or re-record |
| `ecosystem_findings2_interp_test::path_functions_follow_the_runtime` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | interpreter / stdlib runtime | not yet run through rustc | classify with rustc, then fix or re-record |
| `ecosystem_findings_handoff_test::store_threaded_through_adapter_functions_stays_owned` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | ownership (value formals / IR) | not yet run through rustc | classify with rustc, then fix or re-record |
| `int_float_arithmetic_complete_test::test_compound_assignment_f32_times_int` | REAL REGRESSION | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | C4 width solver | emits invalid Rust: f32 *= i32 (E0277), previously cast | fix in the compiler; do not add to the known list |
| `int_float_arithmetic_complete_test::test_impl_block_self_field_int_times_float` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | C4 width solver | not yet run through rustc | classify with rustc, then fix or re-record |
| `int_float_arithmetic_comprehensive_test::test_compound_f32_times_int` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | C4 width solver | not yet run through rustc | classify with rustc, then fix or re-record |
| `int_float_arithmetic_comprehensive_test::test_const_default_int_float` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | C4 width solver | not yet run through rustc | classify with rustc, then fix or re-record |
| `int_float_arithmetic_comprehensive_test::test_impl_self_field_int_times_float` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | C4 width solver | not yet run through rustc | classify with rustc, then fix or re-record |
| `interpreter_conformance_oracle_test::compiled_panic_formats_its_arguments` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | interpreter / stdlib runtime | not yet run through rustc | classify with rustc, then fix or re-record |
| `interpreter_slot_gate_test::a_missing_standard_function_is_an_error_not_a_silent_value` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | interpreter / stdlib runtime | not yet run through rustc | classify with rustc, then fix or re-record |
| `library_multipass_map_key_codegen_test::test_library_multipass_hashmap_with_capacity_usize_no_i64_cast` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | C4 width solver | not yet run through rustc | classify with rustc, then fix or re-record |
| `library_multipass_ownership_codegen_test::test_builder_mutate_then_move_fields_uses_owned_self` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | ownership (value formals / IR) | not yet run through rustc | classify with rustc, then fix or re-record |
| `library_multipass_ownership_codegen_test::test_nested_self_field_snapshot_clones_before_owned_call` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | ownership (value formals / IR) | not yet run through rustc | classify with rustc, then fix or re-record |
| `mixed_int_float_arithmetic_test::test_function_call_int_to_float_autocast` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | C4 width solver | not yet run through rustc | classify with rustc, then fix or re-record |
| `ownership_integration_test::test_parameter_owned_vec` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | ownership (value formals / IR) | not yet run through rustc | classify with rustc, then fix or re-record |
| `self_consuming_ownership_test::test_method_moving_non_copy_field_mid_body` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | ownership (value formals / IR) | not yet run through rustc | classify with rustc, then fix or re-record |
| `toolchain_stale_test::standalone_binary_warns_after_sixty_days_once_per_day` | UNCLASSIFIED (possible ENVIRONMENT) | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | interpreter / stdlib runtime | stale installed wj / same-day warning stamp | classify with rustc, then fix or re-record |
| `types_equal_generic_test::test_vec_passthrough_ownership_inferred_owned` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | ownership (value formals / IR) | not yet run through rustc | classify with rustc, then fix or re-record |
| `vec_index_context_test::test_vec_index_no_clone_on_borrow` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | C4 width solver | not yet run through rustc | classify with rustc, then fix or re-record |
| `vec_index_context_test::test_vec_index_no_clone_on_mut_borrow` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | C4 width solver | not yet run through rustc | classify with rustc, then fix or re-record |
| `vec_repeat_syntax_test::test_vec_repeat_complex_value` | STALE EXPECTATION (representative verified) | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | C4 width solver | emits valid `vec![0_i32; 5_usize]`, test asserts `vec![0; 5]` | update the assertion |
| `vec_repeat_syntax_test::test_vec_repeat_nested` | STALE EXPECTATION (representative verified) | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | C4 width solver | emits valid `vec![0_i32; 5_usize]`, test asserts `vec![0; 5]` | update the assertion |
| `vec_repeat_syntax_test::test_vec_repeat_option` | STALE EXPECTATION (representative verified) | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | C4 width solver | emits valid `vec![0_i32; 5_usize]`, test asserts `vec![0; 5]` | update the assertion |
| `vec_repeat_syntax_test::test_vec_repeat_simple` | STALE EXPECTATION (representative verified) | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | C4 width solver | emits valid `vec![0_i32; 5_usize]`, test asserts `vec![0; 5]` | update the assertion |
| `windjammer_fixtures_integration::stdlib_strings_parse` | UNCLASSIFIED regression | before 895fcf1fa (10 reps) / after (5 reps) / undetermined | interpreter / stdlib runtime | not yet run through rustc | classify with rustc, then fix or re-record |

## New tests failing at head (6)

| Test | Class | Owner | Action |
|---|---|---|---|
| `compiler_findings_widths_test::float_literal_to_f32_crate_parameter` | PRE-EXISTING (ledgered, absent at base) | C4 width solver | stays in the known list |
| `compiler_findings_widths_test::tuple_literal_in_match_arm` | PRE-EXISTING (ledgered, absent at base) | C4 width solver | stays in the known list |
| `drift_repros_test::call_with_a_function_typed_parameter_typechecks` | PRE-EXISTING (ledgered, absent at base) | drift / width | stays in the known list |
| `drift_repros_test::contains_with_a_borrowed_local_typechecks` | PRE-EXISTING (ledgered, absent at base) | drift / width | stays in the known list |
| `drift_repros_test::explicit_ref_of_an_integer_literal_typechecks` | PRE-EXISTING (ledgered, absent at base) | drift / width | stays in the known list |
| `tmp_hygiene_test::temp_dir_uses_outside_the_helper_match_the_shrink_only_allowlist` | UNCLASSIFIED (no ledger) | tooling | investigate; not in the known list |

## Environment / load flakes (17, pass at head)

The installed `wj` was stale (commit 591818c0a), which explains the promise_registry case; the rest failed only under full-suite load and pass at `--test-threads=2`.

- `check_build_differential_test::check_agrees_with_the_build_pipeline`
- `codegen_correctness_test::closure_capture_mutation`
- `codegen_ensures_param_access_test::test_ensures_can_access_moved_parameters`
- `codegen_ensures_param_access_test::test_ensures_with_multiple_moved_parameters`
- `eject_test::multi_file_project_ejects_like_wj_build`
- `optimizer_ratchet_test::optimizer_ratchet_waste_counts_do_not_increase`
- `promise_registry_test::acceptance_cases_hold_for_the_installed_binary`
- `python_port_gates_test::a_damaged_expectation_fails_the_gate`
- `python_port_gates_test::every_port_matches_its_recorded_original`
- `simple_http_server_example_test::example_47_simple_http_server_compiles_with_runtime`
- `spec_conformance_test::stdlib_index_lists_every_std_module`
- `std_ops_import_integration_test::test_std_ops_imports_map_to_rust_stdlib`
- `toolchain_identity_test::identity_matches_git_when_built_from_a_checkout`
- `toolchain_stale_test::silence_switch_suppresses_the_warning_and_the_stamp`
- `vec_capacity_usize_test::test_vec_push_float_unification`
- `vec_capacity_usize_test::test_vec_with_capacity_literal`
- `vec_capacity_usize_test::test_vec_with_capacity_variable`

## Fixed since base (2)

- `public_hygiene_test::tracked_files_contain_no_private_project_names`
- `span_oracle_suite::diag_gate::repo_wide_check_diagnostics_are_annotated_or_ratcheted`

## Pre-existing (47)

Listed, owner-tagged and grouped, in `.config/known-test-failures.txt` (fail at base `4108e5df5` and at `ca7993c0a`). Six entries in the old known list pass in the full run and were removed (typed_closure_string_predicate_must_cargo_check, compiler_findings_widths_test::float_expression_to_float_parameter, round2_findings_test x2, svo_bfs_layout_test, windjammer_fixtures_integration::regression_vec_tuple_string_push). The 20 library unit-test entries come from the old list unchanged; the `all` run does not cover them, so they are unverified here.

- `auto_borrow_vec_new_arg_test::test_auto_borrow_vec_new_at_call_site`
- `backend_conformance_test::corpus_conforms_on_every_backend`
- `bug_auth_api_cross_crate_cookie_map_get_must_not_to_string_test::cookie_build_path_dep_map_get_must_not_own_key`
- `bug_cross_crate_demoted_str_owned_arg_must_auto_borrow_test::cross_crate_demoted_str_owned_arg_must_auto_borrow`
- `bug_cross_crate_owned_encode_named_fn_must_not_borrow_arg_test::cross_crate_owned_encode_named_fn_must_not_borrow_arg`
- `bug_cross_crate_owned_handle_loop_reassign_must_not_emit_mut_ref_test::cross_crate_owned_handle_loop_reassign_must_not_emit_mut_ref`
- `bug_cross_crate_set_if_mut_borrow_test::test_cross_crate_set_if_mut_borrow_when_callee_missing_from_metadata`
- `bug_demoted_str_caller_into_owned_formal_test::multipass_demoted_str_into_cross_crate_owned_formal_must_cargo_check`
- `bug_import_alias_must_not_steal_foreign_fn_ownership_test::import_alias_must_not_steal_foreign_fn_ownership`
- `bug_module_file_feed_unified_owned_startup_must_not_borrow_test::module_file_owned_startup_must_not_receive_ref_args`
- `bug_module_file_readonly_vec_formal_must_not_clone_field_test::module_file_readonly_vec_formal_must_not_clone_field`
- `bug_multipass_cross_crate_render_owned_template_helper_test::multipass_cross_crate_render_owned_template_helper_must_cargo_check`
- `bug_qs_get_literal_into_demoted_key_must_not_string_from_test::qs_get_literal_into_demoted_key_must_not_string_from`
- `bug_regex_is_match_owned_formals_must_receive_owned_test::regex_is_match_owned_formals_must_receive_owned`
- `bug_todo_cli_cross_crate_validate_field_must_auto_borrow_test::cross_crate_validate_field_must_auto_borrow_not_value`
- `bug_untyped_zero_plus_vec_len_must_not_emit_i64_test::untyped_zero_plus_vec_len_must_not_emit_i64`
- `codegen_correctness_test::option_param_consumed`
- `codegen_mcp_frame_content_length_rustc_test::mcp_frame_int_to_dec_len_must_rustc`
- `codegen_proactive_hardening_gates_test::len_to_int_formal_substring_must_cargo_check`
- `codegen_string_len_int_formal_test::string_len_passed_to_int_formal_must_agree`
- `codegen_wj_test_crate_str_formal_format_temp_test::wj_test_crate_format_temps_must_match_demoted_str_formals`
- `codegen_wj_test_crate_str_formal_format_temp_test::wj_test_crate_hashmap_formal_must_accept_borrowed_query`
- `cross_crate_dogfooding_ownership_test::dogfood_substrate_full_store_patch_part_enum_value_layout`
- `cross_crate_dogfooding_ownership_test::dogfood_wal_ffi_snapshot_and_path_borrow_at_call_sites`
- `cross_module_ownership_test::cross_module_trait_field_access_inferred_borrowed`
- `ecosystem_findings2_test::sender_send_string_channel_still_converts_literals`
- `float_inference_field_initializer_test::test_struct_tuple_field_with_type_alias`
- `inference_differential_test::corpus_closure_len_arithmetic_width`
- `inference_differential_test::corpus_closure_mutating_capture_is_mutable`
- `inference_differential_test::corpus_explicit_ref_pin_is_not_double_borrowed`
- `ir_shadow_validation_test::shadow_validation_clean_for_borrowed_string_param`
- `library_multipass_map_key_codegen_test::test_library_multipass_graph_bfs_hashmap_compiles`
- `library_multipass_map_key_codegen_test::test_library_multipass_hashmap_str_key_no_to_string`
- `library_multipass_map_key_codegen_test::test_library_multipass_tuple_writeback_must_not_clone`
- `library_multipass_ownership_codegen_test::test_hashset_contains_no_double_borrow_on_borrowed_string_param`
- `library_multipass_ownership_codegen_test::test_match_borrow_break_ref_binding_clones_non_copy_in_tuple`
- `library_multipass_ownership_codegen_test::test_match_borrow_break_self_index_clone_yields_owned_enum_bindings`
- `library_multipass_ownership_codegen_test::test_static_self_method_borrows_vec_param_not_clone`
- `ownership_workaround_regression_test::dogfood_authz_owned_store_method_returns_self_and_bool`
- `path_and_bytes_borrow_tests::path_bytes_ffi_vec_return_borrowed_at_from_bytes`
- `path_and_bytes_borrow_tests::path_bytes_segment_append_delete_borrows_key_param`
- `path_and_bytes_borrow_tests::path_bytes_segment_append_put_borrows_key_value_params`
- `ref_mut_logic_layered_test::test_if_let_method_return_option`
- `round2_findings_test::json_parse_target_type_inferred_from_later_use`
- `round2_findings_test::untyped_counter_compared_with_len_meets_int_formal`
- `server_serve_handler_ownership_test::serve_closure_passes_request_to_handler_without_double_borrow`
- `type_inference_vec3_f32_test::test_vec3_cross_module_inference`

## Top fixes by tests unblocked

1. Ownership value-formal regressions (IR value-formal / mut-self / self-move): the regression tests, plus the ledgered ones.
2. C4 width solver: f32 *= i32 casts, suffix spelling and usize formals: a group of tests (some are stale assertions fixable by editing the tests).
3. Re-record goldens (build_output_golden, convergence_gates_test::gate1) after 1 and 2 land: 2 tests.
4. Interpreter / stdlib runtime (slot gate, split_lines CRLF, path functions): 5 tests.
5. Stale installed binary and load flakes: fixed by `wj install-dev` and not running the full suite above 2 threads.

//! Consolidated test binary — includes all test files as modules.
//!
//! This eliminates 843 separate link operations, reducing full test suite
//! runtime from 14+ minutes to ~2-3 minutes.
//!
//! Run all tests:  cargo test --release --test all
//! Run one module: cargo test --release --test all -- module_name::test_fn

// Shared test utilities — available to all test modules.
#[path = "common/test_utils.rs"]
mod test_utils;

#[path = "ambiguous_reexports_test.rs"]
mod ambiguous_reexports_test;

#[path = "analyzer_auto_mutability_compound_test.rs"]
mod analyzer_auto_mutability_compound_test;

#[path = "analyzer_auto_mutability_method_test.rs"]
mod analyzer_auto_mutability_method_test;

#[path = "analyzer_auto_mutability_test.rs"]
mod analyzer_auto_mutability_test;

#[path = "analyzer_field_method_mutation_test.rs"]
mod analyzer_field_method_mutation_test;

#[path = "analyzer_method_call_mut_propagation_test.rs"]
mod analyzer_method_call_mut_propagation_test;

#[path = "analyzer_ownership_control_flow_test.rs"]
mod analyzer_ownership_control_flow_test;

#[path = "analyzer_ownership_params_test.rs"]
mod analyzer_ownership_params_test;

#[path = "analyzer_ownership_trait_generic_test.rs"]
mod analyzer_ownership_trait_generic_test;

#[path = "analyzer_self_mutability_inference_test.rs"]
mod analyzer_self_mutability_inference_test;

#[path = "analyzer_self_receiver_inference_test.rs"]
mod analyzer_self_receiver_inference_test;

#[path = "analyzer_storage_comprehensive_tests.rs"]
mod analyzer_storage_comprehensive_tests;

#[path = "analyzer_string_field_assignment_test.rs"]
mod analyzer_string_field_assignment_test;

#[path = "analyzer_traits_comprehensive_tests.rs"]
mod analyzer_traits_comprehensive_tests;

#[path = "array_element_copy_ownership_test.rs"]
mod array_element_copy_ownership_test;

#[path = "array_index_copy_type_test.rs"]
mod array_index_copy_type_test;

#[path = "array_index_ownership_test.rs"]
mod array_index_ownership_test;

#[path = "array_indexing_i32_test.rs"]
mod array_indexing_i32_test;

#[path = "array_literal_codegen_test.rs"]
mod array_literal_codegen_test;

#[path = "asi_paren_integration_test.rs"]
mod asi_paren_integration_test;

#[path = "assignment_type_inference_test.rs"]
mod assignment_type_inference_test;

#[path = "ast_builders_tests.rs"]
mod ast_builders_tests;

#[path = "auto_borrow_methods_test.rs"]
mod auto_borrow_methods_test;

#[path = "auto_clone_loop_and_partial_move_test.rs"]
mod auto_clone_loop_and_partial_move_test;

#[path = "auto_clone_method_receiver_test.rs"]
mod auto_clone_method_receiver_test;

#[path = "auto_deref_regression_test.rs"]
mod auto_deref_regression_test;

#[path = "auto_derive_comprehensive_tests.rs"]
mod auto_derive_comprehensive_tests;

#[path = "auto_derive_copy_string_field_test.rs"]
mod auto_derive_copy_string_field_test;

#[path = "auto_derive_trait_object_field_test.rs"]
mod auto_derive_trait_object_field_test;

#[path = "auto_import_generation_test.rs"]
mod auto_import_generation_test;

#[path = "auto_mut_borrow_arg_test.rs"]
mod auto_mut_borrow_arg_test;

#[path = "auto_ref_deref_copy_test.rs"]
mod auto_ref_deref_copy_test;

#[path = "auto_ref_method_args_test.rs"]
mod auto_ref_method_args_test;

#[path = "auto_ref_test.rs"]
mod auto_ref_test;

#[path = "auto_self_inference_test.rs"]
mod auto_self_inference_test;

#[path = "auto_to_string_test.rs"]
mod auto_to_string_test;

#[path = "binary_ops_3layer_test.rs"]
mod binary_ops_3layer_test;

#[path = "block_semicolon_test.rs"]
mod block_semicolon_test;

#[path = "borrow_break_as_ref_test.rs"]
mod borrow_break_as_ref_test;

#[path = "borrow_context_no_clone_test.rs"]
mod borrow_context_no_clone_test;

#[path = "borrowed_field_clone_test.rs"]
mod borrowed_field_clone_test;

#[path = "borrowed_iter_copy_field_test.rs"]
mod borrowed_iter_copy_field_test;

#[path = "borrowed_string_as_str_test.rs"]
mod borrowed_string_as_str_test;

#[path = "borrowed_string_parameter_generation_test.rs"]
mod borrowed_string_parameter_generation_test;

#[path = "bug_astar_break_test.rs"]
mod bug_astar_break_test;

#[path = "bug_auto_borrow_collision_test.rs"]
mod bug_auto_borrow_collision_test;

#[path = "bug_auto_clone_nested_call_then_struct_test.rs"]
mod bug_auto_clone_nested_call_then_struct_test;

#[path = "bug_clamp_float_args_match_receiver_test.rs"]
mod bug_clamp_float_args_match_receiver_test;

#[path = "bug_copy_field_to_le_bytes_owned_param_test.rs"]
mod bug_copy_field_to_le_bytes_owned_param_test;

#[path = "bug_crate_import_build_prefix_test.rs"]
mod bug_crate_import_build_prefix_test;

#[path = "bug_cross_crate_borrowed_method_call_test.rs"]
mod bug_cross_crate_borrowed_method_call_test;

#[path = "bug_e0277_hashmap_self_field_test.rs"]
mod bug_e0277_hashmap_self_field_test;

#[path = "bug_e0277_self_field_hashmap_test.rs"]
mod bug_e0277_self_field_hashmap_test;

#[path = "bug_e0308_borrowed_struct_field_test.rs"]
mod bug_e0308_borrowed_struct_field_test;

#[path = "bug_e0308_clone_to_ref_test.rs"]
mod bug_e0308_clone_to_ref_test;

#[path = "bug_e0308_field_assign_ref_test.rs"]
mod bug_e0308_field_assign_ref_test;

#[path = "bug_e0596_self_mutation_inference_test.rs"]
mod bug_e0596_self_mutation_inference_test;

#[path = "bug_enum_variant_consumes_param_test.rs"]
mod bug_enum_variant_consumes_param_test;

#[path = "bug_extern_fn_ownership_test.rs"]
mod bug_extern_fn_ownership_test;

#[path = "bug_extern_fn_pub_test.rs"]
mod bug_extern_fn_pub_test;

#[path = "bug_extern_fn_string_expr_arg_test.rs"]
mod bug_extern_fn_string_expr_arg_test;

#[path = "bug_extern_fn_string_to_ffi_no_borrow_test.rs"]
mod bug_extern_fn_string_to_ffi_no_borrow_test;

#[path = "bug_f32_f64_explicit_cast_test.rs"]
mod bug_f32_f64_explicit_cast_test;

#[path = "bug_float_method_ambiguity_test.rs"]
mod bug_float_method_ambiguity_test;

#[path = "bug_hashmap_get_node_string_borrowed_test.rs"]
mod bug_hashmap_get_node_string_borrowed_test;

#[path = "bug_hashmap_string_key_test.rs"]
mod bug_hashmap_string_key_test;

#[path = "bug_if_without_else_unit_test.rs"]
mod bug_if_without_else_unit_test;

#[path = "bug_index_assign_stores_param_test.rs"]
mod bug_index_assign_stores_param_test;

#[path = "bug_index_owned_method_receiver_test.rs"]
mod bug_index_owned_method_receiver_test;

#[path = "bug_is_stored_enum_option_test.rs"]
mod bug_is_stored_enum_option_test;

#[path = "bug_let_method_mut_inference_test.rs"]
mod bug_let_method_mut_inference_test;

#[path = "bug_library_index_owned_method_receiver_test.rs"]
mod bug_library_index_owned_method_receiver_test;

#[path = "bug_local_readonly_method_not_mut_borrow_test.rs"]
mod bug_local_readonly_method_not_mut_borrow_test;

#[path = "bug_match_arm_string_no_double_convert_test.rs"]
mod bug_match_arm_string_no_double_convert_test;

#[path = "bug_metadata_borrowed_param_ownership_test.rs"]
mod bug_metadata_borrowed_param_ownership_test;

#[path = "bug_method_mut_inference_test.rs"]
mod bug_method_mut_inference_test;

#[path = "bug_method_return_binary_float_test.rs"]
mod bug_method_return_binary_float_test;

#[path = "bug_method_self_by_value_test.rs"]
mod bug_method_self_by_value_test;

#[path = "bug_module_string_borrow_call_test.rs"]
mod bug_module_string_borrow_call_test;

#[path = "bug_multi_arg_borrow_symmetry_test.rs"]
mod bug_multi_arg_borrow_symmetry_test;

#[path = "bug_multifile_self_field_mutation_test.rs"]
mod bug_multifile_self_field_mutation_test;

#[path = "bug_multipass_ownership_test.rs"]
mod bug_multipass_ownership_test;

#[path = "bug_multipass_self_ownership_convergence_test.rs"]
mod bug_multipass_self_ownership_convergence_test;

#[path = "bug_mut_reborrow_codegen_test.rs"]
mod bug_mut_reborrow_codegen_test;

#[path = "bug_nested_field_access_test.rs"]
mod bug_nested_field_access_test;

#[path = "bug_nested_field_borrowed_test.rs"]
mod bug_nested_field_borrowed_test;

#[path = "bug_nested_field_method_auto_borrow_test.rs"]
mod bug_nested_field_method_auto_borrow_test;

#[path = "bug_no_manual_unsafe_extern_test.rs"]
mod bug_no_manual_unsafe_extern_test;

#[path = "bug_no_to_string_literal_leakage_test.rs"]
mod bug_no_to_string_literal_leakage_test;

#[path = "bug_option_unwrap_borrow_test.rs"]
mod bug_option_unwrap_borrow_test;

#[path = "bug_ownership_inference_passthrough_test.rs"]
mod bug_ownership_inference_passthrough_test;

#[path = "bug_passthrough_method_call_test.rs"]
mod bug_passthrough_method_call_test;

#[path = "bug_phase2_stable_override_mismatch_test.rs"]
mod bug_phase2_stable_override_mismatch_test;

#[path = "bug_println_bool_test.rs"]
mod bug_println_bool_test;

#[path = "bug_quest_objective_test.rs"]
mod bug_quest_objective_test;

#[path = "bug_quest_state_test.rs"]
mod bug_quest_state_test;

#[path = "bug_redundant_as_str_test.rs"]
mod bug_redundant_as_str_test;

#[path = "bug_self_field_method_call_mut_inference_test.rs"]
mod bug_self_field_method_call_mut_inference_test;

#[path = "bug_self_subfield_copy_read_test.rs"]
mod bug_self_subfield_copy_read_test;

#[path = "bug_snapshot_returns_self_type_not_owned_test.rs"]
mod bug_snapshot_returns_self_type_not_owned_test;

#[path = "bug_stale_game_metadata_overwrites_engine_test.rs"]
mod bug_stale_game_metadata_overwrites_engine_test;

#[path = "bug_static_method_call_int_cast_test.rs"]
mod bug_static_method_call_int_cast_test;

#[path = "bug_std_strings_parse_api_test.rs"]
mod bug_std_strings_parse_api_test;

#[path = "bug_std_strings_scene_parser_test.rs"]
mod bug_std_strings_scene_parser_test;

#[path = "bug_string_clone_to_string_test.rs"]
mod bug_string_clone_to_string_test;

#[path = "bug_string_coercion_test.rs"]
mod bug_string_coercion_test;

#[path = "bug_string_comparison_deref_test.rs"]
mod bug_string_comparison_deref_test;

#[path = "bug_string_comparison_explicit_str_ref_test.rs"]
mod bug_string_comparison_explicit_str_ref_test;

#[path = "bug_struct_builder_max_not_f64_test.rs"]
mod bug_struct_builder_max_not_f64_test;

#[path = "bug_struct_field_auto_clone_test.rs"]
mod bug_struct_field_auto_clone_test;

#[path = "bug_struct_literal_string_param_reuse_test.rs"]
mod bug_struct_literal_string_param_reuse_test;

#[path = "bug_test_target_detection.rs"]
mod bug_test_target_detection;

#[path = "bug_to_gpu_buffer_not_mut_borrow_test.rs"]
mod bug_to_gpu_buffer_not_mut_borrow_test;

#[path = "bug_transitive_self_mut_orchestrator_test.rs"]
mod bug_transitive_self_mut_orchestrator_test;

#[path = "bug_typed_int_literal_test.rs"]
mod bug_typed_int_literal_test;

#[path = "bug_untyped_let_string_ascription_test.rs"]
mod bug_untyped_let_string_ascription_test;

#[path = "bug_usize_with_capacity_cast_clone_test.rs"]
mod bug_usize_with_capacity_cast_clone_test;

#[path = "bug_vec_push_spurious_clone_test.rs"]
mod bug_vec_push_spurious_clone_test;

#[path = "build_system_ffi_deps_test.rs"]
mod build_system_ffi_deps_test;

#[path = "build_system_relative_paths_test.rs"]
mod build_system_relative_paths_test;

#[path = "build_system_stale_cargo_toml_test.rs"]
mod build_system_stale_cargo_toml_test;

#[path = "build_system_test_module_gating_test.rs"]
mod build_system_test_module_gating_test;

#[path = "cache_locality_analysis_test.rs"]
mod cache_locality_analysis_test;

#[path = "cast_plus_int_fix_test.rs"]
mod cast_plus_int_fix_test;

#[path = "cast_with_auto_ref_test.rs"]
mod cast_with_auto_ref_test;

#[path = "cfg_decorator_test.rs"]
mod cfg_decorator_test;

#[path = "chained_call_mutation_false_positive_test.rs"]
mod chained_call_mutation_false_positive_test;

#[path = "circle_aabb_type_consistency_test.rs"]
mod circle_aabb_type_consistency_test;

#[path = "cli_lint_test.rs"]
mod cli_lint_test;

#[path = "cloned_iter_copy_field_test.rs"]
mod cloned_iter_copy_field_test;

#[path = "codegen_arm_string_analysis_test.rs"]
mod codegen_arm_string_analysis_test;

#[path = "codegen_ast_utilities_test.rs"]
mod codegen_ast_utilities_test;

#[path = "codegen_auto_modules_test.rs"]
mod codegen_auto_modules_test;

#[path = "codegen_cast_precedence_test.rs"]
mod codegen_cast_precedence_test;

#[path = "codegen_closures_comprehensive_tests.rs"]
mod codegen_closures_comprehensive_tests;

#[path = "codegen_constant_folding_test.rs"]
mod codegen_constant_folding_test;

#[path = "codegen_copy_type_arg_test.rs"]
mod codegen_copy_type_arg_test;

#[path = "codegen_copy_type_method_arg_test.rs"]
mod codegen_copy_type_method_arg_test;

#[path = "codegen_copy_type_no_clone_test.rs"]
mod codegen_copy_type_no_clone_test;

#[path = "codegen_cross_module_signature_test.rs"]
mod codegen_cross_module_signature_test;

#[path = "codegen_dialog_comprehensive_test.rs"]
mod codegen_dialog_comprehensive_test;

#[path = "codegen_dialog_ownership_inference_test.rs"]
mod codegen_dialog_ownership_inference_test;

#[path = "codegen_dialog_pattern_test.rs"]
mod codegen_dialog_pattern_test;

#[path = "codegen_dialog_remaining_issues_test.rs"]
mod codegen_dialog_remaining_issues_test;

#[path = "codegen_ensures_param_access_test.rs"]
mod codegen_ensures_param_access_test;

#[path = "codegen_enum_copy_derive_test.rs"]
mod codegen_enum_copy_derive_test;

#[path = "codegen_enumerate_no_double_iter_test.rs"]
mod codegen_enumerate_no_double_iter_test;

#[path = "codegen_expression_helpers_test.rs"]
mod codegen_expression_helpers_test;

#[path = "codegen_fixed_array_in_struct_test.rs"]
mod codegen_fixed_array_in_struct_test;

#[path = "codegen_float_binary_op_inference_test.rs"]
mod codegen_float_binary_op_inference_test;

#[path = "codegen_float_method_return_inference_test.rs"]
mod codegen_float_method_return_inference_test;

#[path = "codegen_for_loop_ownership_test.rs"]
mod codegen_for_loop_ownership_test;

#[path = "codegen_format_macro_test.rs"]
mod codegen_format_macro_test;

#[path = "codegen_forward_reference_limitation_test.rs"]
mod codegen_forward_reference_limitation_test;

#[path = "codegen_gameloop_ambiguity_test.rs"]
mod codegen_gameloop_ambiguity_test;

#[path = "codegen_generics_comprehensive_tests.rs"]
mod codegen_generics_comprehensive_tests;

#[path = "codegen_glob_import_ambiguity_test.rs"]
mod codegen_glob_import_ambiguity_test;

#[path = "codegen_hashmap_auto_borrow_test.rs"]
mod codegen_hashmap_auto_borrow_test;

#[path = "codegen_hashmap_comment_false_positive_test.rs"]
mod codegen_hashmap_comment_false_positive_test;

#[path = "codegen_hashmap_remove_test.rs"]
mod codegen_hashmap_remove_test;

#[path = "codegen_helpers_test.rs"]
mod codegen_helpers_test;

#[path = "codegen_if_else_expr_in_assignment_test.rs"]
mod codegen_if_else_expr_in_assignment_test;

#[path = "codegen_if_else_expression_test.rs"]
mod codegen_if_else_expression_test;

#[path = "codegen_if_let_bound_variable_test.rs"]
mod codegen_if_let_bound_variable_test;

#[path = "codegen_if_let_expression_test.rs"]
mod codegen_if_let_expression_test;

#[path = "codegen_if_let_method_call_test.rs"]
mod codegen_if_let_method_call_test;

#[path = "codegen_if_let_string_return_test.rs"]
mod codegen_if_let_string_return_test;

#[path = "codegen_import_alias_test.rs"]
mod codegen_import_alias_test;

#[path = "codegen_index_element_type_test.rs"]
mod codegen_index_element_type_test;

#[path = "codegen_loops_comprehensive_tests.rs"]
mod codegen_loops_comprehensive_tests;

#[path = "codegen_main_in_lib_test.rs"]
mod codegen_main_in_lib_test;

#[path = "codegen_match_arm_expression_vs_statement_test.rs"]
mod codegen_match_arm_expression_vs_statement_test;

#[path = "codegen_match_arm_ownership_test.rs"]
mod codegen_match_arm_ownership_test;

#[path = "codegen_match_arm_semicolons_test.rs"]
mod codegen_match_arm_semicolons_test;

#[path = "codegen_match_arm_single_tuple_test.rs"]
mod codegen_match_arm_single_tuple_test;

#[path = "codegen_match_comprehensive_tests.rs"]
mod codegen_match_comprehensive_tests;

#[path = "codegen_method_call_arg_copy_type_test.rs"]
mod codegen_method_call_arg_copy_type_test;

#[path = "codegen_method_call_borrow_inconsistency_test.rs"]
mod codegen_method_call_borrow_inconsistency_test;

#[path = "codegen_method_call_clone_bug_test.rs"]
mod codegen_method_call_clone_bug_test;

#[path = "codegen_method_call_in_if_test.rs"]
mod codegen_method_call_in_if_test;

#[path = "codegen_method_call_on_variable_test.rs"]
mod codegen_method_call_on_variable_test;

#[path = "codegen_method_call_syntax_test.rs"]
mod codegen_method_call_syntax_test;

#[path = "codegen_method_calls_comprehensive_tests.rs"]
mod codegen_method_calls_comprehensive_tests;

#[path = "codegen_module_string_coercion_test.rs"]
mod codegen_module_string_coercion_test;

#[path = "codegen_multiple_method_call_args_test.rs"]
mod codegen_multiple_method_call_args_test;

#[path = "codegen_nested_module_imports_test.rs"]
mod codegen_nested_module_imports_test;

#[path = "codegen_operators_test.rs"]
mod codegen_operators_test;

#[path = "codegen_option_ref_return_test.rs"]
mod codegen_option_ref_return_test;

#[path = "codegen_owned_param_field_assignment_test.rs"]
mod codegen_owned_param_field_assignment_test;

#[path = "codegen_param_mutability_test.rs"]
mod codegen_param_mutability_test;

#[path = "codegen_param_passed_to_owned_function_test.rs"]
mod codegen_param_passed_to_owned_function_test;

#[path = "codegen_paren_precedence_test.rs"]
mod codegen_paren_precedence_test;

#[path = "codegen_parent_module_reexport_test.rs"]
mod codegen_parent_module_reexport_test;

#[path = "codegen_pattern_analysis_test.rs"]
mod codegen_pattern_analysis_test;

#[path = "codegen_property_test_imports_test.rs"]
mod codegen_property_test_imports_test;

#[path = "codegen_pub_const_test.rs"]
mod codegen_pub_const_test;

#[path = "codegen_pub_use_module_path_test.rs"]
mod codegen_pub_use_module_path_test;

#[path = "codegen_qualified_call_string_literal_test.rs"]
mod codegen_qualified_call_string_literal_test;

#[path = "codegen_qualified_struct_init_test.rs"]
mod codegen_qualified_struct_init_test;

#[path = "codegen_ref_string_auto_clone_test.rs"]
mod codegen_ref_string_auto_clone_test;

#[path = "codegen_ref_string_param_clone_test.rs"]
mod codegen_ref_string_param_clone_test;

#[path = "codegen_sibling_module_imports_test.rs"]
mod codegen_sibling_module_imports_test;

#[path = "codegen_simd_vectorization_test.rs"]
mod codegen_simd_vectorization_test;

#[path = "codegen_str_to_string_test.rs"]
mod codegen_str_to_string_test;

#[path = "codegen_string_analysis_test.rs"]
mod codegen_string_analysis_test;

#[path = "codegen_string_comprehensive_tests.rs"]
mod codegen_string_comprehensive_tests;

#[path = "codegen_string_extended_test.rs"]
mod codegen_string_extended_test;

#[path = "codegen_string_literal_autoconvert_test.rs"]
mod codegen_string_literal_autoconvert_test;

#[path = "codegen_string_param_ownership_test.rs"]
mod codegen_string_param_ownership_test;

#[path = "codegen_string_str_conversion_test.rs"]
mod codegen_string_str_conversion_test;

#[path = "codegen_struct_field_str_shorthand_test.rs"]
mod codegen_struct_field_str_shorthand_test;

#[path = "codegen_struct_init_fields_test.rs"]
mod codegen_struct_init_fields_test;

#[path = "codegen_struct_init_in_loop_test.rs"]
mod codegen_struct_init_in_loop_test;

#[path = "codegen_struct_pattern_shorthand_test.rs"]
mod codegen_struct_pattern_shorthand_test;

#[path = "codegen_test_attribute_test.rs"]
mod codegen_test_attribute_test;

#[path = "codegen_test_cases_string_inference_test.rs"]
mod codegen_test_cases_string_inference_test;

#[path = "codegen_trait_impl_method_call_args_test.rs"]
mod codegen_trait_impl_method_call_args_test;

#[path = "codegen_type_analysis_test.rs"]
mod codegen_type_analysis_test;

#[path = "codegen_unnecessary_parens_test.rs"]
mod codegen_unnecessary_parens_test;

#[path = "codegen_unused_variable_prefix_test.rs"]
mod codegen_unused_variable_prefix_test;

#[path = "codegen_user_defined_fn_shadow_test.rs"]
mod codegen_user_defined_fn_shadow_test;

#[path = "codegen_usize_inference_test.rs"]
mod codegen_usize_inference_test;

#[path = "codegen_vec3_copy_no_ref_test.rs"]
mod codegen_vec3_copy_no_ref_test;

#[path = "codegen_vec_copy_struct_test.rs"]
mod codegen_vec_copy_struct_test;

#[path = "codegen_vec_index_borrow_test.rs"]
mod codegen_vec_index_borrow_test;

#[path = "codegen_vec_ref_push_test.rs"]
mod codegen_vec_ref_push_test;

#[path = "codegen_vec_remove_usize_test.rs"]
mod codegen_vec_remove_usize_test;

#[path = "codegen_vec_string_context_test.rs"]
mod codegen_vec_string_context_test;

#[path = "codegen_windjammer_ui_bugs_test.rs"]
mod codegen_windjammer_ui_bugs_test;

#[path = "collect_turbofish_test.rs"]
mod collect_turbofish_test;

#[path = "comparison_cast_removal_test.rs"]
mod comparison_cast_removal_test;

#[path = "comparison_int_vs_usize_test.rs"]
mod comparison_int_vs_usize_test;

#[path = "comparison_no_clone_test.rs"]
mod comparison_no_clone_test;

#[path = "compiler_improvements_integration_test.rs"]
mod compiler_improvements_integration_test;

#[path = "compiler_test.rs"]
mod compiler_test;

#[path = "compiler_tests.rs"]
mod compiler_tests;

#[path = "compound_assign_int_no_float_cast_test.rs"]
mod compound_assign_int_no_float_cast_test;

#[path = "compound_assign_test.rs"]
mod compound_assign_test;

#[path = "compound_assignment_f32_test.rs"]
mod compound_assignment_f32_test;

#[path = "compound_assignment_math_test.rs"]
mod compound_assignment_math_test;

#[path = "compound_assignment_string_test.rs"]
mod compound_assignment_string_test;

#[path = "compound_operators_test.rs"]
mod compound_operators_test;

#[path = "comprehensive_lexer_tests.rs"]
mod comprehensive_lexer_tests;

#[path = "conformance_go_test.rs"]
mod conformance_go_test;

#[path = "conformance_interpreter_test.rs"]
mod conformance_interpreter_test;

#[path = "conformance_js_test.rs"]
mod conformance_js_test;

#[path = "conformance_rust_test.rs"]
mod conformance_rust_test;

#[path = "conformance_tests.rs"]
mod conformance_tests;

#[path = "constant_folding_f32_test.rs"]
mod constant_folding_f32_test;

#[path = "constructor_no_self_param_test.rs"]
mod constructor_no_self_param_test;

#[path = "constructor_type_inference_test.rs"]
mod constructor_type_inference_test;

#[path = "copy_detection_generic_test.rs"]
mod copy_detection_generic_test;

#[path = "copy_detection_vec_field_test.rs"]
mod copy_detection_vec_field_test;

#[path = "copy_semantics_integration_test.rs"]
mod copy_semantics_integration_test;

#[path = "copy_semantics_test.rs"]
mod copy_semantics_test;

#[path = "copy_tuple_pattern_binding_test.rs"]
mod copy_tuple_pattern_binding_test;

#[path = "copy_type_method_params_integration_test.rs"]
mod copy_type_method_params_integration_test;

#[path = "copy_type_no_clone_test.rs"]
mod copy_type_no_clone_test;

#[path = "copy_type_parameter_inference_test.rs"]
mod copy_type_parameter_inference_test;

#[path = "copy_type_passthrough_mut_test.rs"]
mod copy_type_passthrough_mut_test;

#[path = "crate_imports_test.rs"]
mod crate_imports_test;

#[path = "crate_internal_not_extern_test.rs"]
mod crate_internal_not_extern_test;

#[path = "crate_module_call_no_runtime_ffi_test.rs"]
mod crate_module_call_no_runtime_ffi_test;

#[path = "cross_crate_string_literal_test.rs"]
mod cross_crate_string_literal_test;

#[path = "cross_file_auto_borrow_call_site_test.rs"]
mod cross_file_auto_borrow_call_site_test;

#[path = "cross_file_copy_field_no_clone_test.rs"]
mod cross_file_copy_field_no_clone_test;

#[path = "cross_file_fn_call_ref_arg_test.rs"]
mod cross_file_fn_call_ref_arg_test;

#[path = "cross_file_i32_subtraction_test.rs"]
mod cross_file_i32_subtraction_test;

#[path = "cross_file_method_arg_passthrough_mut_test.rs"]
mod cross_file_method_arg_passthrough_mut_test;

#[path = "cross_file_mut_passthrough_call_site_test.rs"]
mod cross_file_mut_passthrough_call_site_test;

#[path = "cross_file_ownership_meta_test.rs"]
mod cross_file_ownership_meta_test;

#[path = "cross_file_trait_impl_test.rs"]
mod cross_file_trait_impl_test;

#[path = "cross_file_trait_inference_regeneration_test.rs"]
mod cross_file_trait_inference_regeneration_test;

#[path = "cross_module_copy_field_test.rs"]
mod cross_module_copy_field_test;

#[path = "cross_module_extern_unsafe_test.rs"]
mod cross_module_extern_unsafe_test;

#[path = "cross_module_nested_field_mut_test.rs"]
mod cross_module_nested_field_mut_test;

#[path = "cross_module_ordering_test.rs"]
mod cross_module_ordering_test;

#[path = "cross_module_self_field_mut_test.rs"]
mod cross_module_self_field_mut_test;

#[path = "cross_module_struct_literal_test.rs"]
mod cross_module_struct_literal_test;

#[path = "cross_type_method_mutation_test.rs"]
mod cross_type_method_mutation_test;

#[path = "debug_module_leak.rs"]
mod debug_module_leak;

#[path = "decorator_registry_test.rs"]
mod decorator_registry_test;

#[path = "decorator_syntax_test.rs"]
mod decorator_syntax_test;

#[path = "dep_copy_struct_override_test.rs"]
mod dep_copy_struct_override_test;

#[path = "deref_logic_layered_test.rs"]
mod deref_logic_layered_test;

#[path = "dereference_inference_test.rs"]
mod dereference_inference_test;

#[path = "derive_copy_tracking_test.rs"]
mod derive_copy_tracking_test;

#[path = "detect_rust_leakage_test.rs"]
mod detect_rust_leakage_test;

#[path = "doc_comment_after_derive_test.rs"]
mod doc_comment_after_derive_test;

#[path = "doc_comment_impl_tests.rs"]
mod doc_comment_impl_tests;

#[path = "doc_comments_test.rs"]
mod doc_comments_test;

#[path = "dogfooding_comprehensive_test.rs"]
mod dogfooding_comprehensive_test;

#[path = "dogfooding_fixes_match_collections_test.rs"]
mod dogfooding_fixes_match_collections_test;

#[path = "dogfooding_fixes_self_iterator_test.rs"]
mod dogfooding_fixes_self_iterator_test;

#[path = "dogfooding_fixes_strings_ui_test.rs"]
mod dogfooding_fixes_strings_ui_test;

#[path = "double_clone_bug_test.rs"]
mod double_clone_bug_test;

#[path = "duplicate_impl_fn_test.rs"]
mod duplicate_impl_fn_test;

#[path = "e0277_remaining_test.rs"]
mod e0277_remaining_test;

#[path = "e0308_builder_owned_self_receiver_test.rs"]
mod e0308_builder_owned_self_receiver_test;

#[path = "e0308_copy_scalar_ref_test.rs"]
mod e0308_copy_scalar_ref_test;

#[path = "e0308_expected_float_type_test.rs"]
mod e0308_expected_float_type_test;

#[path = "e0308_float_unification_test.rs"]
mod e0308_float_unification_test;

#[path = "e0308_if_let_void_semicolon_test.rs"]
mod e0308_if_let_void_semicolon_test;

#[path = "e0308_index_deref_copy_param_test.rs"]
mod e0308_index_deref_copy_param_test;

#[path = "e0308_match_ref_enum_struct_literals_test.rs"]
mod e0308_match_ref_enum_struct_literals_test;

#[path = "e0308_systematic_test.rs"]
mod e0308_systematic_test;

#[path = "e0308_tuple_index_deref_test.rs"]
mod e0308_tuple_index_deref_test;

#[path = "e0308_verify_pattern_a_test.rs"]
mod e0308_verify_pattern_a_test;

#[path = "e0308_verify_pattern_b_test.rs"]
mod e0308_verify_pattern_b_test;

#[path = "e0382_non_copy_multi_use_test.rs"]
mod e0382_non_copy_multi_use_test;

#[path = "e0432_submodule_crate_import_test.rs"]
mod e0432_submodule_crate_import_test;

#[path = "e0507_final_elimination_test.rs"]
mod e0507_final_elimination_test;

#[path = "e0507_final_test.rs"]
mod e0507_final_test;

#[path = "e0507_option_if_let_mut_field_test.rs"]
mod e0507_option_if_let_mut_field_test;

#[path = "e0507_ownership_inference_test.rs"]
mod e0507_ownership_inference_test;

#[path = "e0507_self_borrow_tests.rs"]
mod e0507_self_borrow_tests;

#[path = "e0507_systematic_test.rs"]
mod e0507_systematic_test;

#[path = "e0596_field_mutation_test.rs"]
mod e0596_field_mutation_test;

#[path = "e0596_inventory_complete_test.rs"]
mod e0596_inventory_complete_test;

#[path = "e0599_generic_impl_clone_bound_test.rs"]
mod e0599_generic_impl_clone_bound_test;

#[path = "e0614_comprehensive_test.rs"]
mod e0614_comprehensive_test;

#[path = "e0614_entity_test.rs"]
mod e0614_entity_test;

#[path = "e0614_final_elimination_test.rs"]
mod e0614_final_elimination_test;

#[path = "e0614_regression_fix_test.rs"]
mod e0614_regression_fix_test;

#[path = "e2e_comprehensive_tests.rs"]
mod e2e_comprehensive_tests;

#[path = "early_return_test.rs"]
mod early_return_test;

#[path = "edge_case_errors_test.rs"]
mod edge_case_errors_test;

#[path = "eject_tests.rs"]
mod eject_tests;

#[path = "end_to_end_tests.rs"]
mod end_to_end_tests;

#[path = "enhanced_javascript_tests.rs"]
mod enhanced_javascript_tests;

#[path = "enum_equality_usage_test.rs"]
mod enum_equality_usage_test;

#[path = "enum_partial_eq_test.rs"]
mod enum_partial_eq_test;

#[path = "enum_struct_destructuring_test.rs"]
mod enum_struct_destructuring_test;

#[path = "enum_variant_method_call_test.rs"]
mod enum_variant_method_call_test;

#[path = "enum_variant_string_coercion_test.rs"]
mod enum_variant_string_coercion_test;

#[path = "error_messages_test.rs"]
mod error_messages_test;

#[path = "explicit_deref_comparison_test.rs"]
mod explicit_deref_comparison_test;

#[path = "explicit_deref_owned_param_test.rs"]
mod explicit_deref_owned_param_test;

#[path = "extended_mutation_detection_test.rs"]
mod extended_mutation_detection_test;

#[path = "extern_borrowed_string_test.rs"]
mod extern_borrowed_string_test;

#[path = "extern_fn_codegen_test.rs"]
mod extern_fn_codegen_test;

#[path = "extern_fn_declaration_test.rs"]
mod extern_fn_declaration_test;

#[path = "extern_fn_integration_test.rs"]
mod extern_fn_integration_test;

#[path = "extern_fn_string_literal_test.rs"]
mod extern_fn_string_literal_test;

#[path = "extern_fn_unsafe_test.rs"]
mod extern_fn_unsafe_test;

#[path = "extern_fn_unsafe_wrapping_test.rs"]
mod extern_fn_unsafe_wrapping_test;

#[path = "extern_submodule_qualifier_test.rs"]
mod extern_submodule_qualifier_test;

#[path = "extern_unsafe_wrapping_test.rs"]
mod extern_unsafe_wrapping_test;

#[path = "f32_f64_cast_explosion_test.rs"]
mod f32_f64_cast_explosion_test;

#[path = "f32_f64_codegen_e0308_test.rs"]
mod f32_f64_codegen_e0308_test;

#[path = "feature_tests.rs"]
mod feature_tests;

#[path = "ffi_auto_mut_pointer_test.rs"]
mod ffi_auto_mut_pointer_test;

#[path = "ffi_extern_comprehensive_tests.rs"]
mod ffi_extern_comprehensive_tests;

#[path = "ffi_module_discovery_test.rs"]
mod ffi_module_discovery_test;

#[path = "ffi_module_test.rs"]
mod ffi_module_test;

#[path = "ffi_string_literal_arg_test.rs"]
mod ffi_string_literal_arg_test;

#[path = "field_array_indexing_i32_test.rs"]
mod field_array_indexing_i32_test;

#[path = "field_chain_method_signature_test.rs"]
mod field_chain_method_signature_test;

#[path = "float_arithmetic_e0277_test.rs"]
mod float_arithmetic_e0277_test;

#[path = "float_comparison_final_test.rs"]
mod float_comparison_final_test;

#[path = "float_field_assignment_inference_test.rs"]
mod float_field_assignment_inference_test;

#[path = "float_impl_method_test.rs"]
mod float_impl_method_test;

#[path = "float_inference_array_vec_test.rs"]
mod float_inference_array_vec_test;

#[path = "float_inference_binary_op_lhs_test.rs"]
mod float_inference_binary_op_lhs_test;

#[path = "float_inference_chained_ops_test.rs"]
mod float_inference_chained_ops_test;

#[path = "float_inference_collision_test.rs"]
mod float_inference_collision_test;

#[path = "float_inference_comparison_test.rs"]
mod float_inference_comparison_test;

#[path = "float_inference_comprehensive_test.rs"]
mod float_inference_comprehensive_test;

#[path = "float_inference_dt_consistency_test.rs"]
mod float_inference_dt_consistency_test;

#[path = "float_inference_duplicate_struct_basename_test.rs"]
mod float_inference_duplicate_struct_basename_test;

#[path = "float_inference_ffi_e0308_test.rs"]
mod float_inference_ffi_e0308_test;

#[path = "float_inference_field_initializer_test.rs"]
mod float_inference_field_initializer_test;

#[path = "float_inference_function_args_test.rs"]
mod float_inference_function_args_test;

#[path = "float_inference_if_else_test.rs"]
mod float_inference_if_else_test;

#[path = "float_inference_local_var_test.rs"]
mod float_inference_local_var_test;

#[path = "float_inference_match_arms_test.rs"]
mod float_inference_match_arms_test;

#[path = "float_inference_parameter_tracking_test.rs"]
mod float_inference_parameter_tracking_test;

#[path = "float_inference_physics_test.rs"]
mod float_inference_physics_test;

#[path = "float_inference_remaining_patterns_test.rs"]
mod float_inference_remaining_patterns_test;

#[path = "float_inference_return_test.rs"]
mod float_inference_return_test;

#[path = "float_inference_struct_fields_test.rs"]
mod float_inference_struct_fields_test;

#[path = "float_inference_tuple_in_vec_push_test.rs"]
mod float_inference_tuple_in_vec_push_test;

#[path = "float_literal_inference_test.rs"]
mod float_literal_inference_test;

#[path = "float_match_arm_unification_test.rs"]
mod float_match_arm_unification_test;

#[path = "float_spurious_f64_cast_test.rs"]
mod float_spurious_f64_cast_test;

#[path = "float_struct_field_inference_test.rs"]
mod float_struct_field_inference_test;

#[path = "float_type_unification_test.rs"]
mod float_type_unification_test;

#[path = "for_loop_borrow_inference_test.rs"]
mod for_loop_borrow_inference_test;

#[path = "for_loop_borrow_propagation_test.rs"]
mod for_loop_borrow_propagation_test;

#[path = "for_loop_mut_in_match_test.rs"]
mod for_loop_mut_in_match_test;

#[path = "for_loop_mut_iteration_test.rs"]
mod for_loop_mut_iteration_test;

#[path = "for_loop_wildcard_test.rs"]
mod for_loop_wildcard_test;

#[path = "forbidden_as_str_test.rs"]
mod forbidden_as_str_test;

#[path = "function_args_3layer_test.rs"]
mod function_args_3layer_test;

#[path = "generic_owned_param_integration_test.rs"]
mod generic_owned_param_integration_test;

#[path = "generic_type_propagation_test.rs"]
mod generic_type_propagation_test;

#[path = "glob_import_struct_field_typing_test.rs"]
mod glob_import_struct_field_typing_test;

#[path = "go_backend_extended_test.rs"]
mod go_backend_extended_test;

#[path = "go_backend_test.rs"]
mod go_backend_test;

#[path = "go_enum_pattern_extraction_test.rs"]
mod go_enum_pattern_extraction_test;

#[path = "hashmap_auto_borrow_test.rs"]
mod hashmap_auto_borrow_test;

#[path = "hashmap_copy_key_auto_borrow_test.rs"]
mod hashmap_copy_key_auto_borrow_test;

#[path = "hashmap_get_deref_test.rs"]
mod hashmap_get_deref_test;

#[path = "hashmap_get_double_ref_test.rs"]
mod hashmap_get_double_ref_test;

#[path = "i32_subtraction_no_cast_test.rs"]
mod i32_subtraction_no_cast_test;

#[path = "if_else_expression_context_test.rs"]
mod if_else_expression_context_test;

#[path = "if_else_expression_semicolons_test.rs"]
mod if_else_expression_semicolons_test;

#[path = "if_else_ownership_consistency_test.rs"]
mod if_else_ownership_consistency_test;

#[path = "if_else_parameter_return_test.rs"]
mod if_else_parameter_return_test;

#[path = "if_else_ref_literal_test.rs"]
mod if_else_ref_literal_test;

#[path = "if_else_string_consistency_test.rs"]
mod if_else_string_consistency_test;

#[path = "if_let_codegen_test.rs"]
mod if_let_codegen_test;

#[path = "if_let_string_consistency_test.rs"]
mod if_let_string_consistency_test;

#[path = "ignore_decorator_test.rs"]
mod ignore_decorator_test;

#[path = "immutability_enforcement_test.rs"]
mod immutability_enforcement_test;

#[path = "impl_trait_param_test.rs"]
mod impl_trait_param_test;

#[path = "import_braced_dedup_test.rs"]
mod import_braced_dedup_test;

#[path = "import_generation_comprehensive_test.rs"]
mod import_generation_comprehensive_test;

#[path = "import_resolution_sibling_explicit_use_test.rs"]
mod import_resolution_sibling_explicit_use_test;

#[path = "index_suffix_usize_test.rs"]
mod index_suffix_usize_test;

#[path = "inline_mod_extern_fn_integration_test.rs"]
mod inline_mod_extern_fn_integration_test;

#[path = "int_float_arithmetic_complete_test.rs"]
mod int_float_arithmetic_complete_test;

#[path = "int_float_arithmetic_comprehensive_test.rs"]
mod int_float_arithmetic_comprehensive_test;

#[path = "int_float_arithmetic_test.rs"]
mod int_float_arithmetic_test;

#[path = "int_float_copy_semantics_test.rs"]
mod int_float_copy_semantics_test;

#[path = "int_float_nested_comprehensive_test.rs"]
mod int_float_nested_comprehensive_test;

#[path = "int_float_ownership_integration_test.rs"]
mod int_float_ownership_integration_test;

#[path = "int_float_real_patterns_test.rs"]
mod int_float_real_patterns_test;

#[path = "int_inference_assignment_and_len_test.rs"]
mod int_inference_assignment_and_len_test;

#[path = "int_inference_binop_propagation_test.rs"]
mod int_inference_binop_propagation_test;

#[path = "int_inference_cast_collision_test.rs"]
mod int_inference_cast_collision_test;

#[path = "int_inference_compound_assignment_test.rs"]
mod int_inference_compound_assignment_test;

#[path = "int_inference_generic_collections_test.rs"]
mod int_inference_generic_collections_test;

#[path = "int_inference_vec_element_test.rs"]
mod int_inference_vec_element_test;

#[path = "int_inference_vec_remove_usize_test.rs"]
mod int_inference_vec_remove_usize_test;

#[path = "int_to_usize_cast_test.rs"]
mod int_to_usize_cast_test;

#[path = "int_usize_bidirectional_test.rs"]
mod int_usize_bidirectional_test;

#[path = "int_usize_comparison_test.rs"]
mod int_usize_comparison_test;

#[path = "integration_backend_conformance_test.rs"]
mod integration_backend_conformance_test;

#[path = "integration_ffi_build_test.rs"]
mod integration_ffi_build_test;

#[path = "integration_test_helpers_self_test.rs"]
mod integration_test_helpers_self_test;

#[path = "interpreter_advanced_test.rs"]
mod interpreter_advanced_test;

#[path = "interpreter_bugs_test.rs"]
mod interpreter_bugs_test;

#[path = "interpreter_test.rs"]
mod interpreter_test;

#[path = "iter_clone_push_test.rs"]
mod iter_clone_push_test;

#[path = "iter_field_copy_no_clone_test.rs"]
mod iter_field_copy_no_clone_test;

#[path = "iter_inference_test.rs"]
mod iter_inference_test;

#[path = "iter_push_clone_test.rs"]
mod iter_push_clone_test;

#[path = "iter_ref_vs_owned_comparison_test.rs"]
mod iter_ref_vs_owned_comparison_test;

#[path = "iter_var_method_call_test.rs"]
mod iter_var_method_call_test;

#[path = "js_backend_test.rs"]
mod js_backend_test;

#[path = "language_consistency_test.rs"]
mod language_consistency_test;

#[path = "len_arithmetic_inference_test.rs"]
mod len_arithmetic_inference_test;

#[path = "len_comparison_literal_test.rs"]
mod len_comparison_literal_test;

#[path = "len_zero_comparison_test.rs"]
mod len_zero_comparison_test;

#[path = "let_immutability_test.rs"]
mod let_immutability_test;

#[path = "lexer_tests.rs"]
mod lexer_tests;

#[path = "lib_rs_regen_test.rs"]
mod lib_rs_regen_test;

#[path = "lib_rs_subdirectory_prevention_test.rs"]
mod lib_rs_subdirectory_prevention_test;

#[path = "lib_vs_mod_generation_test.rs"]
mod lib_vs_mod_generation_test;

#[path = "lifetime_inference_test.rs"]
mod lifetime_inference_test;

#[path = "linter_test.rs"]
mod linter_test;

#[path = "linter_tests.rs"]
mod linter_tests;

#[path = "literal_usize_cast_test.rs"]
mod literal_usize_cast_test;

#[path = "local_var_auto_mut_test.rs"]
mod local_var_auto_mut_test;

#[path = "local_variable_shadowing_test.rs"]
mod local_variable_shadowing_test;

#[path = "loop_variable_ownership_test.rs"]
mod loop_variable_ownership_test;

#[path = "map_type_test.rs"]
mod map_type_test;

#[path = "match_arm_binding_method_call_test.rs"]
mod match_arm_binding_method_call_test;

#[path = "match_arm_comparison_test.rs"]
mod match_arm_comparison_test;

#[path = "match_arm_format_test.rs"]
mod match_arm_format_test;

#[path = "match_binding_iter_comparison_test.rs"]
mod match_binding_iter_comparison_test;

#[path = "match_bound_deref_test.rs"]
mod match_bound_deref_test;

#[path = "match_ergonomics_mut_self_test.rs"]
mod match_ergonomics_mut_self_test;

#[path = "match_ergonomics_mut_test.rs"]
mod match_ergonomics_mut_test;

#[path = "match_string_return_test.rs"]
mod match_string_return_test;

#[path = "matches_macro_optimization_test.rs"]
mod matches_macro_optimization_test;

#[path = "memory_safety_tests.rs"]
mod memory_safety_tests;

#[path = "method_arg_auto_ref_bug_test.rs"]
mod method_arg_auto_ref_bug_test;

#[path = "method_arg_conversion_test.rs"]
mod method_arg_conversion_test;

#[path = "method_body_float_field_assignment_test.rs"]
mod method_body_float_field_assignment_test;

#[path = "method_call_clone_test.rs"]
mod method_call_clone_test;

#[path = "method_call_f32_result_test.rs"]
mod method_call_f32_result_test;

#[path = "method_call_reference_args_test.rs"]
mod method_call_reference_args_test;

#[path = "method_call_string_args_test.rs"]
mod method_call_string_args_test;

#[path = "method_calls_3layer_test.rs"]
mod method_calls_3layer_test;

#[path = "method_chain_string_test.rs"]
mod method_chain_string_test;

#[path = "method_collision_mutation_test.rs"]
mod method_collision_mutation_test;

#[path = "method_mutability_inference_test.rs"]
mod method_mutability_inference_test;

#[path = "method_receiver_mut_inference_test.rs"]
mod method_receiver_mut_inference_test;

#[path = "method_receiver_ownership_test.rs"]
mod method_receiver_ownership_test;

#[path = "method_signature_lookup_integration_test.rs"]
mod method_signature_lookup_integration_test;

#[path = "method_string_arg_test.rs"]
mod method_string_arg_test;

#[path = "mixed_int_float_arithmetic_test.rs"]
mod mixed_int_float_arithmetic_test;

#[path = "mixed_numeric_arithmetic_test.rs"]
mod mixed_numeric_arithmetic_test;

#[path = "mod_rs_auto_reexport_test.rs"]
mod mod_rs_auto_reexport_test;

#[path = "mod_rs_self_path_test.rs"]
mod mod_rs_self_path_test;

#[path = "mod_wj_codegen_test.rs"]
mod mod_wj_codegen_test;

#[path = "module_declaration_bug_test.rs"]
mod module_declaration_bug_test;

#[path = "module_declaration_integration_test.rs"]
mod module_declaration_integration_test;

#[path = "module_declarations_test.rs"]
mod module_declarations_test;

#[path = "module_discovery_scope_test.rs"]
mod module_discovery_scope_test;

#[path = "module_existence_test.rs"]
mod module_existence_test;

#[path = "module_feature_gates_test.rs"]
mod module_feature_gates_test;

#[path = "module_imports_test.rs"]
mod module_imports_test;

#[path = "module_qualified_signature_test.rs"]
mod module_qualified_signature_test;

#[path = "module_reexport_generation_test.rs"]
mod module_reexport_generation_test;

#[path = "module_resolution_rust_keywords_test.rs"]
mod module_resolution_rust_keywords_test;

#[path = "module_system_comprehensive_tests.rs"]
mod module_system_comprehensive_tests;

#[path = "module_system_e0432_test.rs"]
mod module_system_e0432_test;

#[path = "module_system_test.rs"]
mod module_system_test;

#[path = "module_tree_copying_test.rs"]
mod module_tree_copying_test;

#[path = "move_closure_tests.rs"]
mod move_closure_tests;

#[path = "multi_file_no_inline_modules_test.rs"]
mod multi_file_no_inline_modules_test;

#[path = "multi_impl_trait_ownership_test.rs"]
mod multi_impl_trait_ownership_test;

#[path = "multi_target_tests.rs"]
mod multi_target_tests;

#[path = "multiline_doc_comment_test.rs"]
mod multiline_doc_comment_test;

#[path = "mut_error_messages_test.rs"]
mod mut_error_messages_test;

#[path = "mut_self_via_field_method_test.rs"]
mod mut_self_via_field_method_test;

#[path = "mutability_complete_test.rs"]
mod mutability_complete_test;

#[path = "mutated_returned_ownership_test.rs"]
mod mutated_returned_ownership_test;

#[path = "nested_field_access_ownership_test.rs"]
mod nested_field_access_ownership_test;

#[path = "nested_field_borrow_iteration_test.rs"]
mod nested_field_borrow_iteration_test;

#[path = "nested_field_no_intermediate_clone_test.rs"]
mod nested_field_no_intermediate_clone_test;

#[path = "nested_generics_test.rs"]
mod nested_generics_test;

#[path = "nested_index_no_clone_test.rs"]
mod nested_index_no_clone_test;

#[path = "nested_module_import_test.rs"]
mod nested_module_import_test;

#[path = "nested_module_output_path_test.rs"]
mod nested_module_output_path_test;

#[path = "no_copy_with_drop_test.rs"]
mod no_copy_with_drop_test;

#[path = "no_inline_modules_in_files_test.rs"]
mod no_inline_modules_in_files_test;

#[path = "orphan_mod_cleanup_test.rs"]
mod orphan_mod_cleanup_test;

#[path = "out_of_scope_module_prevention_test.rs"]
mod out_of_scope_module_prevention_test;

#[path = "owned_param_insert_clone_test.rs"]
mod owned_param_insert_clone_test;

#[path = "owned_self_field_move_test.rs"]
mod owned_self_field_move_test;

#[path = "ownership_binary_op_test.rs"]
mod ownership_binary_op_test;

#[path = "ownership_copy_fixed_point_e0382_test.rs"]
mod ownership_copy_fixed_point_e0382_test;

#[path = "ownership_copy_type_method_call_test.rs"]
mod ownership_copy_type_method_call_test;

#[path = "ownership_deref_test.rs"]
mod ownership_deref_test;

#[path = "ownership_edge_cases_final_test.rs"]
mod ownership_edge_cases_final_test;

#[path = "ownership_edge_cases_test.rs"]
mod ownership_edge_cases_test;

#[path = "ownership_external_crate_copy_test.rs"]
mod ownership_external_crate_copy_test;

#[path = "ownership_field_test.rs"]
mod ownership_field_test;

#[path = "ownership_final_cleanup_test.rs"]
mod ownership_final_cleanup_test;

#[path = "ownership_inference_method_params_test.rs"]
mod ownership_inference_method_params_test;

#[path = "ownership_inference_return_type_test.rs"]
mod ownership_inference_return_type_test;

#[path = "ownership_inference_self_by_value_test.rs"]
mod ownership_inference_self_by_value_test;

#[path = "ownership_integration_test.rs"]
mod ownership_integration_test;

#[path = "ownership_method_params_test.rs"]
mod ownership_method_params_test;

#[path = "ownership_method_test.rs"]
mod ownership_method_test;

#[path = "ownership_missing_math_methods_test.rs"]
mod ownership_missing_math_methods_test;

#[path = "ownership_mutability_test.rs"]
mod ownership_mutability_test;

#[path = "ownership_option_pattern_test.rs"]
mod ownership_option_pattern_test;

#[path = "ownership_param_type_matches_return_test.rs"]
mod ownership_param_type_matches_return_test;

#[path = "ownership_self_field_mutation_test.rs"]
mod ownership_self_field_mutation_test;

#[path = "ownership_tracker_bindings_api_test.rs"]
mod ownership_tracker_bindings_api_test;

#[path = "ownership_tracker_literals_ops_test.rs"]
mod ownership_tracker_literals_ops_test;

#[path = "ownership_tracker_methods_unary_test.rs"]
mod ownership_tracker_methods_unary_test;

#[path = "ownership_tracker_paths_test.rs"]
mod ownership_tracker_paths_test;

#[path = "ownership_tracker_population_test.rs"]
mod ownership_tracker_population_test;

#[path = "ownership_transitive_mut_test.rs"]
mod ownership_transitive_mut_test;

#[path = "param_ownership_multiple_use_test.rs"]
mod param_ownership_multiple_use_test;

#[path = "param_set_method_mut_test.rs"]
mod param_set_method_mut_test;

#[path = "parent_directory_module_test.rs"]
mod parent_directory_module_test;

#[path = "parser_decorator_kwargs_test.rs"]
mod parser_decorator_kwargs_test;

#[path = "parser_decorator_with_expressions_test.rs"]
mod parser_decorator_with_expressions_test;

#[path = "parser_error_line_numbers_test.rs"]
mod parser_error_line_numbers_test;

#[path = "parser_expression_tests.rs"]
mod parser_expression_tests;

#[path = "parser_item_tests.rs"]
mod parser_item_tests;

#[path = "parser_match_arm_assignment_test.rs"]
mod parser_match_arm_assignment_test;

#[path = "parser_match_block_comma_test.rs"]
mod parser_match_block_comma_test;

#[path = "parser_self_import_test.rs"]
mod parser_self_import_test;

#[path = "parser_simple_imports_test.rs"]
mod parser_simple_imports_test;

#[path = "parser_statement_tests.rs"]
mod parser_statement_tests;

#[path = "parser_test_cases_decorator_test.rs"]
mod parser_test_cases_decorator_test;

#[path = "parser_type_tests.rs"]
mod parser_type_tests;

#[path = "partial_eq_registry_nested_test.rs"]
mod partial_eq_registry_nested_test;

#[path = "passthrough_owned_param_test.rs"]
mod passthrough_owned_param_test;

#[path = "passthrough_qualified_call_test.rs"]
mod passthrough_qualified_call_test;

#[path = "pattern_binding_deref_codegen_test.rs"]
mod pattern_binding_deref_codegen_test;

#[path = "pattern_matching_tests.rs"]
mod pattern_matching_tests;

#[path = "platform_specific_modules_test.rs"]
mod platform_specific_modules_test;

#[path = "playtest_harness_migration_bugs_test.rs"]
mod playtest_harness_migration_bugs_test;

#[path = "primitive_method_calls_test.rs"]
mod primitive_method_calls_test;

#[path = "println_no_macro_test.rs"]
mod println_no_macro_test;

#[path = "project_paths_source_root_test.rs"]
mod project_paths_source_root_test;

#[path = "pub_use_codegen_test.rs"]
mod pub_use_codegen_test;

#[path = "push_str_auto_borrow_test.rs"]
mod push_str_auto_borrow_test;

#[path = "range_iteration_fix_test.rs"]
mod range_iteration_fix_test;

#[path = "range_type_mismatch_test.rs"]
mod range_type_mismatch_test;

#[path = "readonly_param_borrow_inference_test.rs"]
mod readonly_param_borrow_inference_test;

#[path = "recursive_output_prevention_test.rs"]
mod recursive_output_prevention_test;

#[path = "recursive_readonly_self_test.rs"]
mod recursive_readonly_self_test;

#[path = "ref_mut_logic_layered_test.rs"]
mod ref_mut_logic_layered_test;

#[path = "ref_self_mut_upgrade_test.rs"]
mod ref_self_mut_upgrade_test;

#[path = "ref_strip_owned_param_test.rs"]
mod ref_strip_owned_param_test;

#[path = "reference_coercion_test.rs"]
mod reference_coercion_test;

#[path = "reference_handling_test.rs"]
mod reference_handling_test;

#[path = "render_mut_self_inference_test.rs"]
mod render_mut_self_inference_test;

#[path = "repr_c_struct_layout_test.rs"]
mod repr_c_struct_layout_test;

#[path = "return_optimization_if_without_else_test.rs"]
mod return_optimization_if_without_else_test;

#[path = "return_statement_optimization_test.rs"]
mod return_statement_optimization_test;

#[path = "return_statement_test.rs"]
mod return_statement_test;

#[path = "return_string_literal_test.rs"]
mod return_string_literal_test;

#[path = "return_usize_to_int_cast_test.rs"]
mod return_usize_to_int_cast_test;

#[path = "runtime_ffi_import_bug_test.rs"]
mod runtime_ffi_import_bug_test;

#[path = "rust_coercion_rules_test.rs"]
mod rust_coercion_rules_test;

#[path = "rust_leakage_clone_warning_test.rs"]
mod rust_leakage_clone_warning_test;

#[path = "rust_str_string_semantics_verification.rs"]
mod rust_str_string_semantics_verification;

#[path = "same_name_module_bug_test.rs"]
mod same_name_module_bug_test;

#[path = "same_name_regeneration_bug_test.rs"]
mod same_name_regeneration_bug_test;

#[path = "scientific_notation_test.rs"]
mod scientific_notation_test;

#[path = "self_borrow_temp_extraction_test.rs"]
mod self_borrow_temp_extraction_test;

#[path = "self_consuming_ownership_test.rs"]
mod self_consuming_ownership_test;

#[path = "self_copy_field_return_test.rs"]
mod self_copy_field_return_test;

#[path = "self_field_binary_test.rs"]
mod self_field_binary_test;

#[path = "self_field_method_mutation_test.rs"]
mod self_field_method_mutation_test;

#[path = "self_field_mutation_recursion_test.rs"]
mod self_field_mutation_recursion_test;

#[path = "self_no_access_test.rs"]
mod self_no_access_test;

#[path = "self_parameter_inference_test.rs"]
mod self_parameter_inference_test;

#[path = "self_readonly_method_inference_test.rs"]
mod self_readonly_method_inference_test;

#[path = "shader_exclusion_test.rs"]
mod shader_exclusion_test;

#[path = "shader_file_detection_test.rs"]
mod shader_file_detection_test;

#[path = "shader_wjsl_test.rs"]
mod shader_wjsl_test;

#[path = "sibling_module_imports_test.rs"]
mod sibling_module_imports_test;

#[path = "single_file_binary_test.rs"]
mod single_file_binary_test;

#[path = "source_map_portability_test.rs"]
mod source_map_portability_test;

#[path = "source_map_portable_test.rs"]
mod source_map_portable_test;

#[path = "statement_expression_fix_test.rs"]
mod statement_expression_fix_test;

#[path = "static_helper_borrow_inference_test.rs"]
mod static_helper_borrow_inference_test;

#[path = "static_method_inference_integration_test.rs"]
mod static_method_inference_integration_test;

#[path = "static_method_not_enum_variant_test.rs"]
mod static_method_not_enum_variant_test;

#[path = "static_method_passthrough_ownership_test.rs"]
mod static_method_passthrough_ownership_test;

#[path = "std_fs_codegen_test.rs"]
mod std_fs_codegen_test;

#[path = "std_ops_import_integration_test.rs"]
mod std_ops_import_integration_test;

#[path = "str_ref_callsite_method_arg_test.rs"]
mod str_ref_callsite_method_arg_test;

#[path = "str_stored_in_owned_container_test.rs"]
mod str_stored_in_owned_container_test;

#[path = "str_string_codegen_test.rs"]
mod str_string_codegen_test;

#[path = "str_string_hashmap_test.rs"]
mod str_string_hashmap_test;

#[path = "str_string_integration_test.rs"]
mod str_string_integration_test;

#[path = "stress_test_large_codebase.rs"]
mod stress_test_large_codebase;

#[path = "string_auto_borrow_test.rs"]
mod string_auto_borrow_test;

#[path = "string_borrow_inference_integration_test.rs"]
mod string_borrow_inference_integration_test;

#[path = "string_comparison_borrowed_param_test.rs"]
mod string_comparison_borrowed_param_test;

#[path = "string_comparison_test.rs"]
mod string_comparison_test;

#[path = "string_handling_tests.rs"]
mod string_handling_tests;

#[path = "string_interpolation_test.rs"]
mod string_interpolation_test;

#[path = "string_literal_auto_owned_test.rs"]
mod string_literal_auto_owned_test;

#[path = "string_literal_borrowed_param_test.rs"]
mod string_literal_borrowed_param_test;

#[path = "string_literal_enum_test.rs"]
mod string_literal_enum_test;

#[path = "string_literal_if_else_test.rs"]
mod string_literal_if_else_test;

#[path = "string_literal_inference_test.rs"]
mod string_literal_inference_test;

#[path = "string_literal_method_call_test.rs"]
mod string_literal_method_call_test;

#[path = "string_literal_method_chain_test.rs"]
mod string_literal_method_chain_test;

#[path = "string_literal_method_coercion_test.rs"]
mod string_literal_method_coercion_test;

#[path = "string_literal_no_conversion_test.rs"]
mod string_literal_no_conversion_test;

#[path = "string_literal_static_method_call_borrow_test.rs"]
mod string_literal_static_method_call_borrow_test;

#[path = "string_literal_struct_constructor_test.rs"]
mod string_literal_struct_constructor_test;

#[path = "string_match_no_as_str_test.rs"]
mod string_match_no_as_str_test;

#[path = "string_method_call_test.rs"]
mod string_method_call_test;

#[path = "string_optimization_phase2_test.rs"]
mod string_optimization_phase2_test;

#[path = "string_optimization_phase3_test.rs"]
mod string_optimization_phase3_test;

#[path = "string_ownership_inference_test.rs"]
mod string_ownership_inference_test;

#[path = "string_ownership_test.rs"]
mod string_ownership_test;

#[path = "string_parameter_ownership_bug_test.rs"]
mod string_parameter_ownership_bug_test;

#[path = "string_parameter_ownership_test.rs"]
mod string_parameter_ownership_test;

#[path = "string_ref_param_test.rs"]
mod string_ref_param_test;

#[path = "string_type_unification_test.rs"]
mod string_type_unification_test;

#[path = "string_var_method_test.rs"]
mod string_var_method_test;

#[path = "struct_constructor_args_test.rs"]
mod struct_constructor_args_test;

#[path = "struct_field_auto_borrow_test.rs"]
mod struct_field_auto_borrow_test;

#[path = "struct_field_if_else_string_test.rs"]
mod struct_field_if_else_string_test;

#[path = "struct_field_literal_typing_test.rs"]
mod struct_field_literal_typing_test;

#[path = "struct_registry_duplicate_names_test.rs"]
mod struct_registry_duplicate_names_test;

#[path = "struct_some_string_test.rs"]
mod struct_some_string_test;

#[path = "struct_to_bytes_auto_derive_test.rs"]
mod struct_to_bytes_auto_derive_test;

#[path = "struct_visibility_test.rs"]
mod struct_visibility_test;

#[path = "sub_object_mut_method_test.rs"]
mod sub_object_mut_method_test;

#[path = "super_import_flattening_test.rs"]
mod super_import_flattening_test;

#[path = "svo_bfs_layout_test.rs"]
mod svo_bfs_layout_test;

#[path = "svo_shader_compat_test.rs"]
mod svo_shader_compat_test;

#[path = "test_cases_generation_test.rs"]
mod test_cases_generation_test;

#[path = "test_cli_exports.rs"]
mod test_cli_exports;

#[path = "test_cross_crate_float_inference.rs"]
mod test_cross_crate_float_inference;

#[path = "test_cross_file_float_inference.rs"]
mod test_cross_file_float_inference;

#[path = "test_cross_file_int_inference.rs"]
mod test_cross_file_int_inference;

#[path = "test_cross_file_transitive_mutability.rs"]
mod test_cross_file_transitive_mutability;

#[path = "test_float_backward_propagation.rs"]
mod test_float_backward_propagation;

#[path = "test_float_function_param_propagation.rs"]
mod test_float_function_param_propagation;

#[path = "test_float_inference_field_assignment.rs"]
mod test_float_inference_field_assignment;

#[path = "test_float_inference_function_args.rs"]
mod test_float_inference_function_args;

#[path = "test_float_inference_module_aggregation.rs"]
mod test_float_inference_module_aggregation;

#[path = "test_float_inference_multi_file.rs"]
mod test_float_inference_multi_file;

#[path = "test_float_type_propagation.rs"]
mod test_float_type_propagation;

#[path = "test_framework_ffi_dependencies_test.rs"]
mod test_framework_ffi_dependencies_test;

#[path = "test_framework_game_tests_test.rs"]
mod test_framework_game_tests_test;

#[path = "test_framework_lib_generation_test.rs"]
mod test_framework_lib_generation_test;

#[path = "test_framework_lib_name_test.rs"]
mod test_framework_lib_name_test;

#[path = "test_framework_library_deps_test.rs"]
mod test_framework_library_deps_test;

#[path = "test_framework_library_import_test.rs"]
mod test_framework_library_import_test;

#[path = "test_framework_module_conflict_test.rs"]
mod test_framework_module_conflict_test;

#[path = "test_library_preserve_structure.rs"]
mod test_library_preserve_structure;

#[path = "test_method_signature_inference.rs"]
mod test_method_signature_inference;

#[path = "test_module_path_generation.rs"]
mod test_module_path_generation;

#[path = "test_parser_warnings.rs"]
mod test_parser_warnings;

#[path = "test_transitive_mutability_inference.rs"]
mod test_transitive_mutability_inference;

#[path = "test_wjsl_array_indexing.rs"]
mod test_wjsl_array_indexing;

#[path = "test_wjsl_bitwise_ops.rs"]
mod test_wjsl_bitwise_ops;

#[path = "test_wjsl_body_preservation.rs"]
mod test_wjsl_body_preservation;

#[path = "test_wjsl_codegen.rs"]
mod test_wjsl_codegen;

#[path = "test_wjsl_error_messages.rs"]
mod test_wjsl_error_messages;

#[path = "test_wjsl_let_mut.rs"]
mod test_wjsl_let_mut;

#[path = "test_wjsl_parser.rs"]
mod test_wjsl_parser;

#[path = "test_wjsl_shr_body.rs"]
mod test_wjsl_shr_body;

#[path = "trait_auto_borrow_test.rs"]
mod trait_auto_borrow_test;

#[path = "trait_auto_derive_merge_test.rs"]
mod trait_auto_derive_merge_test;

#[path = "trait_bound_inference_test.rs"]
mod trait_bound_inference_test;

#[path = "trait_copy_param_integration_test.rs"]
mod trait_copy_param_integration_test;

#[path = "trait_cross_file_mut_preserved_test.rs"]
mod trait_cross_file_mut_preserved_test;

#[path = "trait_derivation_recursive_test.rs"]
mod trait_derivation_recursive_test;

#[path = "trait_double_ref_bug_test.rs"]
mod trait_double_ref_bug_test;

#[path = "trait_explicit_mut_preserved_test.rs"]
mod trait_explicit_mut_preserved_test;

#[path = "trait_impl_cross_file_self_ownership_test.rs"]
mod trait_impl_cross_file_self_ownership_test;

#[path = "trait_impl_inherent_method_name_collision_test.rs"]
mod trait_impl_inherent_method_name_collision_test;

#[path = "trait_impl_multi_file_test.rs"]
mod trait_impl_multi_file_test;

#[path = "trait_impl_operator_copy_type_test.rs"]
mod trait_impl_operator_copy_type_test;

#[path = "trait_impl_ownership_test.rs"]
mod trait_impl_ownership_test;

#[path = "trait_impl_self_inference_test.rs"]
mod trait_impl_self_inference_test;

#[path = "trait_impl_self_mutation_test.rs"]
mod trait_impl_self_mutation_test;

#[path = "trait_impl_self_param_test.rs"]
mod trait_impl_self_param_test;

#[path = "trait_impl_signature_match_test.rs"]
mod trait_impl_signature_match_test;

#[path = "trait_impl_signature_matching_test.rs"]
mod trait_impl_signature_matching_test;

#[path = "trait_impl_syntax_test.rs"]
mod trait_impl_syntax_test;

#[path = "trait_method_default_impl_self_test.rs"]
mod trait_method_default_impl_self_test;

#[path = "trait_method_mutability_inference.rs"]
mod trait_method_mutability_inference;

#[path = "trait_method_no_body_test.rs"]
mod trait_method_no_body_test;

#[path = "trait_method_no_inference_test.rs"]
mod trait_method_no_inference_test;

#[path = "trait_method_ownership_inference_test.rs"]
mod trait_method_ownership_inference_test;

#[path = "trait_method_self_param_inference_test.rs"]
mod trait_method_self_param_inference_test;

#[path = "trait_method_signature_edge_cases_test.rs"]
mod trait_method_signature_edge_cases_test;

#[path = "trait_object_derive_test.rs"]
mod trait_object_derive_test;

#[path = "trait_owned_self_render_test.rs"]
mod trait_owned_self_render_test;

#[path = "trait_regen_module_leak_test.rs"]
mod trait_regen_module_leak_test;

#[path = "tryop_ownership_inference_test.rs"]
mod tryop_ownership_inference_test;

#[path = "tuple_field_access_test.rs"]
mod tuple_field_access_test;

#[path = "tuple_struct_test.rs"]
mod tuple_struct_test;

#[path = "type_alias_test.rs"]
mod type_alias_test;

#[path = "type_coercion_test.rs"]
mod type_coercion_test;

#[path = "type_comparison_inference_test.rs"]
mod type_comparison_inference_test;

#[path = "type_inference_ambiguity_test.rs"]
mod type_inference_ambiguity_test;

#[path = "type_inference_binary_op_propagation_test.rs"]
mod type_inference_binary_op_propagation_test;

#[path = "type_inference_comparison_with_fields_test.rs"]
mod type_inference_comparison_with_fields_test;

#[path = "type_inference_compound_expression_test.rs"]
mod type_inference_compound_expression_test;

#[path = "type_inference_const_fold_test.rs"]
mod type_inference_const_fold_test;

#[path = "type_inference_cross_module_test.rs"]
mod type_inference_cross_module_test;

#[path = "type_inference_division_literal_test.rs"]
mod type_inference_division_literal_test;

#[path = "type_inference_field_access_binary_op_test.rs"]
mod type_inference_field_access_binary_op_test;

#[path = "type_inference_field_access_test.rs"]
mod type_inference_field_access_test;

#[path = "type_inference_field_in_binary_op_test.rs"]
mod type_inference_field_in_binary_op_test;

#[path = "type_inference_float_astar_pattern_test.rs"]
mod type_inference_float_astar_pattern_test;

#[path = "type_inference_float_method_call_test.rs"]
mod type_inference_float_method_call_test;

#[path = "type_inference_float_test.rs"]
mod type_inference_float_test;

#[path = "type_inference_for_loop_test.rs"]
mod type_inference_for_loop_test;

#[path = "type_inference_function_call_return_test.rs"]
mod type_inference_function_call_return_test;

#[path = "type_inference_function_params_test.rs"]
mod type_inference_function_params_test;

#[path = "type_inference_hashmap_insert_test.rs"]
mod type_inference_hashmap_insert_test;

#[path = "type_inference_if_else_arms_test.rs"]
mod type_inference_if_else_arms_test;

#[path = "type_inference_mat4_inverse_test.rs"]
mod type_inference_mat4_inverse_test;

#[path = "type_inference_match_arm_astar_pattern_test.rs"]
mod type_inference_match_arm_astar_pattern_test;

#[path = "type_inference_match_arm_test.rs"]
mod type_inference_match_arm_test;

#[path = "type_inference_match_arm_unification_test.rs"]
mod type_inference_match_arm_unification_test;

#[path = "type_inference_match_in_let_test.rs"]
mod type_inference_match_in_let_test;

#[path = "type_inference_match_wildcard_test.rs"]
mod type_inference_match_wildcard_test;

#[path = "type_inference_math_constants_test.rs"]
mod type_inference_math_constants_test;

#[path = "type_inference_method_chain_test.rs"]
mod type_inference_method_chain_test;

#[path = "type_inference_method_return_binary_op_test.rs"]
mod type_inference_method_return_binary_op_test;

#[path = "type_inference_mixed_float_binary_ops_test.rs"]
mod type_inference_mixed_float_binary_ops_test;

#[path = "type_inference_return_to_var_test.rs"]
mod type_inference_return_to_var_test;

#[path = "type_inference_struct_field_test.rs"]
mod type_inference_struct_field_test;

#[path = "type_inference_struct_in_loop_test.rs"]
mod type_inference_struct_in_loop_test;

#[path = "type_inference_vec3_f32_test.rs"]
mod type_inference_vec3_f32_test;

#[path = "type_inference_vec_push_test.rs"]
mod type_inference_vec_push_test;

#[path = "type_registry_full_test.rs"]
mod type_registry_full_test;

#[path = "type_registry_integration_test.rs"]
mod type_registry_integration_test;

#[path = "type_registry_method_lookup_test.rs"]
mod type_registry_method_lookup_test;

#[path = "types_equal_generic_test.rs"]
mod types_equal_generic_test;

#[path = "u32_literal_inference_test.rs"]
mod u32_literal_inference_test;

#[path = "unit_struct_test.rs"]
mod unit_struct_test;

#[path = "unknown_method_mut_inference_test.rs"]
mod unknown_method_mut_inference_test;

#[path = "unused_field_test.rs"]
mod unused_field_test;

#[path = "use_crate_path_integration_test.rs"]
mod use_crate_path_integration_test;

#[path = "user_copy_type_no_clone_test.rs"]
mod user_copy_type_no_clone_test;

#[path = "usize_comparison_casting_test.rs"]
mod usize_comparison_casting_test;

#[path = "usize_comparison_test.rs"]
mod usize_comparison_test;

#[path = "usize_i32_cast_test.rs"]
mod usize_i32_cast_test;

#[path = "vec3_no_spurious_cast_test.rs"]
mod vec3_no_spurious_cast_test;

#[path = "vec_capacity_usize_test.rs"]
mod vec_capacity_usize_test;

#[path = "vec_copy_element_deref_test.rs"]
mod vec_copy_element_deref_test;

#[path = "vec_get_ref_test.rs"]
mod vec_get_ref_test;

#[path = "vec_index_clone_test.rs"]
mod vec_index_clone_test;

#[path = "vec_index_context_test.rs"]
mod vec_index_context_test;

#[path = "vec_index_struct_literal_auto_clone_test.rs"]
mod vec_index_struct_literal_auto_clone_test;

#[path = "vec_indexing_borrow_test.rs"]
mod vec_indexing_borrow_test;

#[path = "vec_indexing_ownership_test.rs"]
mod vec_indexing_ownership_test;

#[path = "vec_owned_vs_ref_deref_test.rs"]
mod vec_owned_vs_ref_deref_test;

#[path = "vec_push_copy_type_test.rs"]
mod vec_push_copy_type_test;

#[path = "vec_remove_integer_literal_test.rs"]
mod vec_remove_integer_literal_test;

#[path = "vec_remove_usize_no_ref_test.rs"]
mod vec_remove_usize_no_ref_test;

#[path = "vec_repeat_syntax_test.rs"]
mod vec_repeat_syntax_test;

#[path = "visual_script_json_test.rs"]
mod visual_script_json_test;

#[path = "void_return_semicolon_test.rs"]
mod void_return_semicolon_test;

#[path = "voxel_grid_test.rs"]
mod voxel_grid_test;

#[path = "voxel_meshing_test.rs"]
mod voxel_meshing_test;

#[path = "voxel_octree_test.rs"]
mod voxel_octree_test;

#[path = "wgsl_advanced_test.rs"]
mod wgsl_advanced_test;

#[path = "wgsl_basic_test.rs"]
mod wgsl_basic_test;

#[path = "wgsl_bindings_test.rs"]
mod wgsl_bindings_test;

#[path = "wgsl_decorator_test.rs"]
mod wgsl_decorator_test;

#[path = "wgsl_dogfood_test.rs"]
mod wgsl_dogfood_test;

#[path = "wgsl_entry_points_test.rs"]
mod wgsl_entry_points_test;

#[path = "wgsl_structs_test.rs"]
mod wgsl_structs_test;

#[path = "wgsl_type_safety_test.rs"]
mod wgsl_type_safety_test;

#[path = "wgsl_uniform_type_safety.rs"]
mod wgsl_uniform_type_safety;

#[path = "wgsl_vertex_fragment_test.rs"]
mod wgsl_vertex_fragment_test;

#[path = "windjammer_fixtures_integration.rs"]
mod windjammer_fixtures_integration;

#[path = "wjsl_assignment_parsing_test.rs"]
mod wjsl_assignment_parsing_test;

#[path = "wjsl_assignment_test.rs"]
mod wjsl_assignment_test;

#[path = "wjsl_atmosphere_shader_test.rs"]
mod wjsl_atmosphere_shader_test;

#[path = "wjsl_auto_binding_test.rs"]
mod wjsl_auto_binding_test;

#[path = "wjsl_bloom_shader_test.rs"]
mod wjsl_bloom_shader_test;

#[path = "wjsl_const_decl_test.rs"]
mod wjsl_const_decl_test;

#[path = "wjsl_control_flow_scoping.rs"]
mod wjsl_control_flow_scoping;

#[path = "wjsl_denoise_quality_test.rs"]
mod wjsl_denoise_quality_test;

#[path = "wjsl_field_assignment_test.rs"]
mod wjsl_field_assignment_test;

#[path = "wjsl_fire_shader_test.rs"]
mod wjsl_fire_shader_test;

#[path = "wjsl_game_shaders_test.rs"]
mod wjsl_game_shaders_test;

#[path = "wjsl_heat_distortion_test.rs"]
mod wjsl_heat_distortion_test;

#[path = "wjsl_hud_overlay_test.rs"]
mod wjsl_hud_overlay_test;

#[path = "wjsl_if_assignment_test.rs"]
mod wjsl_if_assignment_test;

#[path = "wjsl_if_expression_test.rs"]
mod wjsl_if_expression_test;

#[path = "wjsl_include_test.rs"]
mod wjsl_include_test;

#[path = "wjsl_less_than_comparison_test.rs"]
mod wjsl_less_than_comparison_test;

#[path = "wjsl_let_mut_loop_test.rs"]
mod wjsl_let_mut_loop_test;

#[path = "wjsl_let_mut_test.rs"]
mod wjsl_let_mut_test;

#[path = "wjsl_lighting_ao_test.rs"]
mod wjsl_lighting_ao_test;

#[path = "wjsl_lighting_quality_test.rs"]
mod wjsl_lighting_quality_test;

#[path = "wjsl_lighting_transpile_test.rs"]
mod wjsl_lighting_transpile_test;

#[path = "wjsl_matrix_constructor_test.rs"]
mod wjsl_matrix_constructor_test;

#[path = "wjsl_nested_generics_test.rs"]
mod wjsl_nested_generics_test;

#[path = "wjsl_particle_shader_test.rs"]
mod wjsl_particle_shader_test;

#[path = "wjsl_shader_regressions.rs"]
mod wjsl_shader_regressions;

#[path = "wjsl_svo64_traversal_test.rs"]
mod wjsl_svo64_traversal_test;

#[path = "wjsl_volumetric_smoke_test.rs"]
mod wjsl_volumetric_smoke_test;

#[path = "wjsl_water_shader_test.rs"]
mod wjsl_water_shader_test;

#[path = "wjsl_workgroup_size_2arg_test.rs"]
mod wjsl_workgroup_size_2arg_test;

#[path = "wjsl_workgroup_var_test.rs"]
mod wjsl_workgroup_var_test;


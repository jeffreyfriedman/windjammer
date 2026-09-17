#!/usr/bin/env bash
# Run tip compiler repro gates. See tests/COMPILER_REPRO_QUEUE.md § P3.262+.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

RED_FILTERS=(
  wdb253_tip_out
  wdb254_tip_out
  wdb239_tip_out
  timefmt_product_must_not_mix_i32_month_or_ref_string
  demoted_str_substring_assign_must_own
  usize_i_plus_one_assign_must_stay_usize
  module_file_for_in_struct_vec_field_must_not_partial_move_parent
  wdb241_tip_out
  wdb242_tip_out
  wdb243_tip_out
  module_file_usize_index_eq_zero_must_not_emit_i32
  wdb223_tip_out_pagerank_must_borrow_owned_f64_map
  module_file_spawn_move_in_worker_loop_must_be_preserved
  mut_owned_vec_u8_return_must_not_demote_to_ref
  int_find_pos_ge_zero_must_not_mix_usize_i64
  module_file_shared_map_get_must_borrow_key
  wdb225_tip_out_ldbc_must_not_pass_string_from
  wdb226_tip_out_cdlp_must_borrow_owned_u32_map
)

GREEN_FILTERS=(
  i32_nested_range_eq_mod_literals_must_not_emit_i64
  module_file_void_while_i32_seg_counter_must_not_emit_i64
  module_file_demoted_str_field_assign_must_to_string
  i32_cy_plus_dy_for_range_must_not_widen_to_i64
  generic_assign_must_not_inject_unbound_t
  i32_range_bounds_sub_add_must_not_split_i64_i32
  int_mod_literal_zero_compare_must_not_split_i64_i32
  generic_type_alias_must_emit_after_struct
  generic_channel_send_owned_param_must_not_demote_to_ref
  generic_channel_recv_must_move_receiver_not_clone
  u32_arith_int_literal_must_not_emit_u64
  module_file_vec_len_gt_zero_must_not_mix_uint_int
  module_file_recv_reassign_must_move_not_clone_receiver
  module_file_int_mul_into_int_acc_must_not_cast_i32
  module_file_nested_while_substring_i_plus_one_must_not_emit_i32
  substring_end_i_plus_one_must_not_emit_i32_into_usize
  module_file_vec_index_loop_must_not_add_i32_to_usize
  i32_range_end_add_must_not_split_i64_i32
  thread_spawn_move_keyword_must_be_preserved
  module_file_spawn_move_keyword_must_be_preserved
  int_increment_literal_must_match_lhs_width
  int_arith_must_not_split_i64_i32
  int_zero_minus_must_keep_lhs_width
  hashmap_string_key_insert_must_not_cast_usize
  vec_push_borrowed_loop_elem_must_clone
  i32_compound_add_must_not_use_usize_literal
  env_trait_forward_owned_string_must_not_borrow
  vec_len_eq_zero_must_not_emit_i64_literal
  trait_impl_owned_vec_forward_must_match_trait_formal
  int_while_usize_compare_compound_assign_must_stay_int
  trait_owned_string_call_must_not_over_borrow
  strings_len_must_unify_int_index_arith
  hashmap_owned_get_helper_must_not_inject_mid_match_defer_drop
  explicit_type_import_must_not_duplicate_prelude
  owned_string_locals_move_into_owned_string_formals
  user_join_two_strings_moves_owned_locals
  engine_i32_formal_into_owned_copy_set_must_not_star_deref_clone
  engine_i32_range_literal_must_not_emit_i64_suffix
  owned_path_extract_must_not_over_borrow
  vec_string_helper_must_not_over_borrow
  thin_vec_forwarder_must_not_demote_owned
  vec_custom_view_helper_must_not_over_borrow
  mut_param_passthrough_must_not_prefix_shared_amp
  cross_module_match_arm_readonly_concat_demotes_to_str
  wdb214_codegen_push_cstring_must_borrow_owned_string_field
  wdb215_codegen_u64_index_must_not_compare_len_as_i64
  wdb216_codegen_demoted_vec_into_owned_ffi_must_clone
  wdb217_codegen_mut_ref_formal_must_reborrow_not_clone
  while_idx_lt_vec_len_must_unify_int_uint
  seed_overlay_int_parse_format_must_cargo_check_without_plus_empty
  hexagonal_seed_overlay_int_parse_format_must_cargo_check_without_plus_empty
  seed_overlay_apply_bank_line_must_cargo_check_without_plus_empty
  hexagonal_seed_overlay_apply_bank_line_must_cargo_check_without_plus_empty
  owned_helper_into_demoted_str_formal_must_auto_borrow
  wdb166_module_file_owned_field_clone_into_str_formal_must_borrow
  seed_overlay_remember_must_cargo_check_without_plus_empty
  hexagonal_seed_overlay_remember_must_cargo_check_without_plus_empty
  i64_bitand_hex_mask_must_not_emit_u8_literal
  wdb157_module_file_string_formal_must_not_emit_impl_into_string_with_clone
  wdb158_module_file_value_compare_must_not_demote_to_mut_ref
  wdb159_module_file_owned_string_into_str_formal_must_borrow_not_clone
  owned_match_binding_cross_fn_owned_string_formal_must_move
  owned_match_binding_hexagonal_cross_module_must_move
  wdb155_module_file_option_match_ast_pipeline_must_keep_owned_formal
  seed_overlay_read_body_must_cargo_check_without_plus_empty
  hexagonal_seed_overlay_read_body_must_cargo_check_without_plus_empty
  request_context_uuid_substring_must_cargo_check_without_plus_empty
  hexagonal_request_context_uuid_substring_must_cargo_check_without_plus_empty
  haystack_contains_substring_int_indices_must_unify_usize
  hexagonal_haystack_contains_substring_int_indices_must_unify_usize
  ledgerkit_clean_account_from_row_must_emit_without_plus_empty
  hexagonal_ledgerkit_clean_account_from_row_must_emit_without_plus_empty
  db_row_get_string_must_not_emit_row_clone
  hexagonal_db_row_get_string_must_not_emit_row_clone
  db_row_col_chain_call_site_must_move_owned_row
  hexagonal_db_row_col_chain_call_site_must_move_owned_row
  vec_string_int_element_must_own_via_to_string
  hexagonal_query_vec_string_params_must_own_demoted_str
  db_row_get_string_match_arms_must_unify_owned_string
  hexagonal_postgres_row_col_chain_must_cargo_check
  ui_builder_string_formal_must_emit_impl_into_string
  row_col_chain_same_file_must_not_demote_mut_row_and_return_owned
  wdb116_module_file_mutually_recursive_struct_fields_must_box
  wdb152_module_file_string_lit_into_owned_string_formal_must_to_string
  wdb153_shift_applies_to_masked_byte_before_add
  std_mime_from_extension_json_matches_application_json_constant
  std_yaml_to_json_rejects_empty_input
  trait_owned_draft_forwarder_must_not_demote_mut
  hexagonal_trait_owned_draft_forwarder_must_not_demote_mut
  string_concat_nested_owned_must_not_over_borrow
  hexagonal_string_concat_nested_owned_must_not_over_borrow
)

echo "Running ${#GREEN_FILTERS[@]} GREEN regression gate(s)..."
for f in "${GREEN_FILTERS[@]}"; do
  echo "--- $f ---"
  cargo test --release --test all "$f" -- --test-threads=1 2>&1 || { echo "REGRESSION: $f"; exit 1; }
  echo "GREEN: $f"
done

if ((${#RED_FILTERS[@]} > 0)); then
  echo "Running ${#RED_FILTERS[@]} RED gate(s)..."
  for f in "${RED_FILTERS[@]}"; do
    echo "--- $f ---"
    if cargo test --release --test all "$f" -- --test-threads=1 2>&1; then
      echo "FIXED: $f — update COMPILER_REPRO_QUEUE.md"
      exit 1
    else
      echo "RED: $f"
    fi
  done
  echo "Done: ${#GREEN_FILTERS[@]} GREEN + ${#RED_FILTERS[@]} RED gates verified."
else
  echo "Done: ${#GREEN_FILTERS[@]} GREEN gates verified (no RED filters)."
fi

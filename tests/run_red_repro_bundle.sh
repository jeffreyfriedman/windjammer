#!/usr/bin/env bash
# Run tip compiler repro gates. See tests/COMPILER_REPRO_QUEUE.md § P3.262+.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

RED_FILTERS=(
)

GREEN_FILTERS=(
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

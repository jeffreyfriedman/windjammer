# Compiler repro queue (dogfooding — do not work around in application code)

Cross-crate / multipass dogfooding surfaced these codegen gaps. Each row has a
**codegen-shape** repro (emitted Rust assert) and/or a runtime fixture. Prefer the
codegen gates as source of truth; fixtures alone can pass while multipass still
mis-emits.

**Verified green on tip** (`cargo test --release --test all` filter below,
2026-08-26): method-index consensus (finance-screens hang), demoted `&str`
clone skip, multi-use owned auto-clone, WDB-108, assert msg var, and
`std::compress` gzip wiring.

| Priority | Bug | Repro test(s) | Status |
|----------|-----|---------------|--------|
| P0 | **`HashMap::contains_key/insert` — call-return / loop-local i64 in multipass** | `test_library_multipass_graph_bfs_hashmap_compiles`, `test_library_multipass_hashmap_i64_*` | ✅ |
| P0 | Loop reused binding — owned binding in loop must borrow for `&T` callee | `bug_loop_reused_binding_borrow_test`, `test_library_multipass_loop_reused_graph_borrow`, `regression_loop_reused_graph_borrow` | ✅ |
| P0 | **`for v in vertices { f(vertices, v) }` — must borrow `vertices`** | `test_library_multipass_for_in_vertices_reuse_borrow` | ✅ |
| P1 | **`HashMap<i64, f64>::insert(k, 0.0)` — literal must infer f64 not f32** | `test_library_multipass_hashmap_i64_f64_zero_literal_insert` | ✅ |
| P1 | String literal → `string` param must emit `&"lit".to_string()` not owned String | `regression_andstring_literal_call_test.wj` | ✅ |
| P1 | Empty string literals into demoted `&str` formals must not emit `.to_string()` at the call site (E0308) | `bug_wdb107_isolate_transpile_empty_literal_vs_str_formal_test` (same-file + tip isolate) | ✅ tip IR GREEN |
| P1 | Cross-crate `Type::new("lit")` with owned `String` formal — no WJ sig → bare `&str` | `codegen_cross_crate_associated_new_bare_literal_must_auto_own_gate_test` | ✅ |
| P1 | Cross-crate builder `.method("lit")` with owned `String` formal — no WJ sig → bare `&str` | `cross_crate_builder_bare_literal_must_auto_own` | ✅ |
| P1 | `strings::split` / `starts_with` Pattern must stay `&str` | `codegen_strings_pattern_must_stay_str_gate_test`, `codegen_starts_with_str_literal_must_not_auto_own` | ✅ tip GREEN under `--module-file` — P3.177 removed `haystack_starts_with`; dogfood uses `strings.starts_with(hay, "lit")` |
| P1 | `strings::starts_with(s, "#")` — literal prefix same as split | `regression_strings_starts_with_literal_test.wj` | ✅ |
| P2 | Cross-module `Vec` helper calls omit `&` borrows | `bug_cross_module_vec_borrow_test.rs` | ✅ |
| P1 | **`map = f(map, k, v)` writeback must not `map.clone()` (WDB-084)** | `test_library_multipass_map_writeback_must_not_clone` | ✅ |
| P1 | **`HashMap::get` borrow-break double `.copied()` on Copy V (WDB-086)** | `test_library_multipass_hashmap_get_borrow_break_single_copied` | ✅ |
| P1 | **Tuple writeback `let t = f(v); v = t.i` must not clone `v` (WDB-087)** | `test_library_multipass_tuple_writeback_must_not_clone` | ✅ |
| P1 | **`let tmp = self.a; self.a = self.b; self.b = tmp` → `mem::swap` (WDB-088)** | `test_self_field_vec_swap_must_use_mem_swap_not_clone` | ✅ |
| P1 | **`HashMap::with_capacity(usize)` multipass must not `as i64` (WDB-089)** | `test_library_multipass_hashmap_with_capacity_usize_no_i64_cast` | ✅ |
| P1 | `while true { break }` Rust parity (regression) | `test_while_true_with_break` | ✅ |
| P1 | `trim_end_matches("/")` must not emit `"/".to_string()` (Pattern) | `codegen_trim_end_matches_owned_string_pattern_gate_test` | ✅ |
| P1 | `find(":")` must not emit `":".to_string()` (Pattern) | `codegen_find_owned_string_pattern_gate_test` | ✅ |
| P1 | **`let Type { mut field } = value` — mut field in struct destructure (Rust parity)** | `test_struct_destructure_mut_field_compiles`, `test_struct_destructure_mut_field_hashmap_set_no_inner_clone` | ✅ |
| P1 | **`std::db::Row` getters must be `&self` (WJ0007 multi-column)** | `codegen_db_row_getter_must_borrow_self_gate_test` | ✅ std stub `get_*` → `&self` (runtime already); multi-column transpile smoke GREEN |
| P1 | **`(Row, T)` chain helpers for multi-column reads (no `&Row`, no move-WJ0007)** | `codegen_db_row_col_string_chain_gate_test` | ✅ tip GREEN (`col_string` / `col_int` dogfood in LedgerKit postgres_*); lockstep gate added |
| P1 | **`method: "GET"` → HttpMethod must auto-import under `--module-file`** | `codegen_http_method_struct_field_str_literal_gate_test`, `codegen_http_method_string_lit_must_auto_import_gate_test`, `codegen_http_method_nested_module_file_auto_import_gate_test`, `codegen_http_method_enum_gate_test` | ✅ tip GREEN — multipass stdlib discovery + FQ `windjammer_runtime::http::HttpMethod::GET` (no sibling import) |
| P1 | **WDB-101: borrowed map getter call site must auto-`&` owned local** | `wdb101_borrowed_vertex_map_getter_must_auto_borrow_at_call_site` | ✅ tip IR GREEN (qualified registry refresh + pure-forwarder keeps `&`) |
| P1 | **WDB-102: `strings.from_chars(chars)` must borrow owned `Vec<char>`** | `wdb102_from_chars_owned_vec_must_borrow_at_call_site` | ✅ tip GREEN (runtime WJ-owned/Rust-borrowed keeps `Vec` formal + call-site `&`) |
| P1 | **WDB-103: owned struct formal must not receive `&arg` (inverse WDB-099)** | `wdb103_owned_host_formal_must_move_not_borrow` | ✅ |
| P1 | **WDB-104: field-mutating method must emit `mut self`** | `wdb104_field_mutating_method_must_emit_mut_self` | ✅ |
| P1 | **WDB-105: explicit `.clone()` in while-loop trait calls must emit** | `wdb105_explicit_clone_in_while_loop_trait_call_must_emit` | ✅ tip GREEN (`loop_body_depth` preserves explicit clone in loops) |
| P1 | **WDB-106: explicit `.clone()` in sequential owned-string calls / is_empty must emit** | `wdb106_explicit_clone_on_first_of_two_owned_string_calls_must_emit`, `wdb106_explicit_clone_for_is_empty_before_move_must_emit` | ✅ tip GREEN |
| P1 | **WDB-108: explicit `.clone()` on sequential owned custom-struct / Vec reuse must emit** | `wdb108_explicit_clone_on_sequential_owned_custom_struct_calls_must_emit`, `wdb108_explicit_clone_on_vec_before_parser_and_cursor_reuse_must_emit` | ✅ tip GREEN — preserve explicit `.clone()` through IR reconcile (demoted `&Vec` included) |
| P1 | **WDB-099 / WDB-100 owned-formal call-site ownership** | `wdb099_owned_*`, `owned_string_reuse_after_two_arg_by_value_helper_should_clone` | ✅ tip IR GREEN (PRE snapshot retired) |
| P1 | **`std::random.range` → `random::int_range` (ecosystem `wj-uuid` v4)** | `bug_std_random_range_codegen_test` | ✅ tip GREEN (`resolve_runtime_emit_method_name` + MethodCall path) |
| P1 | **`std::crypto.sha1_bytes` for UUID v5** | `bug_std_crypto_sha1_bytes_test` | ✅ tip GREEN |
| P1 | **`std::time.utc_now()` for UUID v1** | `bug_std_time_utc_now_test` | ✅ tip GREEN |
| P1 | **`DateTime.timestamp_millis()` for UUID v1** | `bug_std_time_timestamp_millis_test` | ✅ tip GREEN |
| P1 | **Nested match `for` + `Vec<string>::push` (`wj-fs-walk`)** | `bug_for_loop_vec_string_push_test` | ✅ tip GREEN |
| P1 | **`json::Value` / `json::keys` / owned `get` (`wj-json-util`)** | `bug_json_value_keys_for_util_test`, `bug_json_get_owned_option_value_test` | ✅ tip GREEN |
| P1 | **User `join(string,string)` vs `strings.join` name clash (`wj-url`)** | `bug_user_join_name_clash_strings_join_test` | ✅ tip GREEN (owned user formal beats stdlib shared-ref homonym) |
| P1 | **`Ok((text, ""))` must own empty string** | `bug_ok_tuple_empty_string_literal_test` | ✅ tip GREEN (`generate_tuple` peels `Result`/`Option` element types) |
| P1 | **`encoding.base64_encode_string` / `decode_string` (`wj-base64`)** | `bug_std_encoding_base64_string_api_test` | ✅ tip GREEN |
| P1 | **`HashMap.get("lit")` after Result match (`wj-cookie`)** | `bug_hashmap_get_string_literal_to_string_test` | ✅ tip GREEN (collection-key finalize not re-owned) |
| P1 | **`for (k,v) in HashMap` post-loop `drop(map)` (`wj-cookie`)** | `bug_hashmap_for_in_post_loop_drop_test` | ✅ |
| P1 | **Read-only helper param must borrow, not own (`wj-validate`)** | `bug_readonly_helper_param_must_borrow_test` | ✅ tip GREEN |
| P1 | **Owned call-site temp must not become `&` (multi-arm routes)** | tip `codegen_library_multipass_owned_custom_call_site`, `codegen_owned_plus_empty_call_site_must_move` | ✅ tip GREEN — P3.179 dropped product `clone_tenant_slug`; routes pass bare `tenant_slug` (demoted `&str` or owned move) |
| P1 | **Struct field into owned `string` formal must not `&field`** | `codegen_struct_field_owned_string_formal_must_not_borrow` | ✅ tip GREEN — P3.180 dogfood drops `field + ""` into `escape_html` / status helpers |
| P1 | **`trim` local `== "lit"` must not emit `.as_str()` (E0658)** | `codegen_trim_eq_literal_must_not_emit_as_str` | ✅ tip GREEN — product restored bare `fmt == "csv"` |
| P1 | **Demoted `&str` formal must not receive `String.clone()` at call site** | `codegen_demoted_str_formal_must_not_receive_owned_clone_gate_test` | ✅ tip GREEN — if-condition clone guard skips demoted `&str`; IR strips stale `.clone()` before borrow |
| P1 | **Multi-use owned param → two owned `String` formals must auto-`.clone()`** | `codegen_multi_use_owned_param_must_auto_clone_gate_test` | ✅ tip GREEN — analysis-driven reuse clone survives stale shared-borrow registry + IR reconcile; P3.182 dogfood drops hub `title + ""` |
| P1 | **Full `finance-screens` tip codegen hang (type-inference recursion)** | `codegen_method_consensus_scales_with_matching_methods_not_registry_size_gate_test`, tip `wj build … --module-file` on 43-file screens crate | ✅ tip GREEN — method-index consensus (`signatures_for_method_name`); finance-screens tip build <2 min |
| P1 | **Cross-crate `Type::new(copy i64/f64)` must not emit `&arg`** | `codegen_cross_crate_associated_new_copy_arg_must_not_borrow_gate_test` | ✅ tip GREEN — associated `Type::method` fails closed (no bare `new` → `path::new` borrow); P3.183 tip screens regen drops hand-patches |
| P1 | **Multi-use struct field → owned formal then reuse must auto-`.clone()`** | `codegen_multi_use_struct_field_must_auto_clone_gate_test` | ✅ tip GREEN — field-path analysis clone wins over stale demoted/`&T` skip (same as param multi-use); IR reconcile + terminal restore |
| P1 | **Single-use struct field → demoted `&str` formal must borrow, not `.clone()`** | `codegen_struct_field_demoted_str_must_not_clone_gate_test` | ✅ tip GREEN — field clone gated on owned formals only; demoted `&str` callees borrow before auto-clone |
| P1 | **Cross-module struct field → demoted `&str` must borrow, not `+ ""` temp** | `codegen_cross_module_struct_field_demoted_str_must_borrow_gate_test` | ✅ tip GREEN — P3.186 dogfood `home.wj` emits `bank_line_is_unmatched(&line.status)` (multipass demotion registry) |
| P1 | **`env::get("lit")` / `env.get_or` must not auto-own into `&str` formals** | `codegen_env_get_str_literal_must_not_auto_own_gate_test` | ✅ tip GREEN — LedgerKit `lk_db.wj` centralizes `LK_DB`; env adapters use `lk_db_is_postgres()` |
| P1 | **String concat if/else must unify owned arms (alloc macros)** | `codegen_if_else_string_arms_must_unify_gate_test`, `codegen_string_concat_chain_gate_test`, dogfood `domain/actor.wj` / `string_concat.wj` | ✅ tip GREEN |
| P1 | **Cross-module match-arm call to multi-use `string` formal (owned move or demoted `&str`)** | `codegen_cross_module_match_arm_multi_use_owned_formal_gate_test` | ✅ tip GREEN — read-only `json + ""` demotes; identity-move + clone keeps `String`; no `&…clone()` |
| P1 | **Match arms yielding `string` must unify owned (`substring` vs binding)** | `codegen_match_string_arms_must_unify_gate_test` | ✅ tip IR GREEN |
| P1 | **`col_string(rows[0], …)` must not E0507 move from Vec index** | `codegen_vec_row_index_col_chain_gate_test` | ✅ tip GREEN (runtime non-Copy registry + terminal Index clone after IR reconcile) |
| P1 | **Local `len` binding must not shadow `strings::len` in substring end** | `codegen_substring_len_binding_shadow_gate_test` | ✅ tip GREEN |
| P1 | **Cross-module call to `&str` formal must auto-borrow owned temp** | `codegen_decode_cross_module_str_call_site_gate_test` | ✅ tip GREEN (signature-matched demotion or owned formal) |
| P1 | **Bare `!false` / `!call` as impl return under `--module-file`** | `codegen_unary_not_call_expr_return_must_parse_gate_test` | ✅ tip GREEN — `seed_auditor_access.wj` bare `!auditor_principal_requires_grant(...)` |
| P1 | **`strings.substring` int indices → usize (`wj-validate`)** | `bug_substring_int_indices_usize_test` | ✅ tip GREEN (runtime fallback `usize` formal drives cast) |
| P1 | **Loop reuses read-only `string` param (`wj-glob` filter)** | `bug_loop_reuse_readonly_string_param_test` | ✅ tip GREEN (comparison-only helpers demote to `&str`) |
| P1 | **`std::mime` constants + fn wiring (`wj-mime`)** | `bug_std_mime_module_wiring_test` | ✅ tip GREEN (runtime consts + stdlib const scan) |
| P1 | **Module `const string` return codegen as `&str` (`wj-mime`)** | `bug_module_const_string_return_test` | ✅ tip GREEN (`module_string_consts` + owned return/match coercion) |
| P1 | **Recursive owned `Vec<string>` helper over-borrowed at call site (`wj-yaml`)** | `bug_recursive_owned_vec_call_site_test` | ✅ tip GREEN |
| P1 | **`Vec` index with Windjammer `int` loop var (`wj-yaml`)** | `bug_vec_int_index_loop_test` (see also `bug_substring_int_indices_usize_test`) | ✅ tip GREEN (loop-promoted `int` counters still cast at index sites) |
| P1 | **`vec.len() - int` loop bound usize/i64 (`wj-migrate`)** | `bug_vec_len_minus_int_loop_test` | ✅ tip GREEN |
| P1 | **`string` ordinal compare (`ch < "0"`) after substring (`wj-todo-cli`)** | `bug_string_char_ordinal_compare_test` | ✅ tip GREEN (owned text vs str literal → `.as_str()` in comparisons) |
| P1 | **`HashMap.insert` as if-body expr must discard `Option` (`wj-todo-cli`)** | `bug_hashmap_insert_if_body_unit_test` | ✅ tip GREEN (void-block `let _ =` for non-unit expr stmts) |
| P1 | **Module `const string` returns `&str` not `String` (`wj-mime`)** | `bug_module_const_string_returns_str_test` | ✅ tip GREEN (`module_string_consts` registry + `.to_string()` at owned sites) |
| P1 | **Decimal `_` int literals (`60_000`, `1_000_000`)** | `bug_decimal_underscore_int_literal_test` | ✅ tip GREEN (lexer skips `_`; prior `60000` workaround in `wj-proxy` reverted) |
| P1 | **`Vec::push((key, ""))` must own empty string (`wj-querystring`)** | `bug_vec_push_tuple_empty_string_literal_test` | ✅ tip GREEN — `call_arg_expected_type` + specialized `Vec<(String,String)>::push` |
| P1 | **`std::encoding.url_encode` / `url_decode` wiring (`wj-querystring`)** | `bug_std_encoding_url_encode_wiring_test` | ✅ tip GREEN |
| P1 | **Reuse owned `string` after helper call in `${…}` / second call (`wj-multipart`)** | `bug_reused_string_after_owned_call_in_format_test` | ✅ tip GREEN |
| P1 | **Demoted `&str` formal must auto-borrow owned local (`wj-multipart`)** | `bug_demoted_str_formal_owned_local_auto_borrow_test` | ✅ tip GREEN |
| P1 | **`Vec<u8>::push(0)` must infer `u8` not `i64` (`wj-base64`)** | `bug_vec_u8_push_int_literal_infers_u8_test` | ✅ tip GREEN |
| P1 | **Module `const string` into owned formal (`wj-uuid`)** | `bug_module_const_string_owned_formal_call_site_test` | ✅ tip GREEN |
| P1 | **`std::csv.write` via user `fn write` must auto-borrow (`wj-csv`)** | `bug_std_csv_write_owned_rows_auto_borrow_test` | ✅ tip GREEN — qualified runtime-std skips bare-homonym lookup / false recursion strip |
| P1 | **`assert(false, err_var)` must not emit `assert!(false, e)` (`wj-timefmt`)** | `bug_test_assert_err_message_var_test` | ✅ tip GREEN — non-literal messages → `assert!(cond, "{}", msg)` |
| P1 | **`while end < strings.len(s)` int vs usize (`wj-compress`)** | `bug_while_int_lt_strings_len_unify_test` | ✅ tip GREEN — `strings::len` registry usize + skip `usize` mark on annotated `int` locals → cast |
| P1 | **Same-module `Vec<string>` helper reuse emits `.clone()` not `&` (`wj-cors`)** | `bug_same_module_vec_helper_reuse_clone_instead_of_borrow_test` | ✅ tip IR GREEN |
| P1 | **App forwarder → cross-crate owned `String` emits `&` / borrowed emits `.to_string()` (`wj-auth-api`)** | `bug_app_cross_crate_owned_forwarder_emits_borrow_test`, `bug_app_multipass_cross_crate_owned_forwarder_module_file_test`, `bug_cross_crate_owned_formal_multipass_call_site_test`, `bug_app_cross_crate_pretty_without_own_test` | ✅ tip GREEN — regression guards; `wj-fetch` may still keep `own()` defensively |
| P1 | **Private struct field must not emit spurious `use StructName;` (`wj-webhook`)** | `bug_private_struct_field_spurious_use_import_test` | ✅ tip GREEN — regression guard (`BusEventBody` in `Vec` field) |
| P1 | **`HashMap<i64, T>` field `.get(id)` must auto-borrow key (`wj-notes-api`)** | `bug_hashmap_field_get_i64_key_auto_borrow_test` | ✅ tip GREEN — fixture `note_store_hashmap_field_get.wj` |
| P1 | **WDB-112: full `src` `--module-file` demotes owned `string` to `&str` but call sites emit `String.clone()`** | `wdb112_full_library_multipass_demoted_str_formal_must_borrow_clone_call_sites` | ✅ tip GREEN |
| P1 | **WDB-113: full `src` `--module-file` demotes owned struct to `&mut T` but call sites emit `.clone()`** | `wdb113_full_library_multipass_mut_struct_formal_must_not_clone_owned_at_call_site` | ✅ tip GREEN (owned+clone OK; demoted+clone still RED) |
| P1 | **WDB-114: `--module-file` emits `Vec<T>` without importing `T`** | `wdb114_module_file_vec_type_annotation_must_import_element_type` | ✅ tip GREEN |
| P1 | **WDB-115: early `return self.private_method()` mis-emits sibling method** | `wdb115_early_return_private_method_must_emit_correct_callee` | ✅ tip GREEN (regression guard) |
| P1 | **`std::fs::DirEntry.name()` must wire to runtime `file_name()`** | `bug_std_fs_dir_entry_name_wiring_test`, `bug_std_fs_dir_entry_name_multipass_test` | ✅ tip GREEN — fixture `migrate_dir_entry_name.wj` |
| P1 | **Cross-crate `Vec<string>` helper call must auto-borrow** | `bug_cross_crate_vec_helper_must_auto_borrow_test` | ✅ tip GREEN — fixtures `cli_args_*` / `migrate_cli_use_positional.wj` |
| P1 | **`std::http::HttpMethod` lib public port vs `tests/*_test.wj`** | `bug_app_test_http_method_type_mismatch_test`, `bug_app_test_http_method_module_file_test` | ✅ tip GREEN (in-tree `wj`); regression guards |
| P1 | **Multipass HTTP adapter must pass owned `HttpReply` / `string` to response helpers** | `bug_multipass_http_adapter_owned_reply_test`, `bug_multipass_http_adapter_hexagonal_test` | ✅ tip GREEN |
| P1 | **Multipass `for ch in strings.chars` must not emit `&mut char` loop binding** | `bug_multipass_strings_chars_for_in_mut_char_test`, `bug_multipass_config_parse_positive_int_chars_test` | ✅ tip GREEN — fixtures `parse_positive_int_chars.wj`, `config_parse_positive_int_chars.wj` |
| P1 | **`use std::async_runtime::sleep_ms_blocking` must not alias module as `async`** | `bug_multipass_std_async_runtime_import_test` | ✅ tip GREEN — `pause_ms_async_runtime.wj` |
| P1 | **Cross-crate `join_url(base, path)` owned base must cargo-check (`wj-sitegen`)** | `bug_multipass_cross_crate_join_url_owned_base_test` | ✅ tip GREEN |
| P1 | **Cross-crate `render(owned_template(), vars)` must cargo-check (`wj-template`)** | `bug_multipass_cross_crate_render_owned_template_helper_test` | ✅ tip GREEN |
| P1 | **`self.get(id)` must not codegen as `self.delete(id)` when both exist** | `bug_self_get_call_emits_delete_method_test` | ✅ tip GREEN — regression guard (`note_store_self_get_call.wj`) |
| P1 | **WDB-151: owned store `T→T` formal must not demote to `&T`** | `bug_wdb151_module_file_owned_store_formal_must_not_demote_to_ref_test` | ✅ tip GREEN |
| P1 | **WDB-155: Option/match AST pipeline must keep owned formal (not `&mut`)** | `bug_wdb155_module_file_owned_ast_formal_must_not_demote_to_mut_ref_test` (`wdb155_module_file_option_match_ast_pipeline_must_keep_owned_formal`) | ✅ tip GREEN — bare-pass match-scrutinee + struct-literal field skip |
| P1 | **WDB-154: `use crate::<mod>::reexport` must keep module path** | `bug_wdb154_module_file_use_crate_mod_reexport_must_keep_path_test` | ✅ tip GREEN |
| P1 | **WDB-153: `a + (b as u32) << shift` must shift masked byte first** | `bug_wdb153_add_shift_precedence_must_shift_masked_byte_first_test` | ✅ tip GREEN — WJ shifts bind tighter than `+`; Rust emit `value + ((…) << shift)` |
| P1 | **WDB-152: string lit into owned `string` formal must `.to_string()`** | `bug_wdb152_module_file_string_lit_into_owned_string_formal_must_to_string_test` | ✅ tip GREEN — `"worker_hb".to_string()` (recheck 2026-09-10) |
| P1 | **LedgerKit clean row mapping without empty-concat** | `bug_ledgerkit_clean_row_mapping_no_plus_empty_test` | ✅ tip GREEN — P3.241 dogfood |
| P1 | **WDB-116: mutually recursive owned struct fields → Box** | `wdb116_module_file_mutually_recursive_struct_fields_must_box_or_cargo_check` | ✅ tip GREEN (recheck 2026-09-10) |
| P1 | **`strings.substring` int indices must not emit `i64 + 1_usize`** | `bug_haystack_contains_substring_int_index_unify_test` | ✅ tip GREEN — `(i + j + 1) as usize` (cargo+CLI 2026-09-11) |
| P1 | **Match `Ok(body)` → owned `string` formal must move (not `&body`)** | `bug_owned_match_binding_cross_fn_owned_string_formal_test` | ✅ tip GREEN — literal-equality pub APIs keep owned `String` |
| P1 | **LedgerKit request_context UUID/Bearer without empty-concat** | `bug_request_context_uuid_substring_no_plus_empty_test` | ✅ tip GREEN — P3.247 dogfood |
| P1 | **Seed overlay `Ok(body) => body` without empty-concat** | `bug_seed_overlay_read_body_no_plus_empty_test` | ✅ tip GREEN — P3.248 dogfood |
| P1 | **Seed overlay `remember_*` loop/split without empty-concat** | `bug_seed_overlay_remember_no_plus_empty_test` | ✅ tip GREEN — P3.249 dogfood |
| P1 | **Seed overlay `apply_*` BankLineView + module const → owned field** | `bug_seed_overlay_apply_bank_line_no_plus_empty_test` | ✅ tip GREEN — `LINE_STATUS_MATCHED.to_string()` (P3.257) |
| P1 | **Owned helper return → demoted `&str` formal auto-borrow** | `bug_owned_helper_into_demoted_str_formal_must_auto_borrow_test` | ✅ tip GREEN (2026-09-12) |
| P1 | **Module-file string lit → demoted `&str` method formal must not `.to_string()` (`wj-auth-api`)** | `bug_module_file_string_lit_into_demoted_str_must_not_emit_to_string_test` | ⚠️ tip fixture may keep owned `String` (no false RED); product auth demoted + `.to_string()` (P3.259) |
| P1 | **HashMap::get binding → demoted `&str` formal must not `.clone()` (`wj-auth-api` config)** | `bug_hashmap_get_binding_into_demoted_str_must_not_clone_test` | ✅ tip GREEN (P3.262); product interim may remain until auth regen |
| P0 | **`i64` shift/mask inside `Vec<u8>::push` must not emit `_u8` (`wj-uuid`)** | `bug_i64_bitand_hex_mask_must_not_emit_u8_test` | ✅ tip GREEN — cast clears call-arg int context |
| P1 | **Demoted `&str` after `starts_with` → owned formal (`wj-toml`)** | `bug_demoted_str_after_starts_with_must_auto_own_test` | ✅ tip GREEN — keeps owned + `.clone()` / cargo-check |
| P1 | **Single-use owned local → owned `string` formal must move (`wj-toml` get)** | `bug_single_use_owned_local_into_owned_string_formal_must_move_test` | ✅ tip GREEN (P3.254) — bare free-fn not Map::get key-borrow |

## P3.259 (2026-09-12) — wj-auth-api UUID v7 + string-lit demotion dogfood

| Change | Status |
|--------|--------|
| Ecosystem: register → UUID v7 id; JWT `sub`=id; `/me` returns `sub` | ✅ **12/12** on wj 0.50.0 |
| Adapter uses `handle_http(HttpMethod)` (same-crate); tests keep `handle(string)` | ✅ dual-runtime `HttpMethod` mismatch in test crate |
| Product string-label path: demoted `method: &str` + `"GET".to_string()` | ❌ observed on tip multipass (avoided via `handle_http`) |
| Gate `bug_module_file_string_lit_into_demoted_str_must_not_emit_to_string_test` | ⚠️ fixture may keep owned `String` (no false RED); panics if demoted+`.to_string()` |
| Gate `bug_owned_helper_into_demoted_str_formal_must_auto_borrow_test` | ✅ tip GREEN |

**Compiler agent:** when multipass demotes impl `method: string` → `&str`, call-site string lits must stay bare (WDB-168 / `.to_string()` twin). Strengthen fixture until it demotes like product.

## P3.261 (2026-09-13) — HashMap get binding into demoted `&str` + `.clone()`

| Change | Status |
|--------|--------|
| Ecosystem `wj-auth-api` `config_from_toml` via `wj-config`/`wj-toml` | ✅ **15/15** |
| Product: `parse_positive_int(v)` after `map.get` demoted to `&str` but emitted `v.clone()` | ❌ tip RED |
| Interim | `digits_to_int("${v}")` owned template |
| Gate `bug_hashmap_get_binding_into_demoted_str_must_not_clone_test` | filed |

**Compiler agent:** demoted `&str` formals must borrow HashMap get bindings (no `.clone()`). Related to demoted-str clone-skip gates; HashMap Option binding path still RED in auth dogfood.

## P3.254 (2026-09-12) — single-use owned local into owned string formal emits `&`

| Change | Status |
|--------|--------|
| Ecosystem `wj-toml` dotted keys + inline tables | ✅ **17/17** tip + wj 0.50.0 |
| Gate `single_use_owned_local_into_owned_string_formal_must_move` | ✅ tip GREEN |
| Root cause | `is_collection_key_lookup` treated bare free-fn `get` as Map key lookup (unknown receiver → name consensus) |
| Fix | Signature-shaped guard: bare free-fn (no `::`, no self, no receiver) is not a collection key; also harden stdlib-homonym auto-borrow early-return for empty `formal_param_types` |

## P3.257 (2026-09-12) — LedgerKit seed overlay apply BankLineView dogfood

| Change | Status |
|--------|--------|
| Disk cleanup | ✅ ~40–43Gi free |
| Gate `seed_overlay_apply_bank_line_*` (module const → owned field) | ✅ tip GREEN — `.to_string()` |
| Tip recheck WDB-166 field-clone→&str | ✅ tip GREEN (fixture + product analytic) |
| Tip recheck owned-helper→demoted `&str` | ✅ tip GREEN |
| Product `seed_bank_{match,clear}_overlay.wj` apply_* drop empty-concat | ✅ |
| `run_red_repro_bundle.sh` — apply + owned_helper + WDB-166 → GREEN_FILTERS | ✅ |

## P3.249b (2026-09-12) — LedgerKit seed overlay remember dogfood

| Change | Status |
|--------|--------|
| Disk cleanup + tip rebuild release `wj` | ✅ |
| Tip recheck OMB + WDB-155 | ✅ GREEN |
| Fixture + gate `seed_overlay_remember` (no `+ ""`) | ✅ GREEN (2/2 run) |
| Product `seed_bank_*_overlay.wj` path/remember/lookup drop empty-concat | ✅ |
| `run_red_repro_bundle.sh` — seed remember → GREEN_FILTERS | ✅ |

## P3.251 (2026-09-12) — demoted `&str` after starts_with into owned formal

| Change | Status |
|--------|--------|
| Ecosystem `wj-toml` inline tables + dotted keys (TDD) | ✅ **17/17** on wj 0.50.0; pinned still uses `"${raw}"` interim |
| Gate `bug_demoted_str_after_starts_with_must_auto_own_test` | ✅ tip GREEN (added 2026-09-12) |

**Compiler agent:** if a true RED multipass shape resurfaces (demoted `&str` into owned formal without auto-own), file the missing gate; do not treat stale queue ❌ as current tip truth.

## P3.250 (2026-09-12) — `i64` shift/mask → `u8` inside `Vec<u8>::push`

| Change | Status |
|--------|--------|
| Ecosystem `wj-uuid` v7 + nil/max (TDD; **20/20** on `wj` 0.50.0) | ✅ |
| Tip: `out.push(((ms >> 40) & 0xff) as u8)` keeps `_i64` masks/shifts | ✅ |
| Gate `bug_i64_bitand_hex_mask_must_not_emit_u8_test` (pack + push shape) | ✅ tip GREEN |
| Codegen: `generate_cast` clears `call_arg_expected_type` / assign int target | ✅ |
| Inference: cast operand not constrained by outer return/call context | ✅ |

**Root cause:** `Vec<u8>::push` set `call_arg_expected_type = u8`, which suffix-polluted nested `>>` / `&` literals (`40_u8`, `255_u8`). Cast result carries the u8 width; operand literals follow typed peers.

## P3.249 (2026-09-12) — owned match binding + WDB-155

| Change | Status |
|--------|--------|
| Pub free `string` formals with literal-only `==`/`!=` keep Owned (analyzer + str_ref skip) | ✅ |
| Relational `pattern == path` still demotes for loop reuse | ✅ regression GREEN |
| Bare-pass: MutBorrowed skip includes struct-literal field store; restore too | ✅ |
| Bare-pass: `Match` stmt usage includes scrutinee (`match bind_ast(ast)`) | ✅ |
| Gates: OMB same-file + hexagonal; WDB-155 binder-forwarder | ✅ tip GREEN |
| `run_red_repro_bundle.sh` — OMB + WDB-155 → GREEN_FILTERS | ✅ |

## P3.248 (2026-09-11) — seed overlay read-body dogfood

| Change | Status |
|--------|--------|
| Disk cleanup after push | ✅ ~36Gi free |
| Tip recheck `owned_match_binding_*` | ✅ tip GREEN (P3.249) |
| Fixture + gate `seed_overlay_read_body` (return-arm unify, no `+ ""`) | ✅ GREEN |
| Product `seed_bank_*_overlay.wj` ×4 drop `body + ""` / `"" + ""` | ✅ P3.248 |
| `run_red_repro_bundle.sh` — seed overlay → GREEN_FILTERS | ✅ |

## P3.247 (2026-09-11) — owned match binding RED + request_context dogfood

| Change | Status |
|--------|--------|
| Disk cleanup (`/tmp/wj-*`, tip debug) | ✅ ~34Gi free |
| Tip recheck `owned_match_binding_*` (same-file + hexagonal) | ✅ tip GREEN (P3.249) |
| Tip recheck WDB-155 owned AST formal | ✅ tip GREEN (P3.249) |
| Fixture + gate `request_context_uuid_substring` (no `+ ""`) | ✅ GREEN cargo+hexagonal |
| Product `request_context.wj` Bearer + UUID drop empty-concat | ✅ P3.247 |
| `run_red_repro_bundle.sh` — request_context → GREEN | ✅ |

## P3.246 compiler/runtime (2026-09-11) — mime charset, yaml empty, WDB-153

| Change | Status |
|--------|--------|
| `mime` known-ext table mirrors `std/mime.wj` constants (charset) | ✅ |
| `is_text` matches WJ (`application/json` / `javascript` / `xml`) | ✅ |
| `yaml::parse`/`to_json` reject empty/whitespace (`Err("empty yaml")`) | ✅ |
| Parser: shifts bind tighter than additive; Rust emit keeps Rust precedence for parens | ✅ |
| Gates: mime charset, yaml empty, WDB-153; tip recheck join_url/render/hexagonal HTTP | ✅ tip GREEN |
| `run_red_repro_bundle.sh` — mime/yaml/WDB-153 → GREEN_FILTERS | ✅ |

## P3.245 dogfood (2026-09-11) — LedgerKit `haystack_contains` drop empty-concat

| Change | Status |
|--------|--------|
| Disk cleanup | ✅ ~40–47Gi free |
| Tip recheck haystack substring int indices | ✅ GREEN — `(i + j + 1) as usize` |
| Hexagonal + CLI haystack gates | ✅ GREEN |
| Product `domain/string_contains.wj` drop `+ ""` | ✅ |
| `run_red_repro_bundle.sh` — haystack → GREEN_FILTERS | ✅ |

## P3.244 repro harness (2026-09-11) — `std::yaml` empty input parity

| Change | Status |
|--------|--------|
| `bug_std_yaml_empty_parity_test` — `to_json("")` / whitespace-only must `Err`, not Ok(`"null"`) | ✅ tip GREEN — runtime rejects empty/whitespace |

**Dogfood:** ✅ `wj-yaml` dropped empty/whitespace pre-check (2026-09-11).

## P3.243 repro harness (2026-09-11) — `std::mime` charset parity

| Change | Status |
|--------|--------|
| `bug_std_mime_charset_parity_test` — `from_extension`/`from_path` must equal `APPLICATION_*`/`TEXT_*` | ✅ tip GREEN — runtime known-ext table mirrors `std/mime.wj` |

**Dogfood:** ✅ `wj-mime` fully thin-wraps `std::mime` (2026-09-11).

## P3.244 CSV multipass + runtime `&str` literals + `-> i32` counters (2026-09-11)

| Change | Status |
|--------|--------|
| Disk cleanup (artifacts/tmp) | ✅ |
| `prefer_shared_ref`: runtime `&str`/`AsRef` beats partial WJ `emitted_rust_ref` owned slots | ✅ |
| `refresh_call_site_signature_for_arg` challenges `SignatureRegistry::stdlib()` | ✅ |
| `Call(FieldAccess)` module paths (`strings::split`) use refresh (not param-0-only prefer) | ✅ |
| `apply_owned_string_literal_coercion` uses refresh (no post-IR re-own) | ✅ |
| Fallback `strings_split_signature` matches scanner (`Reference(str)` + empty formals) | ✅ |
| `-> i32` trailing counter: cast when emission is still WJ `int`/`i64` | ✅ |
| Gates: `test_library_multipass_csv_*`, `test_library_multipass_strings_split_pipe`, Connection::query prefer_shared, `refresh_split_delimiter` | ✅ tip GREEN |
| Unit: `trailing_int_counter_into_i32_return_casts` | ✅ |
| `haystack_contains_substring_int_indices` — mixed `i64 + 1_usize` → `(expr) as usize` | ✅ tip GREEN |

**Root causes:** Multipass analyzes `std/*.wj` owned stubs without layering the scanned runtime baseline; literal coercion / FieldAccess paths skipped stdlib refresh. Separately, numeric inference unified counters to `i32` from `-> i32` while codegen still emitted `1_i64`. `coerce_arg_str_for_usize_formal` treated compound `… + 1_usize` as already-usize.

## P3.242 repro harness (2026-09-10) — substring int index unify

| Change | Status |
|--------|--------|
| Disk cleanup | ✅ |
| Tip-recheck WDB-116/139/145/149/150/151/152 + LedgerKit clean mapping | ✅ GREEN |
| Tip-recheck join_url / render / http hexagonal / json is_array | ✅ GREEN |
| `bug_haystack_contains_substring_int_index_unify_test` | ✅ tip GREEN — coerce wraps `i + j + 1` as `(…) as usize` (no mixed `1_usize`); cargo+CLI+hexagonal 2026-09-11 |
| Product `domain/string_contains.wj` drop empty-concat | ✅ P3.245 |
| `run_red_repro_bundle.sh` includes haystack RED filter | ✅ |
| windjammer-ui AuthFetch Into note | ✅ tip-GREEN wording |
| Product `make api-check` still int/usize + demote/Self cluster | ⚠️ tip |

**Compiler agent:** ✅ tip GREEN for haystack substring int unify (P3.242). Residual: empty-concat outside row helpers. P3.243 mime charset + P3.244 yaml empty ✅ tip GREEN; ecosystem dogfood complete.

## P3.241 LedgerKit dogfood + tip recheck (2026-09-10)

| Change | Status |
|--------|--------|
| Disk cleanup | ✅ ~25–28Gi free |
| `postgres_row_mapping.wj` drop empty-concat in helpers | ✅ |
| `col_string`/`col_int` call sites drop `"lit" + ""` (8 outbound adapters) | ✅ |
| `bug_ledgerkit_clean_row_mapping_no_plus_empty_test` | ✅ GREEN |
| Tip-recheck WDB-116 / 151 / 152 | ✅ GREEN |
| Residual product empty-concat on query `vec![…]` / owned fields | ⚠️ still present — drop only when tip covers |

**Compiler agent priority:** see P3.242 substring int unify; then residual empty-concat outside row helpers.

## P3.262 (2026-09-13) — tip cold relational/obs clears WDB-167/170/171/172/173 product REDs

| Gate | Tip fixture | Tip cold product emit | Stale `gen/` |
|------|-------------|----------------------|--------------|
| **WDB-167** Provider/`triple.0` | ✅ demotes + `&triple.0` | ✅ keeps **owned** `provider: RelationalDfProvider` | ❌ demoted + bare Cap args |
| **WDB-170** demoted `&str`.clone→String | ✅ (owned caller OK) | ✅ keeps **owned** `sql: String` + move into parse | ❌ `sql: &str` + `sql.clone()` |
| **WDB-171** owned `Vec<u8>`→`&Vec` | ✅ `&response` / `&encode_*()` | ✅ demoted + auto-borrow | ❌ demoted bare owned |
| **WDB-172** CatalogResolver | ✅ non-Copy label demotes + `&resolver` | ✅ bind_ast/resolve_select **owned** | ❌ demoted + `resolver.clone()` into `&` |
| **WDB-173** `&worker_id.clone()` | ✅ owned String + `worker_id.clone()` | ✅ `worker_id.clone()` (no `&`) | ❌ `&worker_id.clone()` |
| HashMap get→demoted `&str` | ✅ tip GREEN | — | product interim `digits_to_int("${v}")` |

**Compiler:** tip multipass already greens these shapes on cold transpile; stale `gen/` was the product RED source. Product gates prefer `.agent-wip/{rel,obs}_tip_out` when present (session cold transpile artifacts).

**Dogfood:** sync tip emit into `wdb-layers/gen/` (`transpile_relational_module_file.sh` + observability module-file + semantic Caps) to clear stale product `cargo check`. No Phase 606+.

**Disk:** non-destructive prune → ~224 Gi free.

## P3.263 (2026-09-13) — WDB-110/111: emitted-ref formals drive clone→to_string

| Gate | Status |
|------|--------|
| `wdb110_*` isolate + same-file | ✅ tip GREEN — `li_path.clone()` stays clone into owned `String` |
| `wdb111_*` multipass | ✅ tip GREEN — no `&…clone()` / no `.to_string()` rewrite |
| IR `owned String → owned String` | ✅ Identity (not `ToOwnedString`) |
| Clone rewrite / lower / finalize | ✅ keyed off `emitted_rust_ref_formals`, not stale `inferred_borrowed_params` |

**Root cause:** analyzer `Borrowed` + IR `ToOwnedString` rewrote owned-place `.clone()` to `.to_string()` / borrowed call sites even when codegen still emitted `String` formals.

**Fix:** coercion Identity for String→String; string clone helpers use codegen-confirmed `&str` emit set.

## P3.264 (2026-09-13) — runtime-std readonly + text-return formals keep owned `String`

| Gate | Status |
|------|--------|
| `for_loop_borrowed_item_clones_into_owned` | ✅ tip GREEN — `strings::` runtime forward + readonly `.len()` keeps owned formal; loop elem not `&item` into owned callee |
| `auto_borrow_vec_new_at_call_site` | ✅ tip GREEN — `callee_emits_shared_rust_ref_param` before bare `Vec<T>` formal check |
| `comparison_only_string_formal_demotes` | ✅ tip GREEN — comparison-only demotion skips params used as call arguments |
| `wdb106_*` / `wdb142_*` explicit clone | ✅ tip GREEN — preserve user `.clone()` at demoted/borrow callees when formal is owned |
| `struct_field_into_owned_string_formal` / `multi_use_struct_field_must_clone` | ✅ tip GREEN — text-returning helpers keep `String` formals; field multi-use auto-clone |
| `wdb110_*` / `wdb111_*` | ✅ tip GREEN — `strings::is_empty` + `.len()` mixed readonly keeps owned `String` formals |
| `wdb169_*` fixture | ✅ tip GREEN — owned helper temp into owned Custom formal (no `&empty_bakeoff_run()`) |
| `wdb169` product gate | ✅ after `gen/relational/wave1_opt_hardware_port.rs` regen (stale Sep-12 artifact) |

**Root cause:** `strings::len(s)` WJ signatures looked owning; runtime-std module detection missed `use windjammer_runtime::strings`; text-return helpers fell through to default `&str` demotion when analyzer marked `str_ref_optimizable`.

**Fix:** signature-scanned runtime-std forward detection; early owned `String` for text-returning APIs; readonly receiver methods skip false owning-use; IR terminal peel for borrowed `for` elems into shared-text callees.

## P3.260 WindjammerDB CQ-C5 — freeze dogfood/tip-cluster; file WDB-170/171 coverage REDs (2026-09-13)

| Gate | Status |
|------|--------|
| Fresh `cargo check --lib` | ⚠️ **297** E0308 (`&str←String` 108, `&T←T` 99, `String←&str` 49, `T←&T` 41) |
| Tip **WDB-167** | ✅ tip cold owned Provider (P3.262); stale gen still demoted |
| Tip **WDB-169** product gen | ⚠️ queue claimed tip GREEN but gen still has `&empty_bakeoff_run()` until resync — re-verify |
| Tip **WDB-170** demoted `&str` `.clone()` → owned `String` | ✅ tip cold owned sql (P3.262) |
| Tip **WDB-171** owned `Vec<u8>` → demoted `&Vec<u8>` | ✅ tip cold auto-borrow (P3.262) |
| `dogfood_gen_p153.py` | ❄️ **FREEZE** — no new transforms; `WDB_DOGFOOD_REFUSE_NEW=1` exits 2 |
| `sync_tip_cluster.sh` | ❄️ requires `WDB_TIP_CLUSTER_OK=1` |

**Superseded by P3.262** for tip truth. Sync `.agent-wip/rel_tip_out` (+ obs) into `gen/` to clear stale product `cargo check`.

## P3.258 WindjammerDB CQ-C5 — WDB-169 &empty_bakeoff into owned + tip-cluster guardrail (2026-09-12)

| Gate | Status |
|------|--------|
| Tip-cluster `pg_wire`+`wave1_opt` | ⚠️ clears WDB-168 `String::from` (owned name) but invents `&empty_bakeoff_run()` → **+82** Bakeoff errors |
| `cargo check --lib` after that cluster | ⚠️ **297** E0308 (was 277) |
| Tip **WDB-167** `&Provider` ← owned | ❌ **product RED** (65 Caps) |
| Tip **WDB-168** lit→demoted `&str` | ✅ product GREEN via tip-cluster owned name (full multipass still at risk) |
| Tip **WDB-169** `&empty_bakeoff_run()` → owned BakeoffRun | ✅ tip GREEN (multi-slot fixture + force_owned Call peel); product gen still stale until tip retranspile sync |

**Compiler agent priority:** sync `wave1_opt_hardware_port` gen from tip; then WDB-167. Prefer full tip over incomplete tip-clusters. No Phase 606+.

## P3.257 WindjammerDB CQ-C5 — coverage REDs WDB-167/168 for ungated build buckets (2026-09-12)

| Gate | Status |
|------|--------|
| Fresh `cargo check --lib` | ⚠️ **277** E0308 (`found &mut` = 0) |
| Tip **WDB-166** field clone→`&str` | ✅ product GREEN after tip-cluster; full multipass still needs tip |
| Tip **WDB-167** owned Provider/`triple.0` → demoted `&Provider` | ❌ **product RED** (65 Caps; tip fixture GREEN when formal stays owned) |
| Tip **WDB-168** lit `"s1"` → demoted `&str` must not `String::from` | ❌ **product RED** (wave1_opt; tip fixture GREEN when formal stays owned) |
| Residual `&str`←`String` beyond 167/168 | ⚠️ still ~162 class; dogfood/tip-cluster interim only |

**Compiler agent priority:** WDB-167 + WDB-168 (signature-driven). Goal: eliminate `dogfood_gen_p153.py` / tip-cluster patches once tip greens these. No Phase 606+.

## P3.256 WindjammerDB CQ-C5 — census after WDB-165 + WDB-166 field-clone→&str (2026-09-12)

| Gate | Status |
|------|--------|
| Tip **WDB-165** owned State call | ✅ tip + product GREEN |
| `cargo check --lib` (in-repo target) | ⚠️ **254** errors (E0308×250, E0382×4); **`found &mut` = 0** |
| Dominant bucket | **168×** `expected &str, found String` (e.g. `emit.sql.clone()` → demoted FFI `sql: &str`) |
| Tip **WDB-159** sequential owned→&str borrow | ✅ tip GREEN (does **not** cover struct-field `.clone()`) |
| Tip **WDB-166** field `.clone()` into demoted `&str` | ✅ tip GREEN (fixture + product analytic; recheck 2026-09-12 P3.257) |

**Compiler agent priority:** WDB-166 (signature-driven borrow of owned field/`clone` into demoted `&str`); then `&T`↔owned Custom (OptDatedArtifact, MvccStore, …).

## P3.255 WindjammerDB CQ-C5 — WDB-165 owned State call over-borrow (2026-09-12)

| Gate | Status |
|------|--------|
| Tip **WDB-163/164** owned ServeState / Store formals | ✅ tip GREEN (product formals owned) |
| Tip **WDB-165** `&state.clone()` into owned formal | ✅ tip GREEN — local `emitted_owned_arg_contract` beats stale global shared-ref; peel `&` before `.clone()` |
| Product `relational_pg_serve_port` | ✅ `on_parse(state.clone(), …)` / `on_sync(state.clone())` (no leading `&`) |
| Gate `demoted_str_after_starts_with` (owned-throughout OK) | ✅ tip GREEN |
| Product `cargo check --lib` | ⚠️ was **~118** after sync; re-sample → see P3.256 (**254**, mut cleared) |

**Compiler agent priority:** residual String/`&str`/Vec/E0596; OptDatedArtifact/`&T`→owned buckets; P3.254 single-use local move.

## P3.253 WindjammerDB CQ-C5 — WDB-163 &mut State early-return (2026-09-12)

| Gate | Status |
|------|--------|
| Tip **WDB-162** `&mut Value` temps | ✅ tip GREEN (product full multipass clear) |
| Tip **WDB-163** `&mut State` formal + bare `(state,)` return | ✅ tip GREEN (superseded by P3.253 store/serve + P3.255) |
| `cargo check --lib` | ⚠️ was **~132**; re-sample after WDB-165 |

**Compiler agent priority:** residual String/`&str`/Vec/E0596.

## P3.253 WindjammerDB CQ-C5 — WDB-163/164 store/serve owned consume-rebind (2026-09-12)

| Change | Status |
|--------|--------|
| Tip **WDB-164** `let mut out = store` keep owned Store | ✅ tip GREEN — Assignment call-hint walk + `param_moved_into_let_binding` skip/restore |
| Tip **WDB-163** early-return owned ServeState | ✅ tip fixture GREEN; product retranspile → owned `PgWireServeState` (no `&mut` formal) |
| Product cold retranspile after tip | ✅ `relational_mvcc_put_version(store: RelationalMvccStore, …)`; serve formals owned |
| `cargo check --lib` | ⚠️ re-sample after this tip |

**Compiler agent priority:** residual String/`&str`/Vec/E0596; sample next rustc bucket.

## P3.252 WindjammerDB CQ-C5 — tip 157–160 product rebuild + WDB-162 (2026-09-12)

| Change | Status |
|--------|--------|
| Rebuild tip `wj` (shared cache) after tip greens | ✅ |
| Cold relational module-file + Cap SQL owned restore | ✅ |
| Tip-cluster: binder/CLI + dispatch/mvcc/opt (as_ref / `&mut Value`) | ✅ gen/relational clean; full multipass still RED |
| `cargo check --lib` | ⚠️ **~236–259** (was ~643 → ~358 → ~236) |
| Tip **WDB-160** | ✅ tip + product GREEN (`exit(2_i32)`) |
| Tip **WDB-161** product as_ref | ✅ tip full multipass cleared (product gate GREEN) |
| Tip **WDB-162** `&mut Value::Int64` into owned Value | ✅ tip GREEN — while/push/assignment struct-literal store skips MutBorrowed demotion |

**Compiler agent priority:** residual String/`&str`/Vec/E0596; store MutBorrowed on put_version if still wrong.

## P3.251 WindjammerDB CQ-C5 — cold tip rebuild + dogfood Cap SQL restore + WDB-160/161 (2026-09-12)

| Change | Status |
|--------|--------|
| Disk cleanup (debug targets + `/tmp/wj_*`) | ✅ ~23 Gi free |
| Tip cold `transpile_relational_module_file` + sync | ✅ tip 0.50.0 |
| Dogfood: **stop** Cap SQL `String→&str` demotion under module-file | ✅ TDD `test_dogfood_module_file_keeps_owned_cap_sql.py` FAIL→PASS |
| `cargo check --lib` after fix | ⚠️ **359** errors (was ~643 stale / ~380 demoted) |
| Tip **WDB-157** `impl Into<String>` | ✅ tip GREEN |
| Tip **WDB-158** Cell/Value `&mut` | ✅ tip GREEN |
| Tip **WDB-159** sql.clone→&str | ✅ tip GREEN; product false path was dogfood demotion |
| Tip **WDB-160** `process::exit` → `i32` not `i64` | ✅ tip GREEN |
| Tip **WDB-161** full multipass `.as_ref()` | ✅ tip GREEN — product retranspile 0×; Gate A cargo-check |

**Compiler agent priority:** residual E0596 / expected-String / Vec; WDB-159 product cold if still clones.

## P3.250 WindjammerDB CQ-C5 — cold-gen storm gates WDB-156–161 (2026-09-12)

| Gate | Status | Product bucket |
|------|--------|----------------|
| **WDB-155** binder-forwarder | ✅ tip GREEN | AST owned formal |
| **WDB-156** writeback owned | ✅ tip GREEN (fixture) | writeback `&mut` |
| **WDB-157** `string` ≠ `impl Into<String>`+clone | ✅ tip GREEN — Into only on `self` methods | `pg_wire_parse` |
| **WDB-158** Cell/Value compare ≠ `&mut` | ✅ tip GREEN — match-scrutinee skip/restore | `found &mut` (~154) |
| **WDB-159** owned string → demoted `&str` borrow not clone | ✅ tip GREEN (fixture); product cold may still `sql.clone()` | `expected &str, found String` |
| **WDB-160** `process::exit` → i32 | ✅ tip GREEN — std `i32` + call-site coerce | int-width exit |
| **WDB-161** no spurious `.as_ref()` on method recv | ✅ tip GREEN — stale gen cleared; explicit-clone walk If/While keeps owned | binder_port |

**Coverage honesty:** tip 157–159 green does **not** clear all cold-gen rustc errors. Next: WDB-160/161 + int-width/Vec residuals.

**Compiler agent priority:** residual E0596 / expected-String / Vec; strengthen WDB-159 if product cold still clones.

## P3.218 WindjammerDB CQ-C5 — multicol Ready-wrap + WDB-155 (2026-09-11)

| Change | Status |
|--------|--------|
| Disk cleanup | ✅ ~33–36Gi free |
| Product: multicol Ready-wrap + OTLP `metric=` KV parse | ✅ `.wj` + tip-sync |
| Tip **WDB-155** emit-only Option/match fixture | ✅ superseded — binder-forwarder gate is source of truth |
| Tip **WDB-155** binder-forwarder (`emit_sql` → `bind_ast`) | ✅ tip GREEN (P3.249 — match-scrutinee bare-pass skip) |
| Product gen cold rebuild | ⚠️ still ~643 rustc after tip WDB-155 — see P3.250 |
| layers full `--lib` | ⚠️ blocked on cold gen |

**Compiler agent:** ✅ WDB-155 tip GREEN. Next: P3.250 (WDB-157/158/159).

## P3.217 WindjammerDB CQ-C5 — semantic Cap call-cycle cut + WDB-154 (2026-09-11)

| Change | Status |
|--------|--------|
| Disk cleanup | ✅ ~49Gi free |
| Product: cut semantic `*from_band_inequality*` Cap call SCC (inequality parents **non-from** equality_readback) | ✅ E0072 cleared; `check_semantic_cap_call_cycles.py` **0** |
| Guardrail: `scripts/check_semantic_cap_call_cycles.py` | ✅ |
| Cap filter `from_band_inequality` | ✅ **20/20** |
| Tip **WDB-116** 2-cycle Box | ✅ tip GREEN (also Boxes one edge in 3-cycle fixture) |
| Tip **WDB-154** `use crate::<mod>::reexport` path keep | ⚠️ FILED |
| Dogfood: column_i64/f64 String::from strip; lsqb owned graph; job_store &store strip; cdlp/sql test path fixes | ✅ |
| layers full `--lib` | ⚠️ running toward INT-EXIT |

**Compiler agent priority:** **WDB-154**, **WDB-152**, **WDB-153**, **WDB-150**.

## P3.216 WindjammerDB CQ-C5 — protobuf fixture/varint + WDB-153 (2026-09-10)

| Change | Status |
|--------|--------|
| Product: full_decode / resource_payload Cap fixtures pad to wire `payload_len=12` | ✅ protobuf **46/46** |
| Product: varint decode uses intermediate `piece` (shift-before-add) | ✅ `test_otlp_protobuf_varint_multi_byte_300` |
| Product: depth_guard malformed fixture uses single-byte len 127 (oversize) | ✅ |
| Tip **WDB-153** `a + (b as u32) << shift` grouping | ⚠️ FILED |
| Dogfood: owned Vec/field formals for protobuf siblings | ✅ |

**Compiler agent priority:** **WDB-153**, **WDB-152**, **WDB-150**.

## P3.215 WindjammerDB CQ-C5 — job store release Cap + WDB-151/152 (2026-09-10)

| Change | Status |
|--------|--------|
| Product: `job_store_release_cap_after_heartbeat` threads claim→heartbeat→release on **one** store (was fresh `job_store_cap_demo()` after heartbeat → `jobs_released=0`) | ✅ product fix — unit tests green |
| Product: enterprise listener release reaper stack Cap no longer double-invokes release_stack Cap (stack overflow) | ✅ flattened; **40** listener reaper stacks batch-fixed |
| Guardrail: `scripts/check_listener_reaper_stack_double_invoke.py` | ✅ **0** offenders |
| `job_store_release` lib filter | ✅ **18/18** |
| Tip **WDB-151** owned store formal must not demote to `&T` | ✅ tip isolate GREEN — product dogfood still after tip-sync |
| Tip **WDB-152** string lit → owned `string` formal must `.to_string()` | ⚠️ **RED** — product dogfood for claim/heartbeat `"worker_hb"` |
| Dogfood: `dogfood_restore_check.py` owned release + strip `&store` + worker_id `.to_string()` | ✅ |
| complete_ops `post_body_trace_complete_ops` filter | ✅ **28/28** |
| Cap call cycles | ✅ **0** |

**Compiler agent priority:** **WDB-152**, **WDB-150**, **WDB-145**, **WDB-139**.

## P3.214 repro harness closure (2026-09-01)

| Change | Status |
|--------|--------|
| `bug_json_is_array_len_owned_value_borrow_test` — multipass `json.is_array`/`json.len` after `json.parse` (`wj-todo-cli` `todos_from_json`) | ⚠️ RED — owned `Value` vs runtime `&Value` E0308 (single-file emit gate passes; multipass `cargo check` fails) |
| `run_red_repro_bundle.sh` — **3-gate** tail repro bundle (+ json is_array/len) | ⚠️ RED |

**Compiler agent:** auto-borrow owned `Value` for `is_array` / `len` / `as_*` predicates (signature registry). **Ecosystem:** `wj-todo-cli` uses `[` prefix check + `get_index` loop until green.

## P3.213 repro harness closure (2026-09-01)

Verified on in-tree tip (`cargo test --test all`, 2026-09-01):

| Change | Status |
|--------|--------|
| `bug_multipass_cross_crate_join_url_owned_base_test` — `wj-sitegen` `join_url` class | ⚠️ RED |
| `bug_multipass_cross_crate_render_owned_template_helper_test` — `wj-template` `render(helper())` class | ⚠️ RED |
| `bug_multipass_http_adapter_hexagonal_test` — `domain/` + `adapters/` layout | ⚠️ RED |
| `bug_multipass_config_parse_positive_int_chars_test` — hexagonal `domain/config` | ✅ tip GREEN |
| `http_adapter_owned_reply.wj` + hexagonal fixtures (DRY) | ✅ shipped |
| `run_red_repro_bundle.sh` — **3-gate** tail RED bundle | ⚠️ verified |

**Compiler agent:** green cross-crate owned/borrow at app→package boundary; hexagonal HTTP adapter owned emit.

## P3.212 repro harness closure (2026-09-01)

Verified on in-tree tip `61e4ed30` (`cargo test --test all`):

| Change | Status |
|--------|--------|
| `bug_multipass_http_adapter_owned_reply_test` — hexagonal HTTP adapter owned reply/body | ✅ tip GREEN |
| `bug_multipass_strings_chars_for_in_mut_char_test` + `parse_positive_int_chars.wj` | ✅ tip GREEN |
| `bug_multipass_std_async_runtime_import_test` + `pause_ms_async_runtime.wj` | ✅ tip GREEN |
| `bug_std_fs_dir_entry_name_multipass_test` — DirEntry multipass (P3.211 carry) | ✅ tip GREEN |
| `run_red_repro_bundle.sh` — **3-gate** tail repro bundle | ✅ all GREEN |

**Compiler agent:** (done) HTTP adapter owned emit, `strings.chars` loop binding, `async_runtime` import.

## P3.211 repro harness closure (2026-08-31)

Verified on in-tree tip `321220a9` (`cargo test --test all`, `CARGO_BIN_EXE_wj`):

| Change | Status |
|--------|--------|
| `migrate_dir_entry_name.wj` + multipass `DirEntry.name` repro | ⚠️ RED |
| `cli_args_*` / `migrate_cli_use_positional.wj` — DRY Vec demote fixtures | ✅ tip GREEN |
| `bug_app_test_http_method_module_file_test` — hexagonal `src/domain` + `wj test` | ✅ tip GREEN (regression guard) |
| `run_red_repro_bundle.sh` — **1-gate** tail RED bundle (multipass DirEntry only) | ⚠️ RED |

**Compiler agent:** green multipass `DirEntry.name()` → `file_name()` wiring (last tail RED gate).

## P3.210 repro harness closure (2026-08-31)

Verified on committed tip `9167b658` (`cargo test`, clean `src/`):

| Change | Status |
|--------|--------|
| `note_store_hashmap_field_get.wj` — shared `wj-notes-api` HashMap fixture (DRY) | ✅ shipped |
| `bug_hashmap_field_get_i64_key_auto_borrow_test` — uses shared fixture | ✅ |
| `bug_app_cross_crate_pretty_without_own_test` — cross-crate `pretty(body)` emit guard | ✅ tip GREEN |
| `bug_private_struct_field_spurious_use_import_test` — `wj-webhook` `BusEventBody` class | ✅ tip GREEN |
| 7-gate `run_red_repro_bundle.sh` re-verified | ⚠️ all RED on tip |

**Compiler agent:** green WDB-112 → WDB-114 → HashMap/Vec/DirEntry/HttpMethod rows.

## P3.209 repro harness closure (2026-08-31)

Verified on committed tip `a1ce1b2a` (`wj` CLI + `cargo test`, no local `src/` patches):

| Change | Status |
|--------|--------|
| `bug_self_get_call_emits_delete_method_test` + `note_store_self_get_call.wj` | ✅ tip GREEN — regression guard (`wj-notes-api` class) |
| `run_red_repro_bundle.sh` — 7-gate RED bundle (dropped GREEN `self_get`; `WDB-115` regression guard) | ✅ shipped |
| Main queue table reconciled — WDB-112/113/114, HashMap, Vec, DirEntry, HttpMethod **RED** | ✅ |

**Compiler agent:** green WDB-112 → WDB-114 → stdlib wiring rows; product roadmap complete.

## P3.208 repro harness closure (2026-08-31)

| Change | Status |
|--------|--------|
| `bug_wdb114_module_file_vec_annotation_must_import_element_type_test` | ⚠️ RED — `Vec<JobRecord>` without `use JobRecord` |
| `bug_wdb115_early_return_private_method_call_test` | ✅ tip GREEN — regression guard |
| `bug_app_test_http_method_type_mismatch_test` — mini app `wj test` with `HttpMethod` lib port | ⚠️ RED on tip |
| `tests/run_red_repro_bundle.sh` — runnable RED bundle | ✅ shipped |
| `wj-auth-api` / `wj-webhook` hexagonal deepen | ✅ product regression |

## P3.207 repro harness closure (2026-08-31)

| Change | Status |
|--------|--------|
| `bug_app_test_http_method_type_mismatch_test` harness (`wj test` layout) | ✅ shipped (gate still RED on tip) |
| WDB-112/113 full-library multipass repros strengthened | ✅ shipped |

## P3.206 repro harness closure (2026-08-30)

Verified on clean tip (`cargo test --test all`, no local `src/` patches):

| Gate | Result |
|------|--------|
| WDB-112 | ✅ |
| WDB-113 | ✅ |
| WDB-114 | ✅ |
| WDB-115 | ✅ tip GREEN (regression guard) |
| Cross-module Vec borrow | ⚠️ RED |
| HashMap field `.get(i64)` | ✅ |
| `DirEntry.name()` | ✅ |
| `pub mod` EOF | ✅ tip GREEN |
| Multipass owned param/literal | ✅ tip GREEN (new regression guards) |

New: `bug_cross_crate_owned_formal_multipass_call_site_test` — `pretty(body)` + `render_html(lit)` multipass.

## P3.205 repro bundle (2026-08-30)

Run all tail RED gates (expect failures on tip until compiler `src/` greens):

```bash
bash tests/run_red_repro_bundle.sh
```

| Gate | Repro test | Status |
|------|------------|--------|
| WDB-112 demoted `&str` + `.clone()` | `wdb112_full_library_multipass_*` | ⚠️ RED |
| WDB-113 demoted `&mut T` + `.clone()` | `wdb113_full_library_multipass_*` | ⚠️ RED |
| WDB-114 `Vec<T>` annotation missing import | `wdb114_module_file_vec_type_annotation_*` | ⚠️ RED |
| Cross-module Vec borrow | `cross_crate_vec_string_helper_*` | ⚠️ RED |
| HashMap field `.get(i64)` | `hashmap_field_get_i64_key_*` | ✅ |
| `DirEntry.name()` wiring | `std_fs_dir_entry_name_*` | ⚠️ RED |
| `HttpMethod` lib port vs `wj test` | `app_test_http_method_*` | ⚠️ RED |
| `self.get` vs `self.delete` homonym | `self_get_call_must_not_emit_delete_method` | ✅ tip GREEN (regression guard) |
| WDB-115 early return callee | `wdb115_early_return_private_method_*` | ✅ tip GREEN (regression guard) |
| `pub mod` EOF truncation | `pub_mod_at_end_*` | ✅ tip GREEN (regression guard) |
| Multipass owned param/literal | `bug_cross_crate_owned_formal_multipass_*` | ✅ tip GREEN (regression guard) |
| App owned forwarder cross-crate | `cross_crate_owned_forwarder_*` | ✅ tip GREEN (regression guard) |

**Note:** Product roadmap complete; tail = compiler hygiene only. Do not work around RED rows in application code.

## P3.200 audit closure (2026-08-30)

Product `finance-screens` shim inventory after PanelHead sweep:

| Shim | Count | Gate | Action |
|------|-------|------|--------|
| `json + ""` public-port delegates | 0 | `codegen_cross_module_match_arm_multi_use_owned_formal` | ✅ tip GREEN — drop shims on regen |
| Inline match `+ ""` in json parsers | 0 | `codegen_match_string_arms_must_unify` | DRY via `clip_json_array_through_bracket` |
| `account_code + ""` in panels | 0 | `codegen_multi_use_struct_field_must_auto_clone` | Green on tip |
| Hand-rolled `hub-kicker` | 0 | — | PanelHead / PanelSectionHead sweep complete (P3.194–P3.198) |

Remaining RED rows above are **compiler-only** — no further product shim drops without tip fixes.

## P3.201 product closure (2026-08-30)

| Change | Status |
|--------|--------|
| Drop `parse_recon_report_fields` / `parse_analytics_schema_fields` `json + ""` delegates | ✅ shipped — bare `json` at public port; tip regen GREEN |
| Bank match: table before recon mounts; toolbar below stack | ✅ shipped — fixes Playwright pointer intercept |
| AccountRail `title_label` + `data-wj-rail-title` checkbook sync | ✅ shipped — WJ-UI runtime uses attr not label textContent |

## P3.202 product closure (2026-08-30)

| Change | Status |
|--------|--------|
| WDB-112 repro gate `wdb112_full_library_multipass_demoted_str_formal_must_borrow_clone_call_sites` | ⚠️ RED — demoted `&str` callee + `.clone()` call sites (compiler `src/` fix pending) |
| `bug_std_fs_dir_entry_name_wiring_test` | ⚠️ RED — `DirEntry.name()` → runtime `file_name()` |
| `bug_db_connection_helper_reuse_invalid_clone_test` | ✅ tip GREEN |
| `finance-ui` owned-`String` wrapper boundary (`.into()` at screens delegates) | ✅ shipped — unblocks `make wasm-boot` |

## P3.203 repro harness closure (2026-08-30)

| Change | Status |
|--------|--------|
| WDB-112 gate: emit assertion + `cargo_check()` when borrow fix lands | ⚠️ RED — demoted `&str` + `.clone()` call sites |
| `bug_cross_crate_vec_helper_must_auto_borrow_test` | ⚠️ RED — demote+clone multipass `&Vec` borrow |
| WDB-113 / `std_fs DirEntry.name` / HashMap i64 `.get` | ⚠️ RED — strengthened in P3.204 |

## P3.204 repro harness closure (2026-08-30)

| Change | Status |
|--------|--------|
| WDB-113 demote+clone multipass fixture + `cargo_check` green path | ⚠️ RED |
| `bug_hashmap_field_get_i64_key_auto_borrow_test` multipass + `cargo_check` | ✅ |
| `bug_std_fs_dir_entry_name_wiring_test` requires `file_name()` emit | ⚠️ RED |
| `bug_pub_mod_at_end_truncates_lib_codegen_test` | ⚠️ RED — `pub mod` at EOF truncates root fns |

## P3.205 repro harness closure (2026-08-30)

| Change | Status |
|--------|--------|
| `bug_app_multipass_cross_crate_owned_forwarder_module_file_test` multipass + `cargo_check` | ⚠️ RED — `own()` + `join_path` emits `&local` |
| COMPILER_REPRO_QUEUE P3.205 repro bundle (`cargo test` filter list) | ✅ shipped |
| Deduped duplicate `pub mod` EOF queue row | ✅ shipped |

**Note:** Product roadmap complete; tail = compiler hygiene. Do not work around RED rows in application code.

## Historical note (superseded)

**2026-08-30 (P3.199 era):** Several queue RED rows since resolved on tip (WDB-110, cross-module match-arm). Remaining RED rows listed above.

## Stdlib adoption P0/P1 (see `tests/STDLIB_ADOPTION_QUEUE.md`)

All rows use **`assert_stdlib_runtime_links`** (`cargo check`, not transpile-only). Fix hints in queue doc.

| Priority | Std gap | Repro test(s) | Status |
|----------|---------|---------------|--------|
| P0 | **`std::encoding` base64 string encode/decode** | `bug_std_encoding_base64_string_api_test` | ✅ |
| P0 | **`std::random.range` → `int_range`** | `bug_std_random_range_codegen_test` | ✅ |
| P0 | **`std::crypto.sha1_bytes`** | `bug_std_crypto_sha1_bytes_test` | ✅ |
| P0 | **`std::crypto.sha256_hex` wiring** | `bug_std_crypto_sha256_hex_wiring_test` | ✅ |
| P0 | **`std::time.utc_now` / `timestamp_millis`** | `bug_std_time_utc_now_test`, `bug_std_time_timestamp_millis_test` | ✅ |
| P0 | **`std::uuid.v4`** | `bug_std_uuid_v4_module_test` | ✅ |
| P0 | **`std::mime` constants + from_extension** | `bug_std_mime_module_wiring_test` | ✅ |
| P0 | **`std::mime` charset parity (`from_extension`/`from_path` = constants)** | `bug_std_mime_charset_parity_test` | ✅ tip GREEN (P3.243) |
| P0 | **`std::path` join / file_name** | `bug_std_path_join_module_test` | ✅ |
| P0 | **`std::jwt` HS256 sign/verify wiring** | `bug_std_jwt_hs256_wiring_test` | ✅ |
| P1 | **`std::yaml` parse / to_json** | `bug_std_yaml_module_test` | ✅ |
| P1 | **`std::yaml` empty/whitespace input rejection** | `bug_std_yaml_empty_parity_test` | ✅ tip GREEN (P3.244) |
| P1 | **`std::csv` idiomatic `Result<…, string>`** | `bug_std_csv_parse_idiomatic_test` | ✅ |
| P1 | **`std::csv.write` owned `Vec<Vec<string>>` auto-borrow** | `bug_std_csv_write_owned_rows_auto_borrow_test` | ✅ tip GREEN (homonym `pub fn write` → `csv.write`) |
| P1 | **`std::db` connect + execute** | `bug_std_db_execute_wiring_test` | ✅ |
| P1 | **`std::time` RFC3339 roundtrip wiring** | `bug_std_time_rfc3339_roundtrip_wiring_test` | ✅ |
| P1 | **`std::encoding.url_encode` / `url_decode`** | `bug_std_encoding_url_encode_wiring_test` | ✅ |
| P1 | **`std::crypto` bcrypt hash/verify** | `bug_std_crypto_bcrypt_password_wiring_test` | ✅ |
| P1 | **`std::compress` gzip encode/decode (`wj-compress`)** | `bug_std_compress_gzip_wiring_test` | ✅ tip GREEN — runtime `compress` + flate2 (Base64 gzip string API) |
| P1 | **`std::regex` wiring (`wj-regex`)** | `bug_std_regex_module_wiring_test` | ✅ tip GREEN (verify) |
| P1 | **Reuse demoted `string` in `Ok((text, ""))` after `split_once` / `contains` (`wj-url`)** | `bug_match_none_arm_string_after_split_test` | ✅ tip GREEN — tuple demoted→`.to_string()`; if/else int↔`strings.len` usize unify; local `join_path` beats runtime-std homonym |

## Application cleanup (after green gates)

1. ✅ **Swap** `graph_vertex_map.wj` Vec backend → HashMap — applied in windjammerdb (2026-08-04); `graph_vertex_map.vec.wj` kept as backup.
2. ✅ **CSV pipe fields** — `lsqb_csv_loader.wj` uses `strings::split(line, "|")` (not `byte_at`/substring). Gate: `test_library_multipass_csv_while_index_owned_string_param` (+ for-in / split multipass). LSQB lib tests green after clean `rm -rf gen && wj build`.
3. ✅ **Harness extract** — `drain_network` uses `match self.network.poll(...)` (WDB-042). Compiler emits in-place `&mut` call; no `let mut net = self.network`.
4. ✅ **Index `consistent()`** — `key_in_range` (no byte-field extract). Network `poll` delegates to `release_held_if_ready`.
5. ✅ **`self.queue.clone()` into owned helpers** — call-arg writeback
   (`let r = f(self.field); self.field = r.sub`) emits `std::mem::take(&mut self.field)`.
   Gate: `codegen_owned_field_call_writeback_gate_test`.

## P3.265 (2026-09-13) — suite filter batch GREEN

Verified on tip after P3.264 + ownership batch:

| Filter | Status |
|--------|--------|
| `function_args_3layer_test`, `codegen_cross_module_signature_test`, `method_call_reference_args_test` | ✅ |
| `param_ownership_multiple_use_test`, `tryop_ownership_inference_test`, `library_multipass_wdb_csr_gates_test` | ✅ |
| `codegen_copy_type_arg_test`, `match_arm_binding_method_call_test`, `void_return_semicolon_test`, … (13/18 named filters) | ✅ |
| `cross_crate_dogfooding_ownership_test` (subset), `codegen_component_library_regen_gates`, `bug_match_none_arm_string_after_split` | 🔴 follow-up |

Fix themes: skip `as i32` when arg already `i32`; text HashMap keys → `&str`; loop-body param borrow; TryOp owned early return; `u32` counter literals + `u32 as usize` vs `.len()`.

## Run repros

```bash
unset CARGO_TARGET_DIR && cargo test --release --test all -- \
  regression_hashmap_i64 regression_loop_reused regression_andstring \
  regression_strings_split regression_strings_starts_with \
  bug_loop_reused_binding_borrow bug_cross_module_vec_borrow \
  test_library_multipass_hashmap_i64 test_library_multipass_strings_split \
  test_library_multipass_loop_reused \
  test_library_multipass_graph_bfs_hashmap \
  test_library_multipass_csv_for_in_line \
  test_library_multipass_csv_while_index \
  test_library_multipass_for_in_vertices \
  cross_crate_associated_new_bare_literal \
  trim_end_matches_string_literal_must_borrow \
  find_owned_string_literal_must_borrow \
  struct_destructure_mut_field \
  dogfood_store_has_key_forward_ref dogfood_lsm_store_apply_patch \
  dogfood_wal_ffi_snapshot path_bytes_ffi_vec \
  -- --test-threads=1
```

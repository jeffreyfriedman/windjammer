# Compiler repro queue (dogfooding — do not work around in application code)

Cross-crate / multipass dogfooding surfaced these codegen gaps. Each row has a
**codegen-shape** repro (emitted Rust assert) and/or a runtime fixture. Prefer the
codegen gates as source of truth; fixtures alone can pass while multipass still
mis-emits.

**Verified green on tip** (`cargo test --release --test all` filter below,
2026-08-04): **18/18** including multipass + `bug_*` shape tests.

| Priority | Bug | Repro test(s) | Status |
|----------|-----|---------------|--------|
| P0 | **`HashMap::contains_key/insert` — call-return / loop-local i64 in multipass** | `test_library_multipass_graph_bfs_hashmap_compiles`, `test_library_multipass_hashmap_i64_*` | ✅ |
| P0 | Loop reused binding — owned binding in loop must borrow for `&T` callee | `bug_loop_reused_binding_borrow_test`, `test_library_multipass_loop_reused_graph_borrow`, `regression_loop_reused_graph_borrow` | ✅ |
| P0 | **`for v in vertices { f(vertices, v) }` — must borrow `vertices`** | `test_library_multipass_for_in_vertices_reuse_borrow` | ✅ |
| P1 | **`HashMap<i64, f64>::insert(k, 0.0)` — literal must infer f64 not f32** | `test_library_multipass_hashmap_i64_f64_zero_literal_insert` | ✅ |
| P1 | String literal → `string` param must emit `&"lit".to_string()` not owned String | `regression_andstring_literal_call_test.wj` | ✅ |
| P1 | Cross-crate `Type::new("lit")` with owned `String` formal — no WJ sig → bare `&str` | `codegen_cross_crate_associated_new_bare_literal_must_auto_own_gate_test` | ✅ |
| P1 | Cross-crate builder `.method("lit")` with owned `String` formal — no WJ sig → bare `&str` | `cross_crate_builder_bare_literal_must_auto_own` | ✅ |
| P1 | `strings::split(line, "\|")` — pipe delimiter must stay `&str` | `bug_loop_reused_binding_borrow_test` (split gate), `test_library_multipass_strings_split_pipe_delimiter`, `test_library_multipass_csv_for_in_line_string_param`, **`codegen_strings_pattern_must_stay_str_gate_test`** | 🚧 tip still emits `String::from("\|")` / `String::from("acct~")` under `--module-file` dogfood (LedgerKit seed overlays); std stub owned `string` vs runtime `&str` |
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
| P1 | **`ServerResponse::new(200,…)` / `::error(int,…)` → u16** | `codegen_http_status_int_literal_to_u16_gate_test` | ✅ tip GREEN (`--module-file`); dogfood may keep typed ctors |
| P1 | **WDB-101: borrowed map getter call site must auto-`&` owned local** | `wdb101_borrowed_vertex_map_getter_must_auto_borrow_at_call_site` (+ PRE `#[ignore]`) | 🚧 |
| P1 | **WDB-102: `strings.from_chars(chars)` must borrow owned `Vec<char>`** | `wdb102_from_chars_owned_vec_must_borrow_at_call_site` | 🚧 |
| P1 | **WDB-103: owned struct formal must not receive `&arg` (inverse WDB-099)** | `wdb103_owned_host_formal_must_move_not_borrow` | 🚧 |
| P1 | **WDB-104: field-mutating method must emit `mut self`** | `wdb104_field_mutating_method_must_emit_mut_self` | 🚧 |
| P1 | **WDB-099 / WDB-100 PRE dogfood gates** | `wdb099_pre_ir_dogfood_*` (existing), `wdb100_pre_ir_dogfood_*` (`#[ignore]`) | 🚧 PRE |
| P1 | **`std::random.range` → `random::int_range` (ecosystem `wj-uuid` v4)** | `bug_std_random_range_codegen_test` | 🚧 |
| P1 | **`std::crypto.sha1_bytes` for UUID v5** | `bug_std_crypto_sha1_bytes_test` | 🚧 |
| P1 | **`std::time.utc_now()` for UUID v1** | `bug_std_time_utc_now_test` | 🚧 |
| P1 | **`DateTime.timestamp_millis()` for UUID v1** | `bug_std_time_timestamp_millis_test` | 🚧 |
| P1 | **Nested match `for` + `Vec<string>::push` (`wj-fs-walk`)** | `bug_for_loop_vec_string_push_test` | 🚧 |
| P1 | **`json::Value` / `json::keys` / owned `get` (`wj-json-util`)** | `bug_json_value_keys_for_util_test`, `bug_json_get_owned_option_value_test` | 🚧 |
| P1 | **User `join(string,string)` vs `strings.join` name clash (`wj-url`)** | `bug_user_join_name_clash_strings_join_test` | 🚧 |
| P1 | **`Ok((text, ""))` must own empty string** | `bug_ok_tuple_empty_string_literal_test` | 🚧 |
| P1 | **`encoding.base64_encode_string` / `decode_string` (`wj-base64`)** | `bug_std_encoding_base64_string_api_test` | 🚧 |
| P1 | **`HashMap.get("lit")` after Result match (`wj-cookie`)** | `bug_hashmap_get_string_literal_to_string_test` | 🚧 |
| P1 | **`for (k,v) in HashMap` post-loop `drop(map)` (`wj-cookie`)** | `bug_hashmap_for_in_post_loop_drop_test` | 🚧 |
| P1 | **Read-only helper param must borrow, not own (`wj-validate`)** | `bug_readonly_helper_param_must_borrow_test` | 🚧 dogfood: `strings.contains` → `haystack_contains`; `HashMap.get` / `query` formals; `RouteQueryParams.format` → `output_format` (format! clash) |
| P1 | **Owned call-site temp must not become `&` (multi-arm routes)** | product `clone_tenant_slug` + tip `codegen_library_multipass_owned_custom_call_site` | 🚧 95eb4716 helps; `clone_tenant_slug(x) + ""` still needed at some sites |
| P1 | **String concat if/else must unify owned arms (alloc macros)** | `codegen_if_else_string_arms_must_unify_gate_test`, `codegen_string_concat_chain_gate_test`, dogfood `domain/actor.wj` / `string_concat.wj` | 🚧 product uses mut writeback / `concat2`; tip still emits if-arm `&str` vs else `String` |
| P1 | **`strings.substring` int indices → usize (`wj-validate`)** | `bug_substring_int_indices_usize_test` | 🚧 |
| P1 | **Loop reuses read-only `string` param (`wj-glob` filter)** | `bug_loop_reuse_readonly_string_param_test` | 🚧 |
| P1 | **`std::mime` constants + fn wiring (`wj-mime`)** | `bug_std_mime_module_wiring_test` | 🚧 |
| P1 | **Module `const string` return codegen as `&str` (`wj-mime`)** | `bug_module_const_string_return_test` | 🚧 |
| P1 | **Recursive owned `Vec<string>` helper over-borrowed at call site (`wj-yaml`)** | `bug_recursive_owned_vec_call_site_test` | 🚧 |
| P1 | **`Vec` index with Windjammer `int` loop var (`wj-yaml`)** | `bug_vec_int_index_loop_test` (see also `bug_substring_int_indices_usize_test`) | 🚧 |
| P1 | **`vec.len() - int` loop bound usize/i64 (`wj-migrate`)** | `bug_vec_len_minus_int_loop_test` | 🚧 |
| P1 | **Module `const string` returns `&str` not `String` (`wj-mime`)** | `bug_module_const_string_returns_str_test` | 🚧 |

## Application cleanup (after green gates)

1. ✅ **Swap** `graph_vertex_map.wj` Vec backend → HashMap — applied in windjammerdb (2026-08-04); `graph_vertex_map.vec.wj` kept as backup.
2. ✅ **CSV pipe fields** — `lsqb_csv_loader.wj` uses `strings::split(line, "|")` (not `byte_at`/substring). Gate: `test_library_multipass_csv_while_index_owned_string_param` (+ for-in / split multipass). LSQB lib tests green after clean `rm -rf gen && wj build`.
3. ✅ **Harness extract** — `drain_network` uses `match self.network.poll(...)` (WDB-042). Compiler emits in-place `&mut` call; no `let mut net = self.network`.
4. ✅ **Index `consistent()`** — `key_in_range` (no byte-field extract). Network `poll` delegates to `release_held_if_ready`.
5. ✅ **`self.queue.clone()` into owned helpers** — call-arg writeback
   (`let r = f(self.field); self.field = r.sub`) emits `std::mem::take(&mut self.field)`.
   Gate: `codegen_owned_field_call_writeback_gate_test`.

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

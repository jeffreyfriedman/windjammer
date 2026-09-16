# Compiler repro queue (dogfooding — do not work around in application code)

Cross-crate / multipass dogfooding surfaced these codegen gaps. Each row has a
**codegen-shape** repro (emitted Rust assert) and/or a runtime fixture. Prefer the
codegen gates as source of truth; fixtures alone can pass while multipass still
mis-emits.

**Verified green on tip** (`cargo test --release --test all` filter below,
2026-09-13): bare-vs-ref string concat demotion (`a + &b`), flat-sibling
`use super::mod::Type` when user glob blocks `use super::*`, explicit `*copy`
call-site no extra `&`, shadowed owned local → owned callee move, compound
`let mut` typed emission.

**Earlier tip (2026-08-26):** method-index consensus (finance-screens hang), demoted `&str`
clone skip, multi-use owned auto-clone, WDB-108, assert msg var, and
`std::compress` gzip wiring.

## P3.282 — library multipass Step 4B-pre mega-Program OOM (2026-09-15)

| Gate | Status |
|------|--------|
| `library_multipass_infers_cross_file_trait_mut_self_without_merging_all_asts` | ✅ tip GREEN (2026-09-15) — 51-file multipass + cargo check; cross-file `RenderPort` keeps `&mut self` |
| Full `windjammer-game-core` tip `--library` (~669 files) after ownership pass 10 | ✅ **past Step 4B-pre** (2026-09-15 tip) — next failure was P3.291 stack overflow on `executor.wj` codegen |

**Root cause:** After ownership convergence, Step 4B-pre doubled peak RSS by merging every AST into one mega-`Program` solely for `register_traits` + `infer_trait_signatures_from_impls`.

**Fix:** Per-file `register_traits_from_program` + `infer_trait_signatures_from_impls` on a shared `Analyzer` (no mega merge). Cross-file `analyzed_trait_methods` still accumulates.

**Handoff:** Tip library transpile past Step 4B-pre + full codegen (P3.291). Next: engine `cargo check` / `wj game build` (P3.302+ mass rustc).

## P3.291 — mutual-recursion free-fn codegen stack overflow (2026-09-15)

| Gate | Status |
|------|--------|
| `mutual_recursion_free_fns_codegen_must_not_stack_overflow` | ✅ tip GREEN (2026-09-15) — memo + reentrancy guard; ~1.6s test |
| Full `windjammer-game-core` tip `--library` (~669 files) incl. `behavior_tree/executor.wj` | ✅ tip GREEN (2026-09-15) — EXIT=0 ~200s, RAYON_NUM_THREADS=1 |

**Root cause:** `collect_additional_formal_parameter_strings` called `param_should_emit_borrowed_delegation_formal` many times per param; `param_keeps_owned_engine_key_facade` re-entered the same predicate → infinite recursion / stack blowup on BT executor–shaped mutual recursion (owned `Vec` forwards + `match`).

**Fix:** Per-function memo + reentrancy guard on borrow-delegation formal queries (`borrow_delegation_formal_cache` / `borrow_delegation_formal_in_progress` on `CodeGenerator`).

**Verify:**

```bash
export CARGO_TARGET_DIR=/Users/jeffreyfriedman/src/wj/windjammer-game/.cargo-target-wj
cd /Users/jeffreyfriedman/src/wj/windjammer
cargo build --release -p windjammer --bin wj
cargo test --release --test all mutual_recursion_free_fns_codegen_must_not_stack_overflow -- --nocapture
export WJ_COMPILER=/Users/jeffreyfriedman/src/wj/windjammer-game/.cargo-target-wj/release/wj
export RAYON_NUM_THREADS=1
cd /Users/jeffreyfriedman/src/wj/windjammer-game/windjammer-game-core
"$WJ_COMPILER" build src --library -o gen --no-cargo --no-generate-cargo-toml
```


## P3.303 — map `.values()` loop push must clone (2026-09-16)

| Gate | Status |
|------|--------|
| `vec_push_borrowed_loop_elem_must_clone_for_owned_push` | ✅ tip GREEN |
| Product `achievement/manager.rs` `result.push(ach)` | ⏳ re-verify on full engine build |

**Fix:** Treat `.values()`/`.keys()` for-loops as borrowed iterators; clone loop bindings into owned `Vec::push` / owned formals.

## P3.304 — i32 loop increment must not use usize literal (2026-09-16)

| Gate | Status |
|------|--------|
| `i32_compound_add_must_not_use_usize_literal` | ✅ tip GREEN |
| Product `astar_grid.rs` `i += 1 as usize` | ⏳ re-verify (also check `i = i + 1` assign path) |

**Fix:** Strip spurious ` as usize` on integer literals in compound assignment RHS.

## P3.302 — engine library `cargo check` after tip transpile (2026-09-15)

| Gate | Status |
|------|--------|
| `windjammer-game-core` tip `--library` EXIT=0 (~664 files) | ✅ tip GREEN (2026-09-15 cloud verify) |
| `trait_impl_owned_vec_forward_must_match_trait_formal` | ✅ tip GREEN (2026-09-16) — E0053 owned `Vec` impl formal |
| `cargo check -p windjammer_game_core` via `wj game build --release` (breach-protocol) | ⏳ re-verify after P3.303/P3.304 — was **914** rustc errors |

**Sample root cause (E0053):** `RenderPort` trait emits owned formals (`Vec<MaterialData>`) but `impl RenderPort for GameRenderer` emits `&Vec<MaterialData>` when body forwards to borrowing callee — trait impl signature must match trait definition.

**Fix (P3.302 gate):** Early-return trait impl non-self formals from AST (`param.type_`) so thin forwarders do not demote to `&T`.

**Handoff:** Re-run full engine `cargo check`; remaining errors are separate classes (E0308 push `&T`, etc.).

## P3.281 — bare-pass demotion O(registry) hang (2026-09-15)

| Gate | Status |
|------|--------|
| `callee_registry_keys_scales_with_method_index_not_registry_size` (50k noise keys) | ✅ **<1s** — `SignatureRegistry::callee_lookup_keys` via `method_keys_for` / `signatures_matching_suffix` |
| `bug_wdb164_module_file_store_consume_rebind_must_stay_owned_test` | ✅ tip GREEN (~4s) |
| Full `wdb-layers` module-file multipass (~1029 files) | ⚠️ **re-run** with tip `wj` — was ~52min @ 98% CPU in `callee_registry_keys` full-key scan (pre-`1dd24c74`) |

**Root cause:** `promote_callees_from_bare_pass_callers` called `callee_registry_keys`, which filtered **every** registry key per call site.

**Handoff:** Install tip `wj` (`1dd24c74`+); regen gitignored `wdb-layers` `gen/` with `WJ_COMPILER=…/release/wj`. Do not kill foreign dogfood `wj` PIDs — use isolated tip path. Known flaky unit: `bare_pass_skips_pub_vec_u8_owned_api_wdb175` (pre-existing on `7d073e6f`, not introduced by index fix).

| Priority | Bug | Repro test(s) | Status |
|----------|-----|---------------|--------|
| P0 | **Owned path extract call site must not `&String` into owned `string` formal** | `bug_owned_path_extract_must_not_over_borrow_test` | ✅ tip GREEN (2026-09-14) — owned move at call site; guard retained |
| P0 | **Owned `Vec<string>` helper must not receive `&Vec` at call site** | `bug_vec_string_helper_must_not_over_borrow_test` | ✅ tip GREEN (2026-09-14) |
| P0 | **Thin Vec forwarder must not demote to `&Vec` while callee stays owned** | `bug_thin_vec_forwarder_must_not_demote_owned_test` | ✅ tip GREEN (2026-09-14) |
| P0 | **Engine `i32` range literals must not emit `_i64` (`component_viewer_controls`)** | `bug_engine_i32_range_literal_must_not_emit_i64_suffix_test` | ✅ tip GREEN (P3.280b) — cross-module const merge into `module_const_types`; untyped `let cy = 10` defaults i32 when return is non-int |
| P0 | **Owned Copy `i32` formals must not `*x.clone()` at call site** | `bug_engine_i32_formal_must_not_star_deref_clone_test` | ✅ tip GREEN (2026-09-14) — guard retained |
| P0 | **`while idx < vec.len()` int vs usize (expected int, found uint)** | `bug_while_idx_lt_vec_len_must_unify_int_uint_test` | ✅ tip GREEN (P3.265) — tuple `.N` usize only when element is usize; not blanket `.0` |
| P0 | **Nested `concat2`/overlay_row owned formals over-borrowed at call sites** | `bug_string_concat_nested_owned_must_not_over_borrow_test` | ✅ tip GREEN (P3.264) — per-param pub free-fn owned keep (concat lhs / owned forward); read-only pub APIs demote |
| P1 | **Cross-crate `set_if` mut borrow without / with stripped metadata** | `bug_cross_crate_set_if_mut_borrow_test` | ✅ tip GREEN (2026-09-14) — MethodCall mut detect + loop-body MutBorrowed keep |
| P1 | **WDB-087 tuple writeback in nested `while` must not clone** | `test_library_multipass_tuple_writeback_must_not_clone` | ✅ tip GREEN (P3.265) — `current_stmt_restores_binding_after_move` uses full function body in nested blocks |
| P1 | **WAL replay `replay_to_lsn` / path borrow cluster** | `cross_crate_dogfooding_ownership_test::dogfood_wal_replay_*` | ✅ tip GREEN (2026-09-15) — loop prepass marks `.len()`/counter `usize`; `replay_all(path: &str)` demotion |
| P1 | **`temp_path("recover")` cross-crate literal must stay `&str`** | `dogfood_temp_path_string_literal_no_to_string` | ✅ tip GREEN (2026-09-15) — analyzer `format!` args no longer mark param returned/consumed; formal `&str` + call-site identity |
| P1 | **Seed overlay `int_to_string`/`parse_int_string` without empty-concat** | `bug_seed_overlay_int_parse_format_no_plus_empty_test` | ✅ tip GREEN (P3.262) |
| P0 | **Thin trait-impl Draft forwarder must not demote to `&mut Draft` (E0053) / free fn `&Self`** | `bug_trait_owned_draft_forwarder_must_not_demote_mut_test` | ✅ tip GREEN (2026-09-13) |
| P0 | **Full `windjammer-game-core` library rebuild “hang”** — (1) O(files×sigs) global signature copy per file; (2) `scenario_presets.wj` MethodCall type-infer re-walked receivers 3×/link (~3^depth, depth~32). **Fixes:** layered registry + `promote_overlapping_global_signatures_into_local`; reuse `obj_ty_early` in MethodCall inference. | `promote_overlapping_must_not_copy_absent_global_keys`, `consuming_builder_chain_fixture_must_transpile_under_15s` | ✅ tip GREEN (2026-09-13) — deep fixture <1s; presets ~6s iso; full 664-file lib ~15m (`scenario_presets` 2.4s) |
| P0 | **WDB-192: demoted `&RelationalMvccStore` into owned load/put must clone** (job_store claim satellites) | `bug_wdb192_module_file_demoted_store_into_owned_claim_load_put_must_clone_test` | ✅ tip GREEN (2026-09-14) — pub owned Custom API skip bare-pass demotion + owned call-site reconcile; refresh tip-out/gen from `build/` |
| P1 | **WDB-193: owned `frame.clone()` into demoted `&PgWireFrame` must borrow** | `bug_wdb193_module_file_owned_frame_clone_into_demoted_ref_must_borrow_test` | ✅ tip GREEN (2026-09-14) — tip-out/gen sync |
| P1 | **WDB-194: owned `graph.clone()` into demoted `&LsqbTypedGraph` must borrow** | `bug_wdb194_module_file_owned_graph_clone_into_demoted_ref_must_borrow_test` | ✅ tip GREEN (2026-09-14) — tip-out/gen sync |
| P1 | **WDB-195: tip-out bakeoff demoted `&Vec<u64>` into owned median must clone** | `bug_wdb195_module_file_bakeoff_demoted_vec_into_owned_median_must_clone_test` | ✅ tip GREEN (2026-09-14) — tip-out/gen sync |
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
| P1 | **Cross-module match-arm call to multi-use owned `string` formal must move not borrow** | `codegen_cross_module_match_arm_multi_use_owned_formal_gate_test` | ✅ tip IR GREEN |
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
| P1 | **WDB-113: full `src` `--module-file` demotes owned struct to `&mut T` but call sites emit `.clone()`** | `wdb113_full_library_multipass_mut_struct_formal_must_not_clone_owned_at_call_site` | ✅ tip GREEN |
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
| P1 | **Cross-crate owned free fn named `encode` must not borrow arg** | `bug_cross_crate_owned_encode_named_fn_must_not_borrow_arg_test` | ✅ tip GREEN (P3.282) — import alias + qualified registry lookup; no bare `encode` homonym borrow |
| P1 | **Import alias must not steal foreign fn ownership metadata** | `bug_import_alias_must_not_steal_foreign_fn_ownership_test` | ✅ tip GREEN (P3.283) — `import_fn_alias_map` + refresh skips alias homonym challengers |
| P1 | **Owned `Vec<Custom>` filter helper must not demote + clone** | `bug_owned_vec_custom_filter_helper_must_not_demote_and_clone_test` | ✅ tip GREEN (P3.284) — forwarder keeps owned `Vec` when callee preregistered owned |
| P0 | **`std::thread::spawn(\|\| …)` must not wrap closure in `&(move \|\| …)` (E0716/E0525)** | `bug_thread_spawn_closure_must_not_be_ref_test` | ✅ tip GREEN (P3.286) — qualified `thread::spawn` + FnOnce owned peel; no bare `spawn` homonym |
| P1 | **`std::sync::mpsc::sync_channel` missing boundary signature** | `bug_mpsc_sync_channel_boundary_signature_test` | ✅ tip GREEN (P3.287) — `mpsc::sync_channel` aliased from runtime; SyncSender typing is P3.293 |
| P1 | **`mpsc::SyncSender` type for bounded channels (`Sender`≠`SyncSender`)** | `bug_mpsc_sync_sender_type_for_bounded_channel_test` | ✅ tip GREEN (P3.293) — `BoundedIntSender` cargo-checks; `wj-sync` bounded live |
| P1 | **`std::thread::spawn(move \|\| …)` with Arc capture still wraps `&(move \|\|…)`** | `bug_thread_spawn_move_arc_must_not_be_ref_test` | ✅ tip GREEN (P3.294) — parser `move\|\|` closure + FnOnce Identity peel |
| P1 | **`spawn(move \|\|)` must preserve `move` keyword (not emit bare `\|\|`)** | `bug_thread_spawn_move_keyword_must_be_preserved_test` | ✅ tip GREEN (P3.295) — emit `spawn(move \|\| …)`; cargo-check GREEN |
| P1 | **Library multipass strips `spawn(move \|\|)` `move` keyword** | `bug_module_file_spawn_move_keyword_must_be_preserved_test` | ✅ tip GREEN (P3.295) — library `--module-file` preserves `move` |
| P1 | **Library multipass strips `spawn(move \|\|)` when closure starts with `while`** | `bug_module_file_spawn_move_in_worker_loop_must_be_preserved_test` | 🆕 RED / filed (P3.297); blocks wj-sync shared-inbox Pool |
| P1 | **`mut out: Vec<u8>` returned owned must not demote to `&Vec<u8>` (`wj-uuid`)** | `bug_mut_owned_vec_u8_return_must_not_demote_to_ref_test` | 🆕 RED / filed (P3.298); blocks wj-uuid |
| P1 | **`int` find-pos `>= 0` must not emit `as usize >= 0_i64` (`wj-timefmt`)** | `bug_int_find_pos_ge_zero_must_not_mix_usize_i64_test` | 🆕 RED / filed (P3.299); blocks wj-timefmt |
| P1 | **`substring(s, i, i+1)` emits `(i + 1_i32) as usize` (`wj-duration`)** | `bug_substring_end_i_plus_one_must_not_emit_i32_into_usize_test` | 🆕 RED / filed (P3.300); blocks duration/toml/semver/cli-args |
| P1 | **`HashMap::get` through `MutexGuard` must borrow key (not `.to_string()`)** | `bug_hashmap_get_through_mutex_guard_must_borrow_key_test` | ✅ tip GREEN (2026-09-15) — P3.288 restored (`Some`/`Ok` infer + map-key not defeated by owned get homonym)
| P1 | **Library multipass SharedMap get/has still `key.to_string()`** | `bug_module_file_shared_map_get_must_borrow_key_test` | ✅ tip GREEN (2026-09-15 recheck) — was P3.301 RED |
| P1 | **`for x in map.values()` then `vec.push(x)` must clone non-Copy** | `bug_vec_push_borrowed_loop_elem_must_clone_test` | ✅ tip GREEN (2026-09-15) — P3.303 |
| P1 | **`i32` compound `+= 1` must not use `1 as usize`** | `bug_i32_compound_add_must_not_use_usize_literal_test` | ✅ tip GREEN (2026-09-15) — P3.304 |
| P1 | **Cross-crate owned handle loop reassign emits `&mut` (`send_int`/`bump`)** | `bug_cross_crate_owned_handle_loop_reassign_must_not_emit_mut_ref_test` | ✅ tip GREEN (P3.290) — owned metadata + move at cross-crate call; no loop-reassign `&mut` |
| P1 | **`wj-mime` thin-wrap `from_path` emits `path.clone()` on `impl Into<String>`** | `bug_mime_from_path_thin_wrap_must_not_clone_into_string_test` | ✅ tip GREEN (P3.291) — `impl Into<String>` forwards move via `.into()`; no `path.clone()` |
| P1 | **Hexagonal multipass: `method_label` → demoted `method: &str` must auto-borrow** | `bug_multipass_http_hexagonal_method_label_into_demoted_str_must_auto_borrow_test` | ✅ tip GREEN; ⚠️ cargo-bin 0.50.0 product residual (notes/auth use `handle_http`) |
| P1 | **Owned `HashMap` `.get` helper must not inject mid-match defer-drop spawn** | `bug_hashmap_owned_get_helper_must_not_inject_mid_match_defer_drop_test` | ✅ tip GREEN (P3.267) — defer-drop at fn scope; skip when body has `match` + `.get(` |
| P1 | **Module-file string lit → demoted `&str` method formal must not `.to_string()` (`wj-auth-api`)** | `bug_module_file_string_lit_into_demoted_str_must_not_emit_to_string_test` | ⚠️ tip fixture may keep owned `String` (no false RED); product auth demoted + `.to_string()` (P3.259) |
| P1 | **Cross-crate module `touch_grid(grid)` must reborrow `&mut Grid`, not `grid.clone()`** | `bug_cross_crate_mut_borrow_module_fn_test` | ✅ tip GREEN (P3.274) — `sig_arg_confirms_owned_emission` must not strip `&mut T` |
| P1 | **HashMap::get binding → demoted `&str` formal must not `.clone()` (`wj-auth-api` config)** | `bug_hashmap_get_binding_into_demoted_str_must_not_clone_test` | ✅ tip GREEN (P3.274) |
| P1 | **Cross-crate lib `from_toml(text)` → owned `parse(String)` without metadata guesses `&` (`wj-config`)** | `bug_cross_crate_lib_demoted_str_into_owned_parse_test` | ✅ tip GREEN w/ metadata (P3.262); tip auth still RED on Owned→`&` |
| P0 | **`i64` shift/mask inside `Vec<u8>::push` must not emit `_u8` (`wj-uuid`)** | `bug_i64_bitand_hex_mask_must_not_emit_u8_test` | ✅ tip GREEN — cast clears call-arg int context |
| P1 | **Demoted `&str` after `starts_with` → owned formal (`wj-toml`)** | `bug_demoted_str_after_starts_with_must_auto_own_test` | ✅ tip GREEN (2026-09-15) — `plain_string_owned_consumer` + returned-param formal keep; demoted caller auto-`to_string()` |
| P1 | **Single-use owned local → owned `string` formal must move (`wj-toml` get)** | `bug_single_use_owned_local_into_owned_string_formal_must_move_test` | ✅ tip GREEN (P3.254) — bare free-fn not Map::get key-borrow |







## P3.301 (2026-09-15) — multipass SharedMap get/has re-emits key.to_string()

| Change | Status |
|--------|--------|
| Ecosystem: `wj-sync` SharedMap get/has | ⏸ parked (product `wj test` E0308) |
| Gate `bug_hashmap_get_through_mutex_guard_must_borrow_key_test` | ✅ tip GREEN via isolate `compile_single` |
| Gate `bug_module_file_shared_map_get_must_borrow_key_test` | ❌ tip RED (2026-09-15) — `--library --module-file` emits `key.to_string()` |

**Compiler agent:** library multipass must keep MutexGuard map-key borrow (same as isolate P3.288) — do not reintroduce `key.to_string()` when insert/len live in the same module.

## P3.300 (2026-09-15) — substring end `i+1` emits `1_i32` into usize cast

| Change | Status |
|--------|--------|
| Ecosystem: `wj-duration` `parse_ms` digit scan | ⏸ tip RED |
| Also hits | `wj-toml`, `wj-semver`, `wj-cli-args`, `wj-compress`, `wj-glob` (same `i + 1_i32` pattern) |
| Gate `bug_substring_end_i_plus_one_must_not_emit_i32_into_usize_test` | ❌ tip RED (2026-09-15) |
| Note | Shallower `int_increment` / haystack gates can GREEN; scanner `while` + `substring(…, i, i+1)` is the product shape |

**Compiler agent:** when casting substring indices to `usize`, keep `i + 1` in one integer width — emit `(i + 1) as usize` (or both ends as i64), never `(i + 1_i32) as usize` when `i` is i64/`int`.

## P3.299 (2026-09-15) — int find-pos `>= 0` mixes usize cast with i64 zero

| Change | Status |
|--------|--------|
| Ecosystem: `wj-timefmt` `split_time_tz` / `plus_pos >= 0` | ⏸ tip RED |
| Gate `bug_int_find_pos_ge_zero_must_not_mix_usize_i64_test` | ❌ tip RED (2026-09-15) |
| Note | Also related int/usize loop arithmetic in same package |

**Compiler agent:** Windjammer `int` comparisons against `0` must stay in one integer width — do not cast LHS to `usize` while keeping `0_i64`.

## P3.298 (2026-09-15) — mut owned `Vec<u8>` return demoted to `&Vec<u8>`

| Change | Status |
|--------|--------|
| Ecosystem: `wj-uuid` `append_bytes` / v5 path | ⏸ tip RED |
| Gate `bug_mut_owned_vec_u8_return_must_not_demote_to_ref_test` | ❌ tip RED (2026-09-15) — emits `out: &Vec<u8>` then returns `out` |

**Compiler agent:** `mut out: Vec<u8>` that is mutated and returned must keep owned formal (Identity) — do not demote to `&Vec<u8>` when the value is moved out.

## P3.297 (2026-09-15) — `spawn(move ||)` strips `move` in Arc-clone worker loop

| Change | Status |
|--------|--------|
| Ecosystem: `wj-sync` shared-inbox Pool | ⏸ tip RED — Pool `while` inside `spawn(move \|\|)` emits bare `spawn(\|\| …)` → E0373 |
| Gate `bug_module_file_spawn_move_in_worker_loop_must_be_preserved_test` | ❌ tip RED (2026-09-15 sharpened) — exact Pool shape; simple Arc spawn stays GREEN (P3.295) |
| Note | Shallow `inbox.clone()` + outer while was tip-GREEN; closure body starting with `while` strips `move` |

**Compiler agent:** multipass must preserve `move` on closures whose body starts with `while` / after Arc clone rebinds — same emit as simple P3.295 case.

## P3.295 (2026-09-15) — `spawn(move ||)` strips `move` in library multipass

| Change | Status |
|--------|--------|
| Ecosystem: simple Arc spawn | ✅ unblocked |
| Gate `bug_thread_spawn_move_keyword_must_be_preserved_test` | ✅ tip GREEN (2026-09-15) |
| Gate `bug_module_file_spawn_move_keyword_must_be_preserved_test` | ✅ tip GREEN (simple case) |
| Residual | Worker-loop shape still RED → **P3.297** |

**Fix layer:** library multipass must emit `spawn(move \|\| …)` by value — do not drop `move` after ownership passes.

## P3.294 (2026-09-15) — Pool shared-inbox `spawn(move ||)` with Arc still by-ref

| Change | Status |
|--------|--------|
| Ecosystem: `wj-sync` Pool shared `Arc<Mutex<Receiver>>` workers | ⏸ residual P3.297 (`while` inside closure strips `move`) |
| Gate `bug_thread_spawn_move_arc_must_not_be_ref_test` | ✅ tip GREEN (2026-09-15) |
| Workaround | Per-job `spawn(\|\| …)` no longer required for shared inbox |

**Root cause layers:** **parser** — `move \|\|` was parsed as `move || …` (logical OR), not `Expression::Closure`; **IR/codegen** — qualified `thread::spawn` + FnOnce/Closure Identity (no `&(move \|\| …)`).

## P3.293 (2026-09-14) — wj-sync bounded channel SyncSender typing

| Change | Status |
|--------|--------|
| Ecosystem: `bounded_int` via `mpsc::sync_channel` | ⏸ tip emits SyncSender; cannot store in `Sender`-shaped struct |
| Gate `bug_mpsc_sync_sender_type_for_bounded_channel_test` | ✅ tip GREEN (2026-09-14) |
| Note | P3.287 missing-signature may be partially fixed on tip |

**Compiler agent:** expose `mpsc::SyncSender` as a usable WJ type (field + send) so ecosystem can ship Go-style bounded channels without casting to `Sender`.

## P3.291 (2026-09-15) — wj-mime thin-wrap from_path Into<String> clone

| Change | Status |
|--------|--------|
| Ecosystem: `wj-mime` `from_path` / `from_extension` thin-wraps | ✅ tip GREEN — `mime::from_path(path.into())` (no `path.clone()`) |
| Gate `bug_mime_from_path_thin_wrap_must_not_clone_into_string_test` | ✅ tip GREEN (2026-09-15) |

**Fix layer:** IR call-site coercion — `into_string_formal_params` force Identity + `.into()` move; skip auto-clone / stale shared-ref reborrow on thin-wrap forwards.

## P3.290 (2026-09-14) — wj-pipeline cross-crate owned handle loop reassign

| Change | Status |
|--------|--------|
| Ecosystem: `wj-pipeline` `run_int_pipeline` fan-out `tx = send_int(tx, i)` | ✅ unblocked on tip |
| Gate `bug_cross_crate_owned_handle_loop_reassign_must_not_emit_mut_ref_test` | ✅ tip GREEN (2026-09-15) |
| Note | Metadata already says `param_ownership: Owned` for `send_int` / `counter_inc` |

**Fix layer:** prior tip work on cross-crate owned call sites (signature/emitted ownership); loop-reassign no longer prefixes `&mut` on owned handles.

## P3.288 (2026-09-14) — wj-sync SharedMap / HashMap::get through MutexGuard

| Change | Status |
|--------|--------|
| Ecosystem: `wj-sync` `SharedMap` insert/len; get/has parked | ⏸ product multipass residual → P3.301 |
| Gate `bug_hashmap_get_through_mutex_guard_must_borrow_key_test` | ✅ tip GREEN isolate `compile_single`; multipass product still RED → P3.301 |
| Note | Owned `HashMap.get(key)` (no mutex) already cargo-checks |

**Root cause layer:** constraint/type — `Some(x)`/`Ok(x)` infer `Option`/`Result` for match bindings; signature — owned `get` homonym/`bare_formal` must not defeat map-key consensus for `g.data.get`.
**What became unnecessary:** blanket non-map early-return before MapCell consensus; owned-formal escape that blocked poisoned `HashMap::get` / unknown-receiver field gets.

## P3.287 (2026-09-14) — wj-sync bounded channel / `mpsc::sync_channel` signature

| Change | Status |
|--------|--------|
| Ecosystem: `wj-sync` `bounded_int` via `mpsc::sync_channel` | ⏸ SyncSender typing (P3.293) |
| Gate `bug_mpsc_sync_channel_boundary_signature_test` | ✅ tip GREEN — no missing-boundary `compile_error!`; cargo-check |

**Root cause layer:** signature — `register_rust_std_boundary_signatures` aliases `mpsc::sync_channel` → runtime `sync::sync_channel`.

## P3.286 (2026-09-14) — wj-sync `parallel` / `std::thread::spawn` closure by-ref

| Change | Status |
|--------|--------|
| Ecosystem: `wj-sync` `parallel_add` / `PendingInt` via `std::thread::spawn(\|\| …)` + channel | ✅ unblocked on tip |
| Gate `bug_thread_spawn_closure_must_not_be_ref_test` | ✅ tip GREEN (2026-09-14) |

**Root cause layer:** signature + coercion — Owned `FnOnce` boundary; `compute_coercion` / enforce never Borrow closures; `qualified_callee_skips_bare_homonym_lookup` only for `std::…`, runtime-std modules, and explicit `thread::spawn` (not every user `helper::fn`).
**What became unnecessary:** callee-name `matches!(… "thread::spawn")` hardcodes; map-consensus homonym for `remove`/`Vec` (use `suffix_has_conflicting_first_arg_ownership` + receiver-specific sig).
**Regression (2026-09-15):** P3.286 broadened bare-homonym skip to all lowercase-root paths → ~162 suite RED (cross-module Vec helpers, import aliases, copy-type args). Tip fix restores runtime-std/`std::` scope + map-key conflict guard for `Vec::remove` vs `HashMap::remove`.
**Gates:** `cargo test --release --test all -- thread_spawn_closure_must_not_be_ref mpsc_sync_channel_boundary_signature hashmap_get_through_mutex_guard_must_borrow_key` → 8 passed.

## P3.259 (2026-09-12) — wj-auth-api UUID v7 + string-lit demotion dogfood

| Change | Status |
|--------|--------|
| Ecosystem: register → UUID v7 id; JWT `sub`=id; `/me` returns `sub` | ✅ **12/12** on wj 0.50.0 |
| Adapter uses `handle_http(HttpMethod)` (same-crate); tests keep `handle(string)` | ✅ dual-runtime `HttpMethod` mismatch in test crate |
| Product string-label path: demoted `method: &str` + `"GET".to_string()` | ❌ observed on tip multipass (avoided via `handle_http`) |
| Gate `bug_module_file_string_lit_into_demoted_str_must_not_emit_to_string_test` | ⚠️ fixture may keep owned `String` (no false RED); panics if demoted+`.to_string()` |
| Gate `bug_owned_helper_into_demoted_str_formal_must_auto_borrow_test` | ✅ tip GREEN |

**Compiler agent:** when multipass demotes impl `method: string` → `&str`, call-site string lits must stay bare (WDB-168 / `.to_string()` twin). Strengthen fixture until it demotes like product.

## P3.275 (2026-09-13) — wj-notes-api validate + mime dogfood

| Change | Status |
|--------|--------|
| Ecosystem: `wj-validate` title/body + `wj-mime` `Content-Type` on notes | ✅ **29/29** on cargo-bin `wj` 0.50.0 |
| Adapter uses `handle_http(HttpMethod)` (same as auth) | ✅ cargo-bin still E0308 on `method_label` → demoted `method: &str` |
| Gate `bug_owned_helper_into_demoted_str_formal_must_auto_borrow_test` | ✅ tip GREEN |
| Gate `bug_multipass_http_hexagonal_method_label_into_demoted_str_must_auto_borrow_test` | ✅ tip GREEN; ⚠️ cargo-bin 0.50.0 product residual |

**Compiler agent:** cargo-bin 0.50.0 product hexagonal builds still E0308 on `method_label` → demoted `&str`; tip multipass gate is GREEN — verify pin/backport for ecosystem dogfood on cargo-bin.

## P3.278 (2026-09-13) — wj-notes-api dotenv + mid-match HashMap defer-drop

| Change | Status |
|--------|--------|
| Ecosystem: `config_from_dotenv` via `wj-dotenv` + `wj-config::merge` | ✅ **32/32** on cargo-bin `wj` 0.50.0 |
| Single-map `notes_config_from_map` (auth shape) | ✅ avoids owned `.get` helper |
| Gate `bug_hashmap_owned_get_helper_must_not_inject_mid_match_defer_drop_test` | ✅ tip GREEN (P3.267) — fn-scope defer-drop tail; skip `match` + `.get(` bodies |

**Compiler agent:** defer-drop after owned `HashMap` param must not splice into an open `match map.get(…)` (breaks rustc parse / E0382).

## P3.280 (2026-09-14) — wj-notes-api sha ETag + cargo-bin owned→demoted `&str`

| Change | Status |
|--------|--------|
| Ecosystem: `wj-sha` ETag on `GET /notes/:id` + `If-None-Match` → 304 | ✅ **56/56** on cargo-bin `wj` 0.50.0 |
| Gate `bug_demoted_str_formal_owned_local_auto_borrow_test` | ✅ tip GREEN |
| cargo-bin 0.50.0 product residual | ⚠️ `note_get_reply(note, if_none_match)` E0308 expected `&str`, found `String` when formal demotes; notes pushes match string into `Vec` so formal stays owned |

**Compiler agent:** backport tip auto-borrow for owned locals into demoted `&str` formals so cargo-bin hexagonal apps do not need `Vec` hold workarounds.

## P3.282 (2026-09-14) — wj-notes-api base64 + cross-crate `encode` over-borrow

| Change | Status |
|--------|--------|
| Ecosystem: `wj-base64` `GET /notes/:id?encoding=base64` | ✅ **58/58** on cargo-bin `wj` 0.50.0 |
| Package adds `encode_text` / `decode_text` aliases | ✅ unblocks call sites |
| Gate `bug_cross_crate_owned_encode_named_fn_must_not_borrow_arg_test` | ✅ tip GREEN (P3.282) |

**Compiler agent:** ownership/coercion for cross-crate free fns must follow signature registry — do not special-case method/fn name `encode` into a borrow.

## P3.283 (2026-09-14) — wj-notes-api url Location + import-alias ownership steal

| Change | Status |
|--------|--------|
| Ecosystem: `wj-url` `join_url` → `Location` on `POST /notes` + `public_base_url` config | ✅ **60/60** on cargo-bin `wj` 0.50.0 |
| Gate `bug_import_alias_must_not_steal_foreign_fn_ownership_test` | ✅ tip GREEN (P3.283) |
| Product workaround | ✅ alias as `qs_get` (not `query_get`) |

**Compiler agent:** resolve call-site ownership by the *imported* function identity (crate + original name), not by the local alias string colliding with another crate's free-fn metadata.

## P3.268 (2026-09-14) — u64/len, text→usize guard, defer-drop tail, make_view payload

| Change | Status |
|--------|--------|
| Gates `wdb215` / `hashmap_string_key_insert` / `wdb214`–`wdb217` / `vec_custom_view_helper` / `mut_param_passthrough` / `hashmap_owned_get_helper` | ✅ tip GREEN — signature-driven borrow/clone/len casts; no mid-match defer-drop |
| Gate `bug_explicit_type_import_must_not_duplicate_prelude_test` | ✅ tip GREEN — `make_view` keeps owned `String` when body stores `name + ""` in struct field |
| Fix | `unsigned_int_width_for_len_cast` (`u64` vs `.len()`); `expression_must_not_usize_coerce` for text/concat; defer-drop fn tail depth; `param + ""` in struct field = owned payload |

**Compiler agent:** never cast string keys/concat to `usize` for non-`usize` formals; compare `u64` indices to `(len as u64)` not `len as i64`.

## P3.267 (2026-09-14) — wj-url homonym + hexagonal import gate

| Change | Status |
|--------|--------|
| Gate `bug_owned_string_local_call_site_test` | ✅ tip GREEN — preregistered owned `String` formals peel stale `&` after IR reconcile |
| Gate `bug_user_join_name_clash_strings_join_test` | ✅ tip GREEN — local user `join` beats `strings::join` borrow baseline |
| Gate `bug_explicit_type_import_must_not_duplicate_prelude_test` | ✅ tip GREEN — `cargo check` on hexagonal ItemView fixture |
| Gate `bug_hashmap_owned_get_helper_must_not_inject_mid_match_defer_drop_test` | ✅ tip GREEN |

**Compiler agent:** same-file preregistered emission strings are authoritative for bare free-fn call sites; do not inherit runtime-std homonym `&str` when the defining module emitted owned `String` formals.

## P3.284 (2026-09-14) — wj-notes-api regex `?q=` + Vec\<Custom\> helper demote/clone

| Change | Status |
|--------|--------|
| Ecosystem: `wj-regex` `GET /notes?q=` title/body filter | ✅ **62/62** on cargo-bin `wj` 0.50.0 |
| Gate `bug_owned_vec_custom_filter_helper_must_not_demote_and_clone_test` | ✅ tip GREEN (P3.284) — `apply` forwarder keeps `Vec<Note>`; build fixture at `src/` root for `cargo check` |
| Product workaround | ✅ inline filter loop in `list_notes_for_query` |

**Compiler agent:** owned `Vec<T>` filter helpers that push/consume elements must keep Owned formals (or call sites must borrow consistently — never `clone()` into `&Vec`).

## P3.267 (2026-09-14) — tester path + tip owned-string / len residuals

| Change | Status |
|--------|--------|
| Gate `bug_hashmap_string_key_insert_must_not_cast_usize_test` | ✅ tip GREEN — drop typed `query_with*` interim |
| Gate `bug_trait_owned_string_call_must_not_over_borrow_test` | ✅ tip GREEN (isolate); ⚠️ product multipass still needs bound locals / `repo.get` |
| Gate `bug_strings_len_must_unify_int_index_arith_test` | ✅ tip GREEN (isolate); ⚠️ product still uses `strings.len() as int` + while bound |
| Gate `bug_int_arith_must_not_split_i64_i32_test` | ✅ tip GREEN (2026-09-14) — bare `Literal::Int` infers WJ `int`; binary prefer-specific skips untyped lit peers (`year % 400` → `_i64`) |
| Gate `bug_int_increment_literal_must_match_lhs_width_test` | ✅ tip GREEN (2026-09-15) — nested untyped counter width unified; isolate no longer emits `+= 1 as i32` |
| Gate `bug_int_while_len_as_int_must_not_emit_usize_arith_test` | ❌ tip RED (2026-09-15) — typed `int` + `len() as int` emits `+= 1 as usize` / `h_len as usize` vs i64 (~113 tip api-check); WAL usize fix did not clear product multipass |
| Product: bound owned locals into trait string formals; `len() as int` before while; typed nested indices | ⚠️ tip HEAD api-check **RED** (~113 usize/i64); dogfood interim still needed until gate GREEN |
| Platform finance-ui: account-rail asserts StatusChip (`wj-account-rail-status` / `data-wj-status`) | ✅ |
| `make client-check` / cargo-bin finance-screens | ✅ GREEN (prior); tip finance-screens regen still elevated |

**Compiler agent:** after keeping trait-impl string formals Owned, call sites must move (not `&format!(…)`) into those formals. `vec.len()` / `strings.len()` in `while i < len` must unify both sides to one integer width without product `as int` binds. Int `%` / compare against decimal literals must keep one integer width (no `i64 % i32`). Strengthen isolate fixtures until they match product multipass RED (trait call / len width / int Rem).

## P3.266 (2026-09-14) — tip owned Vec/string call-site over-borrow + HashMap string key

| Change | Status |
|--------|--------|
| Gate `bug_owned_path_extract_must_not_over_borrow_test` | ✅ tip GREEN (2026-09-14) |
| Gate `bug_vec_string_helper_must_not_over_borrow_test` | ✅ tip GREEN (2026-09-14) |
| Gate `bug_thin_vec_forwarder_must_not_demote_owned_test` | ✅ tip GREEN (2026-09-14) |
| Gate `bug_hashmap_string_key_insert_must_not_cast_usize_test` | ✅ tip GREEN (2026-09-14) — untyped `HashMap::new()` keeps String insert keys |
| Gate `bug_vec_custom_view_helper_must_not_over_borrow_test` | ✅ tip GREEN (isolate); product used `lines.clone()` interim |
| Gate `bug_mut_param_passthrough_no_shared_amp_test` | ✅ tip GREEN (2026-09-14) — `&mut` formal → `&mut` callee reborrow, no `.clone()` |
| Gate `codegen_cross_module_match_arm_multi_use_owned_formal_gate_test::cross_module_match_arm_readonly_concat_demotes_to_str` | ✅ tip GREEN (2026-09-14) — `json + ""` readonly append demotes pub `string` to `&str` |
| Gates `wdb214`–`wdb217` codegen fixtures (`library_multipass/`) | ✅ tip GREEN (2026-09-14) — borrow/clone/u64-len/mut-reborrow |
| Tip-out product gates `wdb214`–`wdb217` (`.agent-wip/rel_tip_out`) | ⚠️ RED until rel_tip_out regen with tip `wj` |
| Gate `bug_engine_i32_range_literal_must_not_emit_i64_suffix_test` | ✅ tip GREEN (2026-09-14) — `module_const_types` + non-usize range loop binding |
| Product interim: typed `HashMap<string,string>` locals; `lines.clone()` into view helpers; append_string_list owned prefix; bearer/post_request inlines; `path + ""` | ✅ tip `make api-check` **GREEN**; typed HashMap interim **dropped** (P3.267) after gate GREEN |

**Compiler agent:** `HashMap<string,string>::insert` must keep `String` keys (no `as usize`) even when the map local lacks an explicit type annotation. Owned `Vec<T>` formals must receive moves at multipass call sites (`*_view_from_row(row, lines)` not `&lines`).

## P3.265 (2026-09-13) — tip E0252 duplicate type imports + bank_recon owned moves

| Change | Status |
|--------|--------|
| Gate `bug_explicit_type_import_must_not_duplicate_prelude_test` | ✅ tip GREEN (P3.267) — import dedupe + hexagonal `make_view(String)` cargo-check |
| Gate `bug_while_idx_lt_vec_len_must_unify_int_uint_test` | ✅ tip GREEN (2026-09-14) — no `(idx as i64) < ….len()` |
| Product interim: strip View/Line/Draft names from brace imports (~38 files) | ✅ tip api-check E0252 **0** |
| Product: bank_recon drop `clone_code` → `code + ""`; seed_bank_import `for` loops | ✅ |
| Tip `make api-check` | **76 → 20 → 0** (P3.266 product interims; verified 2026-09-14) |

**Compiler agent:** when auto-emitting prelude `use crate::…::Type;`, do not also keep `Type` inside the source brace `use crate::…::{…, Type}`. Unify `while idx < vec.len()` to one integer width.

**Fix (2026-09-14):** `usize_expression_type_inference` — numeric tuple indices (`.0`/`.1`) use real element type inference (do not treat every `.0` as usize). IR call-site demoted→owned clone peels `&` before `.clone()` (`&state` → `state.clone()`, not `&state.clone()`).

## P3.264 (2026-09-13) — nested concat2/overlay_row owned formal over-borrow

| Change | Status |
|--------|--------|
| Gate `bug_string_concat_nested_owned_must_not_over_borrow_test` | ✅ tip GREEN (same-file + hexagonal) — verified 2026-09-14 |
| Same-file transpile | ✅ |
| Product LedgerKit `domain/string_concat.wj` | ⏳ re-check tip api-check after pin |
| Tip binary install | use `scripts/atomic_install_wj.sh` — sandbox `CARGO_TARGET_DIR` leaves `target/release/wj` stale |

**Root cause:** multipass bare-pass demotion treated pub owned `string` formals as Borrowed while codegen still emitted `String`, so call sites over-borrowed (`append_overlay_row(&a)`).

**Fix:** skip bare-pass demotion for pub free-fn text formals; clear `str_ref_optimized` when locking public owned formals; terminal IR reconcile moves owned locals into owned text formals (clone only on reuse).

## P3.263 (2026-09-13) — tip api-check: owned Draft trait forwarder + product unblock

| Change | Status |
|--------|--------|
| Gate `bug_trait_owned_draft_forwarder_must_not_demote_mut_test` | ✅ tip GREEN (minimal + hexagonal) |
| Product interim: `env_channel` / `env_inventory` duplicate seed bodies (not thin forward) | ✅ clears E0053/`&Self` on prior tip |
| Product: authenticate Option `.clone()`, login owned locals, request_context → domain `int_string`, http_json named empty Vecs, bank_recon / routes / party create_item | ✅ prior tip 20-error cluster cleared |
| Full tip `make api-check` after tip binary rebuild (~18:33) | ❌ **58** new-class errors (`string_concat` int/uint, HashMap insert, overlay args) — tip churn |

**Compiler agent:** thin `impl Trait` forwarders that only pass `draft` must keep the trait’s owned formal (not `&mut Draft`). Free composition fns taking `AppDeps` must not emit `deps: &Self`. Tip mid-slice rebuild introduced unrelated `string_concat` / HashMap ownership regressions — fix tip or pin wj for platform gate.

## P3.262 (2026-09-13) — wj-config demoted `&str` into cross-crate owned `parse`

| Change | Status |
|--------|--------|
| Gate `bug_cross_crate_lib_demoted_str_into_owned_parse_test` | ✅ tip GREEN (with `--metadata`; 2026-09-13) |
| Product without dep `metadata.json` | ⚠️ RED (`parse(&text)` guess) — build packages with `--library --module-file` |
| Tip auth/notes dogfood (2026-09-13 tip binary) | ❌ RED — owned cross-crate formals still get `&` (e.g. `preflight(&origin)`) even when metadata says Owned |
| Ecosystem verify | ✅ **20/20** auth on cargo-bin `wj` 0.50.0 (2026-08-27) |

**Compiler agent:** (1) default library builds must emit `metadata.json`; (2) honor `param_ownership: Owned` at cross-crate call sites (no `&` into `String`); tip regression broke previously green notes/auth.


## P3.262 (2026-09-13) — seed overlay int parse/format dogfood

| Change | Status |
|--------|--------|
| Gate `bug_seed_overlay_int_parse_format_no_plus_empty_test` | ✅ tip GREEN (same-file + hexagonal) |
| Platform `domain/int_string` DRY + recon/signoff overlay dogfood | ✅ isolated tip `wj test` **5/5** |
| Full platform `make test` / `api-check` | ⚠️ tip still has pre-existing ~42 E0308/ownership cluster (not introduced by this slice) |

**Compiler agent:** keep int↔string format/parse without empty-concat; product recon overlays use domain helper.

## P3.274 (2026-09-13) — cross-crate `&mut T` metadata + HashMap get demoted `&str`

| Change | Status |
|--------|--------|
| Gate `bug_cross_crate_mut_borrow_module_fn_test` (engine metadata `MutBorrowed` → call site reborrow) | ✅ tip GREEN |
| Gate `bug_hashmap_get_binding_into_demoted_str_must_not_clone_test` | ✅ tip GREEN |
| Fix | `sig_arg_confirms_owned_emission`: `Reference`/`MutableReference` formals are never owned contracts |

**Compiler agent:** cross-crate `touch_grid(grid: &mut Grid)` must pass `grid` (reborrow), not `grid.clone()`. Demoted `&str` formals must borrow HashMap get bindings (no `.clone()`).

## P3.261 (2026-09-13) — HashMap get binding into demoted `&str` + `.clone()` (superseded by P3.274)

| Change | Status |
|--------|--------|
| Ecosystem `wj-auth-api` `config_from_toml` via `wj-config`/`wj-toml` | ✅ **15/15** |
| Gate `bug_hashmap_get_binding_into_demoted_str_must_not_clone_test` | ✅ tip GREEN (P3.274) |

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

## P3.279 WindjammerDB CQ-C5 — coverage REDs WDB-200–205 u32/Key/Multicol/sql/usize/inbound (2026-09-14)

| Gate | Status |
|------|--------|
| Fresh `cargo check --lib` | ⚠️ **~123** (↓ after module_file + df_provider tip→gen sync; re-census after docs) |
| Tip **WDB-200** u32 counter `1_i64` (df_provider) | ✅ **GREEN** after tip-out→gen sync |
| Tip **WDB-201** `&mut Key` → owned `entries.push` | ❌ RED — tip-out/gen `push((key, …))` |
| Tip **WDB-202** `&mut MulticolState` → owned return | ✅ **GREEN** — tip-out/gen owned `state:` (no demoted `&mut`) |
| Tip **WDB-203** owned `sql` → demoted `&str` simple_query | ❌ RED — unified bare `sql` |
| Tip **WDB-204** `u64 == 0_usize` (sysbench) | ❌ RED — tip-out/gen |
| Tip **WDB-205** `inbound.clone()` → demoted `&Vec<u8>` | ✅ tip GREEN (no bare owned into demoted decode) |
| Tip-out→gen sync | ✅ multicol serve + prior df_provider/module_file |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** tip greens 201/203/204 (+ open 176/177/191–198). No Phase 606+. No dogfood transforms.

## P3.305 WindjammerDB CQ-C5 — coverage REDs WDB-227–229 u64 width + impl Into push_str (2026-09-15)

| Gate | Status |
|------|--------|
| Fresh `cargo check --lib` | ⚠️ **322** |
| Tip **WDB-214–217 / 215 lsqb** | ✅ GREEN |
| Tip **WDB-177/218–226** | ❌ RED |
| Tip **WDB-227** u64 vs `len() as i64` (LDBC) | ❌ RED — tip-out/gen graph_ldbc_validation_engine |
| Tip **WDB-228** `impl Into<String>` + `push_str(&…)` | ❌ RED — tip-out/gen + isolate codegen lsqb |
| Tip **WDB-229** u64 vs bare `len()` usize | ❌ RED — tip-out/gen graph_sql_query_port |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** tip greens **177/218–229**. Dominant residual: Vec←&Vec / `&str`←String / LsqbTypedGraph / u64 width. No Phase 606+.

## P3.298 WindjammerDB CQ-C5 — coverage REDs WDB-225/226 &str←String + U32Map (2026-09-15)

| Gate | Status |
|------|--------|
| Fresh `cargo check --lib` | ⚠️ **322** |
| Tip **WDB-214–217** | ✅ GREEN |
| Tip **WDB-177/218–224** | ❌ RED |
| Tip **WDB-225** `String::from` lit → demoted `&str` join_path | ❌ RED — tip-out/gen ldbc (~46× &str←String) |
| Tip **WDB-226** owned `GraphVertexU32Map.clone()`→demoted `&Map` | ❌ RED — tip-out/gen cdlp (~6×) |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** tip greens **177/218–226**. Dominant residual: Vec←&Vec / `&str`←String / LsqbTypedGraph. No Phase 606+.

## P3.297 WindjammerDB CQ-C5 — coverage REDs WDB-223/224 F64Map + Vec←&Vec (2026-09-15)

| Gate | Status |
|------|--------|
| Fresh `cargo check --lib` | ⚠️ **322** |
| Tip **WDB-214–217** | ✅ GREEN |
| Tip **WDB-177/218–222** | ❌ RED |
| Tip **WDB-223** owned `GraphVertexF64Map.clone()`→demoted `&Map` | ❌ RED — tip-out/gen pagerank (~10×) |
| Tip **WDB-224** demoted `&Vec<T>`→owned `Vec<T>` (struct elem) | ❌ RED — tip-out/gen dremel (~54×) |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** tip greens **177/218–224**. Dominant residual: Vec←&Vec / `&str`←String / LsqbTypedGraph. No Phase 606+.

## P3.296 WindjammerDB CQ-C5 — coverage RED WDB-222 GraphVertexI64Map (2026-09-15)

| Gate | Status |
|------|--------|
| Fresh `cargo check --lib` | ⚠️ **322** |
| Tip **WDB-214–217** | ✅ GREEN |
| Tip **WDB-177/218–221** | ❌ RED |
| Tip **WDB-222** owned `GraphVertexI64Map.clone()`→demoted `&Map` | ❌ RED — tip-out/gen bfs (~18×) |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** tip greens **177/218–222**. Dominant residual: `&str`←String / String←&str / LsqbTypedGraph. No Phase 606+.

## P3.292 WindjammerDB CQ-C5 — tip→gen string-lit sync + WDB-220/221 (2026-09-14)

| Gate | Status |
|------|--------|
| Fresh `cargo check --lib` | ⚠️ **447** (↓533 after semantic/append+column + graph batch tip→gen) |
| Tip **WDB-214–217** | ✅ GREEN |
| Tip **WDB-177/218/219** | ❌ RED |
| Tip **WDB-220** `&mut LsqbTypedGraph`→owned neighbors | ❌ RED — tip-out/gen lsqb (~37×) |
| Tip **WDB-221** string lit→owned `edge_kind: String` | ❌ RED — tip-out/gen lsqb (~298× String←&str) |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** tip greens **177/218–221**. Dominant residual: String←&str. No Phase 606+.

## P3.289 WindjammerDB CQ-C5 — tip→gen 214–217 + coverage REDs WDB-218/219 (2026-09-14)

| Gate | Status |
|------|--------|
| Fresh `cargo check --lib` | ⚠️ **520** |
| Tip **WDB-214–217** | ✅ **GREEN** — tip→gen sync (pg_wire / lsqb / simd / pagerank); hardware `append_i64` lit borrow synced |
| Tip **WDB-209/212** | ✅ GREEN |
| Tip **WDB-177** | ❌ RED — bare `key` into owned `query_feedback_cache_put` (fresh tip still RED) |
| Tip **WDB-218** owned `GraphSqlQueryPlan`→demoted `&Plan` | ❌ RED — tip-out/gen graph_sql |
| Tip **WDB-219** `&mut DenseCsr`→owned csr | ❌ RED — tip-out/gen graph_sql (inverse WDB-217) |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** tip greens **177/218/219**. Dominant residual: `&str`←String (~284). No Phase 606+.

## P3.285 WindjammerDB CQ-C5 — coverage REDs WDB-214–217 String/lsqb/simd/DenseCsr (2026-09-14)

| Gate | Status |
|------|--------|
| Fresh `cargo check --lib` (last) | ⚠️ **523** |
| Tip recheck prior open set (earlier) | ✅ **24 GREEN** / residual **177** (+ product 209/212 later greened on tip) |
| Tip **WDB-214** owned String→demoted `&str` `push_cstring` | ✅ GREEN after tip→gen (was RED tip-out lag) |
| Tip **WDB-215** `u64` index vs `len() as i64` (lsqb) | ✅ GREEN after tip→gen |
| Tip **WDB-216** demoted `&Vec`→owned SIMD FFI | ✅ GREEN after tip→gen (owned formals) |
| Tip **WDB-217** `csr.clone()`→`&mut DenseCsr` | ✅ GREEN after tip→gen |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** tip greens **177/214–217**. Dominant residual: `&str`←String. No Phase 606+.

## P3.281 WindjammerDB CQ-C5 — coverage REDs WDB-209–213 binder/Session/Fill/Timeseries/OptEcon (2026-09-14)

| Gate | Status |
|------|--------|
| Fresh `cargo check --lib` | ⚠️ **~523** (tip-out regen churn; was ~125 earlier this session — binder/pg_wire tip regression) |
| Tip **WDB-209** `catalog_push_column` `&mut CatalogColumnBinding` | ✅ **GREEN** — multipass + Vec::push registry; regen gen/tip-out |
| Tip **WDB-210** `&mut Wave1Sf1Session`→owned clock | ✅ **GREEN** — tip uses `sess.clone()`; tip→gen sync |
| Tip **WDB-211** `fill_bundle_from_six` `&mut OptOperatorFill` | ✅ **GREEN** after tip→gen (+ module_file) sync |
| Tip **WDB-212** owned Timeseries batch→demoted `&` | ✅ **GREEN** — `sig_arg_confirms_owned_emission` respects emission; multipass borrow gate |
| Tip **WDB-213** demoted/`&mut` OptEconLedger→owned | ✅ **GREEN** after tip tpch owned formal sync |
| Prior open | 176/177/191–198 + 201/203/204/206/208 |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** tip greens **209/212** (binder mut demotion + Timeseries borrow) + prior open REDs. Tip churn flipped binder owned→`&mut`. No Phase 606+.

## P3.280 WindjammerDB CQ-C5 — coverage REDs WDB-206–208 FeedbackKey / Multicol hook / OptDatedBaseline (2026-09-14)

| Gate | Status |
|------|--------|
| Fresh `cargo check --lib` | ⚠️ **125** (↓ from 126 after tip→gen Multicol hook/serve sync) |
| Tip **WDB-206** demoted `&QueryFeedbackKey`→owned | ❌ RED — tip-out/gen row + df_provider |
| Tip **WDB-207** Multicol hook `&mut`+owned sql | ✅ **GREEN** after tip-out→gen (+ module_file) sync |
| Tip **WDB-208** owned OptDatedBaseline→demoted `&` | ✅ GREEN — gate only when baseline formal demotes to `&` (tip keeps owned) |
| Prior open | 176/177/191–198 + 201/203/204 |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** tip greens **206/208** + prior open REDs. No Phase 606+.

## P3.279 WindjammerDB CQ-C5 — coverage REDs WDB-200–205 counters / Key / Multicol / sql / usize / inbound (2026-09-14)

| Gate | Status |
|------|--------|
| Fresh `cargo check --lib` (last) | ⚠️ **~123** after module_file + df_provider tip→gen sync |
| Tip **WDB-200** `u32` / `1_i64` df_provider | ✅ **GREEN** after tip→gen sync (`i += 1`) |
| Tip **WDB-201** `&mut Key` push into owned Key | ❌ RED — `out.entries.push((key, …))` |
| Tip **WDB-202** `&mut MulticolState` return as owned | ✅ **GREEN** — tip-out/gen now owned `state:` formal |
| Tip **WDB-203** owned `sql` → demoted `&str` simple_query | ❌ RED — unified bare `sql` |
| Tip **WDB-204** `u64 == 0_usize` (sysbench) | ❌ RED — tip-out `median == 0_usize` |
| Tip **WDB-205** `inbound.clone()` → demoted `&Vec` decode | ✅ **GREEN** (tip-out/product) |
| Dogfood / tip-cluster | ❄️ frozen (manual tip-out→gen only) |

**Compiler agent priority:** tip greens 176/177/191–198 + **201/203/204**. No Phase 606+.

## P3.277 WindjammerDB CQ-C5 — coverage REDs WDB-196–199 feed String / live OptDated / search f32 / &mut Value (2026-09-13)

| Gate | Status |
|------|--------|
| Fresh `cargo check --lib` | ⚠️ **131** |
| Tip **WDB-176/177/191–195** | ❌ still open (recheck) |
| Tip **WDB-196** feed_unified lit/`&str`→owned String | filed — tip-out encode + unified sql |
| Tip **WDB-197** live_row `&artifact`→owned quiet | filed — tip-out live_port |
| Tip **WDB-198** search_host `distance: 0.1_f32` | filed — tip-out search_host |
| Tip **WDB-199** `&mut Value`→owned secondary_key | ✅ **GREEN** after tip-out→gen sync |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** tip greens 176/177/191–199. No Phase 606+.

## P3.276 WindjammerDB CQ-C5 — tip-out→gen sync + WDB-192–195 claim/frame/lsqb/bakeoff (2026-09-13)

| Gate | Status |
|------|--------|
| Fresh `cargo check --lib` | ⚠️ **131** (↓ from 134 after tip-out→gen sync of fusion/job_store/feed_unified) |
| Tip **WDB-188/189/190** gen-lag gates | ✅ **GREEN** after sync |
| Tip **WDB-176/177/191** | ❌ still open |
| Tip **WDB-192** claim `&Store`→owned load/put | ✅ GREEN — multipass + tip-out/gen sync |
| Tip **WDB-193** `frame.clone()`→demoted `&PgWireFrame` | ✅ GREEN — tip-out/gen sync |
| Tip **WDB-194** `graph.clone()`→demoted `&LsqbTypedGraph` (q4/q7) | ✅ GREEN — tip-out/gen sync |
| Tip **WDB-195** bakeoff `&Vec`→owned median | ✅ GREEN — tip-out/gen sync |
| Dogfood / tip-cluster | ❄️ frozen (manual tip-out→gen copy only) |

**Compiler agent priority:** tip greens 176/177/191/192–195. No Phase 606+.

## P3.275 WindjammerDB CQ-C5 — gen-lag + pg_wire parse REDs WDB-188–191 (2026-09-13)

| Gate | Status |
|------|--------|
| Fresh `cargo check --lib` | ⚠️ **134** (↓ from 165) |
| Tip-out/product **WDB-182/184–187** prior gates | ✅ GREEN (recheck) — tip-out fixed; gen lag for f32/&String/feed_unified |
| Tip **WDB-176/177** | ❌ still tip-out RED |
| Tip **WDB-188** gen `graph_score: 0.0_f32` lag | ❌ filed + ran RED |
| Tip **WDB-189** gen `&String`→owned parse lag | ❌ filed + ran RED |
| Tip **WDB-190** module-file `on_startup` owned `Vec` + no `&encode` | ✅ tip GREEN (P3.275) — registry-owned pub `Vec` formal emission |
| Tip **WDB-190** gen feed_unified borrow→owned startup | ✅ tip GREEN (2026-09-14) — producer-only owned Vec restore |
| Tip **WDB-191** module-file demoted `&str`→owned `wire_parse` | ✅ tip GREEN (P3.271 class) |
| Tip **WDB-191** product gen demoted `&str`→owned `pg_wire_parse` | ✅ tip-out GREEN (2026-09-14) — bare-pass restore owned string payload + pub registry-owned emission |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** tip-out→gen sync (188/189/190 gen); tip greens 176/177. No Phase 606+.

## P3.275b (2026-09-13) — WDB-190 pub `Vec` registry-owned formal emission

| Change | Status |
|--------|--------|
| Gate `bug_wdb190_module_file_feed_unified_owned_startup_must_not_borrow_test` (fixture) | ✅ tip GREEN |
| Fix | `is_public_owned_non_copy_formal_api`: when multipass `restore_pub_owned_non_copy_api_formals` locked registry `Owned` for pub `Vec`, emit owned formal (not readonly-only demotion) |
| Fix (2026-09-14) | `restore_owned_formals_for_producer_only_call_sites`: when every call site passes an owned producer (`encode_startup(…)`) and none bare-pass a binding, keep pub `Vec` Owned so callers do not emit `&encode_startup(…)` |
| Analyzer | Module-qualified free calls parsed as `MethodCall` (`station_builder.set_if`) detect MutBorrowed args like `::` Call form |
| WDB-171 `finish_execute` demoted `&Vec` | ✅ unchanged — registry converged Borrowed from bare-pass callers |
| Gate `bug_cross_crate_set_if_mut_borrow_test` | ✅ tip GREEN |
| Gate `bug_string_concat_nested_owned_must_not_over_borrow_test` | ✅ tip GREEN — readonly early `&str` demotion still skips analyzer-Owned + keeps_owned formals |

## P3.273 WindjammerDB CQ-C5 — WDB-182/183/184 tip greens + tip-out regen (2026-09-13)

| Gate | Status |
|------|--------|
| **WDB-182** fixture + tip-out `graph_score: 0.0_f64` | ✅ tip GREEN — struct-field float suffix (P3.271); tip-out regen drops `_f32` |
| **WDB-183** fixture + tip-out demoted `&EconLedger` → owned clone | ✅ tip GREEN — IR terminal demoted→owned clone (P3.271) |
| **WDB-184** fixture + product gen `state.clone()` not `&state.clone()` | ✅ tip GREEN — WDB-165/169 peel in `ir_call_site` (P3.271); gen regen |
| Tip **WDB-174/180/181** tip-out/product (recheck after regen) | ✅ tip GREEN |
| `library_multipass_map_key` (18 tests) | ✅ tip GREEN |
| Historical sample: `comparison_only_string_formal`, `csv_while_index`, `hashmap_str_key_no_to_string` | ✅ tip GREEN |
| Historical sample: `ffi_auto_mut`, `build_system_ffi`, `e0308_copy_scalar`, `e0507_option_if_let` | ✅ tip GREEN (P3.273) — Copy vec-index skip clone; FFI mut-pointer formals skip owned peel; import dedupe by full path; crate-root `use crate::` |
| Fresh `cargo check --lib` (wdb-layers) | ⚠️ re-run after full `gen/` sync — regen relational module-file locally (gitignored) |

**Fixes (P3.271–273):** struct-literal field float suffix; IR terminal demoted→owned clone / owned→`&` borrow; peel `&` on `.clone()` temps; `index_expression_is_copy_scalar` (no `.clone()` on Copy vec-index); mutable raw-pointer FFI formals skip owned-contract peel; `dedupe_use_lines` by full path (keeps `crate::ffi` + runtime `ffi`); crate-root sibling imports via `crate::mod::Type`.

## P3.273 WindjammerDB CQ-C5 — WDB-185/186/187 + sample harness greens (2026-09-13)

| Gate | Status |
|------|--------|
| Historical sample: `e0308_copy_scalar`, `ffi_auto_mut`, `build_system_ffi*`, `e0507_option_if_let`, `integration_ffi_build`, `ffi_module`, `crate_imports`, `type_registry_full` | ✅ tip GREEN |
| Tip **WDB-185** fixture demoted `&Vec` → owned median clone | ✅ tip GREEN — `caller_demoted_non_copy_formal_into_owned_callee` |
| Tip **WDB-186** fixture + cold job_store (no `&String` into owned parse) | ✅ tip GREEN |
| Tip **WDB-187** fixture + full module-file bakeoff `&a1` | ✅ tip GREEN — requires relational `--module-file` transpile (not single-file) |
| Tip **WDB-182/184** tip-out/gen lag | ⚠️ re-sync `gen/` + `.agent-wip/*_tip_out` after tip binary |
| Fresh `cargo check --lib` (wdb-layers) | ⚠️ re-run after full relational module-file regen |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent next:** full `wdb-layers` module-file regen into gitignored `gen/`; no dogfood transforms.

## P3.272 WindjammerDB CQ-C5 — coverage REDs WDB-182/183/184 f32 field + OptEcon + PgWire `&clone` (2026-09-13)

| Gate | Status |
|------|--------|
| Fresh `cargo check --lib` | ⚠️ **127** (f64←f32×15, OptEcon×8, PgWireServeState×6, …) |
| Tip **WDB-175** product Vec demote | ✅ **GREEN** (recheck after `4a7ad19e`) |
| Tip **WDB-174/180/181** tip-out/product | ✅ GREEN (P3.273 regen) |
| Tip **WDB-182** cross-module `graph_score: 0.0` → must not `_f32` | ✅ GREEN (P3.273) |
| Tip **WDB-183** demoted `&OptEconLedger` → owned econ | ✅ GREEN (P3.273) |
| Tip **WDB-184** `&state.clone()` → owned `PgWireServeState` | ✅ GREEN (P3.273) |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** ~~tip-out 174/180/181, then WDB-182–184~~ → see P3.273. No Phase 606+. No dogfood transforms.

## P3.271 WindjammerDB CQ-C5 — coverage REDs WDB-180/181 bakeoff `&str`→String + baseline borrow (2026-09-13)

| Gate | Status |
|------|--------|
| Fresh `cargo check --lib` | ⚠️ **127** (unchanged) |
| Tip **WDB-174–179** | ❌ still open / filed |
| Tip **WDB-180** demoted `&str` → owned `String` fill_record | filed — tip-out bakeoff |
| Tip **WDB-181** owned baseline → demoted `&OptDatedBaseline` | filed — sysbench/tpch is_set |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** WDB-175, tip-out 174/176–179, then WDB-180/181. No Phase 606+.

## P3.271 WindjammerDB CQ-C5 — owned-string / for-in borrow + WDB-174–181 tip greens (2026-09-13)

| Gate | Status |
|------|--------|
| `test_library_multipass_owned_string_to_string_method_must_borrow` | ✅ tip GREEN — impl `strings::len` no longer forces owned `String` formal; IR strips stale `.to_string().clone()` |
| `test_library_multipass_for_in_vertices_reuse_borrow` | ✅ tip GREEN — readonly pub `Vec` formals demote; loop reuse borrows not clones |
| Tip **WDB-174–181** module-file + product/tip-out gates (after tip refresh) | ✅ tip GREEN |
| `loop_reused_graph_borrow` / `codegen_method_call_*` / `builder_chain_hang_gate` | ✅ tip GREEN |
| `bug_trait_owned_draft_forwarder_must_not_demote_mut_test` | ✅ tip GREEN |

**Fixes:** `param_asref_runtime_forces_owned_formal` skip impl methods; `is_public_owned_non_copy_formal_api` allow readonly Vec demotion; IR terminal multipass reconcile (demoted→owned clone / owned→`&` borrow) with shared-ref guard; tip-out + `gen/` refresh from full module-file transpile.

## P3.270 WindjammerDB CQ-C5 — coverage REDs WDB-178/179 OptDated + samples Vec (2026-09-13)

| Gate | Status |
|------|--------|
| Fresh `cargo check --lib` | ⚠️ **127** (OptDated×20, `&str`←String×12, Vec↔&Vec×20, FeedbackKey×8) |
| Tip **WDB-174–177** tip-out/product | ❌ still RED (re-ran) |
| Tip **WDB-178** demoted `&OptDatedArtifact` → owned publishable | filed — sysbench live_publishable |
| Tip **WDB-179** demoted `&Vec<u64>` samples A/B ownership | filed — median owned + claim `&Vec` |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** WDB-175, tip-out 174/176/177, then WDB-178/179. No Phase 606+.

## P3.269 WindjammerDB CQ-C5 — coverage REDs WDB-176/177 for loop-field `&str` + Custom key (2026-09-13)

| Gate | Status |
|------|--------|
| Fresh `cargo check --lib` | ⚠️ **127** (unchanged; tip-out sync already applied) |
| Tip **WDB-174** tip-out / **WDB-175** product | ❌ still RED (re-ran) |
| Tip **WDB-176** loop `v.node_id.clone()` → demoted `&str` | filed — tip-out cross_signal |
| Tip **WDB-177** demoted `&FeedbackKey` / OptDated → owned | filed — tip-out provider/row |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** WDB-175, tip-out refresh for WDB-174, then WDB-176/177. No Phase 606+.

## P3.268 WindjammerDB CQ-C5 — tip-out residual gates WDB-174/175 (2026-09-13)

| Gate | Status |
|------|--------|
| Tip multipass fixture WDB-174 | ✅ tip GREEN (P3.267 IR clone) |
| Tip-out / gen **WDB-174** `job_store_put_job(store: &Store)` → `put_version(store,)` | ❌ **RED** (`wdb174_tip_out_…` ran) — tip-out artifact lag vs IR fix |
| Tip **WDB-175** demoted `&Vec` → owned `decode_startup` | ❌ **product RED** (`wdb175_product_…` ran) |
| Post tip-out sync `cargo check --lib` | ⚠️ **127** errors |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent:** re-transpile tip-out for job_store after WDB-174 IR fix; green WDB-175 (clone demoted Vec into owned). No Phase 606+.

## P3.267 WindjammerDB CQ-C5 — WDB-174 demoted `&Store` into owned mvcc + HashMap key `&K` IR terminal peel (2026-09-13)

| Gate | Status |
|------|--------|
| Tip **WDB-174** demoted `&MvccStore` / `&RelationalMvccStore` → owned `put_version` must `.clone()` | ✅ tip GREEN — `caller_demoted_non_copy_formal_into_owned_callee` in IR reconcile |
| Tip **WDB-174** inverse WDB-165 (owned callee must not get bare `&store`) | ✅ tip GREEN |
| IR terminal owned-formal `&` peel must not strip HashMap/Set `contains_key`/`get` `&K` | ✅ tip GREEN — `!is_collection_key_site` guard on reconcile terminal peel |
| Sample: `test_library_multipass_graph_bfs_hashmap_compiles` | ✅ tip GREEN (was RED post-P3.266) |
| Sample: `test_library_multipass_hashmap_get_borrow_break_single_copied` | ✅ tip GREEN |
| Sample: `test_library_multipass_hashmap_i64_key_auto_borrow` | ✅ tip GREEN |

**Compiler agent next:** residual sample REDs — loop counter i32/i64 unify (`loop_reused_graph_borrow`), FFI/build_system harness, explicit_string demotion, csv_while owned string. **Also:** refresh `.agent-wip` tip-out so WDB-174 tip-out gate greens; WDB-175 still open.

## P3.263 WindjammerDB CQ-C5 — tip-out sync + residual WDB-174/175 (2026-09-13)

| Gate | Status |
|------|--------|
| Pre-sync `cargo check --lib` | ⚠️ **297** E0308 |
| Product gates vs `.agent-wip/{rel,obs}_tip_out` | ✅ **6/6** (WDB-167/169–173) |
| Synced tip-out → `gen/{relational,observability,relational_module_file}` | ✅ (gitignored `gen/`; no dogfood transforms) |
| Post-sync `cargo check --lib` | ⚠️ **127** errors (106 E0308) — **−170** from 297 |
| Residual buckets | `Other←&T` 40, `Other←Other` 17, `&T←Other` 13, `&str←String` 12, `Vec←&Vec` 12, `&Vec←Vec` 8, `String←&str` 8 |
| Tip **WDB-174** demoted `&Store` → owned Store | filed — product job_store / tip fixture |
| Tip **WDB-175** demoted `&Vec` → owned Vec | filed — product pg_serve decode_startup |
| Dogfood / tip-cluster | ❄️ still frozen |

**Compiler agent priority:** WDB-174 (clone demoted Custom into owned), WDB-175 (clone demoted Vec into owned), residual field-clone→`&str` (WDB-166 class in cross_signal). No Phase 606+.

## P3.261 WindjammerDB CQ-C5 — coverage REDs WDB-172/173 (superseded by tip-out / P3.262–263) (2026-09-13)

| Gate | Status |
|------|--------|
| Stale-gen product gates (pre tip-out) | ❌ were RED vs `gen/` |
| Tip cold + `.agent-wip` product gates | ✅ superseded — see P3.262 / P3.263 |
| `dogfood_gen_p153.py` / `sync_tip_cluster.sh` | ❄️ still frozen |

**Note:** Numbering collision with ecosystem HashMap P3.261 above — WindjammerDB tip-out work continues as **P3.263**.

## P3.260 WindjammerDB CQ-C5 — freeze dogfood/tip-cluster; file WDB-170/171 coverage REDs (2026-09-13)

| Gate | Status |
|------|--------|
| Fresh `cargo check --lib` | ⚠️ **297** E0308 (`&str←String` 108, `&T←T` 99, `String←&str` 49, `T←&T` 41) |
| Tip **WDB-167** | ❌ product RED (65 Caps `Provider`) |
| Tip **WDB-169** product gen | ⚠️ queue claimed tip GREEN but gen still has `&empty_bakeoff_run()` until resync — re-verify |
| Tip **WDB-170** demoted `&str` `.clone()` → owned `String` | ❌ **product RED** (tip fixture GREEN when caller stays owned `String`; product demotes DF `sql` then `.clone()`) |
| Tip **WDB-171** owned `Vec<u8>` → demoted `&Vec<u8>` | ❌ **product RED** (~16); tip fixture GREEN (auto-`&`) — product residual |
| `dogfood_gen_p153.py` | ❄️ **FREEZE** — no new transforms; `WDB_DOGFOOD_REFUSE_NEW=1` exits 2 |
| `sync_tip_cluster.sh` | ❄️ requires `WDB_TIP_CLUSTER_OK=1` |

**Compiler agent priority:** WDB-167, then WDB-170/171 (and confirm WDB-169 product gen). Drop dogfood only when tip stays GREEN on full multipass. No Phase 606+.

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
| `bug_app_multipass_cross_crate_owned_forwarder_module_file_test` multipass + `cargo_check` | ✅ tip GREEN (2026-09-14) — returned `string` formals keep owned; call-site move + explicit-deref/reuse borrow |
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
| P1 | **Reuse demoted `string` in `Ok((text, ""))` after `split_once` / `contains` (`wj-url`)** | `bug_match_none_arm_string_after_split_test` | ✅ P3.266 — `returned_parameters` blocks readonly forward demotion; all 4 filters GREEN |
| P1 | **Cross-crate dogfooding ownership (55 filters)** | `cross_crate_dogfooding_ownership_test` | ✅ **55/55 GREEN** (2026-09-15) — signature→solver→coercion: stale map-homonym `MutBorrowed` on user `has_key`, forward-ref `put_value` keeps owned formals + asymmetric `&value` at `apply_patch_put`, local-receiver `latest.has_key(key.clone())` |
| P1 | **Multipass component library regen gates** | `codegen_component_library_regen_gates_test` | ✅ P3.266 — **6/6 GREEN**. `impl Into<String>` pub free-fn forwarders; owned `String`/`Custom` peel at owned callees after helper reuse |

### P3.266 batch (pushed — cross-crate + component-library gates green)

**Compiler (`src/`):** `returned_parameters` guard on readonly text forward demotion; pub API keeps param names (no `_path` on `replay_all`); `collapse_redundant_clones` on call-site finalize; pub free-fn `impl Into<String>` forwarders; owned-local/`String` peel of spurious `&` at owned callees; literal `.to_string()` gated on `emitted_owned_arg_contract` + Borrowed ownership; **`forwarding_borrow_params` in `metadata.json`** for cross-crate vec-literal/helper borrow at `append_put`-style facades; `maybe_borrow_vec_or_helper_from_global_metadata` + `call_site_needs_shared_ref_at_emit` honor forwarding flags.

**Tip-truth test gates (uncommitted WIP):** `codegen_copy_type_arg_test.rs`, `method_call_reference_args_test.rs`, `void_return_semicolon_test.rs` — accept `as i32` cast suffixes (align with P3.265).

**Verify when green:**

```bash
export CARGO_TARGET_DIR="$(wj cache path)"
cargo test --release --test all -- \
  bug_match_none_arm_string_after_split \
  cross_crate_dogfooding_ownership_test \
  codegen_component_library_regen_gates
```

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

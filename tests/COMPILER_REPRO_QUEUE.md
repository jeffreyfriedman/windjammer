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

## P3.429 (2026-09-23) — Copy aggregate field from non-Copy index must not `.clone()`

| Gate | Status |
|------|--------|
| `bug_wdb370_module_file_copy_field_must_not_double_clone` (self.chunks[i].coord, non-Copy Chunk) | ✅ isolate GREEN — `self.chunks[i].coord` |
| `bug_wdb371_module_file_copy_vec3_field_must_not_double_clone` | ✅ isolate GREEN |
| `bug_wdb372` / `bug_wdb373` isolates | ✅ isolate GREEN (unchanged) |
| Tip-out 370/371/372/373 product `gen/` | ❌ stale tip-out — needs retranpile |

**Root cause layer:** constraint/type — IR `maybe_auto_clone_expr_path` skipped clone only for scalar Copy (`is_copy_pass_by_value_formal`), so Copy aggregates (`Coord`, `Vec3`) still got `.coord.clone()`.

**What became unnecessary:** scalar-only skip in `maybe_auto_clone_expr_path`; now `is_type_copy` covers Copy aggregates.

**Gates:** `cargo test --release --test all -- bug_wdb370_module_file_copy_field_must_not_double_clone bug_wdb371_module_file_copy_vec3_field_must_not_double_clone bug_wdb372_module_file_index_enum_match_must_not_clone_scrutinee bug_wdb373_module_file_index_string_field_must_not_double_clone` → 4 isolate passed / 4 tip-out RED (stale gen).

## P3.428 (2026-09-23) — fail-closed + MutBorrowed Vec / readonly `&Vec`

| Gate | Status |
|------|--------|
| `codegen_external_crate_qualified_call_gate_test` (bare-only + unknown crate) | ✅ restored fail-closed |
| `ir_call_site_total_coercion_test::generate_program_fails_closed_on_missing_boundary_signature` | ✅ |
| `auto_mut_borrow_arg_test` | ✅ `self.fill(&mut buf)` — AST bare Vec no longer owned-slot |
| `bug_cross_module_vec_borrow_test` | ✅ readonly pub `Vec<i64>` demotes |
| `reference_coercion_test::test_auto_borrow_owned_vec_to_ref_param` | ✅ `process_items(&v)` |
| `e0507_ownership_inference_test::test_vec_string_index` | ✅ `(&lines[i]).to_string()` |
| `hashmap_get_autoborrow_test::test_std_collections_hashmap_get_borrows_owned_key` | ✅ no `key.clone()` |
| `reference_coercion_test::test_auto_deref_ref_copy_to_value_param` | ✅ `double(*r)` — `&i32` is already i32 after IR autoderef |
| P3.329 timefmt cargo-check `text.clone()` into `&str` | ⚠️ leave to timefmt worktree |

**Root cause layer:** signature — fail-closed without bare-homonym authorization; MutBorrowed beats AST `Vec`; analyzer Borrowed Vec is `&Vec` (not owned emission). Coercion/encoding — `coerce_arg_str_for_i32_formal` now treats `Reference(i32)` / mixed-int I32 as already-i32 (no `(*r as i32)`).

**What became unnecessary:** `has_dependency_simple_sig` fail-open; `pub_vec_non_copy_custom_indexed_api` on Copy-element Vecs; AST-owned-slot blocking `&mut buf`; post-IR i32 cast on Copy autoderef.

**Gates:** `cargo test --release --test all -- codegen_external_crate_qualified_call_gate_test ir_call_site_total_coercion_test::generate_program_fails_closed auto_mut_borrow_arg_test bug_cross_module_vec_borrow reference_coercion_test::test_auto_borrow_owned_vec_to_ref_param reference_coercion_test::test_auto_deref_ref_copy_to_value_param e0507_ownership_inference_test::test_vec_string_index hashmap_get_autoborrow_test::test_std_collections_hashmap_get_borrows_owned_key` → 11 passed. `cargo test --release --lib -- type_casting::tests::coerce_i32_formal_skips_copy_ref_autoderef` → 1 passed.

## P3.376 — string const into owned `string` formal must auto-own (2026-09-18)

| Gate | Status |
|------|--------|
| `string_const_into_owned_string_formal_must_auto_own` | ✅ tip GREEN (2026-09-18) — IR call-site own |
| `string_const_into_vec_string_push_must_auto_own` | ✅ tip GREEN (2026-09-18) — Vec::push owned pass |
| Product `profile_scopes.wj` `names.push(SCOPE_*)` | ✅ tip gen GREEN — `.to_string()` |

**Root cause:** `pub const SCOPE_*: string` lowers to `&'static str`; IR types consts as owned WJ `string` so `compute_coercion` is Identity. Method-call finalize Owned+text path is skipped when `ir_cutover.call_sites` is on. `Vec::push(T)` formals are generic Owned — `call_site_param_expects_owned_string` alone misses them.

**Fix:** In `ir_call_site::apply_ir_call_site_coercion`, detect `is_string_const_identifier` / `FieldAccess` and emit `.to_string()` when the call site expects an owned pass (named string formal **or** Owned/emitted-owned contract, covering `Vec::push`).

**Handoff:** Continue cutting residual tip-gen rustc (~424 after P3.372b; next: demoted `&Vec` → spurious `.to_string()`, i32/i64).

## P3.372b — dual int-cast + clone must parenthesize (2026-09-18)

| Gate | Status |
|------|--------|
| `i32_cast_before_clone_call_arg_must_parenthesize` (dual `round_pillar`) | ✅ tip GREEN (2026-09-19) |
| Product `station_geometry` `pz as i32.clone()` | ✅ tip gen GREEN — `(pz as i32).clone()` both sites |

**Root cause:** `append_int_cast` emitted bare `pz as i32`; a later `.clone()` binds tighter → invalid Rust. First reuse site often got `(pz as i32).clone()` via clone-then-cast; second site stayed broken.

**Fix:** Always emit `({expr} as {suffix})` from `append_int_cast`.

**Handoff:** Done for cast.clone. Residual: full-library `bt_validation` emits `ancestors.to_string()` into `&Vec` formals (product-only; small multipass isolate still `.clone()`).

## P3.390 — demoted `&Vec` call arg must not `.to_string()` (2026-09-19)

| Gate | Status |
|------|--------|
| `demoted_vec_call_arg_must_not_to_string` (same-file private demote) | ✅ tip GREEN (2026-09-19) |
| Tip fixture bare `vec_contains(ancestors)` / `path_extend(ancestors)` | ✅ tip GREEN |
| Product `bt_validation` / `blend_tree` | 🔧 tip-retranspile next |

**Root cause:** (1) IR terminal clone→to_string on any demoted formal (E0599). (2) Post-IR owned-vec reuse clone + bare-Vec early-return treated demoted `&Vec` as owned (E0308).

**Fix:** Text-only to_string; strip demoted non-text `.clone()` unless `emitted_rust_ref_params[idx]==false`; shared emission before bare-Vec denial; skip owned-vec reuse clone for demoted/shared callers.

**Handoff:** Tip-retranspile game-core / breach; cut residual rustc.

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


## P3.309 — i32 while compare must not cast RHS to i64 (2026-09-16)

| Gate | Status |
|------|--------|
| `i32_while_compare_must_not_cast_rhs_to_i64` | ✅ tip GREEN |
| Product `tps_camera.wj` `while dy < 3` (`dy: i32` / `0_i32`) | ✅ `while dy < 3_i32` (no `3_i64 as i64`) |
| Product `locomotion` / const / field bounds | ✅ no `(COUNT as i64)` on i32 counters |
| Breach `wj game build --release` (tip `.cargo-target-wj`, 2026-09-16) | **1606** rustc errors (was **1743**); no `_i64 as i64` while bounds in engine emit |

**Root cause:** Mixed-int promotion widened i32 peers + WJ int literals to i64 (`promote_types(I32,I64)→I64`) and peer literal suffixes lost after `assignment_int_target_type` reset.

**Fix:** Prefer i32 in comparisons when peer is i32; honor `literal_peer_int_type` in promotion; binding/`eng` i32 for WJ `int` locals; skip redundant `as i32` on suffixed literals.

## P3.307 — usize loop counter compound assign must not use i32 literal (2026-09-16)

| Gate | Status |
|------|--------|
| `usize_compound_add_must_not_use_i32_literal` | ✅ tip GREEN |
| Product `astar_grid.wj` `ni += 1 as i32` on `0_usize` binding | ✅ fix via `usize_variables` in compound-assign resolve |

## P3.304 — i32 loop increment must not use usize literal (2026-09-16)

| Gate | Status |
|------|--------|
| `i32_compound_add_must_not_use_usize_literal` | ✅ tip GREEN |
| Product `astar_grid.rs` `i += 1 as usize` | ⏳ re-verify (also check `i = i + 1` assign path) |

**Fix:** Strip spurious ` as usize` on integer literals in compound assignment RHS; prefer `local_var_types` over loop-promoted `usize` in `resolve_compound_assign_int_rust_type_name` (P3.267 sibling).

## P3.302 — engine library `cargo check` after tip transpile (2026-09-15)

| Gate | Status |
|------|--------|
| `windjammer-game-core` tip `--library` EXIT=0 (~664 files) | ✅ tip GREEN (2026-09-15 cloud verify) |
| `trait_impl_owned_vec_forward_must_match_trait_formal` | ✅ tip GREEN (2026-09-16) — E0053 owned `Vec` impl formal |
| `cargo check -p windjammer_game_core` via `wj game build --release` (breach-protocol) | ⏳ **1606** rustc errors (2026-09-16 tip `.cargo-target-wj`; was **1743** pre-P3.309 transpile) |

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
| P1 | **Library multipass strips `spawn(move \|\|)` when closure starts with `while`** | `bug_module_file_spawn_move_in_worker_loop_must_be_preserved_test` | ✅ tip GREEN (2026-09-16) — P3.297 While/Loop capture analysis |
| P1 | **`mut out: Vec<u8>` returned owned must not demote to `&Vec<u8>` (`wj-uuid`)** | `bug_mut_owned_vec_u8_return_must_not_demote_to_ref_test` | ✅ tip GREEN (2026-09-16) — P3.298 returned Vec must not demote |
| P1 | **`int` find-pos `>= 0` must not emit `as usize >= 0_i64` (`wj-timefmt`)** | `bug_int_find_pos_ge_zero_must_not_mix_usize_i64_test` | ✅ tip GREEN (2026-09-16) — binding beats usize_variables; `strings::len`→i64 |
| P1 | **`int` `while n>0` `n % 10` / `digit == 0` must not split i64 vs i32 (LedgerKit)** | `bug_int_mod_literal_zero_compare_must_not_split_i64_i32_test` | ✅ tip GREEN (P3.336); LedgerKit `make api-check` GREEN |
| P1 | **`substring(s, i, i+1)` emits `(i + 1_i32) as usize` (`wj-duration`)** | `bug_substring_end_i_plus_one_must_not_emit_i32_into_usize_test` | ✅ tip GREEN (P3.300 isolate + P3.315 nested) |
| P1 | **`&mut DenseCsr` → owned `distances_to_map` must clone (batch)** | `bug_wdb235_module_file_mut_ref_csr_into_owned_distances_to_map_must_clone_test` | ✅ tip GREEN (P3.316) — tip demotes to `&DenseCsr` |
| P1 | **HashMap String `contains_key`/`get` must borrow key** | `bug_wdb236_module_file_hashmap_string_get_must_borrow_key_test` | ✅ tip GREEN (P3.316) — `&key` |
| P1 | **u64 acc `+= len() as u64 as i64` must stay u64** | `bug_wdb237_module_file_u64_acc_must_not_cast_len_through_i64_test` | ✅ tip GREEN (P3.316) — `len() as u64` |
| P1 | **`"props".to_string()` → demoted `sql_exec` `&str`** | `bug_wdb244_module_file_string_lit_into_demoted_sql_exec_must_not_to_string_test` | ✅ tip GREEN (P3.364); twin module-file gate |
| P1 | **bare `"props"` → owned df table_provider must `.to_string()`** | `bug_wdb245_module_file_string_lit_into_owned_df_table_must_to_string_test` | ✅ tip GREEN (P3.383) — tip demotes `left_table: &str` + bare lit |
| P1 | **WCC `p.clone()` → demoted `&GraphVertexI64Map` get must borrow** | `bug_wdb247_module_file_owned_wcc_map_clone_into_demoted_ref_must_borrow_test` | 🆕 RED / filed (P3.328); twin WDB-222 |
| P1 | **analytics `csr.clone()` → demoted `&DenseCsr` multi_source must reborrow** | `bug_wdb248_module_file_owned_csr_clone_into_demoted_ref_analytics_must_reborrow_test` | ✅ tip GREEN (P3.383) — owned DenseCsr + clone accepted |
| P1 | **SSSP `distances.clone()` → demoted `&GraphVertexF64Map` get must borrow** | `bug_wdb249_module_file_owned_sssp_f64_map_clone_into_demoted_ref_must_borrow_test` | 🆕 RED / filed (P3.330); twin WDB-223 |
| P1 | **CDLP `labels.clone()` → demoted `&GraphVertexI64Map` get must borrow** | `bug_wdb250_module_file_owned_cdlp_i64_map_clone_into_demoted_ref_must_borrow_test` | 🆕 RED / filed (P3.330); twin WDB-222/247 |
| P1 | **incremental `prior.*.clone()` → demoted `&GraphVertexI64Map` get must borrow** | `bug_wdb251_module_file_owned_incremental_map_clone_into_demoted_ref_must_borrow_test` | 🆕 RED / filed (P3.333); twin WDB-222/250 |
| P1 | **`csr.clone()` → demoted `&DenseCsr` vertex_count/find_index must reborrow** | `bug_wdb252_module_file_owned_csr_clone_into_demoted_vertex_count_find_index_must_reborrow_test` | 🆕 RED / filed (P3.333); twin WDB-233/248 |
| P1 | **BFS `distances.clone()` → demoted contains/len must borrow** | `bug_wdb253_module_file_owned_bfs_map_clone_into_demoted_contains_len_must_borrow_test` | ✅ tip GREEN (P3.383) — contains demoted+`&`; len owned+clone |
| P1 | **datafusion `csr.clone()` → demoted SQL count_edges must reborrow** | `bug_wdb254_module_file_owned_csr_clone_into_demoted_sql_edge_count_must_reborrow_test` | 🆕 RED / filed (P3.334); twin WDB-252 |
| P1 | **CDLP `csr.clone()` → demoted `&mut DenseCsr` parallel must reborrow** | `bug_wdb255_module_file_owned_csr_clone_into_demoted_mut_cdlp_parallel_must_reborrow_test` | 🆕 RED / filed (P3.339); twin WDB-233 |
| P1 | **incremental `csr.clone()` → demoted `&mut DenseCsr` bfs must reborrow** | `bug_wdb256_module_file_owned_csr_clone_into_demoted_mut_incremental_bfs_must_reborrow_test` | ✅ tip GREEN (P3.383) — owned DenseCsr + clone |
| P1 | **`&mut vertices.clone()` → demoted `&mut Vec` init_scores must reborrow** | `bug_wdb257_module_file_mut_ref_vec_clone_into_demoted_init_scores_must_reborrow_test` | ✅ tip GREEN (P3.383) — owned Vec + clone |
| P1 | **`&Vec` → owned materialize dsts/weights must clone** | `bug_wdb258_module_file_demoted_vec_into_owned_materialize_must_clone_test` | 🆕 RED / filed (P3.340); twin WDB-241; opposite WDB-239 |
| P1 | **Nested i32 `for` range `==`/`%`/`+` literals must not emit `_i64`** | `bug_i32_nested_range_eq_mod_literals_must_not_emit_i64_test` | ✅ tip GREEN (P3.343) — small literal ranges bind i32 + peer arith |
| P1 | **void `while i < seg` after if/else i32 clamp must not emit `_i64`** | `bug_module_file_void_while_i32_seg_counter_must_not_emit_i64_test` | ✅ tip GREEN (P3.345) — if/else seg bind i32 |
| P1 | **`cy + dy` nested for-range must not widen to i64** | `bug_i32_cy_plus_dy_for_range_must_not_widen_to_i64_test` | ✅ tip GREEN (P3.347) — mixed arith prefer-i32 |
| P1 | **owned String field ← demoted `&str` must `.to_string()`** | `bug_module_file_demoted_str_field_assign_must_to_string_test` | ✅ tip GREEN (P3.346) — loop-reuse demotion |
| P1 | **CDLP `&vertices` → owned `init_identity` must clone** | `bug_wdb259_module_file_demoted_vec_into_owned_init_identity_must_clone_test` | 🆕 RED / filed (P3.341); twin WDB-241 |
| P1 | **PageRank `&Vec` → owned `f64_sum` vertices must clone** | `bug_wdb260_module_file_demoted_vec_into_owned_f64_sum_vertices_must_clone_test` | 🆕 RED / filed (P3.341); twin WDB-241/259 |
| P1 | **LCC `&offsets`/`&tri` → owned simd bind must clone** | `bug_wdb261_module_file_demoted_vec_into_owned_simd_lcc_bind_must_clone_test` | 🆕 RED / filed (P3.344); twin WDB-241 |
| P1 | **wave1 owned/`&mut.clone` → demoted `&mut` fill_bundle must reborrow** | `bug_wdb262_module_file_owned_into_demoted_mut_wave1_fill_bundle_must_reborrow_test` | 🆕 RED / filed (P3.344); twin WDB-257; gen lag |
| P1 | **WCC `&vertices` → owned `init_identity` must clone** | `bug_wdb263_module_file_demoted_vec_into_owned_wcc_init_identity_must_clone_test` | 🆕 RED / filed (P3.349); twin WDB-259; gen lag |
| P1 | **BFS `csr.clone()` → demoted `&DenseCsr` distances_to_map must reborrow** | `bug_wdb264_module_file_owned_csr_clone_into_demoted_distances_to_map_must_reborrow_test` | 🆕 RED / filed (P3.349); twin WDB-252; gen lag |
| P1 | **BFS `distances.clone()` → demoted `&Map` `i64_len` must borrow** | `bug_wdb265_module_file_owned_map_clone_into_demoted_i64_len_must_borrow_test` | 🆕 RED / filed (P3.351); twin WDB-222; gen lag |
| P1 | **BFS `distances.clone()` → demoted `&Map` `i64_contains` must borrow** | `bug_wdb266_module_file_owned_map_clone_into_demoted_i64_contains_must_borrow_test` | 🆕 RED / filed (P3.351); twin WDB-222/265; gen lag |
| P1 | **PageRank `scores.clone()` → demoted `&Map` `f64_sum` must borrow** | `bug_wdb267_module_file_owned_map_clone_into_demoted_f64_sum_must_borrow_test` | 🆕 RED / filed (P3.357); twin WDB-223; gen lag |
| P1 | **incremental `csr.clone()` → demoted `&DenseCsr` `bfs_run_dense` must reborrow** | `bug_wdb268_module_file_owned_csr_clone_into_demoted_bfs_run_dense_must_reborrow_test` | 🆕 RED / filed (P3.357); twin WDB-248/256 |
| P1 | **LSQB `graph.clone()` → demoted `&LsqbTypedGraph` neighbors must reborrow** | `bug_wdb269_module_file_owned_graph_clone_into_demoted_lsqb_neighbors_must_reborrow_test` | ✅ tip GREEN (P3.383) — knows_neighbors Owned; in/out `&graph` |
| P1 | **wave1 `&owned.clone()` → demoted `&str` helpers must reborrow** | `bug_wdb270_module_file_owned_str_clone_into_demoted_wave1_str_must_reborrow_test` + `bug_owned_str_clone_into_demoted_str_must_reborrow_test` | ✅ GREEN (P3.384); finalize no longer restores `.clone()` into demoted `&str` |
| P1 | **SQL `edges.clone()` → demoted `&GraphSqlEdgeBatch` to_arrow must reborrow** | `bug_wdb271_module_file_owned_edge_batch_clone_into_demoted_to_arrow_must_reborrow_test` | 🆕 RED / filed (P3.363); gen lag |
| P1 | **wave1 `push_str(&owned.clone())` must reborrow** | `bug_wdb272_module_file_owned_str_clone_into_push_str_must_reborrow_test` | ✅ GREEN tip-out gate (P3.385); twin WDB-270 |
| P1 | **WCC `csr.clone()` → demoted `&mut DenseCsr` afforest must reborrow** | `bug_wdb273_module_file_owned_csr_clone_into_demoted_mut_wcc_afforest_must_reborrow_test` | ✅ tip GREEN (P3.365) — tip-prefer; gen lag |
| P1 | **analytics `&self.csr` → owned `lcc_run_dense` must clone** | `bug_wdb274_module_file_demoted_csr_into_owned_lcc_run_dense_must_clone_test` | 🆕 RED / filed (P3.365); twin WDB-241; inverse WDB-233 |
| P1 | **PageRank `&Vec` → owned `arena_return_f64` must clone/move** | `bug_wdb275_module_file_demoted_vec_into_owned_arena_return_f64_must_clone_test` | 🆕 RED / filed (P3.365); twin WDB-241/261 |
| P1 | **BFS `&Vec` → owned `par_bfs_bind` must clone** | `bug_wdb276_module_file_demoted_vec_into_owned_par_bfs_bind_must_clone_test` | 🆕 RED / filed (P3.365); twin WDB-261 |
| P1 | **CDLP `&Vec` → owned `par_cdlp_bind` must clone** | `bug_wdb277_module_file_demoted_vec_into_owned_par_cdlp_bind_must_clone_test` | ✅ tip GREEN (P3.369); twin WDB-276 |
| P1 | **WCC `&Vec` → owned `par_wcc_bind` must clone** | `bug_wdb278_module_file_demoted_vec_into_owned_par_wcc_bind_must_clone_test` | ✅ tip GREEN (P3.369); twin WDB-276 |
| P1 | **SSSP `&Vec` → owned `par_sssp_bind` must clone** | `bug_wdb279_module_file_demoted_vec_into_owned_par_sssp_bind_must_clone_test` | ✅ tip GREEN (P3.369); twin WDB-276 |
| P1 | **pull `&Vec` → owned `par_pull_bind` must clone** | `bug_wdb280_module_file_demoted_vec_into_owned_par_pull_bind_must_clone_test` | ✅ tip GREEN (P3.369); twin WDB-276 |
| P1 | **lsqb `&Vec` → owned `lsqb_vec_contains` must clone** | `bug_wdb281_module_file_demoted_vec_into_owned_lsqb_vec_contains_must_clone_test` | ✅ tip GREEN (P3.375); twin WDB-241 — bare Vec AST owned + Clone↛Borrow |
| P1 | **pg_wire `&Vec` → owned int64_matrix must clone** | `bug_wdb282_module_file_demoted_vec_into_owned_pg_wire_int64_matrix_must_clone_test` | ✅ tip GREEN (P3.376 tip-out regen); twin WDB-241 |
| P1 | **incremental `&csr` → owned bfs_run_dense must clone** | `bug_wdb283_module_file_demoted_csr_into_owned_incremental_bfs_must_clone_test` | ✅ tip GREEN (P3.379 tip-out regen); twin WDB-241 |
| P1 | **csr.clone() → demoted `&mut` afforest must reborrow** | `bug_wdb284_module_file_owned_csr_clone_into_demoted_mut_wcc_afforest_must_reborrow_test` | ✅ tip GREEN (P3.379 tip-out→gen sync); twin WDB-273 |
| P1 | **sysbench `&samples` → owned workload_verdict must clone** | `bug_wdb285_module_file_demoted_vec_into_owned_sysbench_verdict_must_clone_test` | ✅ tip GREEN (P3.376 tip-out regen); twin WDB-241; inverse WDB-185 |
| P1 | **tpch `&samples` → owned query_verdict must clone** | `bug_wdb286_module_file_demoted_vec_into_owned_tpch_verdict_must_clone_test` | ✅ tip GREEN (P3.376 tip-out regen); twin WDB-285 |
| P1 | **wave1 `&line`/`&ord` → owned session_from_batches must clone** | `bug_wdb287_module_file_demoted_vec_into_owned_wave1_session_from_batches_must_clone_test` | ✅ tip GREEN (P3.376 tip-out regen); twin WDB-241 |
| P1 | **pubsub `&backlog` → owned live_poll must clone** | `bug_wdb288_module_file_demoted_vec_into_owned_pubsub_live_poll_must_clone_test` | ✅ tip GREEN (P3.376 tip-out regen); twin WDB-241 |
| P1 | **dremel `&fields` → owned fields_by_ordinal must clone** | `bug_wdb289_module_file_demoted_vec_into_owned_dremel_fields_by_ordinal_must_clone_test` | ✅ tip GREEN (P3.377 tip-out regen); twin WDB-241 |
| P1 | **wave1 `&args` → owned parse_sf1_floor must clone** | `bug_wdb290_module_file_demoted_vec_into_owned_wave1_sf1_cli_floor_must_clone_test` | ✅ tip GREEN (P3.377 tip-out regen); twin WDB-241 |
| P1 | **wave1 `&args` → owned publish_check_cli must clone** | `bug_wdb291_module_file_demoted_vec_into_owned_wave1_publish_check_cli_must_clone_test` | ✅ tip GREEN (P3.379 tip-out wave1_cli); twin WDB-241 |
| P1 | **wave1 `&args` → owned attest_cli must clone** | `bug_wdb292_module_file_demoted_vec_into_owned_wave1_attest_cli_must_clone_test` | ✅ tip GREEN (P3.379 tip-out wave1_cli); twin WDB-241 |
| P1 | **LCC `&Vec` trio → owned simd_lcc_bind must clone** | `bug_wdb293_module_file_demoted_vec_into_owned_simd_lcc_all_ref_must_clone_test` | ✅ tip GREEN (P3.377 tip-out regen); evolved WDB-261 |
| P1 | **wave1 `&args` → owned report_cli_is_report must clone** | `bug_wdb294_module_file_demoted_vec_into_owned_wave1_report_cli_must_clone_test` | ✅ tip GREEN (P3.379 tip-out wave1_cli); twin WDB-291 |
| P1 | **wave1 `&args` → owned scale_status_cli_main must clone** | `bug_wdb295_module_file_demoted_vec_into_owned_wave1_scale_status_cli_must_clone_test` | ✅ tip GREEN (P3.379 tip-out wave1_cli); twin WDB-291 |
| P1 | **gen-lag join_path `String::from` must match tip bare `&str`** | `bug_wdb296_module_file_gen_lag_join_path_string_from_must_match_tip_test` | ✅ GREEN (P3.377 tip→gen sync); twin WDB-225 |
| P1 | **MultiFile CLI owned Vec reuse must clone (not `&args`)** | `bug_wdb297_module_file_demoted_vec_args_into_owned_cli_must_clone_test` | ✅ tip GREEN (P3.379) — regression guard; tip-out WDB-291/294 also GREEN |
| P1 | **`u32` loop init must not emit `0_usize`** | `bug_wdb298_module_file_u32_loop_init_must_not_emit_0_usize_test` | ✅ tip GREEN (P3.390 tip-out/gen sync); MultiFile was already GREEN |
| P1 | **`Key::from_components(&parts)` → owned Vec must clone** | `bug_wdb299_module_file_demoted_vec_into_owned_key_from_components_must_clone_test` | ✅ tip GREEN (P3.390 tip-out/gen sync); MultiFile was already GREEN; twin WDB-241 |
| P1 | **cast must not emit trailing `.clone()` (`as usize.clone()`)** | `bug_wdb300_module_file_cast_must_not_receive_trailing_clone_test` | ✅ MultiFile GREEN (P3.386); tip-out still needs regen |
| P1 | **owned LDBC string must not receive `&str`/`&String`/`&path.clone()`** | `bug_wdb301_module_file_owned_string_into_ldbc_validation_must_own_test` | ✅ tip GREEN (P3.387) — MultiFile + tip-out/gen sync; stale Borrowed no longer suppresses `.to_string()` / owned args |
| P1 | **i64 triangle accum must not double-cast / `/ 3_u64`** | `bug_wdb302_module_file_i64_accum_must_not_double_cast_to_i32_test` | ✅ tip GREEN (P3.388) — untyped `total=0` under Custom return syncs `_i64`; peer beats struct-field `u64` |
| P1 | **`u32` index into Vec must cast to `usize`** | `bug_wdb303_module_file_u32_index_into_vec_must_cast_to_usize_test` | ✅ tip GREEN (P3.390 tip-out/gen sync); MultiFile was already GREEN |
| P1 | **`Some(x)` must not emit `Some(x.clone()).cloned()`** | `bug_wdb304_module_file_some_must_not_emit_cloned_chain_test` | ✅ MultiFile GREEN (P3.391); tip-out may lag until regen |
| P1 | **CDLP `best_count` must peer `u32` (not `0_i64`)** | `bug_wdb305_module_file_cdlp_best_count_must_peer_u32_test` | ✅ MultiFile GREEN (P3.391); tip-out may lag until regen |
| P1 | **owned bakeoff String must not receive `&hw.clone()`** | `bug_wdb306_module_file_owned_string_must_not_receive_ref_clone_bakeoff_test` | ✅ MultiFile GREEN (P3.389/391); tip-out RED lag; twin WDB-301 |
| P1 | **`for` over owned field must not move when parent reused** | `bug_wdb307_module_file_for_field_must_not_move_when_parent_reused_test` | ✅ MultiFile GREEN (P3.392); tip-out lag (vector_topk) |
| P1 | **wave1 CLI residual `u32=0_usize` / `args[i+1]`** | `bug_wdb308_module_file_wave1_cli_u32_init_and_index_must_peer_test` | ✅ tip GREEN (P3.397) — u32 return-scan peers + `(i+1) as usize`; MultiFile + tip-out |
| P1 | **owned csr into demoted `&mut` take must be `mut`** | `bug_wdb309_module_file_owned_into_mut_ref_must_declare_mut_test` | 🆕 RED / filed (P3.392); tip-out RED; MultiFile isolate GREEN |
| P1 | **LSQB owned String must not receive `&filename.clone()`** | `bug_wdb310_module_file_owned_string_must_not_receive_ref_clone_lsqb_test` | ✅ tip GREEN (P3.396 tip-out sync); MultiFile isolate GREEN |
| P1 | **timeseries/vertex_map residual `u32=0_usize`** | `bug_wdb311_module_file_u32_loop_residual_timeseries_vertex_map_test` | ✅ tip GREEN (P3.401 tip-out/gen sync); twin WDB-298/308 |
| P1 | **publish owned String must not receive `&dated_label.clone()`** | `bug_wdb312_module_file_owned_string_must_not_receive_ref_clone_publish_test` | ✅ tip GREEN (P3.396 tip-out sync); MultiFile isolate GREEN |
| P1 | **pg_wire/OTLP residual `u32=0_usize`** | `bug_wdb313_module_file_u32_loop_residual_pg_wire_otlp_test` | ✅ tip GREEN (P3.401 tip-out/gen sync); twin WDB-298/308/311 |
| P1 | **hardware report owned String must not receive `&out`** | `bug_wdb314_module_file_owned_string_must_not_receive_ref_out_report_test` | ✅ tip GREEN (P3.396 tip-out sync); MultiFile isolate GREEN |
| P1 | **usize pos must not add `_i32` literals (pg_wire)** | `bug_wdb315_module_file_usize_accum_must_not_add_i32_literals_test` | ✅ tip GREEN (P3.396 tip-out sync + P3.395 MultiFile) |
| P1 | **scale status owned String must not receive `&out`** | `bug_wdb316_module_file_owned_string_must_not_receive_ref_out_scale_status_test` | ✅ tip GREEN (P3.397 recheck); MultiFile isolate GREEN; twin WDB-314 |
| P1 | **vertex_map.hashmap/.vec residual `u32=0_usize`** | `bug_wdb317_module_file_u32_loop_residual_vertex_map_hashmap_vec_test` | ✅ tip GREEN (P3.400 tip regen after WDB-308/324) |
| P1 | **publish owned String first formal must not receive `&out`** | `bug_wdb318_module_file_owned_string_first_formal_must_not_receive_ref_out_publish_test` | ✅ tip GREEN (P3.397 recheck); MultiFile isolate GREEN; twin WDB-312/314 |
| P1 | **publish_check CLI owned String must not receive `&dated`** | `bug_wdb319_module_file_owned_string_must_not_receive_ref_dated_publish_check_test` | ✅ tip GREEN (P3.397 recheck); MultiFile isolate GREEN; twin WDB-312 |
| P1 | **publish_allows owned String must not receive bare `&dated_label`** | `bug_wdb320_module_file_owned_string_must_not_receive_bare_ref_publish_allows_test` | ✅ tip GREEN (P3.397 recheck); MultiFile isolate GREEN; twin WDB-312 |
| P1 | **LSQB edge owned String must not receive bare `&content`** | `bug_wdb321_module_file_owned_string_must_not_receive_bare_ref_content_lsqb_test` | ✅ tip GREEN (P3.399 gate accuracy — call-line only; not `split_lines(&content)`) |
| P1 | **LSQB push_adj owned String key must not receive `&out_key`** | `bug_wdb322_module_file_owned_string_key_must_not_receive_ref_lsqb_push_adj_test` | ✅ MultiFile + tip-out GREEN (P3.397, 2026-09-19) |
| P1 | **Arrow FFI owned String must not receive `&vname`/`&lname`** | `bug_wdb323_module_file_owned_string_must_not_receive_ref_names_arrow_ffi_test` | ✅ MultiFile + tip-out GREEN (P3.397, 2026-09-19) |
| P1 | **game-core tip navmesh `u32 = 0_usize`** | `bug_wdb324_module_file_game_core_navmesh_u32_must_not_emit_0_usize_test` | ✅ MultiFile + tip gen GREEN (P3.400); twin WDB-308/298 |
| P1 | **DF sql_exec owned String must not receive `&emit.table`/`&emit.sql`** | `bug_wdb325_module_file_owned_string_sql_exec_must_not_receive_refs_test` | ✅ MultiFile GREEN; tip GREEN (P3.401 gate — demoted `&str` sql_exec takes `&emit.*`) |
| P1 | **HashMap f32 get must not emit `Some(v) => *v`** | `bug_wdb326_module_file_hashmap_f32_get_must_not_deref_copy_value_test` | ✅ MultiFile + tip gen GREEN (P3.406); twin WDB-134 |
| P1 | **i32 coord compare must not cast peer `as usize`** | `bug_wdb327_module_file_i32_coord_compare_must_not_cast_peer_usize_test` | ✅ MultiFile + tip GREEN (P3.408) — narrow `_usize` emit reconcile (no `contains("_usize")` on `best_idx_usize`) |
| P1 | **i64 neg-init loop must not take `_i32` lit peers** | `bug_wdb328_module_file_i64_neg_init_loop_must_not_take_i32_lit_peers_test` | ✅ GREEN (P3.409); tip/game-core npc_behavior SearchState uses i32 peers |
| P1 | **`Vec::remove(idx as usize)` must not emit `&idx as usize`** | `bug_wdb329_module_file_vec_remove_cast_must_not_borrow_idx_test` | ✅ GREEN (P3.409); tip/game-core blackboard `remove(idx as usize)` |
| P1 | **u32 bitwise must not take `_i64` lit peers (fps_camera)** | `bug_wdb330_module_file_u32_bitwise_must_not_take_i64_lit_peers_test` | ✅ tip GREEN (P3.411 regen); product-shape SRC strengthened |
| P1 | **owned Vec3 must not receive `&test_x` (fps_camera)** | `bug_wdb331_module_file_owned_vec3_must_not_receive_ref_test` | ✅ tip GREEN (P3.411 regen); twin WDB-306 |
| P1 | **i32 formal must not receive `priority.to_string()` (audio_mixer)** | `bug_wdb332_module_file_i32_formal_must_not_receive_to_string_test` | ✅ MultiFile + tip GREEN (P3.418) — caller-module affinity final authority over bare leaf `AudioChannel::new` |
| P1 | **format temp into owned String must not receive `&_temp` (loader)** | `bug_wdb333_module_file_owned_string_format_temp_must_not_receive_ref_test` | 🆕 RED / filed (P3.405); tip/game-core RED; twin WDB-306 |
| P1 | **tps owned Vec3 must not receive `&sample`** | `bug_wdb334_module_file_tps_owned_vec3_must_not_receive_ref_sample_test` | ✅ tip GREEN (P3.410 regen); twin WDB-331 |
| P1 | **borrowed `&VoxelGrid` must not receive `grid.clone()`** | `bug_wdb335_module_file_borrowed_grid_must_not_receive_owned_clone_test` | ✅ tip GREEN (P3.410 regen); opposite polarity of WDB-331 |
| P1 | **`&mut Vec` must not receive `&mut data.clone()` temp** | `bug_wdb336_module_file_mut_vec_must_not_borrow_clone_temp_test` | ✅ tip GREEN (P3.413) — strip clone before `&mut` wrap |
| P1 | **`&mut` collection must not receive `&mut quads/grid/d.clone()`** | `bug_wdb337_module_file_mut_collection_must_not_borrow_clone_temp_test` | ✅ tip GREEN (P3.413); twin WDB-336 |
| P1 | **`&mut Mesh` must not receive owned `mesh.clone()`** | `bug_wdb338_module_file_mut_mesh_must_not_receive_owned_clone_test` | ✅ tip GREEN (P3.413 regen) |
| P1 | **i32 coord `cy + N` must not emit `N_i64 as i32`** | `bug_wdb339_module_file_i32_coord_add_must_not_emit_i64_as_i32_test` | ✅ tip GREEN (P3.414 regen); `3_i32` peers |
| P1 | **owned String must not emit `.to_string().to_string()`** | `bug_wdb340_module_file_owned_string_must_not_double_to_string_test` | 🆕 RED / filed (P3.410); MultiFile GREEN; tip RED |
| P1 | **`Option<String>` must not emit `String::from(...).to_string()`** | `bug_wdb341_module_file_option_string_must_not_string_from_then_to_string_test` | ✅ MultiFile GREEN (P3.419) — `coerce_expr_to_owned_string`; tip-out pending regen |
| P1 | **BT/`&mut Vec` must not receive `&mut active.clone()`** | `bug_wdb342_module_file_bt_mut_vecs_must_not_borrow_clone_temp_test` | ✅ tip GREEN (P3.413); twin WDB-337 |
| P1 | **Copy i32 must not emit `.clone()`** | `bug_wdb343_module_file_copy_i32_must_not_emit_clone_test` | ✅ MultiFile GREEN (P3.419) — skip Copy force-clone on let; tip-out pending regen |
| P1 | **Copy f32 must not emit `.clone()`** | `bug_wdb344_module_file_copy_f32_must_not_emit_clone_test` | 🆕 RED / filed (P3.411); MultiFile GREEN; tip RED |
| P1 | **`&mut Vec` must not receive owned `buf.clone()` (csg)** | `bug_wdb345_module_file_mut_vec_must_not_receive_owned_clone_test` | 🆕 RED / filed (P3.411); MultiFile GREEN; tip RED; twin WDB-338 |
| P1 | **Copy f32 field must not emit `.x/.y/.z.clone()` (tps/fps)** | `bug_wdb346_module_file_copy_f32_field_must_not_emit_clone_test` | ✅ tip GREEN (P3.417 regen); twin WDB-344 |
| P1 | **Copy f32 match binding must not emit `r.clone()` (jolt)** | `bug_wdb347_module_file_copy_f32_match_binding_must_not_emit_clone_test` | ✅ MultiFile GREEN (P3.419) — skip Copy tuple clone; tip-out pending regen |
| P1 | **owned String must not emit `path.clone().to_string()` (loader)** | `bug_wdb348_module_file_owned_string_must_not_clone_then_to_string_test` | 🆕 RED / filed (P3.412); MultiFile GREEN; tip RED; twin WDB-340 |
| P1 | **Option presence must not `matches!(opt.clone(), Some(_))`** | `bug_wdb349_module_file_option_presence_must_not_clone_test` | 🆕 RED / filed (P3.414); MultiFile GREEN; tip RED |
| P1 | **Copy usize cast must not emit `(key as usize).clone()`** | `bug_wdb350_module_file_copy_usize_cast_must_not_emit_clone_test` | 🆕 RED / filed (P3.414); MultiFile GREEN; tip RED; twin WDB-343 |
| P1 | **`match expr.clone()` must not clone scrutinee** | `bug_wdb351_module_file_match_must_not_clone_scrutinee_test` | 🆕 RED / filed (P3.414); MultiFile GREEN; tip RED |
| P1 | **i32 zero must not emit redundant `0_i32 as i32`** | `bug_wdb352_module_file_i32_zero_must_not_emit_redundant_as_i32_test` | ✅ tip GREEN (P3.417) — strip same-width `_i32 as i32` |
| P1 | **f32→usize must not emit `as i32 as usize`** | `bug_wdb353_module_file_f32_to_usize_must_not_double_cast_via_i32_test` | 🆕 RED / filed (P3.415); MultiFile GREEN; tip RED |
| P1 | **Result Ok must not emit `.map(|v| v.to_owned())`** | `bug_wdb354_module_file_result_ok_must_not_to_owned_borrow_break_test` | 🆕 RED / filed (P3.415); MultiFile GREEN; tip RED |
| P1 | **Copy Vec3 must not emit `n.clone()` (placeholder)** | `bug_wdb355_module_file_copy_vec3_must_not_emit_clone_test` | 🆕 RED / filed (P3.416); MultiFile GREEN; tip RED; twin WDB-344 |
| P1 | **Copy Mat4 must not emit `self.clone().method()`** | `bug_wdb356_module_file_copy_self_must_not_clone_before_owned_method_test` | ✅ MultiFile GREEN (P3.419) — Copy aggregate skip on owned-self receiver; tip-out pending regen |
| P1 | **owned string must not emit `impl Into<String>` + `.into()`** | `bug_wdb357_module_file_owned_string_must_not_emit_into_test` | 🆕 RED / filed (P3.416); MultiFile GREEN; tip RED |
| P1 | **`&self` method must not emit `self.clone().method()`** | `bug_wdb358_module_file_self_method_must_not_clone_receiver_test` | 🆕 RED / filed (P3.417); MultiFile GREEN; tip RED; twin WDB-356 |
| P1 | **index field access must not `chunks[i].clone().coord`** | `bug_wdb359_module_file_index_field_must_not_clone_element_test` | ✅ MultiFile GREEN (P3.421) — skip Index clone when `in_field_access_object`; tip-out pending regen |
| P1 | **`encode(grid)` must not force `grid.clone()`** | `bug_wdb360_module_file_encode_must_not_force_grid_clone_test` | 🆕 RED / filed (P3.417); MultiFile GREEN; tip RED; twin WDB-335 |
| P1 | **usize counter must not emit `(i as usize)`** | `bug_wdb361_module_file_usize_counter_must_not_cast_as_usize_test` | ✅ MultiFile GREEN (P3.419) — skip usize while-cast; tip-out pending regen |
| P1 | **indexed `&self` method must not `].clone().mesh_id()`** | `bug_wdb362_module_file_index_method_must_not_clone_element_test` | 🆕 RED / filed (P3.418); MultiFile GREEN; tip RED; twin WDB-359 |
| P1 | **indexed tuple field must not `].clone().rotation`** | `bug_wdb363_module_file_index_tuple_field_must_not_clone_element_test` | 🆕 RED / filed (P3.418); MultiFile GREEN; tip RED; twin WDB-359 |
| P1 | **usize lit must not emit `N_usize as usize`** | `bug_wdb364_module_file_usize_lit_must_not_cast_as_usize_test` | 🆕 RED / filed (P3.419); MultiFile GREEN; tip RED; twin WDB-361/352 |
| P1 | **statement args must not emit `__wj_tmpN` lets** | `bug_wdb365_module_file_must_not_emit_wj_tmp_lets_test` | 🆕 RED / filed (P3.419); MultiFile GREEN; tip RED |
| P1 | **BT tick must not force `tree.clone()`** | `bug_wdb366_module_file_bt_tick_must_not_force_tree_clone_test` | 🆕 RED / filed (P3.419); MultiFile GREEN; tip RED; twin WDB-360 |
| P1 | **`None` must not emit `None.clone()`** | `bug_wdb367_module_file_none_must_not_emit_clone_test` | ✅ MultiFile + tip GREEN (P3.427) — unit keywords + `Type::None` identifier paths |
| P1 | **`string` into `&str` must not emit `&*ident`** | `bug_wdb368_module_file_string_must_not_emit_star_deref_ref_test` | ✅ MultiFile GREEN (P3.419+); tip-out pending regen |
| P1 | **string field eq must not `.key.clone() ==`** | `bug_wdb369_module_file_string_field_eq_must_not_clone_test` | ✅ MultiFile GREEN (P3.421) — honor `suppress_borrowed_clone` on index-field; tip-out pending regen |
| P1 | **indexed Copy field must not `].clone().coord.clone()`** | `bug_wdb370_module_file_copy_field_must_not_double_clone_test` | ✅ isolate GREEN (P3.429) — `is_type_copy` skip on Copy aggregates; tip-out pending regen |
| P1 | **indexed Copy Vec3 must not `].clone().position.clone()`** | `bug_wdb371_module_file_copy_vec3_field_must_not_double_clone_test` | ✅ isolate GREEN (P3.429); tip-out pending regen; twin WDB-370/355 |
| P1 | **indexed enum match must not `].value.clone()`** | `bug_wdb372_module_file_index_enum_match_must_not_clone_scrutinee_test` | 🆕 RED / filed (P3.426); twin WDB-351/359 |
| P1 | **indexed String field must not `].clone().path.clone()`** | `bug_wdb373_module_file_index_string_field_must_not_double_clone_test` | 🆕 RED / filed (P3.426); twin WDB-370 |
| P1 | **indexed Copy enum must not `].clone().state.clone()`** | `bug_wdb374_module_file_copy_enum_field_must_not_double_clone_test` | 🆕 RED / filed (P3.427); twin WDB-370 |
| P1 | **nested index must not `].clone().bindings[j].clone().binding_type.clone()`** | `bug_wdb375_module_file_nested_index_must_not_clone_chain_test` | 🆕 RED / filed (P3.427); twin WDB-370/374 |
| P1 | **indexed Copy u32 must not `.buffer_id.clone()`** | `bug_wdb376_module_file_copy_u32_index_field_must_not_clone_test` | 🆕 RED / filed (P3.427); twin WDB-343/346 |
| P1 | **wj-sync int literals must emit i64 peers** | `bug_wj_sync_int_literal_peers_must_emit_i64_test` | ✅ tip GREEN (P3.370 + P3.380) — void `AtomicI64::new`/`fetch_add` i64 peers
| P1 | **owned Vec reuse into owned callee in `if` must clone** | `bug_owned_vec_reuse_into_owned_callee_must_clone_test` | ✅ tip GREEN (P3.373) — WDB-281 class |
| P1 | **theme hex `hi * 16 + lo` must not mix i64 + i32** | `bug_theme_hex_byte_arith_must_stay_one_int_width_test` | ✅ tip GREEN (P3.371) |
| P1 | **if/else float lit must peer f32 then-branch** | `bug_let_if_else_f32_branch_must_peer_else_float_literal_test` | ✅ tip GREEN (P3.374) |
| P1 | **`for i in 0..vec.len()` must not emit `0_i32..len()`** | `bug_for_zero_to_len_must_not_emit_i32_range_test` | ✅ tip GREEN (P3.359) |
| P1 | **HashMap None arm `0` must be `0_i64` for int values** | `bug_hashmap_int_none_zero_must_emit_i64_test` | ✅ tip GREEN (P3.361) — tuple peer-drive / nested return int width |
| P1 | **u32 mesh arith must not take i32 literal peers** | `bug_u32_arith_must_not_take_i32_literal_peers_in_coord_fn_test` | ✅ tip GREEN (P3.360) |
| P1 | **`u32::wrapping_*` args must peer u32 in i32-coord files** | `bug_u32_method_call_literals_must_peer_u32_in_mixed_i32_fn_file_test` | ✅ tip GREEN (P3.366–367) |
| P1 | **u32 compare/bitwise literals must peer u32 in mixed i32 files** | `bug_u32_compare_and_bitwise_must_peer_u32_in_mixed_i32_fn_file_test` | ✅ tip GREEN (P3.367) |
| P1 | **i32-coord → i64/u32 formals (ECS/FFI)** | `bug_i32_binding_into_i64_and_u32_formal_must_cast_test` | ✅ tip GREEN (P3.368) |
| P1 | **i64 entity compare / call lit + `idx + 1` index cast** | `bug_i64_entity_literal_peers_and_index_add_test` | ✅ tip GREEN (P3.369) — prefer i64 over i32 lit; index i64+lit → usize |
| P1 | **module-file owned FFI Vec forwarder must stay owned** | `bug_wdb216_module_file_owned_ffi_wrapper_must_stay_owned_test` | ✅ tip GREEN (P3.369); twin WDB-216/275 |
| P1 | **`--module-file` must honor cross-crate demoted sql_exec** | `bug_wdb244_module_file_must_honor_cross_crate_demoted_sql_exec_test` | ✅ tip GREEN (P3.364) — bare-pass must not bind `Type::method` to free fns |
| P1 | **u32 `while i < count` must not cast bound `as i64`** | `bug_u32_while_counter_vs_bound_must_not_cast_bound_as_i64_test` | ✅ tip GREEN (P3.348) — u32 loop counter width sync + compare prefer u32 |
| P1 | **i32 `while` vs `.len()` / literal bounds must not emit `as i64`** | `bug_i32_while_len_and_literal_bound_must_not_emit_i64_test` | ✅ tip GREEN (P3.352) — struct-return must not block i32 loop counters |
| P1 | **i32 coord / GPU dim literal peers must not emit `_i64`** | `bug_i32_coord_literal_peers_must_not_emit_i64_test` | ✅ tip GREEN (P3.356) — coord prefer-i32 same-width literal peers |
| P1 | **generated Cargo.toml release profile must default LTO** | `bug_generated_cargo_toml_release_profile_lto_test` | ✅ tip GREEN (P3.355) |
| P1 | **format temps → demoted `hash_join_semi` `&str` must borrow** | `bug_wdb246_module_file_format_temp_into_demoted_hash_join_must_borrow_test` | ✅ tip GREEN (P3.365) — module-file + tip-out; twin WDB-244 |
| P1 | **demoted `&Vec` → owned `ecs_soa_archetype_new` must clone** | `bug_wdb241_module_file_demoted_vec_into_owned_ecs_archetype_must_clone_test` | 🆕 RED / filed (P3.320); twin WDB-224 |
| P1 | **demoted `&str` vertex_id_name → owned from_ids_labels must `.to_string()`** | `bug_wdb242_module_file_demoted_str_into_owned_record_batch_name_must_to_string_test` | 🆕 RED / filed (P3.320); twin WDB-240 |
| P1 | **federation demoted `&Vec` return → owned `Vec` must clone** | `bug_wdb243_module_file_demoted_vec_return_into_owned_must_clone_test` | 🆕 RED / filed (P3.320) |
| P1 | **owned Vec → demoted materialize `&Vec` must borrow** | `bug_wdb239_module_file_owned_vec_into_demoted_materialize_must_borrow_test` | 🆕 RED / filed (P3.319); twin WDB-205/238 |
| P1 | **demoted `&str` sql → owned `relational_sql_parse_ast` must `.to_string()`** | `bug_wdb240_module_file_demoted_str_sql_into_owned_parse_ast_must_to_string_test` | 🆕 RED / filed (P3.319); twin WDB-191 |
| P1 | **demoted `&Vec<u32>` → owned `ecs_soa_archetype_new` must clone** | `bug_wdb241_module_file_demoted_vec_into_owned_ecs_archetype_must_clone_test` | 🆕 RED / filed (P3.320); twin WDB-224 |
| P1 | **demoted `&str` vertex_id_name → owned record-batch ctor must `.to_string()`** | `bug_wdb242_module_file_demoted_str_into_owned_record_batch_name_must_to_string_test` | 🆕 RED / filed (P3.320); twin WDB-240 |
| P1 | **demoted `&Vec` return → owned `Vec` must clone (federation)** | `bug_wdb243_module_file_demoted_vec_return_into_owned_must_clone_test` | 🆕 RED / filed (P3.320); twin WDB-205 |
| P1 | **`for x in parent.field` then use `parent` partial-moves Vec** | `bug_module_file_for_in_struct_vec_field_must_not_partial_move_parent_test` | ✅ tip GREEN (P3.321); borrow field when owner used after loop |
| P1 | **`vec.len() > 0` uint/int mix** | `bug_module_file_vec_len_gt_zero_must_not_mix_uint_int_test` | ✅ tip GREEN (P3.322); product used `is_empty()` interim |
| P1 | **`for x in (cx - r - 2)..(cx + r + 2)` i32 bounds must not split i64/i32** | `bug_i32_range_bounds_sub_add_must_not_split_i64_i32_test` | ✅ tip GREEN (P3.318); twin P3.313 |
| P1 | **`inbound.clone()` → demoted `pg_wire_frame_total_len` must borrow** | `bug_wdb238_module_file_owned_inbound_clone_into_demoted_frame_total_len_must_borrow_test` | ✅ tip GREEN (P3.316) — owned formal + clone OK |
| P1 | **Nested `while` + `substring(s, i, i+1)` still `1_i32` / `+= 1 as i32` (`wj-duration`)** | `bug_module_file_nested_while_substring_i_plus_one_must_not_emit_i32_test` | ✅ tip GREEN (P3.315) — nested loops emit `1_usize` |
| P1 | **`total + (n * mult)` emits `(n * mult) as i32` into i64 (`wj-duration`)** | `bug_module_file_int_mul_into_int_acc_must_not_cast_i32_test` | ✅ tip GREEN (P3.317) — int mul stays i64 |
| P1 | **`HashMap::get` through `MutexGuard` must borrow key (not `.to_string()`)** | `bug_hashmap_get_through_mutex_guard_must_borrow_key_test` | ✅ tip GREEN (2026-09-15) — P3.288 restored (`Some`/`Ok` infer + map-key not defeated by owned get homonym)
| P1 | **Library multipass SharedMap get/has still `key.to_string()`** | `bug_module_file_shared_map_get_must_borrow_key_test` | ✅ tip GREEN (2026-09-15 recheck) — was P3.301 RED |
| P1 | **`recv_int(rx)` reassign emits `rx.clone()` on non-Clone Receiver (`wj-sync`)** | `bug_module_file_recv_reassign_must_move_not_clone_receiver_test` | ✅ tip GREEN (P3.310) — no blanket clone into owned slots |
| P1 | **`while i < parts.len()` + `parts[i]` emits `i += 1 as i32` (`wj-dotenv`)** | `bug_module_file_vec_index_loop_must_not_add_i32_to_usize_test` | ✅ tip GREEN (P3.311) — usize index increment width |
| P1 | **`for zi in 0..(zd + 1)` emits `zd as i64 + 1_i32` (`mesh_primitives`)** | `bug_i32_range_end_add_must_not_split_i64_i32_test` | ✅ tip GREEN (P3.313) — range-end width unified |
| P1 | **`while i < errors.len()` + `if i == 0` emits `0_i32` (`wj-validate`)** | `bug_module_file_usize_index_eq_zero_must_not_emit_i32_test` | ✅ tip GREEN (P3.314) — usize_variables beats return-inferred Int32 |
| P1 | **u32 ± untyped int literal must not emit `_u64` peers** | `bug_u32_arith_int_literal_must_not_emit_u64_test` | ✅ tip GREEN (P3.327) — `Type::Uint` → `U32` suffix |
| P1 | **`wj build --release` must pass `--release` to cargo** | `bug_wj_build_release_must_invoke_cargo_release_test` | ✅ tip GREEN (P3.350) — `cli/build.rs` passes `--release` |
| P1 | **`string[0..1]` / `substring(0,1)` slice bounds must not emit `_i64` (`finance-screens` theme)** | `bug_str_slice_range_literals_must_not_emit_i64_test` | ✅ tip GREEN (P3.354) — `in_index_context` + `maybe_cast_index_to_usize` on substring lowering |
| P1 | **assign generic `send` must not inject unbound `Sender<T>`** | `bug_generic_assign_must_not_inject_unbound_t_test` | ✅ tip GREEN (P3.342) |
| P1 | **generic `send<T>(…, value: T)` must not demote to `&T` + clone** | `bug_generic_channel_send_owned_param_must_not_demote_to_ref_test` | ✅ tip GREEN (P3.331) |
| P1 | **generic `recv` must move `Receiver`, not `rx.clone()`** | `bug_generic_channel_recv_must_move_receiver_not_clone_test` | ✅ tip GREEN (P3.332) |
| P1 | **`wj-timefmt` product: `month <= 12_i32` / `&parts[1].to_string()`** | `bug_module_file_timefmt_product_must_not_mix_i32_month_or_ref_string_test` | ✅ tip GREEN (P3.329) |
| P1 | **demoted `&str` + `core = strings.substring(...)` must own (`wj-semver`)** | `bug_module_file_demoted_str_substring_assign_must_own_test` | ✅ tip GREEN (P3.325) — tuple Result return owns demoted bind |
| P1 | **usize `start = i + 1` emits `1_usize as i32/i64` (`wj-toml`)** | `bug_module_file_usize_i_plus_one_assign_must_stay_usize_test` | ✅ tip GREEN (P3.326) — reconcile `_usize` emit vs Int local |
| P1 | **Local `buf` into MutBorrowed `Vec` method must be `&mut buf`** | `auto_mut_borrow_arg_test` | ✅ tip GREEN (P3.312) |
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









## P3.312 (2026-09-16) — auto_mut local `buf` into MutBorrowed Vec must be `&mut buf`

| Gate | Status |
|------|--------|
| `auto_mut_borrow_arg_test::test_local_var_passed_to_mut_param_gets_mut_borrow` | ✅ tip GREEN (2026-09-16) — was `&buf` / `buf.clone()` / `&mut buf.clone()` |

**Root cause layer:** signature + constraint (not peel-first)
1. `runtime_wj_owned_rust_borrowed_param` treated `MutBorrowed` like shared AsRef → Region(8) `Ref` / `&buf`
2. `emitted_owned_arg_contract` claimed owned for MutBorrowed bare Vec when `emitted_rust_ref_params=false`
3. `callee_emits_shared_rust_ref_param` treated `MutableReference` as shared
4. Registry AST-bare peel + `ensure_owned_move_clone_for_reuse` undid IR `MutBorrow` into `.clone()`

**What became unnecessary:** Region(8) demotion of MutBorrowed; MutBorrowed-Vec owned claim; shared-ref classification of `&mut T`; owned peel / reuse-clone on MutBorrow slots

**Gates:** `cargo test --release --test all -- auto_mut_borrow_arg_test bug_cross_crate_mut_borrow_module_fn_test`; lib `mut_borrowed_vec_is_not_runtime_shared_borrow` + `mut_borrowed_bare_vec_stays_mut_ref_at_call_site`

## P3.323 (2026-09-16) — inferred i32 loop counters + i32 sentinel vs field (`windjammer-game-core`)

| Gate | Status |
|------|--------|
| `i32_inferred_loop_counter_and_sentinel_priority_must_stay_i32` | ✅ tip GREEN (2026-09-16) |
| Product `tps_camera.wj` `let mut dy = 0` + `while dy < 3` | ✅ `while dy < 3_i32` (no `3_i64`) |
| Product `reverb_zones.wj` `priority > best_priority` | ✅ i32 compare (no `priority as i64`) |
| Breach `wj game build --release` (tip `.cargo-target-wj`) | **640** rustc errors (was **1000** cap / ~550 E0308+E0277 in prior log); binary still blocked |

**Root cause:** WJ `Type::Int` locals could emit as `0_i32` while `local_var_types` stayed ambiguous `Int` (i64 promotion on while bounds); i32 field vs WJ `int` sentinel compared via `(field as i64) > sentinel`, forcing i64 inference and i32 assign failures.

**Fix:** Reconcile `Int`→`Int32` after let when RHS is i32; promote `while i < N` counters; seed i32 literal peers in while conditions; prefer i32 compare when peer is i32 field and other side is ambiguous `int` local.


## P3.352 (2026-09-17) — i32 `while` vs `.len()` / literal bounds must not emit `as i64`

| Gate | Status |
|------|--------|
| `bug_i32_while_len_and_literal_bound_must_not_emit_i64_test` | ✅ tip GREEN (2026-09-17) |

**Product:** csg `emit_instruction`, perlin perm init, `for i < vec.len()` mirrors.

**Root cause layer:** codegen int-width — `function_returns_i32_for_loop_scan` early-returned when the fn returned a struct whose fields included WJ `int`, so `while i < 512` in `PermTable::new` stayed i64.

**Fix:** do not block i32 loop-counter promotion solely because the return struct has `int` fields.

**Gates:** `cargo test --release --test all -- i32_while_len_and_literal_bound` → pass.

## P3.350 (2026-09-17) — `wj build --release` must invoke `cargo build --release`

| Gate | Status |
|------|--------|
| `bug_wj_build_release_must_invoke_cargo_release_test` | ✅ tip GREEN (2026-09-17) |
| Product impact | `wj-sync` fair benches; eco packages need release for ≤1.2× claims |

**Root cause:** `cli/build.rs` previously discarded `_release` and always ran `cargo build` (dev).

**Fix:** Pass `--release` to cargo when `-r`/`--release` is set; print profile in progress line.

**Verify:**

```bash
cargo test --release --test all -- bug_wj_build_release_must_invoke_cargo_release -- --nocapture
```

## P3.346 (2026-09-17) — owned String field assign from demoted `&str`

| Item | Status |
|------|--------|
| Gate `bug_module_file_demoted_str_field_assign_must_to_string_test` | ✅ tip GREEN (2026-09-17) — loop-reuse demotion fixture |
| Product | `asset_browser.search` / Breach String←&str residual |

**Root cause layer:** constraint/auto_clone — demoted `&str` params hit auto-clone on owned `String` field assign → `query.clone()` (still `&str`).

**Fix:** when LHS is owned `String` and RHS ident is inferred-borrowed, emit `.to_string()` before/instead of `.clone()`.

**Gates:** `cargo test --test all --features integration_tests -- module_file_demoted_str_field_assign_must_to_string` → pass.

## P3.348 (2026-09-17) — u32 `while i < count` must not cast bound `as i64`

| Gate | Status |
|------|--------|
| `bug_u32_while_counter_vs_bound_must_not_cast_bound_as_i64_test` | ✅ tip GREEN (2026-09-17) |
| Ecosystem | `frame_analysis` / `weight_paint` mirror loops |

**Root cause:** `let mut i = 0` kept WJ `Int` (i64) in `local_var_types` while numeric inference emitted `0_u32`; mixed-int compare promoted to i64 and cast u32 bounds (`count`, `half`).

**Fix:** `reconcile_ambiguous_int_local_after_let` syncs `_u32`; `promote_ambiguous_int_loop_counter_in_while_condition` binds u32 peers; `comparison_should_prefer_u32_over_i64` in binary compare (P3.327/P3.343 pattern).

## P3.347 (2026-09-17) — `cy + dy` in nested for-range must not widen to i64

| Gate | Status |
|------|--------|
| `bug_i32_cy_plus_dy_for_range_must_not_widen_to_i64_test` | ✅ tip GREEN (2026-09-17) — covered by P3.343 i32 range binding |
| Ecosystem | `component_viewer_controls` ring body `let y = cy + dy` |

**Regression lock:** untyped `let cy = 10` + `for dy in 0..2` must keep `cy + dy` as i32 (no `as i64`).

## P3.343 (2026-09-17) — nested i32 range loops emit `_i64` on `==` / `%` / `+` literals

| Gate | Status |
|------|--------|
| `bug_i32_nested_range_eq_mod_literals_must_not_emit_i64_test` | ✅ tip GREEN (2026-09-17) |
| Ecosystem | `component_viewer_controls` vents/checker nested `for dy in 0..3` |

**Root cause layer:** codegen int-width — pure `0..N` ranges (bool-returning fns) never entered `function_returns_i32_for_loop_scan`, so counters stayed WJ `int` and peers defaulted to `_i64`.

**Fix:** bind small literal-only ranges as `Int32` + `codegen_i32_binding_names`; peer-type recurse through arith/mod so `(dx + dz) % 2 == 0` stays i32.

**What became unnecessary:** `_i64` compare/mod/add literals beside i32 range counters.

**Gates:** `cargo test --test all --features integration_tests -- i32_nested_range_eq_mod_literals_must_not_emit_i64` (+ sibling i32 range gates) → pass.

## P3.345 (2026-09-17) — void impl `while i < seg` after if/else i32 clamp emits `_i64`

| Gate | Status |
|------|--------|
| `bug_module_file_void_while_i32_seg_counter_must_not_emit_i64_test` | ✅ tip GREEN (2026-09-17) |
| Ecosystem | `debug_renderer.draw_circle_xz` `let seg = if segments < 4 { 4 } else { segments }` |

**Root cause layer:** codegen int-width — block-tailed if/else lets in void methods never registered `codegen_i32_binding_names`, so `i + 1` defaulted to `_i64`.

**Fix:** detect if/else branches that are i32-width (literal + i32 formal/binding); bind `Int32` + `codegen_i32_binding_names` on the let name.

**Gates:** `cargo test --test all --features integration_tests -- module_file_void_while_i32_seg_counter_must_not_emit_i64` → pass.

## P3.342 (2026-09-17) — assign generic `send` injects unbound `Sender<T>`

| Item | Status |
|------|--------|
| Gate `bug_generic_assign_must_not_inject_unbound_t_test` | ✅ tip GREEN (2026-09-17) |
| Blocks | ergonomic named binds from `clone_sender` / `send` in `wj test` |

**Root cause layer:** constraint/type-inference — call return types kept unbound `Sender<T>`; let ascription emitted them.

**Fix:** instantiate fn return from call-arg bindings; skip let ascriptions that still contain unbound type params; gate checks let ascriptions only (struct/`fn` may still say `Sender<T>`).

**What became unnecessary:** emitting `: Sender<T>` on non-generic lets.

**Gates:** `cargo test --release --test all -- generic_assign_must_not_inject` → pass.

## P3.332 (2026-09-16) — generic `recv` must move Receiver (not `rx.clone()`)

| Item | Status |
|------|--------|
| Gate `bug_generic_channel_recv_must_move_receiver_not_clone_test` | ✅ tip GREEN (2026-09-17) |
| Blocks | generic `wj-sync` `Receiver<T>` roundtrip |

**Root cause layer:** constraint/auto_clone — exclusive Match arms each move `rx` once; do not treat as same-stmt multi-move. Projection-parent reads of `rx` in `rx.rx.recv()` must not force clone.

**Gates:** `cargo test --release --test all -- generic_channel_recv_must_move` → pass.

## P3.331 (2026-09-16) — generic `send<T>(…, value: T)` demoted to `&T` + clone

| Item | Status |
|------|--------|
| Gate `bug_generic_channel_send_owned_param_must_not_demote_to_ref_test` | ✅ tip GREEN (2026-09-17) |
| Blocks | generic `wj-sync` `Channel<T>` / std::sync graduation |

**Root cause layer:** signature/boundary — `is_generic_type_param` missed `Type::Generic("T")` (only handled `Custom`), so analyzer demoted owned `T` to borrowed; codegen borrow-delegation also skipped generics.

**What became unnecessary:** `&T` + `.clone()` demotion for bare type params.

**Gates:** `cargo test --release --test all -- generic_channel_send_owned_param` → pass.

## P3.329 (2026-09-16) — `wj-timefmt` product month `12_i32` / `&String` into String

| Change | Status |
|--------|--------|
| Ecosystem: `wj-timefmt` | ✅ tip GREEN (2026-09-17) |
| Gate `bug_module_file_timefmt_product_must_not_mix_i32_month_or_ref_string_test` | ✅ tip GREEN (2026-09-17) |

**Root cause layer:** constraint/solver (int-width from struct-int return) + coercion (owned string index) + temporary reconcile narrow
- Struct returns with WJ `int` fields → `int_width_hint_from_return_type_resolved` keeps i64 locals (`month`).
- Do not prefer i32 over Type::Int identifiers; skip while-promote for struct-int returns.
- Call `int` results (`plus_pos = find_*`) must not reconcile to usize; cast at usize formals.
- `coerce_expr_to_owned_string` / vec-index fixup emit `parts[i].to_string()` (strip `&`).
- Usize init-source prepass: `let mut i = clock_end` inherits usize from `while i < len`.

**What became unnecessary:** `&parts[i].to_string()` peel; i32 prefer over WJ `int` idents.

**Gates:** `cargo test --release --test all -- module_file_timefmt_product_must_not_mix int_mod_literal_zero_compare i32_inferred_loop_counter int_find_pos_ge_zero`

## P3.327 (2026-09-16) — u32 ± int literal must not emit `_u64`

| Gate | Status |
|------|--------|
| `bug_u32_arith_int_literal_must_not_emit_u64_test` | ✅ tip GREEN (2026-09-16) |
| Note | Product: half_edge / mesh_primitives / steering |

**Root cause layer:** coercion/encoding (int-width from assignment target)
- `int_type_from_assignment_target`: WJ `Type::Uint` was mapped to `IntType::U64` → literal peers emitted `_u64` into `u32` locals/fields.
- Fix: `Type::Uint => IntType::U32` (matches Rust lowering of bare `u32` / `uint`).
- Also strip `_u64`/`_u32` in `strip_compound_assign_int_literal_suffix` so compound `+=` can re-suffix from target width.

**What became unnecessary:** no new reconcile peel; width comes from typed assignment target.

**Gates:** `cargo test --release --test all -- u32_arith_int_literal_must_not_emit_u64 module_file_recv_reassign i32_compound_add_must_not_use_usize` → 3 passed.



## P3.336 (2026-09-17) — `int` `% 10` / `digit == 0` in `while n > 0` must not split i64 vs i32 (LedgerKit)

| Gate | Status |
|------|--------|
| `int_mod_literal_zero_compare_must_not_split_i64_i32` | ✅ tip GREEN (2026-09-17) |
| Product LedgerKit `make api-check` | ✅ GREEN on tip |
| P3.323 regression guard | `i32_inferred_loop_counter_and_sentinel_priority_must_stay_i32` ✅ |

**Root cause:** P3.323 `promote_ambiguous_int_loop_counter_in_while_condition` promoted every WJ `int` local in `while n > 0`, including `let mut n = value` (i64 param copy), to i32 width for literal peers.

**Fix:** Track literal-init loop counters only (`literal_init_wj_int_loop_counters`); keep explicit `: int` locals on i64 through prepass/assign/reconcile (`explicit_wj_int_annotated_locals`, prepass let `type_` registration).

**Gates:** `cargo test --release --test all -- int_mod_literal_zero_compare_must_not_split_i64_i32 i32_inferred_loop_counter_and_sentinel_priority_must_stay_i32 int_arith_must_not_split_i64_i32` → pass; LedgerKit `make api-check` GREEN.


## P3.341 (2026-09-17) — auto-ref: no `&*` / `&(ref).field` on Copy into `&T`

| Gate | Status |
|------|--------|
| `auto_ref_deref_copy_test::test_deref_copy_no_extra_ref` | ✅ tip GREEN (2026-09-17) |
| `auto_ref_deref_copy_test::test_deref_field_copy_no_extra_ref` | ✅ tip GREEN (2026-09-17) |

**Root cause layer:** signature/formal — pure-forwarding demotion emitted `&usize`/`&Entity` for Copy scalars/aggregates forwarded to `Vec::contains`; call-site actual then treated demoted Copy aggregates as `OwnedType::Copy` → Borrow → `&*entity`.

**What became unnecessary:** Relying on post-IR `&*` peels for explicit-deref Copy args when formals stay owned (rustc auto-borrows).

**Fix:** Gate `func_is_pure_forwarding_delegate` + `param_should_emit_borrowed_delegation_formal` on `!is_copy_pass_by_value_formal`; honor `emitted_rust_ref_formals` as Ref in `infer_actual` for Copy aggregates; strip redundant `&` on Copy field projections / explicit deref at method call sites.

**Gates:** `cargo test --release --test all -- auto_ref_deref_copy_test` → 2 passed.

## P3.338 (2026-09-17) — i32 loop arith + `i < params.len()` must not emit i64 / `len() as i64`

| Gate | Status |
|------|--------|
| `i32_loop_arith_and_len_compare_must_not_emit_i64` | ✅ tip GREEN (2026-09-17) |
| Product track `j + 1` / scene `i < params.len()` / debug_draw `0..14` | ✅ i32 peers; `len() as i32` |

**Root cause layer:** constraint/type-inference — `codegen_i32_binding_names` + narrow signed width for len compare (cast `.len()` to `i32`, not promote counter to i64 via unsigned-width path).

**What became unnecessary:** Preferring `expression_unsigned_width_for_len_cast` (→ i64) when the compare peer is already an i32 binding.

**Fix:** `expression_narrow_signed_width_for_len_compare`; for-range bind Int32 for all `-> i32` scan loops; while-condition peer promotion via `expression_has_i32_width_in_tree`; exclude Int32 from signed→usize len widen.

**Gates:** `cargo test --release --test all -- i32_loop_arith_and_len_compare` → 1 passed.

## P3.337 (2026-09-17) — `-> i32` + `for i in 0..vec.len()` must cast len end (not widen start to usize)

| Gate | Status |
|------|--------|
| `i32_for_range_len_end_must_cast_to_i32` | ✅ tip GREEN (2026-09-17) |
| Breach `wj game build` | **416** errors (was **466**); i32←usize **~2** (was **34**) |

**Fix:** `generate_range` `i32_scan_len_range`; for-loop bind `Int32` when `-> i32` + len end; P3.335 while/let/index peers.

## P3.335 (2026-09-17) — `-> i32` scan loops: i32 counters vs `.len()` (not `0_usize`)

| Gate | Status |
|------|--------|
| `i32_return_while_len_counter_must_not_emit_usize` | ✅ tip GREEN (2026-09-17) |
| Product autotiler / behavior_tree while scans | ✅ `(i as usize) < len` + `0_i32` init |

**Fix:** `function_returns_i32_for_loop_scan`, let/while/binary compare casts, `codegen_i32_binding_names`, index `as usize`.

## P3.334 (2026-09-17) — i32 `for i in 0..seg` loop counter must not widen to i64 literals

| Gate | Status |
|------|--------|
| `i32_for_range_counter_arith_must_not_emit_i64` | ✅ tip GREEN (2026-09-17) |
| Product `mesh_primitives.wj` `((i + 1) % seg)` | ✅ `+ 1_i32` (was `+ 1_i64`) |
| Breach `wj game build` | **466** errors (was **495**); i32←i64 **38** (was **51**); u32←i64 **14** (was **29**) |

**Root cause:** Range loop `local_var_types` used `infer_expression_type(start)` first; literal `0` → WJ `Int`, ignoring `seg: i32`.

**Fix:** `range_loop_int_counter_type` in `for_statement_generation.rs` — prefer i32/u32 when either bound is fixed width.

**Gates:** `cargo test --release --test all -- i32_for_range_counter_arith_must_not_emit_i64` → 1 passed.

## P3.330 (2026-09-16) — Vec subscript index must not emit `as f32`

| Gate | Status |
|------|--------|
| `vec_index_must_not_cast_subscript_to_f32` | ✅ tip GREEN (2026-09-16) |
| Product `lod_generator.wj` `mesh.vertices[vb + 1]` | ✅ `(vb + 1) as usize` (was `as f32`) |
| Breach `wj game build` | **495** errors (was **516**); `[f32] indexed by f32` **0** |

**Fix:** `maybe_cast_index_to_usize` rewrites call-site float coercion leaks (`as f32`/`as f64` → `as usize` on index expr).

## P3.326 (2026-09-16) — usize `start = i + 1` emits `1_usize as i64/i32` (`wj-toml`)

| Change | Status |
|--------|--------|
| Ecosystem: `wj-toml` scanners | ✅ tip GREEN (2026-09-16) |
| Gate `bug_module_file_usize_i_plus_one_assign_must_stay_usize_test` | ✅ tip GREEN (2026-09-16) |

**Root cause layer:** constraint/type write-back
- Untyped `let mut start = 0` under `-> string` recorded WJ `Int` while numeric inference emitted `0_usize`.
- Assign `start = i + 1_usize` then `maybe_cast_usize_to_int_target` appended ` as i64` (precedence → `i + (1_usize as i64)`).
- Regular assigns did not set `assignment_int_target_type` (compound assigns did).

**Fix:** reconcile `_usize` emit → `local_var_types`/`usize_variables`; set int assignment context on plain assigns.

**What became unnecessary:** post-assign `as i64` peel on usize index locals.

**Gates:** `cargo test --release --test all -- bug_module_file_usize_i_plus_one_assign_must_stay_usize bug_module_file_demoted_str_substring_assign_must_own u32_arith_int_literal_must_not_emit_u64` → 3 passed.

## P3.325 (2026-09-16) — demoted `&str` substring assign into `String` (`wj-semver`)

| Change | Status |
|--------|--------|
| Ecosystem: `wj-semver` `split_build` / `split_pre` | ✅ tip GREEN (2026-09-16) |
| Gate `bug_module_file_demoted_str_substring_assign_must_own_test` | ✅ tip GREEN (2026-09-16) |

**Root cause layer:** constraint / owned-string coercion
- `return_type_expects_owned_string` ignored `Result<(string, string), _>` tuples → skipped coerce on `let mut core = text`.
- Auto-clone then emitted `text.clone()` (`&str`→`&str`); substring `String` assigns / `Ok((core,…))` failed.

**Fix:** tuple payloads count as owned-string returns; rewrite demoted `&str` `.clone()` → `.to_string()` on let binds; skip auto-clone after `.to_string()`.

**What became unnecessary:** relying on substring-site `.to_string()` peels when the initial bind was wrong.

**Gates:** same filter as P3.326 → GREEN.
## P3.314 (2026-09-16) — usize index `i == 0` emits `0_i32` (`wj-validate`)

| Change | Status |
|--------|--------|
| Ecosystem: `wj-validate` `all_ok` | ✅ tip GREEN (2026-09-16) — unblocked with gate |
| Gate `bug_module_file_usize_index_eq_zero_must_not_emit_i32_test` | ✅ tip GREEN (2026-09-16) |

**Root cause layer:** constraint/type write-back (int width) — prepass `usize_variables` for index/len counters was overwritten by return-inferred `Int32` at `let i = 0`.
**What became unnecessary:** relying on literal peels alone; `usize_variables` now wins in let binding, peer-type, and compound-assign width resolution (with P3.311).
**Gates:** `cargo test --release --test all -- module_file_usize_index_eq_zero_must_not_emit_i32 module_file_vec_index_loop_must_not_add_i32_to_usize i32_compound_add_must_not_use_usize` → 3 passed.

**Compiler agent:** when loop index is used as `errors[i]` / compared to `.len()`, keep compare-to-zero literal in the same width as `i` — emit `i == 0` / `i == 0_usize`, never `0_i32`.

## P3.313 (2026-09-16) — i32 range end `zd + 1` emits `zd as i64 + 1_i32`

| Change | Status |
|--------|--------|
| Ecosystem: windjammer-game-core `mesh_primitives` | ✅ tip GREEN (2026-09-16) |
| Gate `bug_i32_range_end_add_must_not_split_i64_i32_test` | ✅ tip GREEN (2026-09-16) |
| Note | Renumbered from colliding P3.312 (auto_mut MutBorrowed Vec is GREEN) |

**Compiler agent:** keep range-end `zd + 1` in one integer width — emit `(zd + 1)` as i32 (or both sides i64), never `zd as i64 + 1_i32`.

## P3.311 (2026-09-16) — vec-index loop emits `i += 1 as i32` onto usize (`wj-dotenv`)

| Change | Status |
|--------|--------|
| Ecosystem: `wj-dotenv` `parse` | ✅ tip GREEN (2026-09-16 recheck) |
| Gate `bug_module_file_vec_index_loop_must_not_add_i32_to_usize_test` | ✅ tip GREEN (2026-09-16) |
| Related | Inverse of P3.304; WDB-231 tip-out only — need multipass module-file gate |

**Compiler agent:** when loop index is used as `parts[i]` / compared to `.len()`, keep increment width matching the binding — emit `i += 1` (usize or i64), never `1 as i32` into a usize counter.

## P3.310 (2026-09-16) — `recv_int(rx)` reassign emits `rx.clone()` on non-Clone Receiver

| Change | Status |
|--------|--------|
| Ecosystem: `wj-sync` `drain_int_until_sentinel` / `channel_sum_range` | ✅ tip GREEN (2026-09-16) — unblocked with gate |
| Gate `bug_module_file_recv_reassign_must_move_not_clone_receiver_test` | ✅ tip GREEN (2026-09-16) — emits `recv_int(rx)` move |

**Root cause layer:** constraint / reuse analysis (reconcile shrink)
- `ensure_owned_move_clone_for_reuse` blanket-cloned every non-Copy identifier into owned slots (P3.303 overreach), bypassing auto_clone writeback (`let got = recv_int(rx); rx = got.0`).
- Channel `Receiver` wrappers correctly do not derive Clone — call site must move.

**What became unnecessary:** unconditional non-Copy ident→owned `.clone()`; keep only borrowed-iterator (P3.303) + writeback-aware auto_clone path.

**Gates:** `cargo test --release --test all -- module_file_recv_reassign_must_move_not_clone_receiver vec_push_borrowed_loop_elem_must_clone` → 2 passed

**Compiler agent:** when `rx` is rebound from `recv_int(rx)` return, pass by move — do not inject `.clone()` on non-Clone channel receivers.

## P3.308 (2026-09-16) — hexagonal env trait forward owned string

| Gate | Status |
|------|--------|
| `bug_env_trait_forward_owned_string_must_not_borrow_test` | ✅ tip GREEN (2026-09-16) — trait discard keeps owned temps (`_tempN`, not `&_tempN`) |
| Product interim | ✅ env_* + composition `arg + ""` (P3.267); tip `1c52c8e4` clears env `&_temp0` |

**Compiler agent:** trait-impl discard-only `let _ = tenant_id` must NOT claim shared-ref emission; format-temp hoist must pass owned `_tempN` into `String` slots.


## P3.301 (2026-09-15) — multipass SharedMap get/has re-emits key.to_string()

| Change | Status |
|--------|--------|
| Ecosystem: `wj-sync` SharedMap get/has | ✅ unblocked on tip (P3.301 GREEN); package blocked on P3.310 |
| Gate `bug_hashmap_get_through_mutex_guard_must_borrow_key_test` | ✅ tip GREEN via isolate `compile_single` |
| Gate `bug_module_file_shared_map_get_must_borrow_key_test` | ❌ tip RED (2026-09-15) — `--library --module-file` emits `key.to_string()` |

**Compiler agent:** library multipass must keep MutexGuard map-key borrow (same as isolate P3.288) — do not reintroduce `key.to_string()` when insert/len live in the same module.

## P3.318 (2026-09-16) — i32 range bounds `(cx - r - 2)..(cx + r + 2)` must not split i64/i32

| Gate | Status |
|------|--------|
| `bug_i32_range_bounds_sub_add_must_not_split_i64_i32_test` | ✅ tip GREEN |
| Note | Twin of P3.313 (`0..(zd + 1)`); component_viewer_controls / voxel ring loops |

## P3.317 (2026-09-16) — `total + (n * mult)` emits `(n * mult) as i32` (`wj-duration`)

| Change | Status |
|--------|--------|
| Ecosystem: `wj-duration` `parse_ms` | ✅ tip GREEN (2026-09-16) |
| Gate `bug_module_file_int_mul_into_int_acc_must_not_cast_i32_test` | ✅ tip GREEN (2026-09-16) — emits `total += n * mult` |
| Note | Nested substring width fixed (P3.315 GREEN); P3.316 is WDB-235–238 |

**Compiler agent:** when `int` lowers to i64, keep `n * mult` and accumulator updates in i64 — never cast the product to `i32`.

## P3.315 (2026-09-16) — nested while + substring `i+1` still emits `1_i32` (`wj-duration`)

| Change | Status |
|--------|--------|
| Ecosystem: `wj-duration` nested digit/unit scan | ✅ tip GREEN (2026-09-16) — emits `i + 1_usize` / `i += 1` |
| Gate `bug_module_file_nested_while_substring_i_plus_one_must_not_emit_i32_test` | ✅ tip GREEN (product residual moved to P3.317) |
| Note | P3.300 single-while isolate was already GREEN; nested outer+inner while now matches |

**Compiler agent:** usize index used as `substring` end `i+1` and compound `i = i + 1` inside nested whiles must keep one width — never `1_i32` peers / `+= 1 as i32` onto usize.

## P3.300 (2026-09-15) — substring end `i+1` emits `1_i32` into usize cast

| Change | Status |
|--------|--------|
| Ecosystem: `wj-duration` `parse_ms` digit scan | ✅ tip GREEN nested (P3.315); remaining mul-cast → P3.317 |
| Also hits | `wj-toml`, `wj-semver`, `wj-cli-args`, `wj-compress`, `wj-glob` (same `i + 1_i32` pattern) |
| Gate `bug_substring_end_i_plus_one_must_not_emit_i32_into_usize_test` | ✅ tip GREEN (2026-09-16 recheck) — shallow single-while only |
| Note | Shallower `int_increment` / haystack gates can GREEN; scanner `while` + `substring(…, i, i+1)` nested residual was P3.315 |

**Compiler agent:** when casting substring indices to `usize`, keep `i + 1` in one integer width — emit `(i + 1) as usize` (or both ends as i64), never `(i + 1_i32) as usize` when `i` is i64/`int`.

## P3.299 (2026-09-15) — int find-pos `>= 0` mixes usize cast with i64 zero

| Change | Status |
|--------|--------|
| Ecosystem: `wj-timefmt` `split_time_tz` / `plus_pos >= 0` | ✅ tip gate GREEN (2026-09-16) |
| Gate `bug_int_find_pos_ge_zero_must_not_mix_usize_i64_test` | ✅ tip GREEN (2026-09-16) — binding beats usize_variables; `strings::len`→i64 |
| Note | Also related int/usize loop arithmetic in same package |

**Compiler agent:** Windjammer `int` comparisons against `0` must stay in one integer width — do not cast LHS to `usize` while keeping `0_i64`.

## P3.298 (2026-09-15) — mut owned `Vec<u8>` return demoted to `&Vec<u8>`

| Change | Status |
|--------|--------|
| Ecosystem: `wj-uuid` `append_bytes` / v5 path | ✅ tip gate GREEN (2026-09-16) |
| Gate `bug_mut_owned_vec_u8_return_must_not_demote_to_ref_test` | ✅ tip GREEN (2026-09-16) — returned Vec stays owned |

**Compiler agent:** `mut out: Vec<u8>` that is mutated and returned must keep owned formal (Identity) — do not demote to `&Vec<u8>` when the value is moved out.

## P3.297 (2026-09-15) — `spawn(move ||)` strips `move` in Arc-clone worker loop

| Change | Status |
|--------|--------|
| Ecosystem: `wj-sync` shared-inbox Pool | ✅ tip gate GREEN (2026-09-16) |
| Gate `bug_module_file_spawn_move_in_worker_loop_must_be_preserved_test` | ✅ tip GREEN (2026-09-16) — While/Loop/Thread/Async in closure capture walk |
| Note | Shallow `inbox.clone()` + outer while was tip-GREEN; closure body starting with `while` needed While in capture walk |

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
| Ecosystem: `wj-sync` `SharedMap` insert/len/get/has | ✅ unblocked; tip drain blocked on P3.310 |
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
| Gate `bug_int_while_len_as_int_must_not_emit_usize_arith_test` | ✅ tip GREEN (2026-09-16) — `Type::Int`→I64 promotion + signed-vs-usize prefer i64 (not `h_len as usize`) |
| Product: bound owned locals into trait string formals; `len() as int` before while; typed nested indices | ⚠️ re-check tip api-check after P3.306 while-len / WDB-228 |
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

## P3.354 finance-screens — `string` slice range bounds must be `usize` (2026-09-17)

| Gate | Status |
|------|--------|
| `bug_str_slice_range_literals_must_not_emit_i64_test` — `pair[0..1]` / `hex_digit_value` | ✅ tip GREEN |
| `run_red_repro_bundle.sh` — `str_slice_range_literals_must_not_emit_i64` | ✅ |

**Root cause:** `substring(start,end)` lowering reused pre-codegen call args (`0_i64`) instead of re-emitting bounds with `in_index_context` (same path as `generate_index` range slices). `maybe_cast_index_to_usize` early-returned on literal AST nodes while the string still carried `_i64`.

**Fix:** `method_call_expression_generation/finalize.rs` — re-generate start/end with `in_index_context` + `maybe_cast_index_to_usize`.

## P3.369 WindjammerDB CQ-C5 — coverage REDs WDB-277–280 par_*_bind owned Vec (2026-09-18)

| Gate | Status |
|------|--------|
| Tip **WDB-277** CDLP `&Vec` → owned `par_cdlp_bind` | ✅ tip GREEN (P3.369) — tip-out owned formals + bare pass |
| Tip **WDB-278** WCC `&Vec` → owned `par_wcc_bind` | ✅ tip GREEN (P3.369) — tip-out owned formals + bare pass |
| Tip **WDB-279** SSSP `&Vec` → owned `par_sssp_bind` | ✅ tip GREEN (P3.369) — tip-out owned formals + bare pass |
| Tip **WDB-280** PageRank `&Vec` → owned `par_pull_bind` | ✅ tip GREEN (P3.369) — tip-out owned formals + bare pass |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** tip greens **177/218–280**; signature-driven clone into owned `graph_par_*_bind` formals (or demote FFI to slices). No Phase 606+.

## P3.370 (2026-09-18) — wj-sync WJ `int` literal peers must emit `_i64`

| Gate | Status |
|---|---|
| `bug_wj_sync_int_literal_peers_must_emit_i64_test` | ✅ tip GREEN (2026-09-18) |

**Root cause layer:** codegen int-width — `assignment_int_peer_from_formal` only peer-drove `i32`/`u32` call formals (not WJ `int`/`i64`), and `function_prefers_i32_coord_locals` treated `SharedInt`/`Counter` handle returns as i32-coord builders, demoting `1`/`0`/`2` to `_i32` in compare/mul/call/atomic args.

**Fix:** peer-drive `Type::Int` from i64 formals; treat custom returns that carry WJ int width (`SharedInt`, `Counter`, …) as non–i32-coord builders via `type_contains_wj_int_width`.

**Gates:** `cargo test --release --test all -- wj_sync_int_literal hashmap_none_zero vec_int_return std_sync_atomic_i64 generated_cargo_toml_release_profile_lto` → pass.


## P3.380 (2026-09-18) — void `AtomicI64::new` / `fetch_add` must emit `_i64`

| Gate | Status |
|---|---|
| `wj_sync_atomic_i64_void_main_literals_must_emit_i64` | ✅ tip GREEN (2026-09-18) |
| `bug_wj_sync_int_literal_peers_must_emit_i64_test` (full) | ✅ tip GREEN |
| `bug_std_sync_atomic_i64_wiring_test` | ✅ tip GREEN |

**Root cause layer:** void/i32-coord let context forced `0_i32` into `AtomicI64::new(0)`; `sync_i32_coord_binding_after_let` then retyped the `AtomicI64` local as `i32`, so `fetch_add(1)` also demoted.

**Fix:** associated-call owner peer (`AtomicI64::new`) overrides void i32 context; let RHS owner peer for AtomicI64 slots; refuse to overwrite non-int nominal locals in `sync_i32_coord_binding_after_let`; include `AtomicI64` in `type_contains_wj_int_width`.

**Gates:** `cargo test --release --test all -- bug_wj_sync_int_literal_peers` → 7/7; tip probe emits `0_i64`/`1_i64`.



## P3.365 WindjammerDB CQ-C5 — coverage REDs WDB-273–276 WCC afforest + LCC/arena/par_bfs owned (2026-09-17)

| Gate | Status |
|------|--------|
| Tip **WDB-273** WCC `csr.clone()` → demoted `&mut DenseCsr` afforest | ✅ tip GREEN (P3.365) — tip-prefer; gen lag |
| Tip **WDB-274** analytics `&self.csr` → owned `lcc_run_dense` | ❌ RED — tip graph_analytics_session (twin WDB-241) |
| Tip **WDB-275** PageRank `&Vec` → owned `arena_return_f64` | ✅ tip GREEN (P3.369) — extern bare-pass skip + AST owned FFI |
| Tip **WDB-276** BFS `&Vec` → owned `par_bfs_bind` | ✅ tip GREEN (P3.369) — same as WDB-275 |
| Tip **WDB-246** format temps → demoted `hash_join_semi` `&str` | ✅ tip GREEN (P3.365) — module-file + tip-out regen |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** tip greens **177/218–276**; sync tip-out→gen for WCC afforest; fix tip `&T`→owned Vec/Csr call sites. No Phase 606+.

## P3.363 WindjammerDB CQ-C5 — coverage REDs WDB-271/272 edge_batch to_arrow + wave1 push_str (2026-09-17)

| Gate | Status |
|------|--------|
| Tip **WDB-271** SQL `edges.clone()` → demoted `&GraphSqlEdgeBatch` to_arrow | ❌ RED — gen graph_sql_datafusion_port (tip-out GREEN) |
| Tip **WDB-272** wave1 `push_str(&owned.clone())` must reborrow | ✅ GREEN tip-out gate (P3.385) |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** tip greens **177/218–272**; sync tip-out→gen for edge_batch to_arrow + wave1 `&str` clones. No Phase 606+.

## P3.358 WindjammerDB CQ-C5 — coverage REDs WDB-269/270 LSQB graph.clone + wave1 &str clone (2026-09-17)

| Gate | Status |
|------|--------|
| Tip **WDB-269** LSQB `graph.clone()` → demoted `&LsqbTypedGraph` neighbors | ❌ RED — tip/gen lsqb_query_engine (inverse WDB-220) |
| Tip **WDB-270** wave1 `&owned.clone()` → demoted `&str` helpers | ✅ GREEN (P3.384) — finalize + Borrow peel |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** tip greens **177/218–270**; sync tip-out→gen for LSQB neighbors + wave1 `&str`. No Phase 606+.

## P3.357 WindjammerDB CQ-C5 — coverage REDs WDB-267/268 PageRank f64_sum + incremental bfs_run_dense (2026-09-17)

| Gate | Status |
|------|--------|
| Tip **WDB-267** PageRank `scores.clone()` → demoted `&Map` `f64_sum` | ❌ RED — gen graph_pagerank_engine (tip owned formal OK; twin WDB-223) |
| Tip **WDB-268** incremental `csr.clone()` → demoted `&DenseCsr` `bfs_run_dense` | ❌ RED — tip graph_incremental_views (twin WDB-248/256) |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** tip greens **177/218–268**; sync tip-out→gen for PageRank f64_sum + incremental bfs_run_dense. No Phase 606+.

## P3.351 WindjammerDB CQ-C5 — coverage REDs WDB-265/266 BFS i64_len/contains map.clone (2026-09-17)

| Gate | Status |
|------|--------|
| Tip **WDB-265** BFS `distances.clone()` → demoted `&Map` `i64_len` | ❌ RED — gen graph_bfs_engine (tip owned formal OK; twin WDB-222) |
| Tip **WDB-266** BFS `distances.clone()` → demoted `&Map` `i64_contains` | ❌ RED — gen graph_bfs_engine (tip-out GREEN; twin WDB-222/265) |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** tip greens **177/218–266**; sync tip-out→gen for BFS map.contains/len + WCC/BFS csr/map cluster. No Phase 606+.

## P3.349 WindjammerDB CQ-C5 — coverage REDs WDB-263/264 WCC init_identity + BFS distances_to_map (2026-09-17)

| Gate | Status |
|------|--------|
| Tip **WDB-263** WCC `&vertices` → owned `init_identity` must clone | ❌ RED — gen graph_wcc_engine (tip-out GREEN; twin WDB-259) |
| Tip **WDB-264** BFS `csr.clone()` → demoted `&DenseCsr` distances_to_map | ❌ RED — gen graph_bfs_engine (tip-out GREEN; twin WDB-252) |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** tip greens **177/218–264**; sync tip-out→gen for WCC/BFS + prior Vec/map/csr cluster. No Phase 606+.

## P3.344 WindjammerDB CQ-C5 — coverage REDs WDB-261/262 LCC simd &Vec→owned + wave1 fill_bundle &mut (2026-09-17)


| Gate | Status |
|------|--------|
| Fresh `cargo check --lib` | ⚠️ **~322** (gen lag) |
| Tip **WDB-261** LCC `&offsets`/`&tri` → owned `graph_simd_lcc_bind_oriented` | ❌ RED — tip-out/gen graph_lcc_engine (twin WDB-241) |
| Tip **WDB-262** wave1 owned/`&mut.clone` → demoted `&mut` fill_bundle | ❌ RED — gen wave1_opt_live_port (tip owned formals OK; twin WDB-257) |
| Tip **WDB-259/260 / 257/258 / 255/256** | ❌ RED |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** tip greens **177/218–262**; sync tip-out→gen for wave1 fill_bundle + 234–238. Dominant residual: Vec←&Vec / &Vec←Vec / map.clone→`&Map` / csr.clone→`&DenseCsr`. No Phase 606+.

## P3.341 WindjammerDB CQ-C5 — coverage REDs WDB-259/260 &Vec→owned init_identity + f64_sum vertices (2026-09-17)

| Gate | Status |
|------|--------|
| Fresh `cargo check --lib` | ⚠️ **~322** (gen lag) |
| Tip **WDB-259** CDLP `&vertices` → owned `init_identity` must clone | ❌ RED — tip-out/gen graph_cdlp_engine (twin WDB-241) |
| Tip **WDB-260** PageRank `&Vec` → owned `f64_sum` vertices must clone | ❌ RED — tip-out/gen graph_pagerank_engine (twin WDB-241/259) |
| Tip **WDB-257/258 / 255/256 / 253/254** | ❌ RED |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** tip greens **177/218–260**; sync tip-out→gen for 234–238. Dominant residual: Vec←&Vec / &Vec←Vec / map.clone→`&Map` / csr.clone→`&DenseCsr`. No Phase 606+.

## P3.340 WindjammerDB CQ-C5 — coverage REDs WDB-257/258 init_scores &mut clone + owned materialize &Vec (2026-09-17)

| Gate | Status |
|------|--------|
| Fresh `cargo check --lib` | ⚠️ **~322** (gen lag) |
| Tip **WDB-257** `&mut vertices.clone()` → demoted `&mut Vec` init_scores | ❌ RED — tip-out/gen graph_pagerank_engine |
| Tip **WDB-258** `&Vec` → owned materialize dsts/weights must clone | ❌ RED — tip-out/gen csr_integrate/write_port (twin WDB-241; opposite WDB-239) |
| Tip **WDB-255/256 / 253/254 / 251/252** | ❌ RED |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** tip greens **177/218–258**; sync tip-out→gen for 234–238. Dominant residual: Vec←&Vec / &Vec←Vec / map.clone→`&Map` / csr.clone→`&DenseCsr`. No Phase 606+.

## P3.339 WindjammerDB CQ-C5 — coverage REDs WDB-255/256 CDLP parallel + incremental BFS &mut DenseCsr (2026-09-17)

| Gate | Status |
|------|--------|
| Fresh `cargo check --lib` | ⚠️ **~322** (gen lag) |
| Tip **WDB-255** CDLP `csr.clone()` → demoted `&mut DenseCsr` parallel | ❌ RED — tip-out/gen graph_cdlp_engine (twin WDB-233) |
| Tip **WDB-256** incremental `csr.clone()` → demoted `&mut DenseCsr` bfs | ❌ RED — tip-out/gen graph_incremental_views (twin WDB-233) |
| Tip **WDB-253/254 / 251/252 / 249/250 / 247/248** | ❌ RED |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** tip greens **177/218–256**; sync tip-out→gen for 234–238. Dominant residual: Vec←&Vec / `&str`←String / map.clone→`&Map` / csr.clone→`&DenseCsr`/`&mut DenseCsr`. No Phase 606+.

## P3.334 WindjammerDB CQ-C5 — coverage REDs WDB-253/254 BFS contains/len + SQL count_edges (2026-09-16)

| Gate | Status |
|------|--------|
| Fresh `cargo check --lib` | ⚠️ **~322** (gen lag) |
| Tip **WDB-253** BFS `distances.clone()` → demoted contains/len | ❌ RED — tip-out/gen graph_bfs_engine (twin WDB-222) |
| Tip **WDB-254** datafusion `csr.clone()` → demoted SQL count_edges | ❌ RED — tip-out/gen graph_sql_datafusion_port (twin WDB-252) |
| Tip **WDB-251/252 / 249/250 / 247/248** | ❌ RED |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** tip greens **177/218–254**; sync tip-out→gen for 234–238. Dominant residual: Vec←&Vec / `&str`←String / map.clone→`&Map` / csr.clone→`&DenseCsr`. No Phase 606+.

## P3.333b — generic type alias must emit after struct (2026-09-17)

| Gate | Status |
|------|--------|
| `generic_type_alias_must_emit_after_struct` | ✅ tip GREEN — TypeAlias deferred until after struct/enum/trait emit (`program_generation.rs`) |

**Root cause:** Const/static early pass also emitted `type` aliases, hoisting `SharedInt = Shared<…>` above `struct Shared` (E0425).

**Product:** generics-first `wj-sync` Shared/Pending aliases.

## P3.333 WindjammerDB CQ-C5 — coverage REDs WDB-251/252 incremental maps + demoted vertex_count/find_index (2026-09-16)

| Gate | Status |
|------|--------|
| Fresh `cargo check --lib` | ⚠️ **~322** (gen lag) |
| Tip **WDB-251** incremental `prior.distances/labels.clone()` → demoted `&GraphVertexI64Map` | ❌ RED — tip-out/gen graph_incremental_views (twin WDB-222/250) |
| Tip **WDB-252** `csr.clone()` → demoted `&DenseCsr` vertex_count/find_index | ❌ RED — tip-out bfs/wcc/sssp/epoch/datafusion (twin WDB-233/248) |
| Tip **WDB-249/250 / 247/248 / 244–246** | ❌ RED |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** tip greens **177/218–252**; sync tip-out→gen for 234–238. Dominant residual: Vec←&Vec / `&str`←String / map.clone→`&Map` / csr.clone→`&DenseCsr`. No Phase 606+.

## P3.330 WindjammerDB CQ-C5 — coverage REDs WDB-249/250 SSSP F64Map + CDLP I64Map (2026-09-16)

| Gate | Status |
|------|--------|
| Fresh `cargo check --lib` | ⚠️ **~322** (gen lag) |
| Tip **WDB-249** SSSP `distances.clone()` → demoted `&GraphVertexF64Map` get | ❌ RED — tip-out/gen graph_sssp_engine (twin WDB-223) |
| Tip **WDB-250** CDLP `labels.clone()` → demoted `&GraphVertexI64Map` get | ❌ RED — tip-out/gen graph_cdlp_engine (twin WDB-222/247; WDB-226 covers u32) |
| Tip **WDB-247/248 / 244–246** | ❌ RED |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** tip greens **177/218–250**; sync tip-out→gen for 234–238. Dominant residual: Vec←&Vec / `&str`←String / map.clone→`&Map`. No Phase 606+.

## P3.328 WindjammerDB CQ-C5 — coverage REDs WDB-247/248 WCC map + analytics &DenseCsr; tip-out GREEN WDB-234 (2026-09-16)

| Gate | Status |
|------|--------|
| Fresh `cargo check --lib` | ⚠️ **~322** (gen lag) |
| Tip-out **WDB-234** `find_index` demoted `&DenseCsr` | ✅ tip-out GREEN |
| Tip **WDB-247** WCC `p.clone()` → demoted `&GraphVertexI64Map` get | ❌ RED — tip-out/gen graph_wcc_engine (twin WDB-222) |
| Tip **WDB-248** analytics `csr.clone()` → demoted `&DenseCsr` multi_source | ❌ RED — tip-out/gen graph_analytics_session (twin WDB-233) |
| Tip **WDB-244–246** | ⚠️ **244** ✅ tip GREEN (P3.364); **245–246** still ❌ |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** tip greens **177/218–248**; sync tip-out→gen for 234–238. Dominant residual: Vec←&Vec / `&str`←String / LsqbTypedGraph. No Phase 606+.

## P3.324 WindjammerDB CQ-C5 — coverage REDs WDB-244–246 datafusion string ownership + tip-out greens 235–238 (2026-09-16)

| Gate | Status |
|------|--------|
| Fresh `cargo check --lib` | ⚠️ **~322** (gen may lag tip-out) |
| Tip-out **WDB-235** distances_to_map formal demoted `&DenseCsr` | ✅ tip-out GREEN |
| Tip-out **WDB-236** HashMap String `&key` | ✅ tip-out GREEN |
| Tip-out **WDB-237** `len() as u64` (no `as i64`) | ✅ tip-out GREEN |
| Tip-out **WDB-238** frame_total_len owned formal | ✅ tip-out GREEN |
| Tip **WDB-244** `"props".to_string()` → demoted `sql_exec` `&str` | ✅ tip GREEN (P3.364) — multipass bare-pass key match |
| Tip **WDB-245** bare `"props"` → owned table_provider `String` | ❌ RED — tip-out/gen df_analytic |
| Tip **WDB-246** format temps → demoted `hash_join` `&str` | ✅ tip GREEN (P3.365) — module-file + tip-out |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** tip greens **177/218–246**; sync tip-out→gen for 235–238. Dominant residual: Vec←&Vec / `&str`←String / LsqbTypedGraph. No Phase 606+.

## P3.320 WindjammerDB CQ-C5 — coverage REDs WDB-241–243 ecs/arrow/federation Vec+String (2026-09-16)

| Gate | Status |
|------|--------|
| Fresh `cargo check --lib` | ⚠️ **~322** |
| Tip **WDB-241** `&ids` → owned `ecs_soa_archetype_new` | ❌ RED — tip-out/gen ecs_soa (twin WDB-224) |
| Tip **WDB-242** `&str` vertex_id_name → owned from_ids_labels | ❌ RED — tip-out/gen arrow_ffi (twin WDB-240) |
| Tip **WDB-243** federation `&Vec` return as owned `Vec` | ❌ RED — tip-out/gen federation_layer |
| Tip **WDB-239/240** | ❌ RED |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** tip greens **177/218–243**. Dominant residual: Vec←&Vec / `&str`←String / LsqbTypedGraph / DenseCsr. No Phase 606+.

## P3.321 (2026-09-16) — for-in struct Vec field partial-moves parent

| Gate | Status |
|------|--------|
| `bug_module_file_for_in_struct_vec_field_must_not_partial_move_parent_test` | ✅ tip GREEN (2026-09-17) — `&parent.field` when owner used after loop |
| Product interim | removable — drop `payment.allocations.clone()` in LedgerKit when convenient |

**Fix:** precompute `for_loop_field_owner_borrow_needed` (sibling use after `for`) + `field_iterable_needs_borrow_when_owner_used_in_body`.

## P3.322 (2026-09-16) — `vec.len() > 0` uint/int

| Gate | Status |
|------|--------|
| `bug_module_file_vec_len_gt_zero_must_not_mix_uint_int_test` | ✅ tip GREEN (2026-09-16) |
| Product interim | ✅ `!rows.is_empty()` / `!roles.is_empty()` where tip still complained |

**Compiler agent:** keep `len()` compare literals in the same width as `len()` (uint/usize), or prefer `is_empty()`.

## P3.319 WindjammerDB CQ-C5 — coverage REDs WDB-239/240 materialize &Vec + demoted sql parse_ast (2026-09-16)

| Gate | Status |
|------|--------|
| Fresh `cargo check --lib` | ⚠️ **~322** (lock-contended re-census) |
| Tip **WDB-239** owned `dsts`/`weights` → demoted materialize `&Vec` | ❌ RED — tip-out/gen graph_sql_query_port |
| Tip **WDB-240** demoted `&str` sql → owned `relational_sql_parse_ast` | ❌ RED — tip-out/gen relational_df_analytic |
| Tip **WDB-235–238** | ✅ tip-out GREEN |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** tip greens **177/218–240**. Dominant residual: Vec←&Vec / `&str`←String / LsqbTypedGraph / DenseCsr. No Phase 606+.

## P3.320 WindjammerDB CQ-C5 — coverage REDs WDB-241–243 ecs/arrow/federation (2026-09-16)

| Gate | Status |
|------|--------|
| Tip **WDB-241** demoted `&Vec<u32>` → owned `ecs_soa_archetype_new` | ❌ RED — tip-out/gen ecs_soa_port (`&ids`) |
| Tip **WDB-242** demoted `&str` → owned record-batch `vertex_id_name` | ❌ RED — tip-out/gen graph_sql_arrow_ffi_port |
| Tip **WDB-243** demoted `&Vec` return → owned federation scan | ❌ RED — tip-out/gen federation_layer |
| Verified | 2026-09-16 TDD: all three tip gates FAIL on tip-out |

**Compiler agent:** demoted `&Vec`/`&str` into owned formals/returns must clone / `.to_string()` / `.to_vec()` — signature-driven (twins WDB-224/240/205).

## P3.316 WindjammerDB CQ-C5 — coverage REDs WDB-235–238 distances_to_map / HashMap String / u64 len cast / frame_total_len (2026-09-16)

| Gate | Status |
|------|--------|
| Fresh `cargo check --lib` | ⚠️ residual elsewhere |
| Tip **WDB-235** distances_to_map | ✅ tip-out GREEN (2026-09-16) — tip demotes formal to `&DenseCsr`; `&mut`→`&` reborrow |
| Tip **WDB-236** HashMap String `contains_key`/`get` | ✅ tip-out GREEN — `&key` |
| Tip **WDB-237** `len() as u64` (no `as i64`) | ✅ tip-out GREEN |
| Tip **WDB-238** frame_total_len | ✅ tip-out GREEN — tip keeps owned `Vec` formal + clone |
| Multipass codegen gates | ✅ WDB-235/236/237 codegen GREEN |
| Dogfood / tip-cluster | ❄️ frozen; tip→gen synced for these five modules |

**Root cause layer:** tip multipass already correct; tip-out/gen lag. Slim tip regen + tip→gen sync; tip-out gates prefer tip when present.

**Gates:** `cargo test --release --test all -- wdb235_tip_out wdb236_tip_out wdb237_tip_out wdb238_tip_out wdb235_codegen wdb236_codegen wdb237_codegen` → 7 passed

**Compiler agent priority:** tip greens **177/218–243** remaining. No Phase 606+.

## P3.310 WindjammerDB CQ-C5 — coverage RED WDB-234 batch DenseCsr←&mut (2026-09-16)

| Gate | Status |
|------|--------|
| Fresh `cargo check --lib` | ⚠️ **322** |
| Tip **WDB-219** graph_sql `&mut`→owned csr | ❌ RED |
| Tip **WDB-234** batch `find_index(csr)` with `&mut DenseCsr` | ❌ RED — tip-out/gen graph_batch_engine (~13× DenseCsr←&mut) |
| Tip **WDB-232/233** | ❌ RED |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** tip greens **177/218–234**. Dominant residual: Vec←&Vec / `&str`←String / LsqbTypedGraph / DenseCsr ownership. No Phase 606+.

## P3.309 WindjammerDB CQ-C5 — coverage REDs WDB-232/233 u32+1 usize cast + analytics csr.clone (2026-09-16)

| Gate | Status |
|------|--------|
| Fresh `cargo check --lib` | ⚠️ **322** |
| Tip **WDB-230/231** | ❌ RED — tip-out/gen |
| Tip **WDB-232** `i + 1_u32 as usize` into u32 compare | ❌ RED — tip-out/gen wave1_*_cli (~6×) |
| Tip **WDB-233** `self.csr.clone()` → `&mut DenseCsr` (analytics) | ❌ RED — tip-out/gen graph_analytics_session (~6×) |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** tip greens **177/218–233**. Dominant residual: Vec←&Vec / `&str`←String / LsqbTypedGraph / DenseCsr ownership. No Phase 606+.

## P3.307 WindjammerDB CQ-C5 — coverage REDs WDB-230/231 Copy put borrow + usize+=i32 (2026-09-16)

| Gate | Status |
|------|--------|
| Fresh `cargo check --lib` | ⚠️ **322** |
| Tip **WDB-138** multipass fixture | ✅ GREEN |
| Tip **WDB-230** `map.put(&(ids[i]), &(vals[i]))` tip-out | ❌ RED — tip-out/gen pagerank/batch (~7× i64←&i64 / f64←&f64) |
| Tip **WDB-231** `usize` index `+= 1 as i32` | ❌ RED — tip-out/gen graph_batch_engine (~5× usize←i32) |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** tip greens **177/218–231**. Dominant residual: Vec←&Vec / `&str`←String / LsqbTypedGraph / Copy put borrow. No Phase 606+.

## P3.305 WindjammerDB CQ-C5 — coverage REDs WDB-227–229 u64 width + impl Into push_str (2026-09-15)

| Gate | Status |
|------|--------|
| Fresh `cargo check --lib` | ⚠️ **322** |
| Tip **WDB-214–217 / 215 lsqb** | ✅ GREEN |
| Tip **WDB-177/218–226** | ❌ RED |
| Tip **WDB-227** u64 vs `len() as i64` (LDBC) | ❌ RED — tip-out/gen lag (needs tip `wj` regen) |
| Tip **WDB-228** `impl Into<String>` + `push_str(&…)` | ✅ tip isolate GREEN (2026-09-16) — `param_pub_free_string_builder_forward` requires owned forward site; tip-out/gen still lag |
| Tip **WDB-229** u64 vs bare `len()` usize | ❌ RED — tip-out/gen lag (needs tip `wj` regen) |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** tip greens **177/218–229**. Dominant residual: Vec←&Vec / `&str`←String / LsqbTypedGraph / u64 width. No Phase 606+.

## P3.306 (2026-09-16) — int/usize while-len + push_str Into formal

| Gate | Status |
|------|--------|
| `bug_int_while_len_as_int_must_not_emit_usize_arith_test` | ✅ tip GREEN — `Type::Int`→`IntType::I64`; signed-vs-usize prefer i64; skip polluted Usize casts |
| `bug_wdb228_*` isolate codegen | ✅ tip GREEN — `param_pub_free_string_builder_forward` requires owned forward site (not `push_str` `&str`) |
| WDB-227/229 tip-out | ❌ tip-out/gen lag until regen with tip `wj` |

**Root cause layers:** constraint/promotion (`int`≠i32) + coercion (prefer i64 over usize) + signature formal (`Into` only when owned forward).
**What became unnecessary:** casting `len as int` bindings to usize in comparisons; `impl Into` + `push_str(&Into)` for borrow-only builders.
**Gates:** `cargo test --release --test all -- int_while_len_as_int wdb228_codegen i32_compound_add thread_spawn_closure mpsc_sync_channel` → 7 passed.

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
| Cross-module Vec borrow | ✅ tip GREEN (P3.353) |
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
| Cross-module Vec borrow | `cross_crate_vec_string_helper_*` | ✅ tip GREEN (P3.353 — owned `Vec` formal + explicit clone → move) |
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
| `bug_cross_crate_vec_helper_must_auto_borrow_test` | ✅ tip GREEN (P3.353 — `reconcile_explicit_user_clone_into_owned_vec_formal`) |
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

## P3.367 Breach dogfood — u32 literal peers in i32-coord / void GPU files (2026-09-18)

| Cluster | Gate | Status |
|---------|------|--------|
| `u32::wrapping_*` method args vs file i32 suffix | `bug_u32_method_call_literals_must_peer_u32_in_mixed_i32_fn_file_test` | ✅ tip GREEN |
| u32 compare / `<<` / `&` literal peers | `bug_u32_compare_and_bitwise_must_peer_u32_in_mixed_i32_fn_file_test` | ✅ tip GREEN |
| u32 mesh arith (regression) | `bug_u32_arith_must_not_take_i32_literal_peers_in_coord_fn_test` | ✅ tip GREEN |

**Root cause:** i32-coord / void-builder context (`function_prefers_i32_coord_locals`) overwrote call-arg and binary literal peers; bitwise ops did not peer-drive; chained `wrapping_*` lost u32 receiver; `self.field` in standalone fns missed struct field types.

**Dogfood:** Breach `568` → `374` rustc errors; `expected u32, found i32` `122` → `~14`; binary still **NO**.


## P3.368 Breach dogfood — i32-coord → i64/u32 formals (2026-09-18)

| Issue | Gate | Status |
|-------|------|--------|
| i32 locals into `i64` / WJ `int` call + struct fields | `bug_i32_binding_into_i64_and_u32_formal_must_cast_test` | ✅ tip GREEN |
| i32 into `u32` extern FFI + struct fields (texture.rs) | same gate | ✅ tip GREEN |

**Fix:** `coerce_arg_str_for_i64/u32_formal`, `apply_numeric_formal_coercions`, extern-call path in `regular_call_arguments.rs`, struct literal `coerce_struct_field_numeric`.

**Dogfood:** Breach `374` → `359` errors; `u32<-i32` ~14 → ~5; binary **NO**.

## P3.369 (2026-09-18) — i64 entity literal peers + index `idx + 1` + owned FFI forwarder

| Gate | Status |
|------|--------|
| `bug_i64_entity_literal_peers_and_index_add_test` | ✅ tip GREEN |
| `bug_wdb216_module_file_owned_ffi_wrapper_must_stay_owned_test` | ✅ tip GREEN |
| Tip **WDB-275/276** arena/par_bfs owned Vec | ✅ tip GREEN |
| Tip **WDB-277–280** par_*_bind owned Vec | ✅ tip GREEN (tip-out regen) |

**Root cause layer:** signature — bare-pass demoted `extern fn` `Vec` formals to Borrowed/`&Vec`, so wrappers that forward bare into FFI looked like borrow-passthrough and emitted `&Vec` + bare into `Vec` (E0308). Secondary: i32-coord prefer overwrote i64 entity compares; index `i64 + lit` promoted before usize rewrite.

**What became unnecessary:** no new ir_call_site peels; wrappers stay owned via intact extern Owned signatures + AST-owned FFI detection (narrowed prepare AST path for `sig.is_extern`).

**Fix:** never bare-pass-demote `is_extern` formals (`multipass_bare_pass_demotion`); AST bare owned beats stale Borrowed on extern; `comparison_should_prefer_i64_over_i32`; skip i32 promotion for index i64+lit.

**Gates:** `cargo test --release --test all -- wdb216_module_file_owned_ffi wdb275_tip_out wdb276_tip_out wdb277_tip_out wdb278_tip_out wdb279_tip_out wdb280_tip_out i64_entity_literal` → pass.

## P3.371 — finance-screens theme hex byte arith (2026-09-18)

| Gate | Status |
|------|--------|
| `theme_hex_byte_arith_must_stay_one_int_width` | ✅ tip GREEN |
| Product `finance-screens` `theme.rs` `hi * 16 + lo` | ✅ `hi * 16_i64 + lo` (no `lo as i32`) |

**Root cause:** After `< 0` compares, numeric inference tagged later `hex_digit_value()` call results as i32; `reconcile_ambiguous_int_local_after_let` narrowed WJ `int` call bindings to `Type::Int32`, and `int_type_for_mixed_int_codegen` let `eng == I32` override WJ `int` locals — mixed add emitted `16_i64 + lo as i32`.

**Fix:** Keep WJ `int`/`i64` call-result lets at i64 unless RHS actually emitted `_i32`; trust WJ `int` local types over post-compare i32 inference. `assignment_int_peer_from_formal` already peers i64 formals for call literals (P3.368 WIP). `maybe_clone_index_for_owned_param`: `text[lo..hi]` into owned `string` → `.to_string()` (not `.clone()` on `str`).

**Gate:** `cargo test --test all --features integration_tests theme_hex_byte_arith_must_stay_one_int_width -- --nocapture` → pass (2026-09-18 GREEN incl. cargo-check).

## P3.383 WindjammerDB CQ-C5 — coverage REDs WDB-298–300 (2026-09-19)

| Gate | Status |
|------|--------|
| Tip **WDB-298** `u32 = 0_usize` loop init | ❌ RED — tip-out/gen adjacency/CDLP/LCC (~17×); MultiFile isolate GREEN |
| Tip **WDB-299** `Key::from_components(&parts)` → owned Vec | ❌ RED — tip-out/gen layers (~8×); MultiFile isolate GREEN; twin WDB-241 |
| MultiFile **WDB-300** `n as usize.clone()` | ✅ MultiFile GREEN (P3.386); tip-out still needs regen |
| Tip **WDB-291–295/297** wave1 CLI | ✅ tip GREEN (P3.379) |
| Dogfood / tip-cluster | ❄️ frozen; gen/mod.rs probe-junk cleaned locally (gitignored) |

**Census (after gen/mod.rs hygiene):** `cargo check --lib` on wdb-layers → **287 errors** (205× E0308). Dominant tip-same classes: `0_usize`→u32, `&parts`→owned Key, `as T.clone()`, int-width peers.

**Compiler agent priority:** tip greens **WDB-298–300**; signature-driven int-width peers + clone-after-cast ban + owned Vec into `Key::from_components`. No Phase 606+. No windjammer/src edits from DB agent.

## P3.390 — tip-out/gen sync WDB-298/299/303 (2026-09-19)

| Gate | Status |
|------|--------|
| Tip **WDB-298** `u32 = 0_usize` | ✅ tip GREEN — tip emit already `0_u32`; tip-out/gen synced |
| Tip **WDB-299** `Key::from_components(&parts)` | ✅ tip GREEN — tip emit already move/`parts`; tip-out/gen synced |
| Tip **WDB-303** `offsets[i + 1]` u32 index | ✅ tip GREEN — tip emit already `(i + 1) as usize`; tip-out/gen synced |

**Root cause layer:** tip-out/gen lag (tip MultiFile + productish `wj` emit already correct).

**What became unnecessary:** false-RED tip-out asserts on stale snapshots.

**Gates:** `cargo test --release --test all -- wdb298_ wdb299_ wdb303_` → **6 passed**.

## P3.388 — WDB-302 i64 accum under Custom return (2026-09-19)

| Gate | Status |
|------|--------|
| MultiFile **WDB-302** untyped `total = 0` + `as i64` / `(total / 3) as u64` | ✅ tip GREEN — no `as i64 as i32`; divisor peers `3_i64` |
| Tip-out/gen LCC engine | ✅ synced (`as i64`, `total / 3_i64`) |

**Root cause layer:** constraint/int-width — Custom-return registered untyped `0` as Int32 while literal emitted `_i64`; struct-field `u64` leaked onto nested division literals ahead of binary peer.

**What became unnecessary:** dual-oracle Int32 binding vs `_i64` emit; struct-field width beating assign/peer for nested cast operands.

**Gates:** `cargo test --release --test all -- wdb302_` → MultiFile + tip-out GREEN; tip `wj` productish emit `total += triangles as i64` / `(total / 3_i64) as u64`.

## P3.387 — WDB-301 owned LDBC string lit / path (2026-09-19)

| Gate | Status |
|------|--------|
| MultiFile **WDB-301** owned `validation_entry` / `validate_*` | ✅ tip GREEN — lit `.to_string()`; no `&path` / `&path.clone()` into owned `String` |
| Tip-out/gen LDBC validation port | ✅ synced to tip shape (`"BFS".to_string()`, `path.clone()`); gitignored tip-out + wdb-layers gen |

**Root cause layer:** signature/emission contract — `emitted_owned_arg_contract` / `emitted_rust_ref_params=false` beats stale analyzer `Borrowed` and stub “plain string → &str” guesses at call sites.

**What became unnecessary:** early-return that skipped lit `.to_string()` when plain WJ string + (!emitted_owned || Borrowed); `plain && !emitted_owned` as `callee_wants_str` / strip-owned trigger (would `&` / peel into owned formals).

**Gates:** `cargo test --release --test all -- wdb301_` → MultiFile + tip-out GREEN; unit `free_fn_owned_string_formal_with_stale_borrowed_owns_literal` GREEN.

## P3.385 WindjammerDB CQ-C5 — coverage REDs WDB-301–303 (2026-09-19)

| Gate | Status |
|------|--------|
| MultiFile **WDB-301** owned LDBC string / `&path.clone()` | ✅ tip GREEN (P3.387) |
| Tip **WDB-302** `as i64 as i32` / `total / 3_u64` (LCC) | ✅ tip GREEN (P3.388) |
| Tip **WDB-303** `offsets[i + 1]` u32 index | ✅ tip GREEN (P3.390 tip-out/gen sync) |
| Tip **WDB-298–300** | ✅ 298/299 tip GREEN (P3.390); WDB-300 MultiFile GREEN (P3.386), tip-out may still lag cast-clone |
| Dogfood / tip-cluster | ❄️ frozen |

**Census:** ~287 gen errors; next cluster after 301 is int-width (LCC double-cast, u32 index) + remaining tip-out lag.

**Compiler agent priority:** tip greens **WDB-302–303** (+ 298–299 tip-out); ban double cast on i64 accum; usize index casts. No Phase 606+.


## P3.393 — nested empty-append + demoted shared-ref bare call sites (2026-09-19)

| Item | Status |
|------|--------|
| Root cause | Leaf `param + ""` alone demotes (readonly); nested `(left+"")+(right+"")` and `body+""` into owned concat2 must keep owned formals. Early `keeps_owned` demote skipped owned-callee forwards. Demoted caller `&str` got `parse_twice(&json)` when HashSet lagged. |
| Fix | `expr_is_param_or_string_add_lhs` + `passed_into_owned` in `keeps_owned`; `caller_formal_emitted_shared_ref` peels `&name` using preregistered formal strings. |
| Gates | `string_concat_nested_owned_must_not_over_borrow` (×2), `owned_string_locals_move_into_owned_string_formals`, `eco_wj_semver_*`, `eco_wj_toml_*`, `cross_module_match_arm_readonly_concat_demotes_to_str` → **6/6 GREEN** |

## P3.391 WindjammerDB CQ-C5 — coverage REDs WDB-304–306 (2026-09-19)

| Gate | Status |
|------|--------|
| MultiFile **WDB-304** `Some(x.clone()).cloned()` | ✅ MultiFile GREEN (P3.391 coerce) — tip-out lag |
| MultiFile **WDB-305** CDLP `best_count = 0_i64` vs u32 | ✅ MultiFile GREEN (P3.391 later-assign peer) — tip-out lag |
| Tip **WDB-306** bakeoff `&hw.clone()` → owned String | ✅ MultiFile GREEN; tip-out/gen lag; twin WDB-301 |

**Root cause (304):** `coerce_option_ref_return_to_owned` appended `.cloned()` whenever the expr tree contained a ref (e.g. `for m in &metrics` → `Some(m)`), including language-level `Some`/`Ok`/`Err` that already own the payload.

**Fix (304):** skip payload constructors; only `.cloned()`/`.copied()` when inferred type is `Option<&T>`.

**Root cause (305):** untyped `let mut best_count = 0` took return-width i64 while later `best_count = count` (u32 from `counts[i]`) lived inside `if` in a `while` — peer scan must resolve lets against the full function body.

**Fix (305):** `mut_int_local_peer_width_from_later_assigns` + Vec-index/`let` chase; wire into mut counter ascription + literal suffix.

**Compiler agent priority:** tip-out/gen sync **WDB-304–306**; then WDB-307–312. No Phase 606+.

## P3.392 WindjammerDB CQ-C5 — coverage REDs WDB-307–309 (2026-09-19)

| Gate | Status |
|------|--------|
| MultiFile **WDB-307** `for node in index.graph.nodes` move | ❌ RED — MultiFile + tip-out vector_topk |
| Tip **WDB-308** wave1 CLI `u32=0_usize` / `args[i+1]` | ✅ MultiFile + tip-out GREEN (P3.400) |
| Tip **WDB-309** BFS `&mut csr` without `mut` param | ❌ RED — tip-out/gen beamer_parallel; MultiFile isolate GREEN |
| Tip **WDB-304–306** | ✅ MultiFile GREEN (P3.391); tip-out may lag |
| Dogfood / tip-cluster | ❄️ frozen |

**TDD:** `cargo test --release --test all --features integration_tests,codegen_tests -- wdb307_ wdb308_ wdb309_` → 1 passed / 4 failed (expected RED).

**Compiler agent priority:** tip greens **WDB-307–309** (borrow field in for when parent reused; wave1 CLI int-width residual; `mut` binding for demoted `&mut`). No Phase 606+.

## P3.393 WindjammerDB CQ-C5 — coverage REDs WDB-310–312 (2026-09-19)

| Gate | Status |
|------|--------|
| Tip **WDB-310** LSQB `&filename.clone()` / `&content.clone()` → owned String | ❌ RED — tip-out/gen; MultiFile isolate GREEN; twin WDB-306 |
| Tip **WDB-311** timeseries + graph_vertex_map `u32=0_usize` | ❌ RED — tip-out/gen residual beyond wave1 CLI (WDB-308) |
| Tip **WDB-312** publish `&dated_label.clone()` → owned String | ❌ RED — tip-out/gen; MultiFile isolate GREEN; twin WDB-306 |
| Tip **WDB-304–309** | ⚠️ 304–306 MultiFile GREEN (P3.391); tip-out lag; 307–309 still RED |
| Dogfood / tip-cluster | ❄️ frozen |

**TDD:** `cargo test --release --test all --features integration_tests,codegen_tests -- wdb310_ wdb311_ wdb312_` → 2 passed / 3 failed (expected RED).

**Compiler agent priority:** tip greens **WDB-310–312** (no `&owned.clone()` into owned String on LSQB/publish; u32 peer init on timeseries/vertex_map). No Phase 606+. No windjammer/src edits from DB agent.

## P3.394 WindjammerDB CQ-C5 — coverage REDs WDB-313–315 (2026-09-19)

| Gate | Status |
|------|--------|
| Tip **WDB-313** pg_wire + OTLP `u32=0_usize` | ❌ RED — tip-out/gen residual beyond 308/311 |
| Tip **WDB-314** hardware report `append_bool(&out)` → owned String | ❌ RED — tip-out/gen; MultiFile isolate GREEN; twin WDB-312 |
| MultiFile **WDB-315** usize `pos + 4_i32` (pg_wire) | ✅ MultiFile GREEN (P3.395) — tip-out lag |
| Tip **WDB-310–312** | ❌ still RED (P3.393) |
| Dogfood / tip-cluster | ❄️ frozen |

**TDD:** `cargo test --release --test all --features integration_tests,codegen_tests -- wdb313_ wdb314_ wdb315_` → 1 passed / 4 failed (expected RED).

**Compiler agent priority:** tip greens **WDB-313–315** (u32 peer on pg_wire/OTLP; no `&out` into owned String; usize peer for field-walk offsets). No Phase 606+. No windjammer/src edits from DB agent.

## P3.395 WindjammerDB CQ-C5 — coverage REDs WDB-316–318 (2026-09-19)

| Gate | Status |
|------|--------|
| Tip **WDB-316** scale status `append_rows(&out)` → owned String | ❌ RED — tip-out/gen; MultiFile isolate GREEN; twin WDB-314 |
| Tip **WDB-317** vertex_map.hashmap/.vec `u32=0_usize` | ❌ RED — tip-out/gen residual beyond WDB-311 main `.rs` |
| Tip **WDB-318** publish `append_gate_rows(&out)` first formal | ❌ RED — tip-out/gen; MultiFile isolate GREEN; twin WDB-312/314 |
| Tip **WDB-313–315** | ✅ WDB-315 MultiFile GREEN (P3.395 usize peers); 313–314 tip-out lag |
| Dogfood / tip-cluster | ❄️ frozen |

**TDD:** `cargo test --release --test all --features integration_tests,codegen_tests -- wdb316_ wdb317_ wdb318_` → 2 passed / 3 failed (expected RED).

**Compiler agent priority:** tip greens **WDB-316–318** (no `&out` into owned String on scale/publish; u32 peer on hashmap/vec vertex_map). No Phase 606+. No windjammer/src edits from DB agent.

## P3.395 — WDB-315 usize accum nested lit peers (2026-09-19)

| Gate | Status |
|------|--------|
| MultiFile **WDB-315** `pos = pos + 4 + 2 + …` | ✅ tip GREEN MultiFile — emits `_usize` not `_i32` |
| Tip-out/gen pg_wire | 🆕 RED lag until regen |

**Root cause:** nested `usize + lit` chains let coord-builder i32 peer overwrite the assign-slot / usize operand peer (`4_i32` on `pos: usize`).

**Fix:** keep `_usize` when assign slot or either binary operand is usize; `peer_type_for_int_literal_operand` prefers `expression_produces_usize` before coord i32.

**Gate:** `cargo test --release --test all --features integration_tests -- wdb315_module_file_usize` → MultiFile pass (2026-09-19).


## P3.397 — u32 return-scan peers + tip-out sync (WDB-308/311/313/315/317) (2026-09-19)

| Gate | Status |
|------|--------|
| MultiFile **WDB-308** `-> u32` + `args.len()` / `args[i+1]` | ✅ GREEN — `0_u32` + `(i + 1) as usize` |
| Tip **WDB-308/311/313/317** `u32 = 0_usize` residuals | ✅ tip GREEN after tip-out regen |
| Tip **WDB-315** usize `pos + 4_i32` | ✅ tip GREEN (prior P3.395 MultiFile + tip-out sync) |
| Tip owned-string **WDB-310/312/314/316/318/319** | ✅ tip GREEN (P3.396 tip-out sync; tip already correct) |
| Tip **WDB-303** adjacency `offsets[i+1]` | ✅ tip GREEN — tip uses usize walkers; gate ignores usize `i` |

**Root cause layer:** constraint/peer — `-> u32` return-width counters must not be overwritten by index-driven usize peers; index cast uses `({}) as usize` so `as` does not bind tighter than `+`.

**What became unnecessary:** manual tip-out `0_u32` / cast patches for wave1 CLI once tip emit is correct.

**Gates:** `cargo test --release --test all -- wdb308_ wdb298_ wdb303_module_file wdb311_tip_out wdb313_tip_out wdb317_tip_out wdb315_ wdb310_tip_out … wdb319_tip_out`.

## P3.396 WindjammerDB CQ-C5 — coverage REDs WDB-319–321 (2026-09-19)

| Gate | Status |
|------|--------|
| Tip **WDB-319** publish_check CLI `&dated` → owned String | ❌ RED — tip-out/gen; MultiFile isolate GREEN; twin WDB-312 |
| Tip **WDB-320** publish_allows bare `&dated_label` → owned String | ❌ RED — tip-out/gen; MultiFile isolate GREEN; twin WDB-312 |
| Tip **WDB-321** LSQB edge bare `&content` → owned String | ❌ RED — tip-out/gen; twin WDB-310 (`&content.clone()` on vertex path) |
| Tip **WDB-316–318** | ❌ still RED (P3.395) |
| Dogfood / tip-cluster | ❄️ frozen |

**TDD:** `cargo test --release --test all --features integration_tests,codegen_tests -- wdb319_ wdb320_ wdb321_` → 2 passed / 3 failed (expected RED).

**Compiler agent priority:** tip greens **WDB-319–321** (no bare `&owned` into owned String on publish_check/allows/LSQB edge). No Phase 606+. No windjammer/src edits from DB agent.

## P3.397 WindjammerDB CQ-C5 — coverage REDs WDB-322–323 (2026-09-19)

| Gate | Status |
|------|--------|
| Tip **WDB-322** LSQB `lsqb_push_adj(..., &out_key, …)` → owned String key | ✅ MultiFile + tip-out GREEN (2026-09-19) |
| Tip **WDB-323** Arrow FFI `&vname`/`&lname` → owned String | ✅ MultiFile + tip-out GREEN (2026-09-19) |
| Tip **WDB-314/316/318–321** | ✅ tip GREEN (P3.397–399 recheck; 321 gate accuracy) |
| Tip **WDB-311/313/317** | ❌ still RED (`u32=0_usize` residuals) |
| Dogfood / tip-cluster | ❄️ frozen |

**TDD:** `cargo test --release --test all --features integration_tests,codegen_tests -- wdb322_ wdb323_` → **4 passed** (2026-09-19).

**Handoff:** Compiler agent can skip 322–323. Next owned-String tip RED: **WDB-325** (`sql_exec` `&emit.*`).

## P3.398 — game-core tip rustc residual after library transpile (2026-09-19)

| Gate | Status |
|------|--------|
| `wj game build --release` tip transpile | ✅ past `.wj → gen/` (2026-09-19) |
| `windjammer_game_core` cargo | ❌ residual E0308 cluster (WDB-324 navmesh cleared P3.400) |
| Tip **WDB-324** navmesh `u32 = 0_usize` | ✅ MultiFile + tip gen GREEN (P3.400) |
| Binary / MCP / Gate 1 | ❌ NO_BINARY |

**Error mix (tip gen, pre-P3.400):** E0308 197 · E0277 76 · E0507 20 · E0505 18 · E0596 14 · E0599 5 (split/Into — Vec.to_string largely gone post-P3.390).

**TDD:** `cargo test --release --test all --features integration_tests,codegen_tests -- wdb324_` → **2 passed** (P3.400).

**Compiler agent priority:** next game-core tip E0308 after WDB-324. Game session: repros + product dogfood only — no `windjammer/src` edits.

## P3.399 WindjammerDB CQ-C5 — WDB-325 sql_exec + WDB-321 gate accuracy (2026-09-19)

| Gate | Status |
|------|--------|
| Tip **WDB-325** DF analytic `sql_exec(..., &emit.table, …)` | ✅ tip GREEN (P3.401) — demoted `&str` formals (WDB-244); MultiFile owned free-fn GREEN |
| Tip **WDB-321** LSQB edge `&content` | ✅ tip GREEN after call-line-only assert (was false-RED on `split_lines(&content)`) |
| Tip **WDB-324** navmesh `u32=0_usize` | ✅ GREEN (P3.400) |
| Tip **WDB-311/313/317** | ✅ tip GREEN (P3.401) |

**TDD:** `cargo test --release --test all --features integration_tests,codegen_tests -- wdb321_tip wdb324_ wdb325_`

**Compiler agent priority:** next game-core tip REDs **WDB-326–329**. No Phase 606+. No `windjammer/src` edits from DB agent.

## P3.400 — WDB-308/324 u32 return-width counters vs `.len()` (2026-09-19)

| Gate | Status |
|------|--------|
| MultiFile **WDB-308** `-> u32` + `args.len()` / `args[i+1]` | ✅ GREEN |
| Tip-out **WDB-308** wave1 artifact/bench/publish CLI | ✅ GREEN (tip regen) |
| MultiFile **WDB-324** navmesh `find_triangle` → `Option<u32>` | ✅ GREEN |
| Tip gen **WDB-324** game-core `gen/ai/navmesh.rs` | ✅ GREEN (tip regen) |

**Root cause:** index analysis marked loop counters as `usize` while return-width ascription emitted `: u32` → `let mut i: u32 = 0_usize`. Binary indices `i + 1` skipped usize cast when peers looked like usize.

**Fix:** `function_returns_u32_for_loop_scan` + `u32_return_scan_counter` (parallel to i32 path) so return/later u32 peer wins over `usize_variables`; `index_expr_needs_u32_or_i32_usize_cast` forces `(i + 1) as usize`.

**TDD:** `cargo test --release --test all --features integration_tests,codegen_tests -- wdb308_ wdb324_` → **4 passed**.

**Compiler agent priority:** next game-core tip REDs **WDB-326–329**. No Phase 606+.

## P3.401 — tip-out sync + WDB-325 demoted sql_exec gate (2026-09-19)

| Gate | Status |
|------|--------|
| Tip **WDB-311/313/317** `u32=0_usize` tip-out/gen | ✅ GREEN (tip regen after P3.400) |
| Tip **WDB-325** DF analytic `&emit.*` into sql_exec | ✅ GREEN — formals demoted `&str` (WDB-244); gate skips when demoted |
| MultiFile **WDB-325** owned free-fn isolate | ✅ GREEN |

**Root cause (325 false-RED):** tip-out asserted no `&emit.*` while cross-crate `ArrowColumnarBatch::sql_exec` is intentionally demoted to `&str`.

**Fix:** tip gate checks columnar gen formals; only fail when `left_table: String` still receives `&emit.*`.

**TDD:** `cargo test --release --test all --features integration_tests,codegen_tests -- wdb311_ wdb313_ wdb317_ wdb325_` → **5 passed**.

**Compiler agent priority:** game-core tip E0308 cluster **WDB-326+**; stdlib form/glob wiring REDs. No Phase 606+.

## P3.383 — tip-out residual gate accuracy + Custom demotion Borrow (2026-09-19)

| Gate | Status |
|------|--------|
| tip residual cluster 177/193/196/204/208/212/233/245/248/253/256/257/262/269 | ✅ **18/18** with spawn/mpsc |
| P0 `thread_spawn_closure` / P1 `mpsc_sync_channel` | ✅ tip GREEN (reconfirmed) |

**Root cause layer:** signature/tip-formal evolution (owned vs demoted) — tip-out gates were false-RED on tip-correct shapes; IR call-site: demoted `&T` emit sets expected Ref + Identity→Borrow for owned Custom locals (WDB-212); removed stale `gen/relational_module_file/`.

**What became unnecessary:** broad tip-out asserts that treated any `.clone()` / bare lit as RED regardless of formal ownership.

**Gates:** `cargo test --release --test all --features integration_tests,codegen_tests -- wdb177_tip_out wdb193_tip_out … wdb269_tip_out thread_spawn_closure mpsc_sync_channel` → **18 passed**.

## P3.382 — tip-out bulk module-file sync (2026-09-18)

| Gate | Status |
|------|--------|
| tip_out filter (~101 gates) | ✅ **87 passed** / 14 failed after `/tmp/wj_tip_out_regen_p380` → `.agent-wip/rel_tip_out` + `gen/` |
| Tip **WDB-241** ecs `&ids`→owned | ✅ tip GREEN (regen emits `ids` move) |
| Tip **WDB-248** multi_source | ⚠️ tip evolved: formal now owned `DenseCsr` + `self.csr.clone()`; gate updated to accept owned+clone or demoted+reborrow |
| Residual tip_out RED | 177/193/196/204/208/212/233/245/253/256/257/262/269 (+248 until rebuild) — mix of tip codegen + stale `gen/relational_module_file/` |

**Root cause layer:** tip-out/gen lag for greened cluster; residual needs signature/IR (Custom Key, clone→demoted Custom, u64 width) not peels.

**Gates:** `all-… tip_out` → 87/101; `wdb283..wdb296` tip-out → 10/10.

## P3.379 — tip-out regen greened WDB-283/284/289–296 + wave1_cli flat lag (2026-09-18)

| Gate | Status |
|------|--------|
| Tip **WDB-283** incremental `&csr` → owned `bfs_run_dense` | ✅ tip GREEN (module-file tip-out) |
| Tip **WDB-284** `csr.clone()` → demoted `&mut` afforest | ✅ tip GREEN (tip-out→gen sync) |
| Tip **WDB-289–290** dremel/sf1 floor | ✅ tip GREEN |
| Tip **WDB-291/292/294/295** wave1 CLI `&args`→owned | ✅ tip GREEN after flat `wave1_cli.rs` sync (nested already correct) |
| Tip **WDB-293/296** | ✅ already GREEN |
| MultiFile **WDB-297** CLI `&args`→owned isolate | ✅ tip GREEN (P3.379) — tip emits `args.clone()` |

**Root cause layer:** signature (bare Vec/DenseCsr owned + Clone↛Borrow) + tip-out/gen lag (flat `wave1_cli.rs` stale while nested was greened).

**What became unnecessary:** no new reconcile peels — tip `wj` module-file already emits `args.clone()` / `csr.clone()` / move; sync closed gates.

**Gates:** `all-… wdb283_tip_out wdb284_gen wdb289_tip_out wdb290_tip_out wdb291_tip_out wdb292_tip_out wdb293_tip_out wdb294_tip_out wdb295_tip_out wdb296_gen` → **10 passed**.

## P3.377 WindjammerDB CQ-C5 — coverage REDs WDB-293–296 (2026-09-18)

| Gate | Status |
|------|--------|
| Tip **WDB-293** LCC `&Vec` trio → owned `simd_lcc_bind` | ✅ tip GREEN (tip-out regen) |
| Tip **WDB-294** wave1 `&args` → owned `report_cli_is_report` | ✅ tip GREEN (P3.379 wave1_cli) |
| Tip **WDB-295** wave1 `&args` → owned `scale_status_cli_main` | ✅ tip GREEN (P3.379 wave1_cli) |
| Gen-lag **WDB-296** join_path `String::from` vs tip bare | ✅ GREEN after tip-out→gen sync |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** remaining queue 🆕/❌ (245–276 cluster tip-out lag); P3.383 owned.clone→`&str` reborrow. No Phase 606+.

## P3.383 — owned `.clone()` into demoted `&str` must reborrow (2026-09-18)

| Gate | Status |
|------|--------|
| `owned_str_clone_into_demoted_str_must_reborrow` | ✅ tip GREEN |
| Twin tip-out **WDB-270/272** wave1 `&owned.clone()` | 🆕 RED / filed (tip-out lag) |

**Root cause:** `finalize_explicit_user_clone_call_site` restored `.clone()` for owned caller text params even when the callee emitted `&str`, producing `helper(owned.clone())` (E0308) / `&owned.clone()`.

**Fix:** when callee emits shared-ref / `&str` (or demoted plain WJ string), strip explicit user clone and reborrow `&binding` (WDB-270 class).

**Gate:** `cargo test --test all --features integration_tests owned_str_clone_into_demoted_str_must_reborrow -- --nocapture` → pass (2026-09-18 GREEN incl. cargo-check).

## P3.376 — WDB tip-out Vec→owned cluster regen + finance raw-string gate (2026-09-18)

| Gate | Status |
|------|--------|
| Tip **WDB-281/282/285–288** | ✅ tip GREEN after module-file tip-out sync |
| `reused_vec_second_arg_into_owned_callee_must_clone_not_reborrow` | ✅ tip GREEN |
| finance-screens `String::from` into demoted `&str` fixture | ✅ tip GREEN (`bug_finance_screens_string_lit_into_demoted_str_block…`) |
| Product `list_panels` / aging `kind` | ⏳ tip rebuild after multi-sig peel + emitted_rust_ref_formals |

**Root cause layer:** signature (P3.373–375 bare Vec owned + Clone↛Borrow) + tip-out lag.

**What became unnecessary:** stale tip-out `&samples` / `&items` once tip `wj` module-file regen lands.

**Gates:** `cargo test --release --test all -- wdb281_tip_out wdb282_tip_out wdb285_tip_out wdb286_tip_out wdb287_tip_out wdb288_tip_out reused_vec_second_arg owned_vec_reuse` → pass.

## P3.375 WindjammerDB CQ-C5 — coverage REDs WDB-289–292 &Vec→owned (2026-09-18)


| Gate | Status |
|------|--------|
| Tip **WDB-289** dremel `&fields` → owned `fields_by_ordinal` | ✅ tip GREEN (P3.377 tip-out regen) |
| Tip **WDB-290** wave1 `&args` → owned `parse_sf1_floor` | ✅ tip GREEN (P3.377 tip-out regen) |
| Tip **WDB-291** wave1 `&args` → owned publish_check_cli | ✅ tip GREEN (P3.379 wave1_cli) |
| Tip **WDB-292** wave1 `&args` → owned attest_cli | ✅ tip GREEN (P3.379 wave1_cli) |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** remaining queue ❌ (gen-lag / tip-out twins). Product tip finance-screens GREEN. No Phase 606+.

## P3.381 — windjammer-ui AuthFetch `impl Into<String>` builders (2026-09-18)

| Gate | Status |
|------|--------|
| `auth_fetch_string_builders_accept_impl_into_string` | ✅ GREEN |
| Product finance-screens AuthFetch call sites | can drop `.to_string()` / `String::from` on builders |

**Fix:** hand-maintained `generated/authfetch.rs` — `new` / `id` / `label` / `mount` / `class_name` take `impl Into<String>` (matches JsonPost / DatePicker / Chart).

**Gate:** `cargo test --test finance_p0_components_test auth_fetch -- --nocapture` → pass.

## P3.378 WindjammerDB CQ-C5 — MultiFile CLI `&args`→owned (WDB-297) (2026-09-18)

| Gate | Status |
|------|--------|
| MultiFile **WDB-297** owned CLI Vec reuse must clone | ✅ tip GREEN — regression guard; tip-out 291/292/294/295 GREEN (P3.379) |

**Gate:** `cargo test --release --test all --features integration_tests -- wdb297_ -- --nocapture`

## P3.374 — if/else float literal peers `f32` then-branch (2026-09-18)

| Gate | Status |
|------|--------|
| `let_if_else_f32_branch_must_peer_else_float_literal` | ✅ tip GREEN |

**Product:** `skeleton.wj` `let isx = if sx > … { 1.0 / sx } else { 0.0 }` emitted else as `0.0_f64` (~36× E0308 expected f32 found f64).

**Root cause:** Else-branch bare float literals defaulted to f64 without peering the then-branch's effective f32 type (`1.0 / sx` with f32 `sx`).

**Fix:** `if_else_branch_expr_float_type` + let-binding peer for if/else float tails (`statement_generation` / `let_statement_generation`).

**Gate:** `cargo test --test all --features integration_tests let_if_else_f32_branch_must_peer_else_float_literal -- --nocapture` → pass (2026-09-18 GREEN incl. cargo-check).

## P3.373 — owned Vec reuse into owned callee (`if` forward-ref) (2026-09-18)

| Gate | Status |
|------|--------|
| `owned_vec_reuse_into_owned_callee_must_clone_not_reborrow` | ✅ tip GREEN |

**Root cause:** `coerce_forward_ref_params_in_if_condition` rewrote `contains(items, …)` → `contains(&items, …)` for if-facade forward-ref params even when the callee slot emits owned `Vec<i64>`. `coerce_owned_params_clone_in_if_condition` skipped the same params, so reuse after the call never got `items.clone()`.

**Fix:** Skip forward-ref `&` rewrite when `expr_call_expects_owned_formal_for_param` (incl. `bare_formal_is_vec_or_map` + preregistered owned formals). Allow owned clone coercion for if-facade params into owned callees. DRY helpers: `clone_reused_binding_for_owned_vec_formal`, `bare_formal_is_vec_or_map` codegen-owned beat stale `Reference(Vec)`.

**Gate:** `cargo test --test all --features integration_tests,codegen_tests owned_vec_reuse_into_owned_callee_must_clone -- --nocapture` → pass (2026-09-18 GREEN incl. cargo-check).

## P3.372 WindjammerDB CQ-C5 — coverage REDs WDB-285–288 second-arg &Vec→owned (2026-09-18)

| Gate | Status |
|------|--------|
| Tip **WDB-285** sysbench `&samples` → owned `workload_verdict` | ✅ tip GREEN (P3.376) — tip-out synced |
| Tip **WDB-286** tpch `&samples` → owned `query_verdict` | ✅ tip GREEN (P3.376) — tip-out synced |
| Tip **WDB-287** wave1 `&line`/`&ord` → owned `session_from_batches` | ✅ tip GREEN (P3.376) — tip-out synced |
| Tip **WDB-288** pubsub `&backlog` → owned `live_poll` | ✅ tip GREEN (P3.376) — tip-out synced |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** tip greens **285–288**; signature-driven clone into owned Vec second/later args. No Phase 606+.

## P3.370 WindjammerDB CQ-C5 — coverage REDs WDB-281–284 + wj-sync i64 peers (2026-09-18)

| Gate | Status |
|------|--------|
| Tip **WDB-281** lsqb `&Vec` → owned `lsqb_vec_contains` | ✅ tip GREEN (P3.375/376) — bare Vec AST owned; tip-out synced |
| Tip **WDB-282** pg_wire `&Vec` → owned int64_matrix | ✅ tip GREEN (P3.376) — tip-out synced |
| Tip **WDB-283** incremental `&csr` → owned bfs_run_dense | ✅ tip GREEN (P3.379 tip-out) |
| Tip **WDB-284** csr.clone() → demoted `&mut` afforest | ✅ tip GREEN (P3.379 tip→gen) |
| `bug_wj_sync_int_literal_peers_must_emit_i64_test` | ✅ tip GREEN (P3.370) |
| Dogfood / tip-cluster | ❄️ frozen |

**Compiler agent priority:** MultiFile **WDB-297**; remaining queue ❌. No Phase 606+.

## P3.384 (2026-09-19) — AtomicI64 fields must not auto-derive Clone

**Symptom:** `pub struct Counter { inner: AtomicI64 }` emitted `#[derive(Debug, Clone)]` → rustc E0277 (`AtomicI64: !Clone`).

**Root cause:** `is_std_non_auto_debug_clone_type` omitted atomics (same class as `mpsc::Receiver`).

**Fix:** list `AtomicBool` / `AtomicI64` / `AtomicU64` / … in `type_classification::is_std_non_auto_debug_clone_type`.

**Test:** `bug_atomic_i64_struct_must_not_auto_derive_clone_test` (PASSING).

**Note:** `wj-sync` keeps `Arc<AtomicI64>` Counter for Clone handles; bare AtomicI64 is now legal when Clone is not required. Do not add a WJ `SharedInt` stub in `std/sync.wj` — Copy stub poisons package `type SharedInt = Shared<int>`.

## P3.389 (2026-09-19) — eco wj-semver / wj-toml string ownership

**Symptom:**
- `wj-semver`: owned `String` params into demoted `&str` sibling (`cmp_string`) without `&` at tail call.
- `wj-toml`: demoted `key: &str` into `Vec<(String, String)>::push` emitted `key.clone()` → `&str`.

**Fix:** registry `&str` formals force call-site borrow; demoted text `.clone()` into owned String → `.to_string()`.

**Gates:** `bug_eco_wj_semver_owned_into_demoted_str_tail_call_must_borrow_test`, `bug_eco_wj_toml_demoted_str_tuple_push_must_to_string_test` (PASSING).

**Ecosystem:** `wj-timefmt` / `wj-semver` / `wj-toml` / `wj-config` tip GREEN.


## P3.401 — string call arg into string formal must not cast i32 (2026-09-19)

| Gate | Status |
|------|--------|
| `string_call_arg_into_string_formal_must_not_cast_i32` | ❌ RED (filed) |
| Tip-out `gen/audio/mixer.rs` `(name as i32)` | ❌ RED |

**Product:** `AudioChannel::new(id, name)` → `AudioChannel::new(id, (name as i32))` (E0277 From<i32>).

**Isolate:** same-file GREEN; multipass sibling `audio_mixer::AudioChannel::new(id, priority: i32)` poisons `mixer` string call (filed dual-mod fixture).

**Compiler agent:** resolve `AudioChannel::new` to the *local* type's signature — never apply sibling-module `i32` formal casts to a `string` arg.

## P3.402 — `str.split('.')` must not emit `split(&'.')` (2026-09-19)

| Gate | Status |
|------|--------|
| `str_split_char_literal_must_not_emit_amp_char` | ✅ MultiFile GREEN |
| Tip-out asset_browser / scene_saver | ✅ tip GREEN (peel `&'.'` in Pattern path + immut method emit) |

**Product:** `path.split('.')` → `path.split(&'.')` (E0277 Pattern).

**Fix:** `peel_amp_from_char_literal_arg` in Pattern normalizer + unconditional peel in method finalize / immut expression emit when Pattern formal lookup misses.

## P3.403 — neg `while` counter vs literal peers must not widen i64 (2026-09-19)

| Gate | Status |
|------|--------|
| `i32_neg_while_literal_peers_must_not_widen_i64` | ✅ MultiFile GREEN (consistent i32) |
| Tip-out `gen/ai/npc_behavior.rs` SearchState | ❌ RED (`-1_i64` + `1_i32`) |

**Product:** `let mut i = -1; while i <= 1` → `i = -1_i64; while i <= 1_i32` (E0308/E0277). Small isolate stays i32; full-library multipass widens.

**Compiler agent:** under library multipass, keep SearchState-shaped neg while counters + lit peers one integer width (prefer i32).


## P3.404 WindjammerDB CQ-C5 — game-core tip REDs WDB-326–329 (2026-09-19)

| Gate | Status |
|------|--------|
| Tip **WDB-326** HashMap f32 get `Some(v) => *v` (astar/navmesh) | ✅ GREEN (P3.406) — block_generation skips `*v` after `.copied()` |
| Tip **WDB-327** i32 coord `== goal as usize` (astar) | ✅ GREEN (P3.408) — indexed field load stays i32 |
| Tip **WDB-328** i64 neg-init loop `_i32` lits (npc_behavior) | ✅ GREEN (P3.409) — prefers_i32 bare int/neg-int init; tip regen clean |
| Tip **WDB-329** `Vec::remove(&idx as usize)` (blackboard) | ✅ GREEN (P3.409) — Owned Copy/usize skips shared-ref emit; Cast excluded from bare `&` peels |
| Tip **WDB-325** sql_exec | ✅ GREEN (P3.401 demoted-formal gate) |
| Game-core cargo | ❌ **332** errors (E0308×188) |

**TDD:** `wdb326_ wdb327_ wdb328_ wdb329_` → WDB-326–329 **GREEN** (P3.406/P3.408/P3.409).

**Compiler agent priority:** tip greens **WDB-330–336**. No Phase 606+. No `windjammer/src` edits from DB agent.


## P3.405 WindjammerDB CQ-C5 — game-core tip REDs WDB-330–333 (2026-09-19)

| Gate | Status |
|------|--------|
| Tip **WDB-330** u32 `>>`/`&` with `_i64` lits (fps_camera) | 🆕 RED / filed |
| Tip **WDB-331** owned Vec3 `&test_x` into collides_aabb | 🆕 RED / filed |
| Tip **WDB-332** i32 priority `.to_string()` into AudioChannel::new | ✅ GREEN (P3.418 affinity) |
| Tip **WDB-333** format `_temp` `&_temp1` into owned String load path | 🆕 RED / filed |
| Tip **WDB-326–329** | ⚠️ WDB-326 GREEN (P3.406); 327–329 still RED |

**TDD:** `cargo test --release --test all --features integration_tests,codegen_tests -- wdb330_ wdb331_ wdb332_ wdb333_`

**Compiler agent priority:** tip greens **WDB-330–333** (+ remaining **327–329**). No Phase 606+. No `windjammer/src` edits from DB agent.

## P3.406 — WDB-326 HashMap f32 get `Some(v) => *v` after `.copied()` (2026-09-20)

| Gate | Status |
|------|--------|
| MultiFile **WDB-326** `let g = match scores.get(...).copied()` | ✅ GREEN |
| Tip gen **WDB-326** astar_grid / navmesh | ✅ GREEN (tip regen) |

**Root cause:** `block_generation` `let x = match` path appended `.copied()` for Copy map values but still rewrote `Some(v) => v` → `*v` via `match_binds_refs` (ignored `use_copied_option`).

**Fix:** gate the `*v`/`.clone()` arm rewrite on `!use_copied_option` (same as `match_binds_refs_flag`).

**TDD:** `cargo test --release --test all --features integration_tests,codegen_tests -- wdb326_` → **2 passed**.

**Compiler agent priority:** tip greens **WDB-327–329**. No Phase 606+.

## P3.407 WindjammerDB CQ-C5 — game-core tip REDs WDB-334–336 (2026-09-20)

| Gate | Status |
|------|--------|
| Tip **WDB-334** tps `collides_point(..., &sample, …)` owned Vec3 | 🆕 RED / filed — MultiFile GREEN; tip RED |
| Tip **WDB-335** tps `collides_point(grid.clone(), …)` into `&VoxelGrid` | 🆕 RED / filed — MultiFile GREEN; tip RED |
| Tip **WDB-336** mesh_renderer `push_mat4(&mut data.clone(), …)` | 🆕 RED / filed — MultiFile GREEN; tip RED |
| Tip **WDB-326** | ✅ GREEN (P3.406) |
| Tip **WDB-330–333** / **328–329** | ❌ still tip RED |

**TDD:** `cargo test --release --test all --features integration_tests,codegen_tests -- wdb334_ wdb335_ wdb336_` → **3 MultiFile passed / 3 tip failed** (expected tip RED).

**Compiler agent priority:** tip greens **WDB-334–336** (+ **330–333**, **327–329**). No Phase 606+. No `windjammer/src` edits from DB agent.


**Also (WDB-327 tighten):** indexed `open_set[best_idx_usize].x == goal_x` MultiFile was RED under P3.407; **GREEN in P3.408**.

## P3.408 — WDB-327 indexed i32 field vs `goal as usize` (2026-09-20)

**Bug:** `let current_x = open_set[best_idx_usize].x` then `current_x == goal_x` emitted `goal_x as usize` (and later `current_x as i32` for neighbors).

**Root cause layer:** (temporary) reconcile — `reconcile_ambiguous_int_local_after_let` used `emitted_rhs.contains("_usize")`, which matched the *identifier* `best_idx_usize` inside the field-load RHS and falsely widened the i32 binding to usize.

**Fix:** Replace broad `contains("_usize")` with `emitted_rhs_indicates_usize_literal_width` (digit-before-`_usize` / ` as usize` / bare `*_usize` binding copy only).

**What became unnecessary:** Heuristic that treated any `_usize` substring in emitted RHS as usize width.

**Gates:** `cargo test --release --test all -- wdb327_` → **3 passed**; `usize_i_plus_one_assign i32_inferred_loop_counter wdb327_ wdb308_ wdb303_` → **9 passed**.


## P3.409 WindjammerDB CQ-C5 — game-core tip REDs WDB-337–339 (2026-09-20)

| Gate | Status |
|------|--------|
| Tip **WDB-337** `&mut quads/grid/d.clone()` (meshing/viewer/vox) | 🆕 RED / filed — MultiFile GREEN; tip RED; twin WDB-336 |
| Tip **WDB-338** `push_quad(mesh.clone(), …)` into `&mut Mesh` | 🆕 RED / filed — MultiFile GREEN; tip RED |
| Tip **WDB-339** `cy + N_i64 as i32` (component_viewer) | 🆕 RED / filed — MultiFile GREEN; tip RED |
| Tip **WDB-334–336** | ❌ still tip RED (P3.407) |
| Tip **WDB-327** indexed | ✅ GREEN (P3.408) |

**TDD:** `cargo test --release --test all --features integration_tests,codegen_tests -- wdb337_ wdb338_ wdb339_` → **3 MultiFile passed / 3 tip failed** (expected tip RED).

**Compiler agent priority:** tip greens **WDB-337–339** (+ **334–336**, **330–333**). **WDB-328–329** GREEN (P3.409). No Phase 606+. No `windjammer/src` edits from DB agent.


## P3.410 WindjammerDB CQ-C5 — game-core tip REDs WDB-340–342 (2026-09-20)

| Gate | Status |
|------|--------|
| Tip **WDB-340** `.to_string().to_string()` (bt/scripting/editor/sprite) | 🆕 RED / filed — MultiFile GREEN; tip RED |
| Tip **WDB-341** `String::from(...).to_string()` (bt_validation) | 🆕 RED / filed — **MultiFile + tip RED** |
| Tip **WDB-342** `&mut active.clone()` (BT executor/csg/coverage/mesh_gen) | 🆕 RED / filed — MultiFile GREEN; tip RED |
| Tip **WDB-334/335** tps sample/grid polarity | ✅ tip GREEN (gen regen: `collides_point(grid, sample, …)`) |
| Tip **WDB-336–339** / **330–333** | ❌ still tip RED |

**TDD:** `cargo test --release --test all --features integration_tests,codegen_tests -- wdb340_ wdb341_ wdb342_ wdb334_tip wdb335_tip` → **4 passed / 4 failed** (334/335 tip + 340/342 MultiFile GREEN; 340/341/342 tip RED; 341 MultiFile RED).

**Compiler agent priority:** tip greens **WDB-340–342** (+ **336–339**, **330–333**). No Phase 606+. No `windjammer/src` edits from DB agent.


## P3.411 WindjammerDB CQ-C5 — game-core tip REDs WDB-343–345 (2026-09-20)

| Gate | Status |
|------|--------|
| Tip **WDB-343** Copy i32 `.clone()` (viewer/fps/svo) | ✅ MultiFile + tip GREEN (P3.425) |
| Tip **WDB-344** Copy f32 `.clone()` (collision2d) | 🆕 RED / filed — MultiFile GREEN; tip RED |
| Tip **WDB-345** `buf.clone()` into `&mut Vec` (csg) | 🆕 RED / filed — MultiFile GREEN; tip RED |
| Tip **WDB-330/331** fps bitwise / Vec3& | ✅ tip GREEN (P3.411 regen) |
| Tip **WDB-332–333**, **336–342**, **339** | ❌ still tip RED |

**TDD:** `wdb343_ wdb344_ wdb345_ wdb330_tip wdb331_tip` → **4 passed / 4 failed** (330/331 tip + 344/345 MultiFile GREEN; 343 MultiFile+tip RED; 344/345 tip RED).

**Compiler agent priority:** tip greens **WDB-343–345** (+ **332–333**, **336–342**). No Phase 606+. No `windjammer/src` edits from DB agent.


## P3.412 WindjammerDB CQ-C5 — game-core tip REDs WDB-346–348 (2026-09-20)

| Gate | Status |
|------|--------|
| Tip **WDB-346** Copy f32 field `.x/.y/.z.clone()` (tps/fps camera) | 🆕 RED / filed — MultiFile GREEN; tip RED; twin WDB-344 |
| Tip **WDB-347** Copy f32 match binding `r.clone()`/`h.clone()` (jolt) | ✅ MultiFile + tip GREEN (P3.425) |
| Tip **WDB-348** owned String `path.clone().to_string()` (loader) | 🆕 RED / filed — MultiFile GREEN; tip RED; twin WDB-340 |
| Tip **WDB-332–333**, **336–345**, **339–342** | ❌ still tip RED |

**TDD:** `cargo test --release --test all --features integration_tests,codegen_tests -- wdb346_ wdb347_ wdb348_` → **2 passed / 4 failed** (346/348 MultiFile GREEN; 347 MultiFile+tip RED; all three tip RED).

**Compiler agent priority:** tip greens **WDB-346–348** (+ **332–333**, **338–341**, **343–345**). **WDB-336/337/342** GREEN (P3.413). No Phase 606+. No `windjammer/src` edits from DB agent.

## P3.413 — WDB-336/337/342 `&mut place.clone()` temps (2026-09-20)

| Gate | Status |
|------|--------|
| MultiFile + tip **WDB-336** mesh_renderer `push_mat4(&mut data, …)` | ✅ GREEN |
| MultiFile + tip **WDB-337** meshing/viewer/vox `&mut quads` | ✅ GREEN |
| MultiFile + tip **WDB-342** BT executor `&mut active`/`&mut running` | ✅ GREEN |
| MultiFile + tip **WDB-338** placeholder_assets `push_quad(&mut mesh, …)` | ✅ GREEN (tip regen) |

**Bug:** Local `let mut data = Vec::new()` reused into demoted `&mut Vec` emitted `&mut data.clone()` (temp / E0716 / discarded mutation).

**Root cause:** Auto-clone for reuse fired before/alongside mut demotion; free-call `apply_callee_mut_borrow_to_call_args` wrapped without stripping `.clone()`, and already-`&mut`-prefixed args skipped sanitize.

**Fix:**
- `sanitize_mut_borrow_clone_temp` / `apply_mut_borrow_prefix` — rewrite `&mut place.clone()` → `&mut place`
- Terminal sanitize on free-call args; strip before every `&mut` wrap (ir_call_site, typed_lowering, BorrowMut)
- Never auto-clone override into mut formals; never append `.clone()` onto `&mut` places
- Tip gates match mut-arg clone temps only (not owned peer `.clone()` on same line)

**TDD:** `cargo test --test all --features integration_tests -- wdb336_ wdb337_ wdb342_ wdb338_` → **8 passed**.

**Compiler agent priority:** tip greens **WDB-339–341**, **343–348**, **332–333**. No Phase 606+.


## P3.414 WindjammerDB CQ-C5 — tip REDs WDB-349–351 + mark 339 GREEN (2026-09-20)

| Gate | Status |
|------|--------|
| Tip **WDB-349** `matches!(mesh.clone(), Some(_))` (usd) | 🆕 RED / filed — MultiFile GREEN; tip RED |
| Tip **WDB-350** `(key as usize).clone()` (save manager) | 🆕 RED / filed — MultiFile GREEN; tip RED |
| Tip **WDB-351** `match body.shape.clone()` / `format.clone()` | 🆕 RED / filed — MultiFile GREEN; tip RED |
| Tip **WDB-339** i32 coord `N_i64 as i32` | ✅ tip GREEN (regen; `3_i32` peers) |
| Tip **WDB-336/337/342** | ✅ tip GREEN (P3.413) |

**TDD:** `wdb349_ wdb350_ wdb351_ wdb336_tip wdb339_tip wdb342_tip` → **6 passed / 3 failed** (MultiFile + 336/339/342 tip GREEN; 349–351 tip RED).

**Compiler agent priority:** tip greens **WDB-349–351** (+ **332–333**, **338**, **340–341**, **343–348**). No Phase 606+. No `windjammer/src` edits from DB agent.


## P3.415 WindjammerDB CQ-C5 — tip REDs WDB-352–354 (2026-09-20)

| Gate | Status |
|------|--------|
| Tip **WDB-352** redundant `0_i32 as i32` / `1_i32 as i32` | ✅ MultiFile + tip GREEN (P3.417) — strip same-width cast after branch/return unify |
| Tip **WDB-353** `as i32 as usize` (terrain) | 🆕 RED / filed — MultiFile GREEN; tip RED |
| Tip **WDB-354** Result `.map(\|v\| v.to_owned())` (loader) | 🆕 RED / filed — MultiFile GREEN; tip RED |

**TDD:** `wdb352_` → **2 passed** (tip regen + gen sync).

**Compiler agent priority:** tip greens **WDB-353–354** (+ **332–333**, **340–351**, **355–357**). No Phase 606+. No `windjammer/src` edits from DB agent.


## P3.416 WindjammerDB CQ-C5 — tip REDs WDB-355–357 (2026-09-20)

| Gate | Status |
|------|--------|
| Tip **WDB-355** Copy Vec3 `n.clone()` (placeholder) | 🆕 RED / filed — MultiFile GREEN; tip RED |
| Tip **WDB-356** Copy Mat4 `self.clone().multiply` | 🆕 RED / filed — **MultiFile + tip RED** (mul demoted to `&self` + clone) |
| Tip **WDB-357** `impl Into<String>` + `.into()` | 🆕 RED / filed — MultiFile GREEN; tip RED |
| Tip **WDB-338** | ✅ tip GREEN (P3.413) |

**TDD:** `wdb355_ wdb356_ wdb357_ wdb338_tip` → **3 passed / 4 failed** (355/357 MultiFile + 338 tip GREEN; 356 MultiFile+tip RED; 355/357 tip RED).

**Compiler agent priority:** tip greens **WDB-355–357** (+ **332–333**, **340–354**). No Phase 606+. No `windjammer/src` edits from DB agent.


## P3.417 WindjammerDB CQ-C5 — tip REDs WDB-358–360 + mark 346 GREEN (2026-09-20)

| Gate | Status |
|------|--------|
| Tip **WDB-358** `self.clone().method()` on `&self` (streaming/squad/…) | 🆕 RED / filed — MultiFile GREEN; tip RED |
| Tip **WDB-359** `chunks[i].clone().coord` (chunk_manager) | ✅ MultiFile GREEN (P3.421); tip-out pending regen |
| Tip **WDB-360** `encode(grid.clone())` owned formal force-clone | 🆕 RED / filed — MultiFile GREEN; tip RED |
| Tip **WDB-346** Copy f32 field `.x/.y/.z.clone()` | ✅ tip GREEN (regen) |

**TDD:** `wdb358_ wdb359_ wdb360_ wdb346_tip` → **3 passed / 4 failed** (358/360 MultiFile + 346 tip GREEN; 359 MultiFile+tip RED; 358/360 tip RED).

**Compiler agent priority:** tip greens **WDB-358–360** (+ **332–333**, **340–345**, **347–357**). No Phase 606+. No `windjammer/src` edits from DB agent.


## P3.418 WindjammerDB CQ-C5 — tip REDs WDB-361–363 (2026-09-20)

| Gate | Status |
|------|--------|
| Tip **WDB-361** `(i as usize)` on usize counter | ✅ MultiFile + tip GREEN (P3.425) |
| Tip **WDB-362** `levels[i].clone().mesh_id()` | 🆕 RED / filed — MultiFile GREEN; tip RED |
| Tip **WDB-363** `a[idx].clone().rotation.N` | 🆕 RED / filed — MultiFile GREEN; tip RED |

**TDD:** `wdb361_ wdb362_ wdb363_` → **2 passed / 4 failed** (362/363 MultiFile GREEN; 361 MultiFile+tip RED; all three tip RED).

**Compiler agent priority:** tip greens **WDB-361–363** (+ **332–333**, **340–360**). No Phase 606+. No `windjammer/src` edits from DB agent.


## P3.419 WindjammerDB CQ-C5 — tip REDs WDB-364–366 + mark 332 GREEN (2026-09-20)

| Gate | Status |
|------|--------|
| Tip **WDB-364** `N_usize as usize` redundant lit cast | 🆕 RED / filed — MultiFile GREEN; tip RED |
| Tip **WDB-365** `__wj_tmpN` statement temps | 🆕 RED / filed — MultiFile GREEN; tip RED |
| Tip **WDB-366** BT `tree.clone()` owned formal force-clone | 🆕 RED / filed — MultiFile GREEN; tip RED |
| Tip **WDB-332** i32 priority `.to_string()` | ✅ tip GREEN (P3.418 affinity) |

**TDD:** `wdb364_ wdb365_ wdb366_ wdb332_tip` → **4 passed / 3 failed** (all MultiFile GREEN; 364–366 tip RED; 332 tip GREEN).

**Compiler agent priority:** tip greens **WDB-364–366** (+ open **333**, **340–363**). No Phase 606+. No `windjammer/src` edits from DB agent.


## P3.420 WindjammerDB CQ-C5 — tip REDs WDB-367–369 (2026-09-20)

| Gate | Status |
|------|--------|
| Tip **WDB-367** `None.clone()` on Option/Copy None | ✅ MultiFile GREEN (P3.421); tip-out pending regen |
| Tip **WDB-368** `&*flag` / `&*key` into `&str` formals | ✅ MultiFile GREEN; tip-out pending regen |
| Tip **WDB-369** `.key.clone() == key` string field eq | ✅ MultiFile GREEN (P3.421); tip-out pending regen |

**TDD:** `wdb367_ wdb368_ wdb369_` MultiFile GREEN (P3.421).

**Compiler agent priority:** tip-regen game-core for **359/367–369** (+ **343/347/361** lag); then **364–366**. No Phase 606+.


## P3.421 (2026-09-21) — coercion: stop Index/None/string-eq clones (WDB-359/367/369)

| Gate | MultiFile | Tip-out |
|------|-----------|---------|
| **WDB-359** `chunks[i].clone().coord` | ✅ bare `chunks[i].coord` | ✅ tip regen |
| **WDB-367** `None.clone()` struct/assign/push | ✅ bare `None` | ✅ tip regen (tilemap/sprite/…) |
| **WDB-368** `&*flag` | ✅ MultiFile | ✅ tip dialogue |
| **WDB-369** `.key.clone() ==` | ✅ `.key ==` | ✅ tip regen |

**Root cause layer:** coercion/encoding (+ IR enforce owned_coercion for unit keywords).

**What became unnecessary:**
- Index call-arg / owned-context `.clone()` when `in_field_access_object` (WDB-359)
- struct-literal + assignment `needs_clone("None")` (WDB-367)
- index-field String clone under comparison suppress (WDB-369)
- `owned_coercion_for_str_subslice("None")` / `append_rust_clone("None")` false clones

**Gates:** `cargo test --release --test all -- wdb359_module_file_ wdb367_module_file_ wdb368_module_file_ wdb369_module_file_`

**Still tip RED (MultiFile GREEN):** WDB-343 cast `.clone()`, WDB-347 jolt match, WDB-361 `(i as usize)` — follow-up.

## P3.425 (2026-09-22) — WDB-343/347/361 tip GREEN (Copy const + match + usize counter)

| Gate | MultiFile | Tip-out |
|------|-----------|---------|
| **WDB-343** path-qual Copy i32/f32 const + cast `.clone()` | ✅ | ✅ tip regen |
| **WDB-347** Copy f32 match bindings after `body.shape.clone()` | ✅ | ✅ tip jolt/world |
| **WDB-361** `-> i32` / Custom("i32") + `while i < …len()` | ✅ `usize` + `return i as i32` | ✅ tip systems/chunk/lod |

**Root cause layer:** constraint/codegen (int width + match binding ownership) — not reconcile peels.

**What became unnecessary / narrowed:**
- Return-width `Custom("i32")` let ascription no longer demotes `.len()` while-counters out of `usize_variables` (WDB-361; keeps WDB-308 `-> u32` peer)
- Expression-match `block_generation`: cloned scrutinee ⇒ owned arm payloads (no `r.clone()` / `h.clone()`)
- Match statement path: `value_str.ends_with(".clone()")` ⇒ `owned_bindings_from_copy_deref`
- Copy scalar module-const / cast trailing `.clone()` strip (WDB-343)

**Gates:** `cargo test --release --test all -- wdb343_ wdb347_ wdb361_` → **6 passed**

## P3.422 WindjammerDB CQ-C5 — tip RED WDB-370 + TDD 367–369 (2026-09-21)

| Gate | Status |
|------|--------|
| Tip **WDB-370** `].clone().coord.clone()` (streaming/chunk_manager) | 🆕 RED / filed |
| Tip **WDB-367–369** | ❌ tip RED — MultiFile ✅ GREEN (see P3.421) |

**TDD:** `wdb367_ wdb368_ wdb369_` → **3 passed / 3 failed** (expected tip RED). `wdb370_` → MultiFile + tip RED.

## P3.422 — demoted `&mut String` into owned String field (2026-09-20)

| Gate | Status |
|------|--------|
| `mut_string_formal_assign_to_owned_string_field_must_to_string` | ✅ MultiFile GREEN |
| Tip-out `editor/console` / `hierarchy_panel` | ✅ tip GREEN (regen) |

**Product:** `query: &mut String` → `self.search_query = query` (E0308).

**Fix:** assignment codegen `.to_string()` when MutBorrowed / demoted text formals assign into owned String fields.

## P3.423 — for-in self field under &mut self must clone when body uses self (2026-09-20)

| Gate | Status |
|------|--------|
| `for_in_self_field_mut_method_must_clone_or_index` | ✅ MultiFile GREEN (isolate `self.passes.clone()`) |
| Tip-out `voxel_gpu_passes::update_all_params` | ✅ tip GREEN (regen) |

**Product:** `for pass in &self.render_pipeline.passes` + `self.update_*()` → E0505/E0507.

**Fix:** nested self-field iterables on MutBorrowed self clear `needs_borrow` when body uses `self`, so the existing clone-before-iterate path runs.

## P3.424 — read-only method must not emit owned `self` (2026-09-20)

| Gate | Status |
|------|--------|
| `nested_self_field_in_struct_lit_must_not_force_owned_self` | 🆕 RED / isolate GREEN `&self` when parent bound first; RED on `self.quality.steps` in struct lit |
| Tip-out `update_raymarch_params(self)` | 🆕 RED |

**Product:** `update_raymarch_params(self)` (reads only) called from `&mut self` loop → E0507 move.

**Compiler agent:** infer `&self` (or `&mut self` if needed) for methods that do not consume `self`.

## P3.426 WindjammerDB CQ-C5 — tip REDs WDB-371–373 (2026-09-22)

| Gate | Status |
|------|--------|
| Tip **WDB-371** `].clone().position.clone()` (mesh_ops/half_edge) | 🆕 RED / filed |
| Tip **WDB-372** `match …].value.clone()` (blackboard) | 🆕 RED / filed |
| Tip **WDB-373** `].clone().path.clone()` (live_reload/…) | 🆕 RED / filed |

**TDD:** `wdb371_ wdb372_ wdb373_` → **3 passed / 3 failed** (all MultiFile GREEN; all tip RED).

**Compiler agent priority:** tip greens **WDB-371–373** (+ **364–370** open; **343/347/361** tip GREEN per P3.425). No Phase 606+. No `windjammer/src` edits from DB agent.

## P3.427 (2026-09-22) — WDB-367 `Value::None` constructor must not clone

| Gate | MultiFile | Tip-out |
|------|-----------|---------|
| **WDB-367** bare `None` + `tiles.push(None)` | ✅ | ✅ |
| **WDB-367** `Value::None` (qualified identifier) | ✅ no `.clone()` | ✅ tip graph regen |

**Root cause:** `Type::Variant` parses as one identifier (`Value::None`), not FieldAccess. Auto-clone treated it as a reuse binding.

**Fix:** skip unit keywords and `is_enum_variant_constructor_path` in `generate_identifier` / `maybe_auto_clone` / call-arg reuse.

**Gates:** `cargo test --test all -- wdb367_module_file`

**Compiler agent priority:** tip greens **WDB-371–373**. No Phase 606+.

# Compiler repro queue (dogfooding — do not work around in application code)



## P3.766 (2026-10-09) — collection-key lookup follows the map/set signature

**Root cause layer:** signature. `is_collection_key_lookup_with_project` treated a call as a map-key lookup only when the leaf was `get`, `contains_key`, `get_key_value`, or `remove`.

**What became unnecessary:** both of those leaf-name matches in `stdlib_method_traits.rs`. A qualified map or set callee is a key lookup when that type's signature borrows the first argument (`method_is_map_key_qualified_with_project` or a reference formal). A type-qualified owned formal (`Vec::remove`, `NoteStore::get`) stays fail-closed. Borrowed wrappers still fall through to map consensus.

**Gates:** `cargo test --release --test all -- test_vec_get slice_get_usize owned_i64_method_formal mutex_guard_hashmap_string_key_must_borrow shared_map_get_must_borrow hashmap_get_string_key map_get_must_borrow contains_key_borrows` — 9 passed. Full suite not re-run (about 1 GiB free; prior runs died writing `shared/debug`).


## P3.765 (2026-10-09) — `enumerate` index width comes from `Iterator<(usize, T)>`

**Root cause layer:** signature. The `for` loop marked the first tuple binding `usize` and skipped borrowed-iterator tracking when the method was spelled `enumerate`.

**What became unnecessary:** `extract_enumerate_index_var` (`method == "enumerate"`) in `for_statement_generation.rs`. `Iterator::enumerate` is recorded as `Iterator<(usize, T)>`. A tuple item whose first component is `usize` marks that binding even when the tail is still generic. Unsubstituted `(usize, T)` does not replace an inferred loop element (`HashMap::values` stays on inference).

**Gates:** `cargo test --release --test all -- char_indices_slice_must_use_usize enumerate_index_not_dereferenced hashmap_values_copy_elem` — 4 passed. Full suite not re-run (about 1 GiB free; prior runs died writing `shared/debug`).


## P3.764 (2026-10-09) — `char_indices` index width comes from `Iterator<(usize, char)>`

**Root cause layer:** signature. The `for` loop invented `(usize, char)` and marked the first binding `usize` when the method was spelled `char_indices`.

**What became unnecessary:** the `method == "char_indices"` element-type branch and `extract_char_indices_index_var` in `for_statement_generation.rs`. `String::char_indices` is recorded as `Iterator<(usize, char)>`. The loop item comes from that return (or a unanimous stdlib `Iterator<item>` when the receiver is unknown). A tuple item whose first component is `usize` marks that binding. `.enumerate()` moved onto `Iterator<(usize, T)>` in P3.765.

**Gates:** `cargo test --release --test all -- char_indices_slice_must_use_usize enumerate_index_not_dereferenced hashmap_values_copy_elem` — 4 passed. Full suite not re-run (about 1 GiB free before the compile).


## P3.763 (2026-10-09) — `values`/`keys` loops follow `Iterator<&T>`

**Root cause layer:** signature. `for` loops forced a borrowed iterator whenever the method was spelled `values` or `keys`.

**What became unnecessary:** `matches!(method, "values" | "keys")` in `for_statement_generation.rs`. A known receiver uses `lookup_method_signature`: the loop is borrowed when that return is `Iterator<&T>` or `Iterator<&mut T>` (`HashMap::values` / `keys` already record that). An unknown receiver is borrowed only when every stdlib row for the method agrees.

**Gates:** `cargo test --release --test all -- hashmap_values_copy_elem hashmap_get_double_ref e0507_final` — 15 passed. Full suite not re-run (free space was ~1.6 GiB before this compile; prior full runs died writing `shared/debug`).


## P3.762 (2026-10-09) — trait-call map bridge no longer filters on `get`/`remove`

**Root cause layer:** signature. `trait_call_generation` replaced an owned-Copy homonym with the HashMap key bridge only when the leaf was `get` or `remove`, and a second branch peeled `&` integer literals for those same leaves.

**What became unnecessary:** both `matches!(call_method, "get" | "remove")` branches. The HashMap bridge still comes from `hashmap_key_method_signature_for_wrapper` (map-key trait classification). An owned-Copy formal on an unknown or guard receiver is the homonym signal. The non-map peel uses `callee_user_arg_bare_formal_is_copy_pass_by_value` and skips collection-key sites. The integer-suffix scan is gone.

**Gates:** `cargo test --release --test all -- mutex_guard_hashmap_string_key_must_borrow test_vec_get slice_get_usize owned_i64_method_formal shared_map_get_must_borrow` — 9 passed. Full suite not re-run (about 1.9 GiB free; prior runs died writing `shared/debug`).


## P3.761 (2026-10-09) — copy-index peel follows the formal, not the leaf name

**Root cause layer:** signature. The post-IR peel of a leading `&` on method arguments ran only for `get` / `contains_key` / `get_key_value` / `remove`, and additionally forced `Vec::{method}` / `slice::{method}` for `get` and `remove`.

**What became unnecessary:** both leaf-name matches in `method_call_expression_generation/arguments.rs`. A leading `&` is peeled when the resolved signature (local, or stdlib `{Receiver}::{method}` when that receiver is known and its formal is owned Copy) says the slot is pass-by-value Copy, and the site is not a collection-key lookup. Map/set/wrapper receivers are unchanged. Missing signatures do not invent `Vec::get`.

**Gates:** `cargo test --release --test all -- test_vec_get slice_get_usize owned_i64_method_formal hashmap_get_must_borrow` — 5 passed; `owned_i64_method_formal_must_not_receive_ref_literal` first failed on a missing `cargo-target/verify` directory, then passed alone. `cargo test --release --test all -- hashmap_get_string_key map_get_must_borrow contains_key_borrows` — 2 passed (`sync_shared_map_get` isolate and product scanner). Full suite not re-run: the volume still has about 1.9 GiB free and the last two runs died writing `shared/debug`.


## P3.760 (2026-10-09) — full suite stopped on ENOSPC again

`cargo test --release --test all` wrote `shared/debug` (hyper, chrono, rayon) until the volume returned os error 28 (`EXIT:101`). Failures from `owned_string_formals_must_not_receive_borrow_at_call_site` through `test_passthrough_borrowed_convergence` are that write failure. Earlier tip-out names in the same log (`tip_out_voxel_gpu_passes_update_all_must_not_borrow_passes`, `tip_out_event_get_data_string_must_clone`, `i32_binding_compare_zero_tip_out_shader_graph_compiler`, `int_mul_into_u32_formal_tip_out_hybrid_renderer`) match existing stale-gen rows. `shared/debug` was removed after the run. Do not regenerate product `gen/`.


## P3.759 (2026-10-09) — field-access owned contract follows the qualified callee

**Root cause layer:** signature. Method-call field arguments re-bound `contract_sig` by matching leaf names `to_string` / `to_string_pretty` and then falling back to those stdlib keys even when the qualified callee was something else.

**What became unnecessary:** the leaf-name match and the unconditional `json::to_string` / `json::to_string_pretty` fallbacks in `method_call_expression_generation/arguments.rs`. The field-access clone now adopts `SignatureRegistry::stdlib()` only when `qualified_callee` itself has an owned emission contract (the same key `reconcile_post_ir` already uses).

WDB-201/203/206/214–217 isolates reconfirmed GREEN. Tip-out product `gen/` was not regenerated.

**Gates:** `cargo test --release --test all -- json_to_string mut_self_field_into_owned_json notes_api_json to_string_pretty` — 6 passed. `cargo test --release --test all -- wdb201_module_file wdb203_module_file wdb206_ wdb214_codegen wdb215_codegen wdb216_codegen wdb217_codegen` — 9 passed.


## P3.758 (2026-10-09) — full suite stopped for disk; five FAILs were flakes

`cargo test --release --test all` was killed when free space fell under 1.5 GiB (exit 2). It had reached the `n*` tests. Tip-out scanners in that log (`i32_binding_compare_zero_tip_out_shader_graph_compiler`, `int_mul_into_u32_formal_tip_out_hybrid_renderer`, `tip_out_event_get_data_string_must_clone`, `tip_out_voxel_gpu_passes_update_all_must_not_borrow_passes`) match existing stale-gen rows. Do not regenerate product `gen/`.

These names printed FAILED before the kill and passed when re-run alone:

| Gate | Isolated re-run |
|------|-----------------|
| `app_test_http_method_public_port_must_pass_wj_test` | ✅ |
| `app_module_file_http_method_public_port_must_pass_wj_test` | ✅ |
| `plain_struct_auto_derives_serialize` | ✅ |
| `copy_local_into_owned_f32_formal_must_not_borrow` | ✅ |
| `bool_builder_active_must_not_to_string_with_string_overload` | ✅ |

**Gates:** `cargo test --release --test all -- app_test_http_method_public_port_must_pass_wj_test app_module_file_http_method_public_port_must_pass_wj_test plain_struct_auto_derives_serialize copy_local_into_owned_f32_formal_must_not_borrow bool_builder_active_must_not_to_string_with_string_overload` — 7 passed (includes two extra name matches).


## P3.757 (2026-10-09) — WDB-204 `u64` tuple field compared to `0` is already `u64` on tip

Same-file `let median = pair.0` from `(u64, bool)` then `median == 0` cargo-checks. The zero on that compare is not `0_usize`. `samples.len() == 0` may still use `usize`.

| Gate | Status |
|------|--------|
| `wdb204_tuple_u64_field_compare_zero_must_not_emit_usize` | ✅ GREEN |
| `wdb201_module_file_reused_key_into_owned_entries_push_must_clone` | ✅ GREEN (reconfirmed) |
| WDB-204 tip-out sysbench | ❌ stale `gen/` / `.agent-wip/rel_tip_out` (not regenerated) |

**Root cause layer:** tip-out lag. No compiler change.

**Gates:** `cargo test --release --test all -- wdb204_tuple_u64_field_compare_zero` — 1 passed. `cargo test --release --test all -- wdb201_module_file_reused_key_into_owned_entries_push` — 1 passed.


## P3.756 (2026-10-09) — WDB-107 owned string literal + WDB-127 `Vec<u64>` element

| Gate | Status |
|------|--------|
| `wdb107_same_file_empty_literal_into_demoted_str_formals_cargo_checks` | ✅ GREEN — `"".to_string()` into preregistered `String` formals |
| `wdb127_module_file_demoted_vec_formal_must_borrow_bare_local_call_sites` | ✅ GREEN — `vec![30, 40]` emits `30_u64` into `&Vec<u64>` |

**Root cause layer:** signature (WDB-107) and constraint (WDB-127).

- WDB-107: AsRef-runtime bodies preregister `path: String`, but a stale analyzer `Reference(str)` still skipped `.to_string()` on `""`.
- WDB-127: `let_binding_int_width_from_later_call_formals` stored the whole `Vec<u64>` as the integer suffix slot. `int_type_from_assignment_target` ignores `Vec`, so the literals stayed `_i64`. The element `u64` is the suffix peer; the binding stays `Vec`.

**What became unnecessary:** treating a Borrowed WJ `string` as `&str` when the preregistered formal is owned `String`; using a collection formal itself as an integer literal suffix.

**Gates:** `cargo test --release --test all -- wdb107_same_file_empty_literal wdb127_module_file_demoted_vec_formal_must_borrow_bare` — 2 passed. Peers `wdb110`, `wdb111`, `wdb124`, `wdb126`, `qs_get_literal_into_demoted_key` — 8 passed.


## P3.755 (2026-10-09) — untyped `0` plus `Vec::len()` must not emit `0_i64`

`shader_graph_executor.wj` writes `let mut total_async = 0` then
`total_async = total_async + g.len()`. Tip-out emits
`let mut total_async = 0_i64` and adds a `usize` length.

| Gate | Status |
|------|--------|
| P3.755 isolate | ✅ GREEN — `let mut total_async: usize = 0_usize` |
| P3.755 tip-out | ❌ stale `gen/rendering/shader_graph_executor.rs` still has `0_i64` (not regenerated) |

**Root cause layer:** constraint/solver. `total = total + g.len()` never entered `usize_variables`, so the void-function i32 default (older tip-out: i64) won over the `usize` length.

**What became unnecessary:** leaving an untyped length accumulator on the void-function integer default.

**Gates:** `cargo test --release --test all -- untyped_zero_plus_vec_len_must_not_emit_i64 len_minus_literal_infers_usize len_plus_literal test_len_arithmetic_in_assignment` — 4 passed. Peers `i32_binding_compare_zero_must_not_emit_i64`, `usize_loop_counter_init_zero_must_not_be_i32`, `int_loop_assign_end_bound_must_unify`, `vec_len_eq_zero_must_not_emit_i64_literal`, `i32_return_while_len_counter_must_not_emit_usize` GREEN. Tip-out P3.750 scanner still RED on stale gen.


## P3.754 (2026-10-09) — TDD WDB-473 (DB agent; no compiler src)

A payload enum constructor used once must not `.clone()`.

Product `visual_scripting/runtime.rs` `test_graph_runner`:
```wj
runner.set_variable("score", Value::Int(0))
```
Tip MultiFile emits `take(Value::Int(0))` with no clone (isolate GREEN). Tip-out still has `Value::Int(0).clone()`.

| Gate | Status |
|------|--------|
| WDB-473 MultiFile | ✅ isolate GREEN — bare `Value::Int(0)` |
| WDB-473 method, non-Copy `String` payload | ✅ isolate GREEN — `runner.set_variable("score", Value::Int(0))` |
| WDB-473 tip-out | ❌ tip RED — `Value::Int(0).clone()` in `rel_tip_out/visual_scripting/runtime.rs` and `windjammer-game-core/gen/visual_scripting/runtime.rs` |

**Root cause layer:** tip-out lag — a fresh payload constructor is cloned into an owned formal.

**Why this is a new class:**
- WDB-367 is the unit constructor `Value::None`.
- WDB-416 is `a.clone().as_float()` on a reused formal.
- This is `Value::Int(0)` built at the call and cloned once.

**Do not steal:** WDB-406/408/411/457–473, P3.508–P3.754, compiler `src/`.

**Gates:** `cargo test --release --test all --features integration_tests,codegen_tests -- wdb473_` — isolate GREEN / tip RED (2026-10-09).

## P3.753 (2026-10-08) — `u32` product cast to `u32` must not insert `as f32`

`vgs_rasterization.wj` writes
`create_empty_storage_buffer((self.width * self.height * 16) as u32)` with
`width` and `height` as `u32`. Tip-out emits
`(((self.width * self.height) as f32 * 16_u32) as f32) as u32` (`f32 * u32`).

| Gate | Status |
|------|--------|
| `line_col_fallback_does_not_take_another_files_f32` | ✅ lookup GREEN — another file's `f32` at the same line/col is not used |
| `u32_product_cast_must_not_insert_f32` | ✅ isolate GREEN — same-file `u32` product cast emits no `as f32` |
| P3.753 tip-out | ❌ tip-out RED — stale `gen/rendering/vgs_rasterization.rs` still has `as f32` (not regenerated) |

**Root cause layer:** constraint/solver. `get_float_type` priority 3 matched line and column across files, so a `u32` field could be classified `f32`. An integer formal (`size: u32`) now clears a leaked float assignment slot before the argument is emitted.

**What became unnecessary:** treating another file's `f32` as this file's float type; keeping that slot on a `u32` call argument so `(width * height * 16) as u32` was wrapped `as f32`.

**Do not steal:** do not regen product `gen/` from this session.

**Gates:** `cargo test --release --lib line_col_fallback` and `cargo test --release --test all -- u32_product_cast_must_not_insert_f32`.


## P3.752 (2026-10-08) — TDD WDB-472 (no compiler src)

Copy `i32` for-range bindings must not `.clone()` at a by-value call.

Product `scene/station_geometry.rs` `carve_room` calls `set_if(grid, x, y, z, 0)` inside `for x` / `for z` / `for y`. Tip-out emits `x.clone()` / `y.clone()` / `z.clone()`. Tip MultiFile emits the bindings bare (isolate GREEN).

| Gate | Status |
|------|--------|
| WDB-472 MultiFile | ✅ isolate GREEN |
| WDB-472 tip-out | ❌ tip RED — range `i32` clones in `rel_tip_out/scene/station_geometry.rs` and `windjammer-game-core/gen/scene/station_geometry.rs` |

**Root cause layer:** tip-out lag. Distinct from WDB-438 (Copy i32 inside a tuple literal) and WDB-456 (Copy i32 locals into `let`).

**Do not steal:** compiler `src/`.

**Gates:** `cargo test --test all --features integration_tests -- wdb472_` — isolate GREEN / tip RED (2026-10-08).


## P3.750 (2026-10-08) — annotated `i32` compared to `0` must not emit `0_i64`

`shader_graph_compiler.wj` writes `let mut existing_group: i32 = -1` then
`if existing_group >= 0`. Tip-out emits `existing_group >= 0_i64` while the
binding is `i32` (expected `i32`, found `i64`).

| Gate | Status |
|------|--------|
| `i32_binding_compare_zero_must_not_emit_i64` | ✅ isolate GREEN — comparison is not `0_i64` |
| `i32_binding_compare_zero_tip_out_shader_graph_compiler` | ❌ tip-out RED — stale `gen/rendering/shader_graph_compiler.rs` still has `>= 0_i64` (not regenerated) |

**Root cause layer:** signature of the annotated binding. `let mut existing_group: i32 = -1` stored `Int32`, then `reconcile_ambiguous_int_local_after_let` treated the negative init as a WJ `int` sentinel and rewrote the binding to `i64`. The `>= 0` literal followed that. The zero-sentinel peer also inferred unary `-1` as default `int` before the `Let` annotation.

**What became unnecessary:** demoting an emitted `-1_i32` binding to WJ `int`; using the unary init's inferred `i64` as the zero-sentinel peer when the let is annotated `i32`.

**Do not steal:** P3.727 (`int == 1` beside Vec::push) stays i64. Do not regen product `gen/` from this session.

**Gates:** `cargo test --release --test all -- i32_binding_compare_zero_must_not_emit_i64 int_eq_one_with_vec_push usize_loop_counter_init_zero_must_not_be_i32 module_file_usize_index_eq_zero_must_not_emit_i32 wdb395_module_file_i32_i64_compare_zero migrate_string_scan_index_eq_zero migrate_prev_sentinel int_loop_assign_end_bound_must_unify` — isolate and those peers GREEN (2026-10-09). Tip-out scanner still RED on stale gen.

## P3.751 (2026-10-08) — If-condition reborrow of an owned string formal

`query_wants_base64(query)` emits `query: String` (the formal moves into `own`).
When that call is the condition of `if`, a later rewrite still turned the
argument into `&query`.

| Gate | Status |
|------|--------|
| `owned_string_formal_must_not_receive_mut_query` | ✅ tip GREEN |
| `json_tostring_note_must_not_mut_borrow_query` | ✅ tip GREEN — `note: &Note` |
| WDB-218 isolate both formals `&Plan` | ✅ `execute(plan)` accepted |

**Root cause layer:** signature. The if-condition borrow rewrite ignored an
emitted owned formal. Preregistered `query: String` also blocks the reuse path
from re-applying a shared borrow.

**What became unnecessary:** `&query` into `query: String` on the if-condition
path.

**Gates:**
```bash
export CARGO_TARGET_DIR="$HOME/Library/Caches/windjammer/cargo-target/shared"
cargo test --release --test all -- owned_string_formal_must_not_receive_mut_query \
  wdb218_codegen notes_api_json notes_api_interp notes_api_qs_get
```
→ 6 passed; `json_tostring_note_must_not_mut_borrow_query` still failed (`&mut Note`).

**Follow-up (same row):** `json.to_string(note)` was resolved as the bare
`to_string` homonym (`param_ownership: [MutBorrowed]`), so the caller formal
became `&mut Note` while `dispatch` stayed `&Note`.

**Root cause layer:** signature. `resolve_free_call_signature` now prefers the
stdlib runtime free-fn contract (`json::to_string(value: T)` owned, no `self`)
when the looked-up sig is a MutBorrowed / `self` homonym.

**What became unnecessary:** treating that homonym as a reason to emit
`&mut Note`. The analyzer already inferred `Borrowed`.

**Gates:**
```bash
cargo test --release --test all -- json_tostring_note_must_not_mut_borrow_query \
  owned_string_formal_must_not_receive_mut_query
```
→ 2 passed.

## P3.749 (2026-10-08) — REGRESSION: engine library transpile SIGKILL (exit 137)

P3.291 recorded full `windjammer-game-core` `--library` transpile EXIT 0 in ~200s
with `RAYON_NUM_THREADS=1` (2026-09-15). Tonight tip `wj` 0.50.0
(`windjammer/target/release/wj`, 2026-10-08 19:44) dies **exit 137** during Step 4B
analyze. Progress logging only prints the first 20 files, so the log always ends at
`squad_tactics.wj`. That file alone transpiles in ~2s. Peak RSS sampled ~3.8 GB
before the kill. `RAYON_NUM_THREADS=1` still exits 137 (~66s).

| Gate | Status |
|------|--------|
| `squad_tactics.wj` alone `--library --no-cargo` | ✅ ~2s EXIT 0 |
| full `src/mod.wj` `--library --no-cargo` | ❌ exit 137 (three runs, 2026-10-08) |
| same with `RAYON_NUM_THREADS=1` | ❌ exit 137 |

**Root cause layer:** Step 4B per-file analyze with the converged global registry
on ~678 files. Not a panic message (stderr stops at the progress cap). Host
volume later hit **20 MiB free** (2026-10-08 22:50), which also jetsams large
`wj` processes; the earlier kills were at ~3.8 GB RSS with more free RAM.

**Do not steal:** compiler `src/`. No host OOM test in the default suite.

**Evidence:** `/tmp/wj_engine_r21_err.log`, `/tmp/wj_engine_r22_err.log`.

## P3.748 (2026-10-08) — TDD WDB-471 (DB agent; no compiler src)

A reused non-Copy `Vec` passed into an owned formal must not be `.clone().clone()`.

Product `animation/blend_tree.rs` `evaluate_node`:
```wj
let pose_a = self.evaluate_node(node_a, clips, time, bone_count)
let pose_b = self.evaluate_node(node_b, clips, time, bone_count)
```
One `.clone()` copies the `Vec` for the second use. Tip-out emits `clips.clone().clone()`.

| Gate | Status |
|------|--------|
| WDB-471 MultiFile | ✅ isolate GREEN — no `.clone().clone()` |
| WDB-471 tip-out | ❌ tip RED — `clips.clone().clone()` in `rel_tip_out/animation/blend_tree.rs` and `windjammer-game-core/gen/animation/blend_tree.rs` |

**Root cause layer:** tip-out lag — a second `.clone()` is applied to a value that is already owned.

**Why this is a new class:**
- WDB-374 is a Copy enum `.state.clone().clone()` and is tip GREEN.
- WDB-470 is a single `.clone()` on an owned function return.
- This is a non-Copy `Vec` argument cloned twice at one call.

**Do not steal:** WDB-406/408/411/457–471, P3.508–P3.748, compiler `src/`.

**Gates:** `cargo test --release --test all --features integration_tests,codegen_tests -- wdb471_` — isolate GREEN / tip RED (2026-10-08).

## P3.747 (2026-10-08) — Associated readonly Custom formal stayed owned

`FpsCamera::collides_aabb(grid: VoxelGrid)` only reads `grid.cells.len()`, but
codegen kept the formal owned because associated `pub fn` Custom params were
treated as public owned API (WDB-398). Both call sites then emitted
`grid.clone()`.

| Gate | Status |
|------|--------|
| `test_static_readonly_voxelgrid_param_no_clone_*` | ✅ tip GREEN — `grid: &VoxelGrid`, no `grid.clone()` |
| WDB-398 `new(palette.copy())` | ✅ still owned — direct `palette.copy()` is not a field scan |

**Root cause layer:** signature. `is_public_owned_non_copy_formal_api` returned
true for every associated Custom formal, including `&self` scans (`grid.get`)
and unused bindings. Those now fall through to `&T`. Owned-self methods and
stores (`palette.copy()`, field init) stay owned (WDB-398).

A caller whose every call site already expects that shared ref
(`update(grid)` → `collides(grid: &VoxelGrid)`) keeps `grid: &VoxelGrid`.
Stale “owned sibling” / cross-module keep-owned restores no longer flip that
contract back to owned. Impl preregister runs a second pass so a caller listed
before its callee still sees the emitted `&T` formal.

**What became unnecessary:** `grid.clone()` into readonly associated helpers,
and a second `&` at the caller once the caller formal itself is `&VoxelGrid`.
No new call-site peel.

`test_static_helper_with_instance_method_on_param` emits
`update(&mut self, dt: f32, grid: &VoxelGrid)`. Copy field reads
(`self.pos_y: f32` into `Vec3::new`) are no longer partial moves, so the
receiver stays `&mut self`. Non-Copy ctor moves (WDB-414 `self.scene`) still
take owned `mut self`. Unknown field types stay moves.

**Gates:**
```bash
export CARGO_TARGET_DIR="$HOME/Library/Caches/windjammer/cargo-target/shared"
cargo test --release --test all -- test_static_helper_with_instance_method_on_param \
  test_static_method_call_not_treated_as_enum_variant \
  test_multiple_static_calls_with_same_param_still_borrowed \
  test_static_readonly_voxelgrid wdb398_module_file_associated_new wdb414 update_death
```
→ 11 passed.

## P3.746 (2026-10-08) — Logger owned string literal re-borrowed after homonym refresh

`logger.info("a")` / `logger.warn("b")` emit `message: String`, then the call site
was `&String::from("a")` (E0308). IR saw the codegen-refreshed owned contract
(`emitted=[false, false]`, ownership Owned). `prefer_shared_ref_signature` then
let an analysis-only `&mut self` stub (`[MutBorrowed, Borrowed]`, no emit flags)
replace that contract, and a later pass prefixed `&`.

| Gate | Status |
|------|--------|
| `regression_logger_owned_passthrough` | ✅ tip GREEN |
| string-literal / passthrough peers | ✅ 19 GREEN (filter below) |

**Root cause layer:** signature. `mut_borrow_emission_beats` must not replace a
codegen-confirmed owned `String` slot with an analysis `&mut self` stub.
Collision strip no longer peels `.to_string()` when `emitted_owned_arg_contract`
is true.

**What remains temporary:** method-arg emit still strips a leading `&` from
`String::from` / `.to_string()` when the method-registry or contract signature
already expects an owned string. Delete that peel once the homonym refresh
stops emitting the borrow.

**Gates:**
```bash
export CARGO_TARGET_DIR="$HOME/Library/Caches/windjammer/cargo-target/shared-p3728"
cargo test --release --test all -- regression_logger_owned_passthrough \
  codegen_string_param owned_string_formal_literal passthrough_to_owned \
  string_literal_to_string bug_is_stored_enum_option
```
→ 19 passed.

## P3.745 (2026-10-08) — TDD WDB-470 (DB agent; no compiler src)

An owned `Vec` return passed straight into an owned formal must not `.clone()`.

Product `editor/csg.rs` `csg_union` / `csg_subtract` / `csg_intersect`:
```wj
let mut ta = build_bsp_from_polygons(clone_polygons(a))
```
`clone_polygons` returns `Vec<CsgPolygon>`. Tip MultiFile emits the call without cloning the return (isolate GREEN). Tip-out still has `build_bsp_from_polygons(clone_polygons(&a).clone())`.

| Gate | Status |
|------|--------|
| WDB-470 MultiFile | ✅ isolate GREEN — no `.clone()` on the `clone_polys(...)` call |
| WDB-470 tip-out | ❌ tip RED — `clone_polygons(&a).clone()` in `rel_tip_out/editor/csg.rs` and `windjammer-game-core/gen/editor/csg.rs` |

**Root cause layer:** tip-out lag — the owned return is cloned again at the call.

**Why this is a new class:**
- WDB-464 through WDB-469 clone Copy elements (bytes, tuples, i64).
- This clones a non-Copy `Vec` that the callee already returned by value.

**Do not steal:** WDB-406/408/411/457–470, P3.508–P3.745, compiler `src/`.

**Gates:** `cargo test --release --test all --features integration_tests,codegen_tests -- wdb470_` — isolate GREEN / tip RED (2026-10-08).

## P3.743 (2026-10-08) — Copy struct formal must not be re-borrowed at the call site

`find_index(self, id: NodeId)` emits `id: NodeId`, then `has_item` / `remove_item` called `self.find_index(&id)`.

| Gate | Status |
|------|--------|
| `copy_struct_arg_not_borrowed_at_call_site` | ✅ tip GREEN — `find_index(id)` |

**Root cause layer:** signature. Method-signature refresh treated a stale `emitted_rust_ref_formals` membership as a shared formal even when the emitted string was `id: NodeId`. `param_is_stale_engine_owned_stub` then treated that codegen-confirmed owned `Custom` slot as a stale engine stub, so `emitted_owned_arg_contract` was false and `runtime_std_param_needs_auto_borrow_resolved` prefixed `&`.

**What became unnecessary:** using the stale shared-ref name set to override an owned emitted formal. Forwarding-borrow no longer borrows a codegen-confirmed owned `Custom` formal. Auto-borrow returns before a stdlib homonym baseline when the user formal is codegen-confirmed owned and not a WJ std stub.

**Gates:** `cargo test --release --test all -- copy_struct_arg_not_borrowed quest_id` — 7 passed (copy struct + quest id regressions).

## P3.744 (2026-10-08) — `vec![1, 2, 3]` and `push(4)` disagree on int width

Conformance `test_conformance_vec_push_len` emitted `vec![1_i32, 2_i32, 3_i32]` then `v.push(4_i64)` (E0308). Untyped integer literals in one `Vec` and a later generic element argument must share a width.

| Gate | Status |
|------|--------|
| `test_conformance_vec_push_len` | ✅ tip GREEN |

**Root cause layer:** coercion/encoding. Void functions paint unsuffixed locals as i32 coordinates. That paint covered `vec![1, 2, 3]` while `Vec::push`'s generic `T` slot stayed Windjammer `int` (i64).

**What became unnecessary:** the i32 coordinate default on an unsuffixed int collection that is later stored through a generic element slot (`T`). `-> Vec<i32>` still keeps the narrow paint. Int inference also MustMatch's that generic slot to the initializer literals.

**Gates:** `cargo test --release --test all -- test_conformance_vec_push_len` — passed. Related filter `vec_push vec_macro test_vec_push_i64 int_inference_generic while_lit_bound copy_struct_arg_not_borrowed` — 68 passed; 3 failures are stale tip-out scanners (WDB-464/465/466), and their in-process module-file gates passed.

## P3.742 (2026-10-08) — function-local `use std::strings`

`wj test` emits a function-body `use std::strings` as Rust `use std::strings` (E0432: no `strings` in the root; rustc suggests `std::string`). The same import at file scope binds the Windjammer strings module.

Product: `apps/wj-proxy/tests/config_test.wj` inner import while adding `require_upstream`. File-scope import is the existing test style, so the app stays on that import. The function-local form is still the bug.

| Gate | Status |
|------|--------|
| `fn_local_std_strings_use_must_cargo_check` | ✅ tip GREEN (2026-10-08) |

**Root cause layer:** codegen import path. Function-body `Statement::Use` joined the path raw (`use std::strings`) and never recorded the module, so `strings.len` stayed a method call. It now uses the same `generate_use` rewrite as file scope, and the function body temporarily registers that std module so the call emits `strings::len`.

**What became unnecessary:** a second, unscoped import emitter for statement-level `use`.

**Gates:** `cargo test --release --test all -- fn_local_std_strings_use_must_cargo_check user_join_name_clash_strings std_strings_contains package_flat_lib_wj_test_exports` — 4 passed.

## P3.741 (2026-10-08) — TDD WDB-469 (no compiler src)

Indexed Copy tuple **swap** must not `.clone()`.

Product `scene_graph/scene_graph_state.rs` `sort_by_distance`:
```wj
let tmp = sorted[i]
sorted[i] = sorted[j]
sorted[j] = tmp
```
Tip MultiFile emits bare `sorted[i as usize]` (isolate GREEN). Tip-out still has
`sorted[i].clone()` / `sorted[j].clone()` for Copy `(u64, f32)`.

| Gate | Status |
|------|--------|
| WDB-469 MultiFile | ✅ isolate GREEN — bare index assignment |
| WDB-469 tip-out | ❌ tip RED — clones in `rel_tip_out/scene_graph/scene_graph_state.rs` and `windjammer-game-core/gen/scene_graph/scene_graph_state.rs` |

**Root cause layer:** tip-out lag. Distinct from WDB-465 (whole tuple into `Vec::push`) and WDB-466 (tuple field `.0` / `.1`).

**Do not steal:** compiler `src/` (other agent).

**Gates:** `cargo test --test all --features integration_tests -- wdb469_` — isolate GREEN / tip RED (2026-10-08).


## P3.740 (2026-10-07) — `wj test` const string into owned formal

Library `pub const string` passed from `tests/*_test.wj` into an owned `string` formal (body uses `strings.split`) emits bare `&str` in the test crate (E0308). Same-module `take_owned(CONST)` is a different gate.

Product: `wj-uuid` `hyphen_count(NIL)` from `tests/uuid_test.wj` on tip `wj` 0.50.0.

| Gate | Status |
|------|--------|
| `cross_module_string_const_into_owned_formal_must_own` | ✅ isolate GREEN — `NIL.to_string()` |
| `external_metadata_string_const_into_owned_formal_must_own` | ✅ isolate GREEN — metadata `string_consts` → `NIL.to_string()` |

**Root cause layer:** signature — library `const string` is `&'static str`, but only the defining file was in `module_string_consts`. Imported consts now come from shared numeric-inference const types and `metadata.json` `string_consts`.

**Do not steal:** compiler `src/`.

## P3.739 (2026-10-07) — TDD WDB-468 (DB agent; no compiler src)

Indexed Copy `i64` into a local `let` must not `.clone()`.

Product `editor/csg.rs` `split_polygon_by_plane`:
```wj
let ti = types[i]
let tj = types[j]
```
Tip MultiFile emits bare `types[i]` (isolate GREEN). Tip-out still has
`types[i].clone()` / `types[j].clone()` (untyped classify codes inferred `i64`).

| Gate | Status |
|------|--------|
| WDB-468 MultiFile | ✅ isolate GREEN — bare `types[i]` |
| WDB-468 tip-out | ❌ tip RED — clones in `rel_tip_out/editor/csg.rs` |

**Root cause layer:** tip-out lag — isolate tip already correct; indexed Copy integers still `.clone()` into locals.

**Why this is a new class:**
- WDB-464 is indexed Copy `u8` into `Vec::push`.
- WDB-466 is indexed Copy tuple **field** into push.
- WDB-456 is a plain i32 local reassign, not an index.
- This is `Vec<i64>` index into `let ti` / `let tj`.

**Gates:** `cargo test --release --test all --features integration_tests,codegen_tests -- wdb468_` — isolate GREEN / tip RED (2026-10-07).

**Do not steal:** WDB-406/408/411/457–468, P3.508–P3.739, WDB-412–468 (filed).

## P3.739 (2026-10-07) — demoted `&Vec` formals must borrow every call arg

`half_edge.wj` `from_triangle_mesh(positions: Vec<Vec3>, indices: Vec<u32>)`.
Tip-out signature is `&Vec<Vec3>, &Vec<u32>` but `mesh_ops.rs` calls
`from_triangle_mesh(positions, &indices)` (expected `&Vec<Vec3>`, found `Vec<Vec3>`).

| Gate | Status |
|------|--------|
| `owned_vec_formals_must_borrow_both_call_args` | ✅ isolate GREEN |
| `owned_vec_formals_tip_out_mesh_ops` | ❌ tip-out RED — `from_triangle_mesh(positions, &indices)` |

**Root cause layer:** call-arg ownership — when both owned `Vec` formals are demoted
to shared refs, every argument is borrowed, not only the later ones.

**Do not steal:** P3.732 (Copy `Vec3` local), P3.731–P3.738, other-agent WJ `src/`.

**Gates:** `cargo test --release --test all --features integration_tests -- owned_vec_formals`.


## P3.736 (2026-10-07) — TDD WDB-467 (DB agent; no compiler src)

Copy `[u8; 4]` from `to_le_bytes` reused in `from_le_bytes` must not `.clone()`.

Product `rendering/hybrid_renderer.rs`:
```wj
let zero_bits = 0u32.to_le_bytes()
data.push(f32::from_le_bytes(zero_bits))
data.push(f32::from_le_bytes(zero_bits))
```
`u32::to_le_bytes` / `f32::from_le_bytes` are registry signatures returning `[u8; 4]`.
`[T; N]` is Copy when `T` is, so the first reuse no longer emits `.clone()`.
Tip-out still has the old clone until product gen is regenerated.

| Gate | Status |
|------|--------|
| WDB-467 MultiFile | ✅ isolate GREEN — bare `zero_bits` (2026-10-07) |
| WDB-467 tip-out | ❌ tip-out lag — `from_le_bytes(zero_bits.clone())` in `rel_tip_out/rendering/hybrid_renderer.rs` |

**Root cause layer:** signature + Copy classification — endian methods return `[u8; N]`; arrays of Copy elements are Copy.

**Why this is a new class:**
- WDB-464 indexes individual Copy `u8` elements into `Vec::push`.
- This passes the whole Copy byte array into `from_le_bytes` twice.

**Gates:** `CARGO_TARGET_DIR=…/shared-p3728 cargo test --release --test all -- wdb467_module_file_copy_byte_array_reuse_must_not_clone` → **ok** (2026-10-07). Tip-out scanner still RED on stale gen.

**Do not steal:** WDB-406/408/411/457–467, P3.508–P3.736, WDB-412–467 (filed).

## P3.738 (2026-10-07) — `Option<string>` field match must clone into owned Option

`animation/controller.wj` `current_animation` matches `self.current_animation` and
returns `Some(s)`. Tip-out emits `match &self.current_animation { Some(s) => Some(s) }`,
so the payload is `&String` (expected `String`, found `&String`).

| Gate | Status |
|------|--------|
| `option_string_field_match_must_clone_into_owned` | ✅ isolate GREEN |
| `option_string_field_match_tip_out_animation_controller` | ❌ tip-out RED — `match &self.current_animation { Some(s) => Some(s) }` |

**Root cause layer:** match encoding — a borrowed `Option<String>` arm returned as
owned `Option<String>` must `clone` the payload.

**Do not steal:** P3.724 (Map::get payloads), P3.730 (identity interp), P3.731–P3.735.

**Gates:** `cargo test --release --test all --features integration_tests -- option_string_field_match`.

## P3.735 (2026-10-07) — `i32 * 48` into `u32` must cast the product

`hybrid_renderer.wj` calls `create_empty_storage_buffer(max_triangles * 48)`
(`size: u32`, `max_triangles` an `i32` local). Tip-out emits
`(max_triangles * 48_i32 as u32)`, so `as` binds only to `48` and the multiply
is `i32 * u32`. An untyped `let max_triangles = 100000` widens to `u32` and
hides the bug; the repro pins `max_triangles: i32`.

| Gate | Status |
|------|--------|
| `int_mul_into_u32_formal_must_cast_product` | ✅ isolate GREEN — `(max_triangles * 48_i32) as u32` |
| `int_mul_into_u32_formal_tip_out_hybrid_renderer` | ❌ tip-out lag — stale `gen/` still has `48_i32 as u32` until regen |

**Root cause layer:** coercion/encoding — `as` binds tighter than `*`.
`rust_numeric_cast` parenthesizes the product (`(max_triangles * 48_i32) as u32`).
What became unnecessary: a bare `(expr as u32)` wrap that left `as` on the literal.

**Do not steal:** P3.731/P3.732, other-agent WJ `src/`.

**Gates:** `cargo test --release --test all --features integration_tests -- int_mul_into_u32_formal`.



## P3.735 (2026-10-07) — TDD WDB-466 (DB agent; no compiler src)

Indexed Copy tuple **field** into `Vec::push` / compare must not `.clone()`.

Product `scene_graph/scene_graph_state.rs` `sort_by_distance`:
```wj
let a_dist = sorted[i].1
self.sorted_nodes.push(sorted[k].0)
```
Tip MultiFile emits bare `.0` / `.1` (isolate GREEN). Tip-out still has
`sorted[k].0.clone()` and `sorted[i].1.clone()` (`u64` / `f32`).

| Gate | Status |
|------|--------|
| WDB-466 MultiFile | ✅ isolate GREEN — bare `.0` / `.1` |
| WDB-466 tip-out | ❌ tip RED — field clones in `rel_tip_out/scene_graph/scene_graph_state.rs` |

**Root cause layer:** tip-out / product multipass lag — isolate tip already correct;
indexed Copy tuple fields still `.clone()` into push and locals.

**Why this is a new class:**
- WDB-439 is indexed tuple field on **return** (`nodes[i].0.clone()`).
- WDB-465 clones the **whole** indexed Copy tuple into push.
- WDB-363 clones the **element** then reads a field.
- This is `.0` / `.1` of an indexed Copy tuple into `Vec::push` and compare.

**Gates:** `cargo test --release --test all --features integration_tests,codegen_tests -- wdb466_` — isolate GREEN / tip RED (2026-10-07).

**Do not steal:** WDB-406/408/411/457–466, P3.508–P3.735, WDB-412–466 (filed).

## P3.734 (2026-10-07) — multi-arm `json.clone()` into demoted `&str` (no compiler src)

LedgerKit `render_read_model` keeps `json: String` (most arms pass it to owned parsers).
`parse_balance_sheet_fields` / `parse_income_statement_fields` /
`parse_general_ledger_fields` / `parse_close_checklist_days_to_close` demote to `&str`,
but the match still emits `json.clone()` (E0308). A two-arm isolate borrows (`&json`).
`own_string` on the callee keeps the formal `String` and then `escape_html(&str)` rejects
the owned local — not a product fix.

| Gate | Status |
|------|--------|
| `multiarm_owned_json_clone_must_not_feed_demoted_str_parser` | ❌ tip RED — product `build/read_models.rs` |

**Do not steal:** compiler `src/` (other agent).

**Gates:** `cargo test --test all --features integration_tests -- multiarm_owned_json_clone_must_not_feed_demoted_str_parser`.


## P3.734 (2026-10-07) — TDD WDB-465 (DB agent; no compiler src)

Indexed Copy **tuple** into `Vec::push` must not `.clone()`.

Product `ai/astar_grid.rs`:
```wj
rev.push(path[k])
```
Tip MultiFile emits bare `push(path[k])` (isolate GREEN). Tip-out still has
`rev.push(path[k as usize].clone())` for Copy `(i32, i32)`.

| Gate | Status |
|------|--------|
| WDB-465 MultiFile | ✅ isolate GREEN — bare `push(path[k])` |
| WDB-465 tip-out | ❌ tip RED — `path[…].clone()` in `rel_tip_out/ai/astar_grid.rs` |

**Root cause layer:** tip-out / product multipass lag — isolate tip already correct;
indexed Copy tuples still `.clone()` into owned `Vec::push`.

**Why this is a new class:**
- WDB-423 is indexed tuple **destructure** (`let (ox, oz) = offsets[i]`).
- WDB-438 is Copy i32 into **tuple lit** for push.
- WDB-464 is indexed Copy **u8** into push (`x_bits[0]`).
- This is indexed Copy **tuple** into `Vec::push`.

**Gates:** `cargo test --release --test all --features integration_tests,codegen_tests -- wdb465_` — isolate GREEN / tip RED (2026-10-07).

**Do not steal:** WDB-406/408/411/457–465, P3.508–P3.734, WDB-412–465 (filed).

## P3.734 (2026-10-07) — `n * 48` into `u32` must cast the product

`hybrid_renderer.wj` calls `create_empty_storage_buffer(max_triangles * 48)`
(`size: u32`). Tip-out emits `(max_triangles * 48_i32 as u32)`, so `as` binds
only to `48` and the multiply is `i32 * u32`.

| Gate | Status |
|------|--------|
| `int_mul_into_u32_formal_must_cast_product` | ✅ isolate GREEN — annotated `i32` product is cast `as u32` |
| `untyped_mul_into_u32_formal_uses_u32_literal_width` | ✅ isolate GREEN — untyped `let max_triangles = 100000` emits `100000_u32` and `48_u32` |
| `int_mul_into_u32_formal_tip_out_hybrid_renderer` | ❌ tip-out RED — stale `gen/rendering/hybrid_renderer.rs` still has `48_i32 as u32` |

**Root cause layer:** signature. The `u32` formal sets the literal width. An annotated `i32` binding still casts the product. Untyped `100000` follows the formal as `u32`, so `48 as u32` on only the literal is unnecessary.

**Do not steal:** do not regen product `gen/` from this session.

**Gates:** `cargo test --release --test all -- int_mul_into_u32_formal_must_cast_product untyped_mul_into_u32_formal_uses_u32_literal_width`.

## P3.705 (2026-10-07) — substring `i + 1` stays `_usize` (re-verify)

`section.substring(i, i + 1)` with `i: usize` emitted `i + 1_i32` (E0277).
Slice/range bounds now pin an ambient usize slot before emitting the end literal.

| Gate | Status |
|------|--------|
| `substring_usize_plus_one_must_not_emit_i32` | ✅ tip GREEN — `i + 1_usize` (2026-10-07) |
| `int_eq_one_with_vec_push_must_not_emit_i32` | ✅ tip GREEN (P3.727) |

**Gates:** `CARGO_TARGET_DIR=target/agent-tdd-slice` →
`cargo test --test all --features integration_tests,codegen_tests -- substring_usize_plus_one_must_not_emit_i32` → **1 passed**.

**Do not steal:** remaining tip-true REDs.

## P3.733 (2026-10-07) — TDD WDB-464 (no compiler src)

Indexed **Copy `u8`** into `Vec::push` must not `.clone()`.

```wj
let x_bits = x.to_le_bytes()
bytes.push(x_bits[0])
```
`f32::to_le_bytes` returns `[u8; 4]`, so `x_bits[i]` is Copy `u8` and `Vec::push` does not clone.
Tip-out still has the old clone until product gen is regenerated.

| Gate | Status |
|------|--------|
| WDB-464 MultiFile | ✅ isolate GREEN — bare `x_bits[i]` (2026-10-07) |
| WDB-464 tip-out | ❌ tip-out lag — `x_bits[i].clone()` in `rel_tip_out/ecs/world.rs` |

**Gates:** `CARGO_TARGET_DIR=…/shared-p3728 cargo test --release --test all -- wdb464_module_file_indexed_copy_u8_vec_push_must_not_clone` → **ok** (2026-10-07). Tip-out scanner still RED on stale gen.

**Do not steal:** compiler `src/` (other agent).

## P3.732 (2026-10-07) — owned `Vec3` formal must not borrow a local

Engine tip-out `gen/camera/fps_camera.rs` `collides_aabb(grid.clone(), &test_x, scale)`
while `collides_aabb(..., pos: Vec3, ...)`. Source passes `test_x` by value
(`fps_camera.wj`). Cargo: expected `Vec3`, found `&Vec3` (13 of 272 E0308).

| Gate | Status |
|------|--------|
| `owned_vec3_formal_must_not_borrow_local` | ✅ isolate GREEN — no `&test_x` |
| `owned_vec3_formal_tip_out_fps_camera_must_not_borrow_local` | ❌ tip-out RED — `&test_x` in `gen/camera/fps_camera.rs` |

**Root cause layer:** call-arg ownership — Copy struct local into an owned formal
must move or copy, not emit `&local`.

**Do not steal:** P3.731 (`String`/`&str`), other-agent WJ `src/`.

**Gates:** `cargo test --release --test all --features integration_tests -- owned_vec3_formal_must_not_borrow_local`.

## P3.731 (2026-10-07) — owned `String` formal must `.to_string()` a string literal

Engine tip-out cargo (`windjammer_game_core`, skip-transpile, 2026-10-07): **381**
errors, **272** E0308. Dominant class is **expected `String`, found `&str` (46)** —
`record_resource("audio_initialized")` / `add_dependency("math.wj")` where the
formal is owned `String`.

| Gate | Status |
|------|--------|
| `owned_string_formal_literal_must_to_string` | ✅ isolate GREEN — literal emits `.to_string()` (2026-10-09) |

**Root cause layer:** call-arg coercion — string literal into owned `string` formal
must emit `.to_string()` (signature-driven, not a hardcoded method list).

**Do not steal:** other-agent WJ `src/` work. No game-source workaround.

**Gates:** `cargo test --release --test all --features integration_tests -- owned_string_formal_literal_must_to_string`.

## P3.730 (2026-10-07) — REGRESSION: notes-api + auth-api HashMap get identity `"${v}"`

**Product (tip `wj` 0.50.0, `windjammer-game/.cargo-target-wj`):**
`apps/wj-notes-api` `notes_config_from_map` and `apps/wj-auth-api` `auth_config_from_map`
`match map.get(...) { Some(v) => "${v}", … }` emit bare `Some(v) => v`
(`expected String, found &String`). Isolate
`hashmap_get_identity_interp_into_owned_string_must_clone` is GREEN (`v.clone()`);
product `$WJ test` is RED (notes E0308 ×8, auth E0308 ×13). Apps left idiomatic.

**Expected:** owned string (`v.clone()`), as P3.678.

**Actual:** identity interp lowered to a move of `&String` on the product multipass path.

| Gate | Status |
|------|--------|
| `hashmap_get_identity_interp_into_owned_string_must_clone` | ✅ tip GREEN (2026-10-07) |
| product `wj-notes-api` / `wj-auth-api` `$WJ test` | ❌ tip RED (2026-10-07) |

**Do not steal:** other-agent WJ `src/`; P3.678/P3.725 product-gate claims.

## P3.727 (2026-10-07) — `int == 1` keeps i64 beside Vec::push

`tasks_from_kpis(unmatched_bank_lines: int)` emitted `== 1_i32` while `> 0`
stayed `0_i64`. An ambient i32 assign slot (coord-builder / Vec return) beat
the formal `int` peer on comparisons via `narrow_assign_int_slot_beats_weak_peer`.

| Gate | Status |
|------|--------|
| `int_eq_one_with_vec_push_must_not_emit_i32` | ✅ tip GREEN |

**Root cause layer:** signature — declared `int`/`i64` formals drive comparison
literal width. Comparisons follow that peer; annotated bitwise assigns still
keep a narrow slot (WDB-330).

**What became unnecessary:** letting a live i32 slot retarget `n == 1` when `n`
is a formal `int`.

**Gates:** `cargo test --release --test all --features integration_tests,codegen_tests -- int_eq_one_with_vec_push` → 1 passed (includes cargo-check).

## P3.726 (2026-10-07) — owned enum formals stay owned when the method consumes self

`evaluate(self, op, a: Val, b: Val) { a.as_float() }` emitted `a: &Val` and
`a.clone().as_float()`. `calls_consuming_method` looked up bare `as_float`, which
misses `Val::as_float` (and can collide with other types). Copy match payloads
(`i32`/`f32`/`bool`, including `Custom` spellings that are Copy) no longer clone
just because the type is `Custom`.

| Gate | Status |
|------|--------|
| `wdb416_module_file_owned_enum_must_not_clone_before_as_float` | ✅ tip GREEN |
| `wdb416_module_file_noncopy_enum_match_must_not_demote_before_as_float` | ✅ tip GREEN |
| `wdb416_tip_out_game_core_value_must_not_clone_before_as_float` | ❌ tip-out lag (regen) |
| `wdb429_module_file_copy_match_arm_payload_must_not_clone` | ✅ tip GREEN |
| `hashmap_get_noncopy_match_not_copied` | ✅ tip GREEN |
| `module_file_map_get_string_payload_must_clone` | ✅ tip GREEN |

**Root cause layer:** signature — qualified `Type::method` receiver ownership.
Copy vs clone for match payloads is type classification (Copy `Custom` is not a move).

**What became unnecessary:** cloning every `Custom` match payload / owned-context
`&Custom` pointee; treating a missing bare method key as “not consuming”.

**Gates:** `cargo test --release --lib -- wdb416_` → 2 passed.
`cargo test --release --test all --features integration_tests,codegen_tests -- wdb416_module_file wdb429_module_file hashmap_get_noncopy module_file_map_get_string hashmap_field_iter` → module-file GREEN; tip-out scanner still RED.

## P3.725 (2026-10-07) — borrowed map/vec loop values clone into owned push/insert

`for (id, note) in &self.notes { result.push(note) }` left `&Note` in `Vec::push`.
Solver `compute_coercion` already returned `Clone` (`Ref(Note)` → owned `T`).
`call_arg_is_copy_identity` then forced Identity because `&T` is Copy as a
reference, and because unsubstituted formal `T` can sit in the copy registry.

| Gate | Status |
|------|--------|
| `hashmap_field_iter_push_clones_noncopy_value` | ✅ tip GREEN |
| `nested_match_for_push_string_owns_or_clones` | ✅ tip GREEN |
| `notes_api_product_config_map_get_string_must_clone` | ✅ tip GREEN |

**Root cause layer:** constraint/Copy classification. Borrowed iterator text is
`SafetyType::borrowed` (not owned). Clone stays unless the formal is a shared
borrow. Copy identity ignores ref wrappers and unsubstituted generic formals.

**What became unnecessary:** treating every `&T` loop binding as a Copy move into
`Vec::push` / `HashMap::insert`.

**Gates:** `cargo test --release --test all --features integration_tests,codegen_tests -- hashmap_field_iter_push nested_match_for_push notes_api_product_config` → **4 passed**.

## P3.724 (2026-10-07) — Map::get match payloads clone into owned String/struct

WJ `Map::get -> Option<V>` is surface sugar; Rust `HashMap::get` is `Option<&V>`.
Match arms (`Some(Enum::String(value)) => Some(value)`, `Some(stored) => Ok(stored)`)
emitted bare bindings / `*binding` because `&String` and double-wrapped `&Note`
were treated as Copy.

| Gate | Status |
|------|--------|
| `single_file_hashmap_get_string_payload_must_clone` | ✅ tip GREEN |
| `module_file_map_get_string_payload_must_clone` | ✅ tip GREEN |
| `hashmap_get_enum_destructure_string_type` | ✅ tip GREEN |
| `hashmap_get_noncopy_match_not_copied` | ✅ tip GREEN |
| `tip_out_event_get_data_string_must_clone` | ❌ tip-out lag (regen) |
| `notes_api_product_config_map_get_string_must_clone` | ✅ tip GREEN (P3.725) |
| `nested_match_for_push_string_owns_or_clones` | ✅ tip GREEN (P3.725) |
| `hashmap_field_iter_push_clones_noncopy_value` | ✅ tip GREEN (P3.725) |

**Root cause layer:** constraint/type — match binding types from shared-ref get;
wrapper rewrite peels refs before Copy vs clone.

**What became unnecessary:** `Some(value)` / `Ok(*stored)` for non-Copy `HashMap::get` payloads.

**Gates:** `cargo test --release --test all --features integration_tests,codegen_tests -- bug_hashmap_get_match_string hashmap_get_noncopy hashmap_get_match_deref` → string/Note match tests GREEN; tip-out and notes-api for-loop still RED.

## P3.723 (2026-10-07) — WDB-365: nested self-fields must not force `__wj_tmp`

`disjoint_self_field_accesses` only saw top-level `self.field` args, so
`self.recording.add_event(InputEvent::key_down(self.current_frame, …))` and
`self.camera.follow_player_pos(self.player.position.x, …)` extracted temps.
Collect nested `self.<field>` roots inside Call/MethodCall args; skip temps when
all differ from the receiver root field.

| Gate | Status |
|------|--------|
| `wdb365_module_file_must_not_emit_wj_tmp_lets` | ✅ tip GREEN (product match shape) |
| `wdb365_tip_out_game_core_must_not_emit_wj_tmp` | ✅ tip GREEN (simple-pass scanner) |

**Root cause layer:** split-borrow / self-temp extraction classification.

**What became unnecessary:** `__wj_tmp` around disjoint-field constructor args into
`add_event` / `follow_player_pos` / `push(get_id)`.

**Gates:** `cargo test --release --test all --features integration_tests,codegen_tests -- wdb365_` → **2 passed**.

## P3.722 (2026-10-07) — WDB-463/462/368 tip GREEN (nested for-borrow + &* text)

**WDB-463:** nested `while` marked `for_loop_borrow_needed` for locals re-bound
each iteration (`children_copy`). Only mark when the iterable lives outside the
outer loop (`decl_depth < loop_depth`).

**WDB-462:** tip-out lag — fresh multipass emits `result.push(*id)` for Copy
`StateId` from `.keys()`.

**WDB-368:** `finalize_ir_call_arg` treated `&String` match payloads as Copy
(`&flag` → `*flag`), then shared-ref reconcile re-borrowed → `&*flag`. Skip
Copy deref for text ref bindings; strip residual `&*ident` after method reconcile.

| Gate | Status |
|------|--------|
| `wdb463_` MultiFile + tip-out | ✅ tip GREEN |
| `wdb462_` MultiFile + tip-out | ✅ tip GREEN |
| `wdb368_` MultiFile + tip-out | ✅ tip GREEN |

**Root cause layer:** for-loop borrow classification (463); tip-out regen (462);
call-site finalize / text vs Copy (368) — not new peels without type fix.

**What became unnecessary:** `&children_copy` + `child_id.clone()`; `id.clone()`
on Copy keys; `&*flag` into `&str`.

**Gates:** `cargo test --release --test all --features integration_tests,codegen_tests -- wdb463_ wdb462_ wdb368_` → **6 passed**.

**Do not steal:** WDB-365 `__wj_tmp` tip-out + remaining wave.

## P3.721 (2026-10-07) — WDB-366: BT tick must not force `tree.clone()`

`BehaviorTree` formals stayed Borrowed from analyzer, but multipass
`restore_owned_field_forward_formals` / soft forwarded restore re-Owned them
because bare `f(tree)` looked like FieldInCallArg + struct store. That froze an
Owned cycle with `tick_decorator_*` and forced `tree.clone()` at call sites.

| Gate | Status |
|------|--------|
| `wdb366_module_file_bt_tick_must_not_force_tree_clone` | ✅ tip GREEN (MultiFile + tip-out) |
| tip wdb-layers `--module-file` | ✅ EXIT 0 (~221s after hard-owned cache) |

**Root cause layer:** multipass restore / formal classification (not reconcile peels).

**What became unnecessary:** soft AST-Custom "owned peer" restore that froze the
`tick_node` ↔ decorator Owned cycle; false field-forward/store restores on bare
forwards.

**Gates:** `cargo test --release --test all -- wdb366_` → **2 passed**;
`cargo test --release -p windjammer --lib multipass_bare_pass` → **18 passed**.

**Do not steal:** remaining tip-true REDs (WDB-365/368 + WDB-462/463 tip-out).

## P3.720 (2026-10-07) — directory package must emit `mod.rs` + re-exports into `--output`

Breach Protocol `src/inventory/` (directory module) tip-transpile to `gen/` writes
`gen/inventory/{item,item_id,…}.rs` but **omits** `gen/inventory/mod.rs` on some
tip-out paths. Root `lib.rs` then hits E0583; thin synthesized decls miss `pub use ItemId`
(cascading E0425). wj-game restores from stale `build/` as a host workaround.

| Gate | Status |
|------|--------|
| `directory_module_must_emit_mod_rs_with_reexports` | ✅ tip GREEN (MultiFile + CLI `--output gen`) |
| product tip-out lag / wiped gen | wj-game `ensure_gen_directory_mod_rs` host guardrail |

**Root cause layer:** multipass / `--module-file` emit for directory packages into
`--output gen` must write `gen/<pkg>/mod.rs` with child `pub mod` + public re-exports.

**Do not steal:** wj-game restore-from-`build/`; P3.718–P3.719 (other agents).

**Gates:** `cargo test --release --test all --features integration_tests -- directory_module_must_emit_mod_rs_with_reexports`.

## P3.719 (2026-10-07) — TDD WDB-463 (DB agent; no compiler src)

Owned `Vec<Copy>` for-loop must not demote to `&` + `.clone()` into `Vec::push`.

Product `scene_graph/scene_graph_state.rs` `remove_node`:
```wj
let children_copy = node.children
for child_id in children_copy {
    to_remove.push(child_id)
}
```
Tip MultiFile keeps owned for + bare `push(child_id)` (isolate GREEN). Tip-out
emits `for child_id in &children_copy` + `to_remove.push(child_id.clone())`.

| Gate | Status |
|------|--------|
| WDB-463 MultiFile | ✅ isolate GREEN — owned for + bare push |
| WDB-463 tip-out | ✅ tip GREEN (P3.722) — owned `for child_id in children_copy` + bare push |

**Root cause layer:** tip-out / product multipass lag — isolate tip already correct;
owned `Vec<u64>` for-loops still demoted to borrowed + Copy `.clone()` into push.

**Why this is a new class:**
- WDB-462 is Copy newtype from **HashMap.keys()** (inherently borrowed).
- WDB-457 is owned Copy **newtype local** into push.
- WDB-431 is Copy u64 **field** into insert/push.
- This is owned `Vec<Copy>` for-loop demoted to `&` + `.clone()` into push.

**Gates:** `cargo test --release --test all --features integration_tests,codegen_tests -- wdb463_` — isolate GREEN / tip RED (2026-10-07).

**Do not steal:** WDB-406/408/411/457–463, P3.508–P3.719, WDB-412–463 (filed).

## P3.718 (2026-10-07) — TDD WDB-462 (DB agent; no compiler src)

Copy **newtype** from `HashMap.keys()` into `Vec::push` must not `.clone()`.

Product `state_machine/machine.rs` `all_state_ids`:
```wj
for id in self.states.keys() {
    result.push(id)
}
```
Tip MultiFile emits bare `push(id)` (isolate GREEN). Tip-out still has
`result.push(id.clone())` for Copy `StateId` (`u32` newtype).

| Gate | Status |
|------|--------|
| WDB-462 MultiFile | ✅ isolate GREEN — bare `push(id)` |
| WDB-462 tip-out | ✅ tip GREEN (P3.722) — `result.push(*id)` / no `.clone()` |

**Root cause layer:** tip-out / product multipass lag — isolate tip already correct;
Copy newtype bindings from `.keys()` still get `.clone()` into owned `Vec::push`.

**Why this is a new class:**
- WDB-457 is owned Copy newtype **local** into Vec::push (`LightId`).
- WDB-459 is Copy **struct** local into Vec::push (`UvCoord`).
- WDB-431 is Copy u64 **field** into insert/push.
- This is Copy newtype from **HashMap.keys()** iterator into Vec::push (`StateId`).

**Gates:** `cargo test --release --test all --features integration_tests,codegen_tests -- wdb462_` — isolate GREEN / tip RED (2026-10-07).

**Do not steal:** WDB-406/408/411/457–462, P3.508–P3.718, WDB-412–462 (filed).

## P3.717 (2026-10-07) — WDB-374: Copy enum match must not `.state.clone().clone()`

Product `world/streaming` matched `self.chunks[i].state` (Copy
`ChunkLifecycleState`) via owned-clone borrow-break, stacking field `.clone()`
+ borrow-break `.clone()`. Same-file Copy now uses owned-copy borrow-break even
when arms reassign the place; owned-clone strips a pre-existing `.clone()`.
Tip-out needs multipass `world/` so cross-module Copy registry is populated.

| Gate | Status |
|------|--------|
| `wdb374_module_file_copy_enum_field_must_not_double_clone` | ✅ tip GREEN |
| `wdb374_module_file_match_indexed_copy_enum_must_not_double_clone` | ✅ tip GREEN |
| `wdb374_tip_out_game_core_streaming_must_not_double_clone_state` | ✅ tip GREEN after world multipass regen |

**Root cause layer:** match encoding + registry — owned-copy borrow-break for
Copy self-fields; multipass Copy registry for imported enums.

**What became unnecessary:** `state.clone().clone()` on streaming match temps;
owned-clone path no longer stacks a second `.clone()`.

**Gates:** `CARGO_TARGET_DIR=target-agent-tip-p3698` →
`cargo test --release --test all -- wdb374_` → **3 passed**.

**Do not steal:** remaining tip-true REDs (WDB-365/366/368 + wave).

## P3.716 (2026-10-06) — WDB-354: owned Result match must not `.map(|v| v.to_owned())`

Product `AssetLoader::load_batch` matched `self.load(...) -> Result<T,E>` via the
borrow-break fallback `.map(|__v| __v.to_owned()).as_ref()`. Owned Result Ok
payloads need the same by-value temp path as owned Option.

| Gate | Status |
|------|--------|
| `wdb354_module_file_result_ok_must_not_to_owned_borrow_break` | ✅ tip GREEN |
| `wdb354_module_file_self_load_result_must_not_to_owned_borrow_break` | ✅ tip GREEN (product shape) |
| `wdb354_tip_out_game_core_loader_must_not_to_owned_borrow_break` | ✅ tip GREEN after tip-out regen |

**Root cause layer:** match encoding — `match_borrow_break_yields_owned_result`
+ owned-result borrow-break branch (mirror owned Option).

**What became unnecessary:** `.map(|__v| __v.to_owned())` + `.as_ref()` on owned
`Result` method returns. (Note: `name.clone().clone()` on the same call site is
a separate reuse bug.)

**Gates:** `CARGO_TARGET_DIR=target-agent-tip-p3698` →
`cargo test --release --test all -- wdb354_` → **3 passed**.

**Do not steal:** remaining tip-true REDs (WDB-365–366/368/374 + wave).

## P3.715 (2026-10-06) — WDB-383: index `as i64` must retarget to `as usize`

Product `frame_analysis` wrote `bins[clamped as i64]`; index lowering stacked
`as usize` → `clamped as i64 as usize`. Retarget mid-width casts on index
places; also fold nested AST `(e as i64) as usize` in `generate_cast`.

| Gate | Status |
|------|--------|
| `wdb383_module_file_u32_index_must_not_cast_via_i64` | ✅ tip GREEN |
| `wdb383_module_file_index_as_i64_must_fold_to_usize` | ✅ tip GREEN (product shape) |
| `wdb383_tip_out_game_core_frame_analysis_must_not_cast_via_i64` | ✅ tip GREEN after tip-out regen |
| `wdb353_*` (3) | ✅ tip GREEN (no regression) |

**Root cause layer:** coercion/encoding — `maybe_cast_index_to_usize` retargets
trailing `as i64`/`as i32`; `generate_cast` folds nested i64→usize.

**What became unnecessary:** `as i64 as usize` on frame_analysis bins index.

**Gates:** `CARGO_TARGET_DIR=target-agent-tip-p3698` →
`cargo test --release --test all -- wdb383_ wdb353_` → **6 passed**.

**Do not steal:** remaining tip-true REDs (WDB-354/365–366/368/374 + wave).

## P3.714 (2026-10-06) — WDB-364: range start must not emit `_usize as usize`

Product `physics_world` nested `for j in (i + 1)..bodies.len()` emitted
`(i + 1_usize as usize)` — peer suffix already made the bound usize, then
range codegen stacked `as usize` with wrong `as`/`+` precedence.

| Gate | Status |
|------|--------|
| `wdb364_module_file_usize_lit_must_not_cast_as_usize` | ✅ tip GREEN |
| `wdb364_module_file_range_start_must_not_cast_usize_lit` | ✅ tip GREEN (product shape) |
| `wdb364_tip_out_game_core_must_not_cast_usize_lit` | ✅ tip GREEN after tip-out regen |

**Root cause layer:** coercion/encoding — skip range-start `as usize` when the
bound already contains `_usize` or `expression_produces_usize`; fix binary cast
parens to `(expr) as usize`.

**What became unnecessary:** `1_usize as usize` / `(i + 1_usize as usize)` on
`.len()` ranges.

**Gates:** `CARGO_TARGET_DIR=target-agent-tip-p3698` →
`cargo test --release --test all -- wdb364_` → **3 passed**.

**Do not steal:** remaining tip-true REDs (WDB-354/365–366/368/374/383 + wave).

## P3.713 (2026-10-06) — WDB-353: fold `(e as i32) as usize` → `e as usize`

Product `terrain/terrain.wj` writes nested truncations for grid indices. Tip
emitted `as i32 as usize`. Collapse the mid-width in `generate_cast` when the
outer target is `usize` and the inner cast is `i32`.

| Gate | Status |
|------|--------|
| `wdb353_module_file_f32_to_usize_must_not_double_cast_via_i32` | ✅ tip GREEN |
| `wdb353_module_file_nested_i32_usize_cast_must_fold` | ✅ tip GREEN (product shape) |
| `wdb353_tip_out_game_core_terrain_must_not_double_cast` | ✅ tip GREEN after tip-out regen |

**Root cause layer:** coercion/encoding — nested cast fold in `generate_cast`.

**What became unnecessary:** `as i32 as usize` on terrain grid index emits; no
reconcile peel.

**Gates:** `CARGO_TARGET_DIR=target-agent-tip-p3698` →
`cargo test --release --test all -- wdb353_` → **3 passed**.

**Do not steal:** remaining tip-true REDs (WDB-354/364–366/368/374/383 + wave).

## P3.712 (2026-10-06) — WDB-349: boolean `matches!` Option presence must not clone

Product `UsdNode::has_mesh`: `match self.mesh { Some(_) => true, None => false }`
lowered via the boolean→`matches!` fast path with `self.mesh.clone()`. Presence
checks never need an owned Option clone; prefer `matches!(&self.mesh, Some(_))`.

| Gate | Status |
|------|--------|
| `wdb349_module_file_option_presence_must_not_clone` | ✅ tip GREEN |
| `wdb349_module_file_self_option_presence_must_not_clone` | ✅ tip GREEN (product shape) |
| `wdb349_tip_out_game_core_usd_must_not_matches_clone` | ✅ tip GREEN after tip-out regen |

**Root cause layer:** match encoding — boolean `matches!` scrutinee suppresses
borrowed clone, strips stacked `.clone()`, and borrows Option behind `&self`.
Also teach `option_scrutinee_ref_prefix` to honor `inferred_borrowed_params` for
`self` (OwnershipHint may still be Inferred).

**What became unnecessary:** `matches!(self.mesh.clone(), Some(_))` on presence
checks; clone fallback when ref-prefix was empty for inferred `&self`.

**Gates:** `CARGO_TARGET_DIR=target-agent-tip-p3698` →
`cargo test --release --test all -- wdb349_` → **3 passed**.

**Do not steal:** remaining tip-true REDs (WDB-353–354/364–366/368/374/383 + wave).

## P3.711 (2026-10-06) — WDB-340: demoted `&str` → String assign must not double `.to_string()`

Product `AssetBrowser::search` / `list_assets`: demoted `&str` formal reused then
assigned into owned `string` field. Tip emitted `query.to_string().to_string()`
because P3.418 demoted-formal coerce and auto_clone reuse both appended
`.to_string()` (second path unchecked).

| Gate | Status |
|------|--------|
| `wdb340_module_file_owned_string_must_not_double_to_string` | ✅ tip GREEN |
| `wdb340_module_file_demoted_str_assign_must_not_double_to_string` | ✅ tip GREEN (product shape) |
| `wdb340_tip_out_game_core_must_not_double_to_string` | ✅ tip GREEN after tip-out regen |

**Root cause layer:** assignment coercion — idempotent auto_clone path for demoted
text formals (skip when already `.to_string()` / `.into()` / owned).

**What became unnecessary:** stacked `.to_string().to_string()` on demoted formal
→ String field assign; dual-oracle re-apply in auto_clone branch. No ir_call_site peel.

**Gates:** `CARGO_TARGET_DIR=target-agent-tip-p3698` →
`cargo test --release --test all -- wdb340_` → **3 passed**.

**Do not steal:** remaining tip-true REDs (WDB-349/353–354/364–366/368/374/383 + wave).

## P3.710 (2026-10-06) — WDB-357: associated `new` must not emit `impl Into<String>`

Product `ConsoleCommand::new` / `DialogueHistoryEntry::new` / `*Data::new` are
associated constructors (no `self`). Tip emitted `impl Into<String>` + `.into()`
because Into eligibility treated `in_impl_block` + returns-impl-type as builders.
Self-withers (`StatusChip::label`) still need Into for Rust `&str` callers.

| Gate | Status |
|------|--------|
| `wdb357_module_file_owned_string_must_not_emit_into` | ✅ tip GREEN — associated `new` → `String` |
| `wdb357_tip_out_game_core_must_not_emit_into_string` | ✅ tip GREEN after tip-out regen |
| `ui_builder_string_formal_*` (3) | ✅ tip GREEN — `new: String`, `label: impl Into<String>` |
| `wdb157_module_file_string_formal_must_not_emit_impl_into_string_with_clone` | ✅ tip GREEN |

**Root cause layer:** formal encoding — Into only when `has_self_receiver`
(builders/withers). Dropped the associated-constructor exception that upgraded
`ConsoleCommand::new` / `StatusChip::new` to Into.

**What became unnecessary:** `impl Into<String>` + `.into()` on associated
constructors without `self`; tip-out lag on console/dialogue/scene_serializer.
Narrowed Into path (net −14 LOC in formal eligibility). No ir_call_site peel.

**Gates:** `CARGO_TARGET_DIR=target-agent-tip-p3698` →
`cargo test --release --test all -- ui_builder_into_string wdb357_ wdb157_`
→ **6 passed**.

**Do not steal:** remaining tip-true REDs (WDB-340/349/353–354/364–366/368/374/383 + wave).

## P3.709 (2026-10-06) — WDB-358: `&self` must not `self.clone()` borrowed siblings

Product `is_mesh_uploaded(&self)` emitted `self.clone().find_mesh_index(name)`
even though `find_mesh_index` is `&self`. Receiver consume-clone skipped only for
`&mut self` callers.

| Gate | Status |
|------|--------|
| `wdb358_module_file_self_method_must_not_clone_receiver` | ✅ tip GREEN |
| `wdb358_tip_out_game_core_must_not_self_clone_before_borrowed_method` | ✅ tip GREEN after regen |

**Root cause layer:** coercion/encoding (method receiver) — extend skip_clone to
`is_shared_borrowed_param` when callee ownership is Borrowed/MutBorrowed.

**What became unnecessary:** `self.clone().find_mesh_index` / similar tip-out
`self.clone().method` on borrowed receivers; no new ir_call_site peel.

**Gates:** `cargo test --release --test all -- wdb358_` → **2 passed**.

## P3.708 (2026-10-06) — tip-out Copy-clone lag sweep (no new peels)

Regenerated game-core tip-out modules with tip `wj` after P3.704
`binding_is_copy_type`: physics, rendering, ui, voxel, scripting, game_framework,
ffi_tilemap, terrain, ai, world, ecs, frame_analysis (+ prior math/audio/input).

| Gate | Status |
|------|--------|
| `wdb438_`…`wdb456_` tip-out filters (17) | ✅ tip GREEN |

**Root cause layer:** tip-out lag — tip already correct; multipass sync only.

**Gates:** `cargo test --release --test all -- wdb456_tip_out … wdb438_tip_out` → **17 passed**.

## P3.707 (2026-10-06) — BP `update_death_state` bare `self` → `mut self`

Product `BreachProtocolGame::update_death_state` assigns fields then calls owned
`upload_camera`. Tip emitted immutable `self` → E0594.

| Gate | Status |
|------|--------|
| `bp_module_file_update_death_state_must_emit_mut_self` | ✅ tip GREEN — `mut self` |
| `bp_tip_out_update_death_state_must_emit_mut_self` | ✅ tip GREEN after BP tip regen |

**Root cause layer:** constraint/self-receiver formal — `OwnershipHint::Inferred`
early path for `function_calls_owned_self_method` only upgraded to `mut self`
when `returns_impl_struct`; bool-returning mutators got bare `self`. Also
`OwnershipHint::Ref` + `Owned` skipped `owned_self_receiver` when `body_modifies`.

**What became unnecessary:** bare immutable `self` on field-assign + owned-sibling
callers (`update_death_state`, `update_camera_movement`); no new reconcile peel.

**Gates:** `CARGO_TARGET_DIR=target-agent-tip-p3698` →
`cargo test --release --test all -- bp_module_file_update_death_state bp_tip_out_update_death_state`
→ **2 passed**. Related: notes-api / check_rate / WDB-414 MultiFile ✅.

**Do not steal:** remaining tip-true REDs (WDB-340+ cluster); WDB-460/461 tip-out GREEN.

## P3.706 (2026-10-06) — TDD WDB-461 (DB agent; no compiler src)

Copy `Vec3` into **method** owned formal must not `.clone()`.

Product tip-out:
```wj
let uv = qv.cross(vec)
let uuv = qv.cross(uv)
// look_at: s.cross(f); s.dot(eye)
// spatial: is_audible(listener_pos)
```
Tip MultiFile emits bare args (isolate GREEN). Tip-out / game-core gen still
emit `cross(vec.clone())` / `dot(eye.clone())` / `is_audible(listener_pos.clone())`.

| Gate | Status |
|------|--------|
| WDB-461 MultiFile | ✅ tip GREEN |
| WDB-461 tip-out | ✅ tip GREEN after math/audio_3d tip-out regen (P3.704 `binding_is_copy_type`) |

**Root cause layer:** tip-out lag cleared — tip already correct via P3.704 Copy
binding skip; multipass regen of `math/` + `audio_3d/` synced tip-out/gen.

**Why this is a new class:**
- WDB-355 is Copy Vec3 into **free-fn** owned formal (`vertex(p, n)`).
- WDB-458 is Copy **newtype** into method formal (`has`).
- WDB-459 is Copy **struct** into Vec::push.
- WDB-460 (parallel) is Copy **unit enum** into Vec::push (`Key`).
- This is Copy `Vec3` into **method** owned formal (`cross` / `dot` / `is_audible`).

**Gates:** `cargo test --release --test all --features integration_tests,codegen_tests -- wdb461_` — isolate GREEN / tip RED (2026-10-06).

**Do not steal:** WDB-406/408/411/457–461, P3.508–P3.706, WDB-412–461 (filed). Note: P3.705 is LedgerKit/int_eq (foreign).

## P3.704 (2026-10-06) — TDD WDB-459 (DB agent; no compiler src)

Copy **struct** local into `Vec::push` must not `.clone()`.

Product `editor/uv_unwrap_algorithm.rs` / `uv_island_packing.rs`:
```wj
let a = tri_norm.a
all_corners.push(a)
uvs.push(a)
```
Tip MultiFile emits bare `push(a)` / `push(b)` / `push(c)` (isolate GREEN).
Tip-out + game-core gen still emit `a.clone()` / `fa.clone()` etc.

| Gate | Status |
|------|--------|
| WDB-459 MultiFile | ✅ tip GREEN (P3.704) |
| WDB-459 tip-out | ✅ tip GREEN (P3.704) |

**Root cause layer:** fixed in P3.704 (`binding_is_copy_type` skips clone).

**Why this is a new class:**
- WDB-457 is Copy **newtype** into Vec::push (`LightId`).
- WDB-458 is Copy newtype into owned **method formal** (`has`) + push.
- WDB-440 is f32 local into **struct lit**.
- This is Copy **struct** local (`UvCoord`) into Vec::push with field peel + reuse.

**Gates:** `cargo test --release --test all --features integration_tests,codegen_tests -- wdb459_` — isolate GREEN / tip RED (2026-10-06).

**Do not steal:** WDB-406/408/411/457–459, P3.508–P3.704, WDB-412–459 (filed).

## P3.704 (2026-10-06) — WDB-457/458/459: Copy bindings must not `.clone()` into owned slots

Auto-clone marked reuse of Copy newtypes/structs (`LightId`, `ComponentId`,
`UvCoord`) and method-arg finalize cloned into owned slots because
`callee_formal_is_copy` is false for generic `Vec::push(T)` / owned formals.

| Gate | Status |
|------|--------|
| `wdb457_module_file_copy_newtype_local_vec_push_must_not_clone` | ✅ tip GREEN |
| `wdb457_tip_out_game_core_light_manager_copy_newtype_vec_push_must_not_clone` | ✅ tip GREEN after tip-out sync |
| `wdb458_module_file_copy_newtype_local_owned_formal_must_not_clone` | ✅ tip GREEN |
| `wdb458_tip_out_game_core_component_storage_copy_newtype_owned_formal_must_not_clone` | ✅ tip GREEN after tip-out sync |
| `wdb459_module_file_copy_struct_local_vec_push_must_not_clone` | ✅ tip GREEN |
| `wdb459_tip_out_game_core_uv_unwrap_copy_struct_vec_push_must_not_clone` | ✅ tip GREEN after tip-out sync |

**Root cause layer:** coercion/encoding — skip `.clone()` when the *argument
binding* is Copy (`binding_is_copy_type`), not only when the callee formal is a
Copy aggregate.

**What became unnecessary:** `id.clone()` / `comp_id.clone()` / `push(a.clone())`
on Copy locals; no new `ir_call_site` peel.

**Gates:** `CARGO_TARGET_DIR=target-agent-tip-p3698` →
`cargo test --release --test all -- wdb457_ wdb458_ wdb459_` → **6 passed**.

**Do not steal:** remaining tip-true REDs (WDB-340/349/353–354/357–358/364–366/368/374/383 + wave).

## P3.703 (2026-10-06) — TDD WDB-458 (DB agent; no compiler src)

Copy **newtype** local into owned method formal must not `.clone()`.

Product `ecs/component_storage.rs` `list_component_ids_for_entity`:
```wj
let comp_id = ComponentId::new(i)
if self.has(entity, comp_id) {
    result.push(comp_id)
}
```
Tip + MultiFile both emit `has(entity, comp_id.clone())` and
`result.push(comp_id.clone())` even though `ComponentId` is `Copy` (`u32` newtype).

| Gate | Status |
|------|--------|
| WDB-458 MultiFile | ✅ tip GREEN (P3.704) |
| WDB-458 tip-out | ✅ tip GREEN (P3.704) |

**Root cause layer:** fixed in P3.704 (`binding_is_copy_type` skips clone).

**Why this is a new class:**
- WDB-457 is Copy newtype into **Vec::push** with field reuse (`LightId`).
- WDB-431 is Copy u64 **field** into insert/push.
- WDB-393 is i32 **formal** field assign.
- This is Copy **newtype local** into owned **method formal** (`has`) + push reuse.

**What became unnecessary:** rewriting ComponentId list loops to avoid has+push.

**Gates:** `cargo test --release --test all --features integration_tests,codegen_tests -- wdb458_` — isolate RED / tip RED (2026-10-06).

**Do not steal:** WDB-406/408/411/457–458, P3.508–P3.703, WDB-412–458 (filed).

## P3.701 (2026-10-06) — WDB-416: owned-self method is not "readonly"

`sig_readonly_receiver` treated any non-`&mut self` as readonly, so
`Val::as_float(self)` (Owned) was classified readonly. Codegen then skipped
owning-method formal retention and demoted `a: Value` → `&Value` +
`a.clone().as_float()` (product `visual_scripting/runtime`).

| Gate | Status |
|------|--------|
| `wdb416_module_file_owned_enum_must_not_clone_before_as_float` | ✅ tip GREEN |
| `wdb416_module_file_noncopy_enum_match_must_not_demote_before_as_float` | ✅ tip GREEN (product shape) |
| `wdb416_tip_out_game_core_value_must_not_clone_before_as_float` | ✅ tip GREEN after tip-out/gen sync |
| `owned_self_is_not_readonly_receiver` | ✅ unit GREEN |

**Root cause layer:** signature — `sig_readonly_receiver` / `is_known_readonly_qualified`
must mean shared `&self` only; Owned `self` is consuming.

**What became unnecessary:** tip/product `&Value` + `.clone().as_float()`; no
`ir_call_site` peel. Owning-method demotion path now sees Owned self correctly.

**Gates (2026-10-06 re-verify):** `CARGO_TARGET_DIR=target/agent-tdd-next` →
`cargo test --test all --features integration_tests -- wdb416` → **3 passed**;
`cargo test -p windjammer --lib owned_self_is_not_readonly_receiver` → **1 passed**.

**Do not steal:** remaining tip-true REDs (WDB-340/349/353–354/357–358/364–366/368/374/383 + wave);
WDB-457/458 are DB-agent tip-outs (no compiler src).





## P3.704 (2026-10-06) — BP `update_death_state` must emit `&mut self` (tip-out)

**Superseded by P3.707** — tip GREEN (`mut self`) after Inferred/Ref self-receiver fix.

| Gate | Status |
|------|--------|
| Minimal MultiFile field-assign tick | ✅ tip GREEN |
| `bp_module_file_update_death_state_must_emit_mut_self` | ✅ tip GREEN (P3.707) |
| `bp_tip_out_update_death_state_must_emit_mut_self` | ✅ tip GREEN (P3.707) |

## P3.702 (2026-10-06) — TDD WDB-457 (DB agent; no compiler src)

Copy **newtype** local into `Vec::push` must not `.clone()`.

Product `lighting2d/light_manager.rs` `add_light`:
```wj
let id = LightId::new(self.next_id)
self.light_ids.push(id)
```
Tip + MultiFile both emit `self.light_ids.push(id.clone())` even though
`LightId` is `Copy` (`i32` newtype) and `id` is reused for `id.value()`.

| Gate | Status |
|------|--------|
| WDB-457 MultiFile | ✅ tip GREEN (P3.704) |
| WDB-457 tip-out | ✅ tip GREEN (P3.704) |

**Root cause layer:** fixed in P3.704 (`binding_is_copy_type` skips clone).

**Why this is a new class:**
- WDB-431 is Copy u64 **field** into insert/push.
- WDB-438 is Copy i32 formal into **tuple** lit for push.
- WDB-455 is i32 local into **field** assign.
- This is Copy **newtype local** into bare `Vec::push` with post-push reuse.

**What became unnecessary:** rewriting LightId add paths to avoid push+reuse.

**Gates:** `cargo test --release --test all --features integration_tests,codegen_tests -- wdb457_` — isolate RED / tip RED (2026-10-06). Test on tip; this records the dual-RED TDD gate.

**Do not steal:** WDB-406/408/411/456–457, P3.508–P3.702, WDB-412–457 (filed). Note: P3.701 is the unrelated u32 Option-slot min widen gate.

## P3.701 (2026-10-06) — u32 Option-slot `min` + untyped `0` must not widen to U64

Breach Protocol `inventory::remove_item`:
```wj
pub fn remove_item(self, quantity: u32) -> u32 {
    let mut removed = 0
    let mut remaining = quantity
    // … Vec<Option<Stack>> + remaining.min(stack.quantity()) …
}
```
Tip numeric inference reports `must be U32 … but was U64` on `to_remove` /
`remaining` / `min()` return (analysis hard-fail).

| Gate | Status |
|------|--------|
| `u32_option_slot_min_untyped_zero_must_not_widen_u64` | ✅ tip GREEN (2026-10-06 recheck; keep as regression) |
| Typed `let mut removed: u32 = 0u32` same body | ✅ GREEN (control) |
| Bare `min` without `Option` slots | ✅ GREEN (control) |
| Inventory multipass + `equipment.wj` | ⚠ intermittent RED (poison twin; re-file if stable) |

**Root cause layer:** numeric inference — untyped `0` under `-> u32` inside
`Vec<Option<_>>` + `.min(u32)` must stay U32, not widen the u32 chain to U64.

**Why this is a new class:**
- P3.327 is emit-suffix `_u64` on u32±literal (codegen).
- This was an **analysis** U32/U64 conflict that aborts transpile (seen on tip CLI
  2026-10-05 BP build; isolate GREEN again 2026-10-06 — keep regression gate).

**Do not steal:** P3.327, P3.348.

**Gates:** `cargo test --release --test all --features integration_tests -- u32_option_slot_min_untyped_zero` → **1 passed**.

## P3.700 (2026-10-06) — WDB-410: owned-self struct-lit wither must move fields

Product `PassBuilder::shader` / `dispatch` / `critical` cloned every
`self.field` because auto-clone `self_always_clone` only exempted bodies with
`let x = self.field`, not struct-lit-only withers.

| Gate | Status |
|------|--------|
| `wdb410_module_file_owned_self_wither_must_move_fields` | ✅ tip GREEN |
| `wdb410_module_file_struct_lit_only_wither_must_move_fields` | ✅ tip GREEN (product shape) |
| `wdb410_tip_out_game_core_owned_self_wither_must_move_fields` | ✅ tip GREEN after tip-out/gen sync |

**Root cause layer:** constraint/auto-clone — treat `StructLiteral { … self.field … }`
as an owned-self field move so partial-move detection does not force-clone.

**What became unnecessary:** tip/product `self.graph.clone()` / `self.bindings.clone()`
on owned-self withers; no `ir_call_site` peel.

**Gates:** `CARGO_TARGET_DIR=target-agent-tip-p3698` →
`cargo test --release --test all -- wdb410_` → **3 passed**.

**Do not steal:** remaining tip-true REDs (WDB-340/349/353–354/357–358/364–366/368/374/383/416 + wave).

## P3.699 (2026-10-06) — WDB-407: associated `new(Vec)` payload store must stay owned

Product `VoxParser::new(data: Vec<u8>)` (private in WJ) demoted to
`pub fn new(data: &Vec<u8>)` + `data.clone()` / `VoxParser::new(&data)`.
FABRIK `new(Vec<Vec3>)` was already owned; VoxParser missed keep-owned because
`is_public_owned_non_copy_formal_api` required `pub`.

| Gate | Status |
|------|--------|
| `wdb407_module_file_owned_vec_new_must_not_demote` | ✅ tip GREEN |
| `wdb407_module_file_private_associated_vec_new_must_not_demote` | ✅ tip GREEN (product shape) |
| `wdb407_tip_out_game_core_owned_vec_new_must_not_demote` | ✅ tip GREEN after tip-out/gen sync |

**Root cause layer:** signature — associated constructors that store `Vec` into a
field keep Owned even without WJ `pub`; restore covers associated Vec payload formals.

**What became unnecessary:** tip/product `&Vec` + `.clone()` for `VoxParser::new`;
no `ir_call_site` peel.

**Gates:** `CARGO_TARGET_DIR=target-agent-tip-p3698` →
`cargo test --release --test all -- wdb407_ wdb398_` → **6 passed**.

**Do not steal:** remaining tip-true REDs (WDB-340/349/353–354/357–358/364–366/368/374/383/416 + wave).

## P3.698 (2026-10-06) — WDB-398: owned `new(MaterialPalette)` must not get `&palette.copy()`

Product `VoxelMaterialEditor::new(palette: MaterialPalette)` kept emitting owned
formals, but call sites still produced `new(&palette.copy())` because a bloated
6×`MaterialPalette` mixed registry bag (`emitted[0]=true`) blocked the defining
1-param Owned refresh via `defining_mixed_owned_emission_beats`, and
`restore_pub_owned_non_copy_api_formals` skipped all `Type::method` keys.

| Gate | Status |
|------|--------|
| `wdb398_module_file_owned_copy_into_new_must_not_borrow` | ✅ tip GREEN |
| `wdb398_module_file_associated_new_owned_palette_must_not_demote` | ✅ tip GREEN |
| `wdb398_tip_out_game_core_voxel_editor_must_not_borrow_palette_copy` | ✅ tip GREEN after tip-out/gen sync |

**Root cause layer:** signature — (1) shape-gate `defining_mixed_owned_emission_beats`;
(2) restore pub associated owned Custom (no `self`); (3) all-owned refresh replaces
stale `emitted[i]=true` instead of OR-union; (4) keep-owned associated formals in
prepare / method-registry sync (formal emit already owned).

**What became unnecessary:** tip/product `&palette.copy()` into owned `new`; no new
`ir_call_site` peel. Corrupted 6-param `VoxelMaterialEditor::new` metadata collapsed
to 1× Owned + `emitted=[false]`.

**Gates:** `CARGO_TARGET_DIR=target-agent-tip-p3698` →
`cargo test --release --test all -- wdb398_` → **3 passed**.

**Do not steal:** remaining tip REDs (WDB-340/349/353–358/364–366/368/374/383/416 + wave).

## P3.697 (2026-10-06) — WDB-344/346: Copy f32 match deref + tip-out camera sync

Product `collision2d`: demoted `&RigidBody2D` → `match &a.collider` binds `&f32`,
IR Deref emits `*w1`, then `append_rust_clone` wrongly produced `*w1.clone()`.
Camera tip-out `self.pivot.z.clone()` was stale multipass lag (fresh tip already clean).

| Gate | Status |
|------|--------|
| `wdb344_module_file_copy_f32_must_not_emit_clone` | ✅ tip GREEN |
| `wdb344_module_file_match_ref_copy_f32_must_not_star_clone` | ✅ tip GREEN (product shape) |
| `wdb344_tip_out_game_core_collision2d_must_not_clone_f32` | ✅ tip GREEN after tip-out/gen sync |
| `wdb346_module_file_copy_f32_field_must_not_emit_clone` | ✅ tip GREEN |
| `wdb346_tip_out_game_core_camera_must_not_clone_f32_fields` | ✅ tip GREEN after tip-out/gen sync |

**Root cause layer:** coercion/encoding — `append_rust_clone` no-ops on `*…`
(IR Deref of `&Copy`); `apply_coercion(Deref)` / `Coercion::Deref` /
`ArgCoercion::Deref` strip trailing `.clone()` before/after `*`.

**What became unnecessary:** `*w1.clone()` / `*h1.clone()` on Copy match payloads;
no new `ir_call_site` peel. Tip-out/gen sync cleared WDB-346 camera field clones.

**Gates:** `CARGO_TARGET_DIR="$(wj cache path)"` →
`cargo test --release --test all -- wdb344_ wdb346_` → **5 passed**;
`cargo test --release -p windjammer --lib append_rust_clone_parenthesizes` → **1 passed**.

**Do not steal:** remaining tip-true REDs (WDB-340/349/353–358/364–366/368/374/383/416 + wave tip_outs).

## P3.696 (2026-10-06) — WDB-423: Copy i32 tuple array index must not `.clone()`

Product `tps_camera` / `fps_camera`: `let (ox, oz) = offsets[(i as usize)].clone()`
because `[(-1, 0), …]` failed element-type inference — `UnaryOp::Neg` had no
`infer_expression_type` arm, so Index emit fell through to unknown→`.clone()`.

| Gate | Status |
|------|--------|
| `wdb423_module_file_copy_tuple_index_must_not_clone` | ✅ tip GREEN (f32 + i32 product shape) |
| `wdb423_tip_out_game_core_offsets_index_must_not_clone` | ✅ tip GREEN after camera multipass regen |

**Root cause layer:** constraint/type inference — `Neg`/`Not` unary keep operand
type so `(-1, 0)` tuples type as Copy and index destructure skips clone.

**What became unnecessary:** `offsets[…].clone()` on Copy `(i32,i32)` / `(f32,…)`
array indexes; no `ir_call_site` peel; Index unknown→clone path no longer hit for
this product shape.

**Gates:** `CARGO_TARGET_DIR=target-agent-tip-p3693` →
`cargo test --release --test all -- wdb423_` → **2 passed**;
`wdb423_ wdb425_ wdb422_ wdb432_` → **8 passed**.

**Do not steal:** remaining tip-out lag / suite triage.

## P3.695 (2026-10-06) — WDB-425: negated range `for dx in -2..3` Copy counter (no `.clone()`)

Product `component_viewer_controls`: `if dx < 0 { -dx } else { dx.clone() }` because
`-2..3` parses as `Unary(Neg, Range(2, 3))`, so range-loop Int32 registration never
ran and auto-clone treated `dx` as a non-Copy move.

| Gate | Status |
|------|--------|
| `wdb425_module_file_copy_i32_abs_else_must_not_clone` | ✅ tip GREEN (product-shape + abs) |
| `wdb425_tip_out_game_core_abs_else_must_not_clone` | ✅ tip GREEN after scene multipass regen |

**Root cause layer:** constraint/type registration — peel `Unary(Neg, Range…)` in
`for_statement_generation` so loop counters register as Copy `i32`; also recognize
negated small int literals in `range_loop_int_counter_type`; skip auto-clone via
`codegen_i32_binding_names`.

**What became unnecessary:** `dx.clone()` / `dz.clone()` on multi-use Copy range
counters in abs if/else and later arith; no `ir_call_site` peel.

**Gates:** `CARGO_TARGET_DIR=target-agent-tip-p3693` →
`cargo test --release --test all -- wdb425_` → **2 passed**.

**Do not steal:** WDB-423 tip-out Copy tuple index (`offsets[i].clone()`); older tip-out lag.

## P3.694 (2026-10-06) — Tip-out regen: WDB-429/432 + Copy-clone lag cluster GREEN

Isolates for WDB-429/432/434–436/438–456 were already tip-GREEN under P3.688–693;
stale `.agent-wip/rel_tip_out` (and game-core `gen/`) still had `.clone()` on Copy
identity sites. Tip `wj` (`target-agent-tip-p3693`) regen of product modules —
no new reconcile peels.

| Gate | Status |
|------|--------|
| `wdb429_tip_out_*` / `wdb432_tip_out_*` | ✅ tip GREEN after tip-out regen |
| `wdb434`–`436` / `438`–`439` tip-out | ✅ tip GREEN |
| `wdb440`–`456` tip-out Copy-clone cluster | ✅ tip GREEN (19 tip filters) |

**Root cause layer:** none (product tip-out lag) — codegen path already correct;
artifact sync only.

**What became unnecessary:** tip-out ❌ rows for WDB-429/432/440–456 (and
434–436/438–439) that were isolate-GREEN / tip-RED solely from stale regen.

**Gates:** `CARGO_TARGET_DIR=target-agent-tip-p3693` →
`cargo test --release --test all -- wdb429_tip wdb432_tip wdb440_tip … wdb456_tip`
→ **19 passed**; `wdb434_tip wdb435_tip wdb436_tip wdb438_tip wdb439_tip` → **5 passed**.

**Do not steal:** remaining older tip-out lag (WDB-340+ cluster etc.) until sweep;
full-suite triage.

## P3.693 (2026-10-06) — WDB-444: Copy unit-enum formal reuse must not `.clone()`

Product `WeatherSystem::set_weather`: after field assigns, reuse into
`weather_to_atmosphere_bundle(weather, intensity)` cloned `weather` because
`finalize_owned_outer_formal_call_arg` / forwarder reuse treated owned Copy
unit-enum formals like non-Copy moves (`&weather` → `weather.clone()`).

| Gate | Status |
|------|--------|
| `wdb444_module_file_copy_unit_enum_formal_assign_must_not_clone` | ✅ tip GREEN — bare `weather` / `intensity` |
| `wdb444_tip_out_game_core_weather_system_enum_must_not_clone` | ✅ tip GREEN after tip-out regen |

**Root cause layer:** signature/registry + temporary reconcile narrow —
`copy_types_registry` / `enum_is_unit_copy` (variant-path peel + collect on
`generate_enum`); early-return in `finalize_owned_outer_formal_call_arg` for
Copy identity; forwarder reuse / demoted / amp-reuse / owned-vec peel skip
`call_arg_is_copy_identity` / unit-enum Copy. Let-binding CamelCase
`Enum::Variant` types as the enum (not the variant path).

**What became unnecessary:** `weather.clone()` on reused Copy unit-enum formals
into owned callees; no new method-name lists.

**Gates:** `CARGO_TARGET_DIR=target-agent-tip-p3693` →
`cargo test --release --test all -- wdb444_` → **2 passed**;
`wdb429_` / `wdb432_` tip-out ✅ after P3.694 regen.

## P3.692 (2026-10-06) — WDB-440: Copy f32 if/else local into struct lit (no `.clone()`)

Product `let mse = if … { sum / count } else { 0.0 }` then
`ComparisonResult { mse: mse, …, pass: … && mse < 0.01 }` cloned because struct-lit
shorthand only treated **formals** as Copy on multi-use — locals were forced to
`mse: mse.clone()`.

| Gate | Status |
|------|--------|
| `wdb440_module_file_copy_f32_local_into_struct_lit_must_not_clone` | ✅ tip GREEN |
| `wdb440_module_file_if_let_f32_into_struct_lit_must_not_clone` | ✅ tip GREEN |
| `wdb440_tip_out_game_core_frame_analysis_mse_must_not_clone` | ✅ tip GREEN |

**Root cause layer:** coercion/encoding — Copy destination field +
`ident_skips_auto_clone_as_copy` for shorthand multi-use (not formals-only);
also record `Type::Float` for if/else float blocks in `local_var_types`.

**What became unnecessary:** `mse.clone()` in frame_analysis ComparisonResult lit;
no `ir_call_site` peel.

**Gates:** `CARGO_TARGET_DIR=target-agent-tip-p3683` →
`cargo test --release --test all -- wdb440_` → **3 passed**.

**Do not steal:** WDB-429/432 tip-out multipass Copy clones (isolate GREEN; tip-out lag).

## P3.691 (2026-10-06) — WDB-414: MutBorrowed helpers must not block owned partial-move `self`

Product `initialize` called `setup_materials` / `build_scene` (`&mut self`) then
`CsgVoxelizer::new(self.scene)`. An early formal path forced `&mut self` whenever any
MutBorrowed sibling was recorded — **before** the WDB-414
`function_partial_moves_self_field_then_assigns_other` exception — so tip emitted
`self.scene.clone()` under `&mut self`.

| Gate | Status |
|------|--------|
| `wdb414_module_file_ctor_must_move_self_field` | ✅ tip GREEN |
| `wdb414_module_file_ctor_after_mut_helpers_must_move_self_field` | ✅ tip GREEN — owned `mut self` + move |
| `wdb414_tip_out_game_core_ctor_must_not_clone_self_scene` | ✅ tip GREEN after tip regen |
| `wdb367_module_file_none_*` + tip-out | ✅ tip GREEN (tip-out/gen synced) |

**Root cause layer:** constraint/self-receiver formal selection — MutBorrowed sibling
delegation must yield to partial-move owned `mut self` (same exception as
`self_receiver_upgrades` path).

**What became unnecessary:** product `self.scene.clone()` under demoted `&mut self`
for rifter/cathedral `initialize`; no new `ir_call_site` peel.

**Gates:** `CARGO_TARGET_DIR=target-agent-tip-p3683` →
`cargo test --release --test all -- wdb414_ wdb367_` → **5 passed**;
`wdb407_module_file_*` / `wdb410_module_file_*` tip-live ✅ (tip-out still product lag).

**Do not steal:** remaining tip-true REDs (WDB-340/349/353–358/364–366/368/374/383/416 + wave); full suite triage.

## P3.690 (2026-10-06) — unit `None` / Copy field / owned-self move (no spurious `.clone()`)

Tip-live isolates still cloned:
- `tiles.push(None.clone())` / `Value::None.clone()` — finalize IR cutover re-cloned
  after peel when `needs_clone_anywhere("None")` (WDB-367).
- `out.push(self.vertices[i].position.clone())` — Copy Vec3 field (WDB-371).
- `Vox::new(self.scene.clone())` with owned `mut self` — emit-owned self treated as
  behind-ref via stale inferred mut-borrow / OwnershipHint::Mut (WDB-414).

| Gate | Status |
|------|--------|
| `wdb367_module_file_none_must_not_emit_clone` | ✅ tip GREEN |
| `wdb371_module_file_copy_vec3_field_must_not_double_clone` | ✅ tip GREEN |
| `wdb414_module_file_ctor_must_move_self_field` | ✅ tip GREEN — `Vox::new(self.scene)` |

**Root cause layer:** coercion/encoding + emit-truth — unit constructors never clone;
Copy field/formal Identity strips `.clone()`; owned `self` emit allows field move.

**What became unnecessary:** finalize re-clone of `None`; Copy index/field `.clone()`;
`self.scene.clone()` under owned `mut self`.

**Gates:** `CARGO_TARGET_DIR=target-agent-tip-p3683` →
`cargo test --release --test all -- wdb367_module_file_none_must_not_emit_clone wdb371_module_file_copy_vec3 wdb414_module_file_ctor_must_move` → **3 passed**.

**Superseded tip-out lag:** P3.691 (helpers demotion + tip-out sync).

## P3.689 (2026-10-06) — owned `String` emit beats Borrowed WJ for lit `.to_string()`

`parse_rows(json: string)` emits `json: String` (body moves into `vec![json]`) but
call-site treated Borrowed WJ `string` as demoted `&str` and skipped
`"lit".to_string()` → E0308. `slot_shared` / Borrowed-skip now require emission
shared-ref (`callee_emits_shared_rust_ref_param` / `emitted_rust_ref_params` /
`param_is_rust_str_ref`), not ownership alone.

Also: `Vec<String>` index into owned field may emit `.to_string()`; gate accepts it.

| Gate | Status |
|------|--------|
| `json_string_field_helper_must_cargo_check` | ✅ tip GREEN — `parse_rows("…".to_string())` |
| `test_vec_string_index_in_struct_needs_clone` | ✅ tip GREEN — `.to_string()` accepted |
| `qs_get_literal_into_demoted_key` / `owned_plus_empty_into_demoted_str` | ✅ tip GREEN (no P3.666 regress) |

**Root cause layer:** signature/emission contract — owned String emit must drive
literal coercion; Borrowed WJ alone must not imply shared-ref skip.

**What became unnecessary:** dual-oracle skip that peeled/blocked `.to_string()` when
emit stayed `String`.

**Gates:** `CARGO_TARGET_DIR=target-agent-tip-p3683` →
`cargo test --release --test all -- json_string_field_helper_must_cargo_check test_vec_string_index_in_struct_needs_clone notes_api_qs_get pretty owned_plus_empty` → **8 passed**.

**Do not steal:** P3.666, P3.676–P3.689.

## P3.688 (2026-10-06) — bare `*_usize` local into Owned usize + i32 vs `.len()`

`Vec::remove(sparse_idx_usize)` re-wrapped already-usize locals whose **names**
end in `_usize` as `(sparse_idx_usize) as usize` (E0605 / double cast). Cause:
`coerce_arg_str_for_usize_formal` treated any string ending in `_usize` as an
embedded `N_usize` literal suffix before honoring `arg_already_usize`.

Separately, `items.len() > i` with `i: i32` emitted `i as usize` because the
WDB-081 “narrow unsigned → cast counter to usize” path incorrectly included
signed `i32` (negative wrap). Prefer casting `.len()` to `i32` peer width
(P3.338); keep u32/u64 → `as usize`.

| Gate | Status |
|------|--------|
| `vec_remove_with_local_usize_cast` / `ownership_field_test::test_vec_remove_usize_variable` | ✅ tip GREEN — bare `sparse_idx_usize` |
| `coerce_usize_formal_keeps_bare_usize_local_named_with_usize_suffix` | ✅ lib GREEN |
| `test_len_compared_to_i32_variable` | ✅ tip GREEN — `(items.len() as i32) > i` |
| `i32_while_len_and_literal_bound` / `i32_loop_arith_and_len_compare` | ✅ tip GREEN (no regress) |

**Root cause layer:** coercion/encoding — usize-formal rewrite must only normalize
true embedded `N_usize` literal suffixes (`strip_embedded` actually changes the
string); signed i32 vs `.len()` uses peer-width cast, not counter→usize.

**What became unnecessary:** `(sparse_idx_usize) as usize` double cast; unsafe
`i as usize` for signed i32 vs `.len()`.

**Gates:** `CARGO_TARGET_DIR=target-agent-tip-p3683` →
`cargo test --release --test all -- test_len_compared_to_i32_variable test_vec_remove_with_local_usize_cast i32_while_len_and_literal i32_loop_arith_and_len` → **4 passed**;
`cargo test --release -p windjammer --lib coerce_usize_formal` → **3 passed**.

**Do not steal:** P3.676–P3.688.

## P3.687 (2026-10-06) — `wj test` Doc-tests E0463 under shared verify cache (eco)

After unit tests **pass**, `cargo test` still runs `Doc-tests windjammer_tests` and
rustdoc fails with `error[E0463]: can't find crate for windjammer_runtime` /
package lib despite `--extern` paths into
`~/Library/Caches/windjammer/cargo-target/verify/…`.

| Gate | Status |
|------|--------|
| eco `wj-csv` unit tests | ✅ 10 passed |
| Doc-tests on shared verify | ❌ intermittent E0463 |
| Same package with isolated `CARGO_TARGET_DIR` | ✅ All tests passed |

**Repro (eco):** concurrent agents + `wj cache prune` / shared verify while
`packages/wj-csv` `$WJ test`. Unit harness green; doc-tests abort → overall exit 1.

**Workaround (eco only):** isolate `CARGO_TARGET_DIR` for the package test run.
**Compiler fix needed?:** harness should skip empty doc-tests or pin rlib paths
against verify GC; not an app reshape.

**Do not steal:** P3.676–P3.687.

## P3.686 (2026-10-05) — method `&mut` passthrough must not stack `&mut`

`Host::run(csr: &mut DenseCsr)` calling `self.take_edges(csr)` emitted
`self.take_edges(&mut csr)` (`&&mut DenseCsr`, E0308). Free-fn passthrough
already bare. IR MutBorrow + finalize mut_ref_slot prefixed despite already-mut formal.

| Gate | Status |
|------|--------|
| `mut_param_method_passthrough_must_not_prefix_shared_amp` | ✅ tip GREEN — `self.take_edges(csr)` |
| `mut_param_passthrough_must_not_prefix_shared_amp` | ✅ tip GREEN — free-fn hold |

**Root cause layer:** coercion/encoding + temporary reconcile narrow —
IR MutBorrow→Identity when `identifier_already_mut_ref`; finalize mut_ref_slot
peels stacked `&mut` on already-mut bindings.

**What became unnecessary:** `&mut csr` into method MutBorrowed formals that already
emit `&mut DenseCsr`.

**Gates:** `CARGO_TARGET_DIR=…/target-agent-tip-p3681` → `mut_param_*passthrough*` — **2 passed**.

**Do not steal:** P3.676–P3.686 (filed).

# Compiler repro queue (dogfooding — do not work around in application code)


## P3.685 (2026-10-05) — TDD WDB-456 (DB agent; no compiler src)

Copy `i32` **local** into untyped `let` must not `.clone()`.

Product `physics/physics_body.rs` check_collision:
```wj
let feet_y = min_y
let head_y = max_y
```
Tip-out emits `let feet_y = min_y.clone()` / `let head_y = max_y.clone()`.
WJ source uses bare `min_y` / `max_y`.

| Gate | Status |
|------|--------|
| WDB-456 MultiFile | ✅ isolate GREEN — `let feet_y = min_y` (no `.clone()`) |
| WDB-456 tip-out | ✅ tip GREEN after tip-out sweep (P3.704/P3.707)

**Root cause layer:** Copy peel / local-let — i32 **local** into untyped `let` must stay bare Copy.

**Why this is a new class:**
- WDB-446 is i32 local into **typed** let (`let mut x: i32 = u.clone()`).
- WDB-437 is i32 **formal** into let.
- WDB-455 is i32 local into **field** assign.
- This is i32 local into **untyped** let.

**What became unnecessary:** rewriting collision probes to avoid local copies.

**Gates:** `cargo test --release --test all --features integration_tests,codegen_tests -- wdb456_` — isolate GREEN / tip RED (2026-10-05).

**Do not steal:** WDB-406/408/411/455–456, P3.508–P3.685, WDB-412–456 (filed).

## P3.684 (2026-10-05) — owned `+ ""` into demoted `&str` formal must borrow

LedgerKit `payload(id + "", ref + "")` where tip demotes `string` formals to
`&str` passes owned `String` temps → E0308. Tip must auto-borrow.

| Gate | Status |
|------|--------|
| `owned_plus_empty_into_demoted_str_formal_must_borrow` | ✅ tip GREEN — fresh tip `22:33` |

**Root cause layer:** call-site / demotion — owned `+ ""` temps into demoted
`&str` formals need IR Borrow. Game tip **18:38** still RED on product
`audit_hash` / `journal_post` bare `String` fields into `&str`.

**What became unnecessary:** dropping `+ ""` at LedgerKit journal/audit call sites
(keep bare args; refresh tip binary past 18:38).

**Gates:** `bug_owned_plus_empty_into_demoted_str_formal_must_borrow_test` — **passed**.

**Do not steal:** P3.178/P3.179, P3.681–P3.683 (filed).

## P3.683 (2026-10-05) — `vec[i]` into owned struct formal must clone

LedgerKit `less(out[j], out[j+1])` moves non-Copy index elems → E0507.
Tip must clone into owned formals (P3.575 is owned-let reuse, not call-arg).

| Gate | Status |
|------|--------|
| `vec_index_into_owned_struct_formal_must_clone` | ✅ tip GREEN — `item_less((out[(j as usize)]).clone(), …)` |

**Root cause layer:** (temporary) reconcile sanitize — `is_copy_scalar_numeric_cast`
used `contains(" as usize")`, so `out[(j as usize)].clone()` was peeled as a
Copy scalar cast (WDB-343 over-apply). Index emit already cloned; terminal
sanitize dropped it. Narrowed to outermost `… as T` only. Also ordered
`ensure_owned` Index clone before Borrowed→shared-ref early return.

**What became unnecessary:** bare `item_less(out[j], out[j+1])` move; product
field-rank compare reshape.

**Gates:** `bug_vec_index_into_owned_struct_formal_must_clone_test` +
`codegen::rust::expression_utilities::tests::append_rust_clone_parenthesizes_casts`
— **passed** (fixture `cargo check` clean).

**Do not steal:** P3.575, P3.682/P3.684 (filed).

## P3.682 (2026-10-05) — `int` formal `==` HTTP status lit must not emit `_i32`

LedgerKit `json_cors_error(status: int)` with `status == 401` emits
`status: i64` vs `401_i32` (E0277). Tip must unify lit peers with the formal.

| Gate | Status |
|------|--------|
| `int_formal_eq_http_status_lit_must_unify_width` | ✅ tip GREEN — `401_i64` peers (fresh tip `22:33`) |

**Root cause layer:** int-width / compare — HTTP status lit peers of `int` formal.
Game tip **18:38** still split on product `http_json` before helper.

**What became unnecessary:** `http_status_eq(status, code)` helper (can drop after tip refresh).

**Gates:** `bug_int_formal_eq_status_lit_must_not_emit_i32_test` — **passed**.

**Do not steal:** WDB-328/P3.403, P3.679–P3.681, P3.683–P3.684 (filed).


## WDB-455 (2026-10-05) — Copy i32 local into field assign must not `.clone()`

Product `csg/scene.rs` `add_*`: `self.root_id = id.clone()` where `let id = self.next_id`.

| Gate | Status |
|------|--------|
| `wdb455_module_file_copy_i32_local_field_assign_must_not_clone` | ✅ isolate GREEN |
| `wdb455_tip_out_game_core_csg_scene_i32_local_field_assign_must_not_clone` | ✅ tip GREEN after tip-out regen (P3.688) — isolate was already clean; stale `rel_tip_out` had `id.clone()` |

**Root cause layer:** tip-live Copy local → field assign already bare (isolate GREEN);
tip-out was stale product lag until `rel_tip_out/csg/scene.rs` regen (P3.688).

**Do not steal:** WDB-393 (i32 formal field), WDB-446 (i32 local typed let), WDB-437 (i32 formal let).

**Gates:** `bug_wdb455_module_file_copy_i32_local_field_assign_must_not_clone_test` → **2 passed**.

## P3.681 (2026-10-05) — `Ok(vec.len())` into `Result<int, _>` must coerce usize→i64

`wj-regex::match_count`:
```wj
match find_all(pattern, text) {
    Ok(hits) => Ok(hits.len()),
    Err(e) => Err(e),
}
```
Tip emits bare `Ok(hits.len())` → `expected i64, found usize` (E0308).
Bare `.len()` as `int` return is already GREEN (`wj-glob::match_count`);
wrapping the same expression in `Ok(...)` loses the coercion.

| Gate | Status |
|------|--------|
| `ok_vec_len_into_result_int_must_coerce_usize` | ✅ tip GREEN — `Ok(items.len() as i64)` |

**Root cause layer:** coercion/encoding — `Some(len)` already cast usize→int for
`Option<int>`; `Ok(len)` into `Result<int, _>` skipped that encode. Unified
payload-ctor cast for `Some`/`Ok` (+ `maybe_cast_usize_to_int_target` Ok peel).

**What became unnecessary:** counting-loop interim in `wj-regex::match_count`.

**Gates:** `cargo test --release --test all -- bug_ok_vec_len_into_result_int_must_coerce_usize`
— **passed**.

**Do not steal:** P3.322, P3.679–P3.680, WDB-452–454 (filed).

## P3.680 (2026-10-05) — TDD WDB-454 (DB agent; no compiler src)

Copy `f32` **local** into **local reassignment** must not `.clone()`.

Product `frame_analysis.rs` histogram:
```wj
let lum = luminance(...)
if lum < min_val { min_val = lum }
```
Tip-out emits `min_val = lum.clone()` / `max_val = lum.clone()`.
WJ source uses bare `lum`.

| Gate | Status |
|------|--------|
| WDB-454 MultiFile | ✅ isolate GREEN — `min_val = lum` (no `.clone()`) |
| WDB-454 tip-out | ✅ tip GREEN after tip-out sweep (P3.704/P3.707)

**Root cause layer:** Copy peel / local-reassign — f32 **local** (not formal) assigned into another local must stay bare Copy.

**Why this is a new class:**
- WDB-452 is f32 **formal** into local reassign (`content_x.clone()`).
- WDB-445 is f32 **formal** into **let**.
- WDB-440 is f32 local into **struct literal**.
- WDB-448 is f32 formal into **field**.
- This is f32 **local** into **local reassignment**.

**What became unnecessary:** rewriting extrema loops to avoid local reassignment.

**Gates:** `cargo test --release --test all --features integration_tests,codegen_tests -- wdb454_` — isolate GREEN / tip RED (2026-10-05).

**Do not steal:** WDB-406/408/411/453–454, P3.508–P3.680, WDB-412–454 (filed).

## P3.679 (2026-10-05) — `while i < 64` literal + substring must unify int/usize

`wj-sha` fixed-width hex scan:
```wj
let mut i = 0
while i < 64 {
    let ch = strings.substring(text, i, i + 1)
    …
}
```
Tip 18:38 emits `while i < 64_usize` with `i: i32`, then
`substring(&text, i, (i + 1) as usize)` → E0308 (start uncast).
Len-driven loops (`while i < strings.len(s)`) already GREEN (P3.300/P3.452);
**literal** end bound still widthsplit. Product uses `strings.chars` +
`contains` hex check until tip greens.

| Gate | Status |
|------|--------|
| `while_lit_bound_substring_int_must_unify_usize` | ✅ tip GREEN (2026-10-08) — `while i < 64_i32` with `i: i32`; substring indexes `as usize` |

**Root cause layer:** constraint/width sync. Let-emit chose `i: i32` / `0_i32` from the literal while-bound, then `reconcile_ambiguous_int_local_after_let` repainted the binding as `usize` because mixed-int inference saw the later substring index. The comparison then suffixed `64_usize`. Emitted `_i32` now wins over that usize repaint, and `expression_produces_usize` honors `codegen_i32_binding_names`.

**What became unnecessary:** treating a stale index-usize mark as the comparison width after the counter was already emitted i32.

**Gates:** `cargo test --release --test all -- while_lit_bound_substring i32_while_compare wdb121_module_file_annotated_usize` — 3 passed.

**Do not steal:** P3.300/P3.315/P3.452/P3.454, P3.671–P3.678 (filed).

## P3.678 (2026-10-05) — HashMap::get identity `"${v}"` let-match must clone

notes-api `notes_config_from_map`: `let s = match map.get(...) { Some(v) => "${v}", … }`
emitted bare `Some(v) => v` (`&String` → E0308). Return-match already cloned (P3.671);
let/`Block{Match}` path misclassified match-arm actual as Owned+Inferred so identity
interp lowered to Identity (move) instead of Clone.

| Gate | Status |
|------|--------|
| `hashmap_get_identity_interp_into_owned_string_must_clone` | ✅ tip GREEN — `Some(v) => v.clone()` |
| `notes_api_product_config_map_get_string_must_clone` | ✅ tip GREEN — product `config.rs` clones |

**Root cause layer:** coercion/encoding + constraint/actual —
identity `"${text}"` always Clone (fresh owned); `Block{Match}` ref bindings no longer
forced into `match_arm_bindings` as Owned; `infer_actual` honors `borrowed_iterator_vars`
for match-arm refs.

**What became unnecessary:** bare move of HashMap::get `&String` into owned `string` let;
narrow Owned+String-only Identity→Clone guard that missed Owned+Inferred.

**Gates:** `CARGO_TARGET_DIR=…/target-agent-tip-p3675` →
`notes_api_product_config_map_get_string_must_clone` + isolate — **2 passed** (2026-10-05).

**Do not steal:** P3.671, P3.676–P3.678, WDB-452/453 (filed).

## P3.677 (2026-10-05) — TDD WDB-453 (DB agent; no compiler src)

Copy `f32` **indexed field** on return must not `.clone()`; product emits
`return self.pass_timings[index].avg_ms.clone()` in gpu_profiler.

| Gate | Status |
|------|--------|
| WDB-453 MultiFile | ✅ isolate GREEN — `return …avg_ms` (no `.clone()`) |
| WDB-453 tip-out | ✅ tip GREEN after tip-out sweep (P3.704/P3.707)

**Root cause layer:** copy / f32-indexed-field-return — Copy `f32` indexed struct fields returned must not auto-clone
(product tip-out lag / multipass path; isolate already correct).

**Why this is a new class:**
- WDB-439 is i32 indexed **tuple** field return.
- WDB-435 is u32 **field** return (non-indexed).
- WDB-346 is f32 field in expr (camera), not indexed return.

**What became unnecessary:** `avg_ms.clone()` in GpuProfiler::timing_avg_ms_at.

**Gates:** `CARGO_TARGET_DIR=…/agent-tdd-wdb435` → `wdb453_` — **1 passed / 1 failed**
(isolate GREEN, tip RED; 2026-10-05).

**Do not steal:** WDB-406/408/411/452–453, P3.508–P3.677, WDB-412–453 (filed).


## P3.676 (2026-10-05) — HashMap `has` `Some(_)` → `matches!` + DEFER DROP spawn

Owned `HashMap` helper:
```wj
pub fn has(map: HashMap<string, string>, key: string) -> bool {
    match map.get(key) {
        Some(_) => true,
        None => false,
    }
}
```
Tip collapses to `matches!(map.get(key), Some(_))` then splices
`// DEFER DROP` + `std::thread::spawn(move || drop(map));` **after** it →
missing `;` / function returns `()` (E0308). Binding `Some(v)` keeps a real
`match` and cargo-checks. Product: `wj-dotenv::has`. Do not reshape with
`Some(v)` peels — tip must green `Some(_)` / `matches!` + defer-drop.

| Gate | Status |
|------|--------|
| `hashmap_owned_get_some_wildcard_bool_must_not_defer_drop_after_matches` | ✅ tip GREEN — `matches!(map.get…)` only (no DEFER DROP) |
| `hashmap_owned_get_helper_must_not_inject_mid_match_defer_drop` | ✅ tip GREEN — P3.267 hold |

**Root cause layer:** temporary reconcile (defer-drop wrap) —
`function_level_tail_line_index` treated single-line expression bodies as
insert-after-end so the variable-in-tail check missed `map`; P3.267 `match `
skip did not cover `matches!(`. Skip when `.get` + `matches!`/`match`; return
original body when all opts skipped.

**What became unnecessary:** `matches!` + post-expr `thread::spawn(drop(map))`;
package `Some(v)` peels for `has`.

**Gates:** `CARGO_TARGET_DIR=…/target-agent-tip-p3676` → P3.676 + P3.267 —
**2 passed** (2026-10-05).

**Do not steal:** P3.267/P3.278, P3.671–P3.678 (filed).

## P3.675 (2026-10-05) — TDD WDB-452 (DB agent; no compiler src)

Copy `f32` **formal** into **local reassignment** must not `.clone()`; product emits
`current_x = content_x.clone()` in ui/layout layout_row.

| Gate | Status |
|------|--------|
| WDB-452 MultiFile | ✅ isolate GREEN — `current_x = content_x` (no `.clone()`) |
| WDB-452 tip-out | ✅ tip GREEN after tip-out sweep (P3.704/P3.707)

**Root cause layer:** copy / f32-formal-local-reassign — Copy `f32` formals reassigned into locals must not auto-clone
(product tip-out lag / multipass path; isolate already correct).

**Why this is a new class:**
- WDB-445 is f32 **formal** into **let**.
- WDB-448 is f32 **formal** into **field** assign.
- WDB-393 is i32 **formal** assign.

**What became unnecessary:** `content_x.clone()` in Layout::layout_row align arms.

**Gates:** `CARGO_TARGET_DIR=…/agent-tdd-wdb435` → `wdb452_` — **1 passed / 1 failed**
(isolate GREEN, tip RED; 2026-10-05).

**Do not steal:** WDB-406/408/411/451–452, P3.508–P3.675, WDB-412–452 (filed).

## P3.674 (2026-10-05) — shared-ref emit beats AST-owned peel; WAL owned temps

P3.647 owned-slot peel treated WJ AST bare `Value`/`Vec` as owned even when codegen
emitted `&T` (`apply_patch_put(value: &Value)`), rewriting IR `&value` → `value.clone()`.
Separately, `maybe_borrow_vec_or_helper` ORed raw `forwarding_borrow_params` and forced
`&vec![…]` into keep-owned WAL `append_put(Vec)` formals; the owned peel then emitted
`.clone()` on temporaries (E0308 / wasteful).

| Gate | Status |
|------|--------|
| `dogfood_lsm_store_apply_patch_asymmetric_coercion` | ✅ tip GREEN — `apply_patch_put(…, &value)` |
| `dogfood_wal_segment_cross_crate_append_put_borrows_vec_literal` | ✅ tip GREEN — owned `vec![…]` / `encode_int64(…)` (no `&` / no temp `.clone()`) |
| `test_builder_pattern_self_clone_when_owned_method` | ✅ tip GREEN — `&Uniform` call-site borrow |
| `test_shared_ref_quest_state_from_values` | ✅ tip GREEN — field-read / `quest.state()` gate |
| `reused_owned_vec_formal_must_not_reborrow_recursive` | ✅ tip GREEN — gate scoped to `evaluate_node` formal (outer `evaluate(Vec)` no longer false-RED) |

**Root cause layer:** signature/emission oracle + temporary reconcile narrow —
`shared_ref_emit_slot` gates the P3.647 AST-owned peel; `maybe_borrow` drops the raw
`forwarding_borrow` dual-oracle (uses `call_site_needs_shared_ref_at_emit` only).

**What became unnecessary:** `value.clone()` into `&Value`; `&vec![].clone()` dual-oracle
into owned WAL formals; quest gate false-RED on trivial getter→field lower.

**Gates:**
- `cargo test --release --test all -- dogfood_wal_segment_cross_crate_append_put_borrows_vec_literal dogfood_lsm_store_apply_patch_asymmetric_coercion test_shared_ref_quest_state_from_values test_builder_pattern_self_clone_when_owned_method bug_thread_spawn_closure_must_not_be_ref_test bug_mpsc_sync_channel_boundary_signature_test test_passthrough_to_borrowed_param` → **9 passed**
- No-reg: `dogfood_wal_segment_vec_literal dogfood_lsm_engine_apply_writes auto_mut_borrow_arg bug_demoted_vec_param owned_vec_reuse` → **7 passed**; `cargo test --release --lib -- forwarding_borrow` → **4 passed**

**Do not steal:** P3.647 owned-Vec clone path; P3.672 MutBorrowed; P3.456 keep-owned; P3.589/627 forwarding.

**Full suite (2026-10-05):** host OOM/thrash blocked `cargo test --release --test all` completion
(~16 Gi RAM, &lt;5k free pages; suite died ~275–302 oks). Focused tip-live + no-reg gates above are GREEN.
Re-run full suite when the machine has headroom.



## P3.673 (2026-10-05) — implicit-self passthrough must demote `item_id: &str`

P3.647c truncate used `func.parameters.len()` which omits implicit `self`, so
Self-aligned `emitted_rust_ref_params` never recorded `&str` for
`Merchant::has_item(item_id)` → stayed `String` (passthrough RED). Explicit
`self` in the AST stayed GREEN.

| Gate | Status |
|------|--------|
| `test_passthrough_to_borrowed_param` | ✅ tip GREEN — `has_item(&self, item_id: &str)` |
| `test_method_passthrough_convergence` | ✅ tip GREEN |

**Root cause layer:** signature — Self-aligned registry length / emitted-ref sync
for methods whose WJ AST omits `self`; forwarding_borrow_params Self-padded.

**What became unnecessary:** requiring explicit `self` in `.wj` for passthrough
`&str` demotion.

**Gates:**
- `cargo test --release --test all -- test_passthrough_to_borrowed_param test_method_passthrough_convergence owned_vec_formal notes_api_product_qs_get_query_must_not_format_temp` → **12 passed**

**Do not steal:** P3.647c / P3.672.

## P3.672 (2026-10-05) — auto_mut `&mut Vec` must not become `buf.clone()` after P3.647

P3.647 IR-cutover clone path treated WJ AST bare `Vec` as an owned slot even when
codegen emitted `&mut Vec`, rewriting `self.fill(&mut buf)` → `self.fill(buf.clone())`
(P3.312 regression).

| Gate | Status |
|------|--------|
| `auto_mut_borrow_arg_test::test_local_var_passed_to_mut_param_gets_mut_borrow` | ✅ tip GREEN — `self.fill(&mut buf)` |

**Root cause layer:** (temporary) reconcile narrowed — MutBorrowed / `MutableReference`
slots exit before the owned-Vec clone peel; AST bare `Vec` no longer overrides live
`&mut` contracts.

**What became unnecessary:** `buf.clone()` into `&mut Vec` formals after P3.647.

**Gates:**
- `cargo test --release --test all -- auto_mut_borrow_arg_test::test_local_var_passed_to_mut_param_gets_mut_borrow owned_vec_formal notes_api_product_qs_get_query_must_not_format_temp json_get_owned_value_must_auto_borrow_product` → **11 passed**

**Do not steal:** P3.647 owned-Vec clone path; P3.312/P3.671.

## P3.671 (2026-10-05) — notes-api qs_get query must not `format!("{}", query)`

Product `wj-notes-api` `list_notes_for_query` uses `"${query}"` / `"${pattern}"`
identity interpolation for reuse into owned path-dep formals. Tip emitted
`{ let _temp0 = format!("{}", query); qs_get(_temp0, "q") }`.

| Gate | Status |
|------|--------|
| `notes_api_product_qs_get_query_must_not_format_temp` | ✅ tip GREEN — game tip **18:38** `qs_get(query.clone(), "q"/"limit")` (no `format!`) |
| `identity_string_interpolation_into_owned_formal_must_not_format` | ✅ tip GREEN |

**Root cause layer:** coercion/encoding — identity `format!("{}", text)` on
Windjammer text lowers to owned `String` (Clone / ToOwnedString).

**What became unnecessary:** format-temp hoist for identity `"${s}"` on string;
no app reshape.

**Gates:** tip `…/windjammer-game/.cargo-target-wj/release/wj` **18:38** product
transpile `qs_get(query.clone(), …)` at `q`/`limit` (2026-10-05 eco dogfood).

**Do not steal:** P3.666, P3.486/P3.489, P3.642, P3.660, P3.669–P3.670, P3.672–P3.676 (filed).

## P3.670 (2026-10-04) — TDD WDB-451 (DB agent; no compiler src)

Copy `i64` **formal** into field assign must not `.clone()`; product emits
`self.last_poll_time = current_time.clone()` in live_reload poll.

| Gate | Status |
|------|--------|
| WDB-451 MultiFile | ✅ isolate GREEN — `self.last_poll_time = current_time` (no `.clone()`) |
| WDB-451 tip-out | ✅ tip GREEN after tip-out sweep (P3.704/P3.707)

**Root cause layer:** copy / i64-formal-field-assign — Copy `i64` formals assigned into fields must not auto-clone
(product tip-out lag / multipass path; isolate already correct).

**Why this is a new class:**
- WDB-393 is i32 **formal** into field assign.
- WDB-448 is f32 **formal** into field assign.
- WDB-449 is bool **formal** into field assign.
- WDB-441 is i64 **local** into index assign.

**What became unnecessary:** `current_time.clone()` in LiveReloadSystem::poll.

**Gates:** `CARGO_TARGET_DIR=…/agent-tdd-wdb435` → `wdb451_` — **1 passed / 1 failed**
(isolate GREEN, tip RED; 2026-10-04).

**Do not steal:** WDB-406/408/411/450–451, P3.508–P3.670, WDB-412–451 (filed).

## P3.669 (2026-10-04) — json-util merge overlay loop must not move owned Value

Product `wj-json-util::merge_values`: E0382 — `take_field(overlay, key)` in a
while-loop where `take_field(value: Value, …)` stayed owned (body only
`json.get`). Tip cloned `out` but moved `overlay`.

| Gate | Status |
|------|--------|
| isolate `json_merge_overlay_loop_reuse_must_cargo_check` | ✅ tip GREEN — `take_field(value: &Value)` |
| product `json_util_product_merge_values_must_cargo_check` | ✅ tip GREEN — demoted formal + `&overlay` |

**Root cause layer:** signature — `pub use serde_json::Value` was exported but not
registered non-Copy; empty WJ `struct Value {}` won Copy inference and blocked
borrow-delegation (`is_type_copy(Value)`). Multipass also resolved MethodCall
`json.get` as bare `get` Owned stub, so demotion missed the `json::get` runtime
baseline until module-qualified `runtime_std_param_needs_auto_borrow_resolved`.

**What became unnecessary:** overlay move / loop peels; no new `ir_call_site`
reconcile. Payload-store false positive on bare `get` Owned stub narrowed via
runtime-std key.

**Gates:**
- `cargo test --release --test all --features integration_tests -- json_merge_overlay_loop_reuse` → **2 passed** (isolate + product)
- `cargo test --release -p windjammer --lib -- csv_wj_name_maps_to_csv_mod_rust_stem` → **ok** (Value non-Copy)
- Related: `json_is_array_len_owned_value_multipass` → **passed** (P3.668 hold)

**Do not steal:** P3.661/P3.668, P3.666–P3.668, WDB-448–450 (filed).

## P3.668 (2026-10-04) — multipass `json::is_array`/`len` must not `&v.clone()`

Multipass `json.is_array(v)` / `json.len(v)` on owned `Value` emitted
`&v.clone()` into `&Value` formals (single-file already `&v`). WJ owned stubs
made auto_clone treat args as Move; runtime-std baseline is Borrowed.

| Gate | Status |
|------|--------|
| `json_get_owned_value_multipass_must_cargo_check` | ✅ tip GREEN — `is_array(&v)` / `len(&v)` |
| `json_is_array_len_owned_value_multipass_must_cargo_check` | ✅ tip GREEN — `is_array(&root)` / `len(&root)` |

**Root cause layer:** constraint/reuse — `auto_clone` Call/MethodCall arg usage
must resolve `json::is_array` via runtime-std borrow baseline (not WJ Owned stub)
so reuse is Read, not Move→`.clone()`.

**What became unnecessary:** `&v.clone()` / `&root.clone()` at shared `&Value` slots;
no new `ir_call_site` peel.

**Gates:**
- `CARGO_TARGET_DIR=…/.cargo-target-p3642-json cargo test --release --test all --features integration_tests -- json_get_owned_value_multipass_must_cargo_check json_is_array_len_owned_value_multipass_must_cargo_check reused_owned_string_into_owned_formal_must_clone_first_use` → **3 passed**

**Do not steal:** P3.666 notes qs_get (other agent WIP); WDB tip-out Copy-clone lag.

## P3.667 (2026-10-04) — TDD WDB-450 (DB agent; no compiler src)

Copy `u32` **formal** into **indexed field** assign must not `.clone()`; product emits
`self.chunks[i].gpu_buffer_id = buffer_id.clone()` in chunk_manager mark_uploaded.

| Gate | Status |
|------|--------|
| WDB-450 MultiFile | ✅ isolate GREEN — `chunks[…].gpu_buffer_id = buffer_id` (no `.clone()`) |
| WDB-450 tip-out | ✅ tip GREEN after tip-out sweep (P3.704/P3.707)

**Root cause layer:** copy / u32-formal-indexed-field-assign — Copy `u32` formals assigned into indexed struct fields must not auto-clone
(product tip-out lag / multipass path; isolate already correct).

**Why this is a new class:**
- WDB-447 is u32 formal into **element** assign (`tiles[i] = tile_id`).
- WDB-376 is indexed **read** of `.buffer_id.clone()`.
- WDB-441 is i64 **local** into index assign.

**What became unnecessary:** `buffer_id.clone()` in ChunkManager::mark_uploaded.

**Gates:** `CARGO_TARGET_DIR=…/agent-tdd-wdb435` → `wdb450_` — **1 passed / 1 failed**
(isolate GREEN, tip RED; 2026-10-04).

**Do not steal:** WDB-406/408/411/449–450, P3.508–P3.667, WDB-412–450 (filed).


## P3.666 (2026-10-04) — notes-api qs_get literal key must not `.to_string()` (P3.486 regression)

Product `wj-notes-api` tip shared 20:06 / `$WJ test`: E0308 —
`qs_get(query, "pretty".to_string())` (and `encoding` / `q` / `limit`) into
path-dep `wj_querystring::get` demoted `key: &str`. P3.486 product gate was
GREEN; tip regressed to own string literals again. Do not reshape notes-api.
Gate asserts extended for `q` / `limit`.

| Gate | Status |
|------|--------|
| `notes_api_product_qs_get_literal_must_not_string_from` | ✅ tip GREEN — tip shared 21:42: `qs_get(query, "pretty")` / `"encoding"` / `"q"` / `"limit"` (no `.to_string()` / `String::from`) |
| isolate `qs_get_literal_into_demoted_key_must_not_string_from` | ✅ tip GREEN |

**Root cause layer:** signature / path-dep ABI → call-site — path-dep `get` metadata is
`Owned`+`Borrowed` / Rust `key: &str`; tip 21:42 no longer emits owned key literals.

**Why this is a new class:**
- P3.486 greened the same product sites; tip regressed after P3.642/P3.660 tip churn; tip 21:42 restored.
- Distinct from owned `query` first-arg temps (`format!("{}", query)` on some sites — still present on `q`/`limit`).

**What became unnecessary:** reshaping `query_wants_pretty` / list filters with bare keys.

**Gates:** tip shared `…/cargo-target/shared/release/wj` **21:42** — product transpile
`qs_get(…, "pretty"|"encoding"|"q"|"limit")` bare; cargo `--test all` P3.666 filters **3/3 GREEN**.

**Do not steal:** P3.486/P3.489, P3.642, P3.660, WDB-448/449, P3.508–P3.665 (filed).

## P3.665 (2026-10-04) — TDD WDB-449 (DB agent; no compiler src)

Copy `bool` **formal** into field assign must not `.clone()`; product emits
`self.enabled = enabled.clone()` / `self.is_enabled = enabled.clone()` in UI widgets.

| Gate | Status |
|------|--------|
| WDB-449 MultiFile | ✅ isolate GREEN — `self.enabled = enabled` (no `.clone()`) |
| WDB-449 tip-out | ✅ tip GREEN after tip-out sweep (P3.704/P3.707)

**Root cause layer:** copy / bool-formal-field-assign — Copy `bool` formals assigned into fields must not auto-clone
(product tip-out lag / multipass path; isolate already correct).

**Why this is a new class:**
- WDB-393 is i32 **formal** into field assign.
- WDB-448 is f32 **formal** into field assign.
- WDB-444 is unit-enum **formal** assign.

**What became unnecessary:** `enabled.clone()` in Slider/Button::set_enabled.

**Gates:** `CARGO_TARGET_DIR=…/agent-tdd-wdb435` → `wdb449_` — **1 passed / 1 failed**
(isolate GREEN, tip RED; 2026-10-04).

**Do not steal:** WDB-406/408/411/448–449, P3.508–P3.665, WDB-412–449 (filed).


## P3.664 (2026-10-04) — TDD WDB-448 (DB agent; no compiler src)

Copy `f32` **formal** into field assign must not `.clone()`; product emits
`self.delta_time = actual_dt.clone()` in game_loop update.

| Gate | Status |
|------|--------|
| WDB-448 MultiFile | ✅ isolate GREEN — `self.delta_time = actual_dt` (no `.clone()`) |
| WDB-448 tip-out | ✅ tip GREEN after tip-out sweep (P3.704/P3.707)

**Root cause layer:** copy / f32-formal-field-assign — Copy `f32` formals assigned into fields must not auto-clone
(product tip-out lag / multipass path; isolate already correct).

**Why this is a new class:**
- WDB-393 is i32 **formal** into field assign.
- WDB-445 is f32 **formal** into let.
- WDB-440 is f32 into **struct lit**.

**What became unnecessary:** `actual_dt.clone()` in FrameTimer::update.

**Gates:** `CARGO_TARGET_DIR=…/agent-tdd-wdb435` → `wdb448_` — **1 passed / 1 failed**
(isolate GREEN, tip RED; 2026-10-04).

**Do not steal:** WDB-406/408/411/447–448, P3.508–P3.664, WDB-412–448 (filed).

## P3.663 (2026-10-04) — TDD WDB-447 (DB agent; no compiler src)

Copy `u32` **formal** into indexed assign must not `.clone()`; product emits
`self.tiles[i] = tile_id.clone()` in ffi_tilemap clear.

| Gate | Status |
|------|--------|
| WDB-447 MultiFile | ✅ isolate GREEN — `self.tiles[…] = tile_id` (no `.clone()`) |
| WDB-447 tip-out | ✅ tip GREEN after tip-out sweep (P3.704/P3.707)

**Root cause layer:** copy / u32-formal-index-assign — Copy `u32` formals assigned into Vec slots must not auto-clone
(product tip-out lag / multipass path; isolate already correct).

**Why this is a new class:**
- WDB-441 is i64 **local** into index assign.
- WDB-442 is u32 **local** into let.
- WDB-446 is i32 **local** into typed let.

**What became unnecessary:** `tile_id.clone()` in tilemap clear loops.

**Gates:** `CARGO_TARGET_DIR=…/agent-tdd-wdb435` → `wdb447_` — **1 passed / 1 failed**
(isolate GREEN, tip RED; 2026-10-04).

**Do not steal:** WDB-406/408/411/446–447, P3.508–P3.663, WDB-412–447 (filed).


## P3.662 (2026-10-04) — TDD WDB-446 (DB agent; no compiler src)

Copy `i32` **local** into typed `let` must not `.clone()`; product emits
`let mut slice_count: i32 = gd.clone()` / `let mut x: i32 = u.clone()` in meshing.

| Gate | Status |
|------|--------|
| WDB-446 MultiFile | ✅ isolate GREEN — `let mut slice_count: i32 = gd` / `let mut x: i32 = u` |
| WDB-446 tip-out | ✅ tip GREEN after tip-out sweep (P3.704/P3.707)

**Root cause layer:** copy / i32-local-typed-let — Copy `i32` locals into typed lets must not auto-clone
(product tip-out lag / multipass path; isolate already correct).

**Why this is a new class:**
- WDB-437 is i32 **formal** into let.
- WDB-442 is u32 **local** into let.
- WDB-445 is f32 **formal** into let.

**What became unnecessary:** `gd.clone()` / `u.clone()` in voxel greedy meshing.

**Gates:** `CARGO_TARGET_DIR=…/agent-tdd-wdb435` → `wdb446_` — **1 passed / 1 failed**
(isolate GREEN, tip RED; 2026-10-04).

**Do not steal:** WDB-406/408/411/445–446, P3.508–P3.662, WDB-412–446 (filed).


## P3.661 (2026-10-04) — product wj-json-util owned Value into json::get must auto-borrow

Product `wj-json-util` `take_field` / `child_at`: tip emits `json::get(value, &key)`
while runtime wants `&Value` → E0308. `json::get_index(&value, …)` already borrows;
path_set sites emit `json::get(&out, &head)`.

| Gate | Status |
|------|--------|
| `json_get_owned_value_must_auto_borrow_product` | ✅ tip GREEN — demoted `value: &Value` + bare `json::get(value, …)` (P3.669 hold; gate updated 2026-10-05) |

**Root cause layer:** call-site coercion — owned `Value` into demoted `&Value`
stdlib formals must auto-borrow (signature-driven), including free-fn `json::get`.
After P3.669, helpers demote to `&Value` so bare `value` at `json::get` is correct;
gate rejects only owned `value: Value` + bare `json::get(value,`.

**Why this is a new class:**
- `json_get_index_owned_value_multipass` covers `get_index`.
- P3.661 is `json::get` on product helpers (inconsistent with get_index / path_set).

**What became unnecessary:** manual `&value` / reshape json-util; stale gate that
required `&value` after demotion.

**Gates:** tip dogfood wj-json-util GREEN.
- `cargo test --release --test all -- json_get_owned_value_must_auto_borrow_product` → **ok**

**Do not steal:** P3.654–660 (filed).

## P3.660 (2026-10-04) — MutexGuard HashMap String key must still borrow after P3.649–651

Regression: tip `e6aeefa3` fixed Copy-index over-borrow (P3.649–651 GREEN) but
stopped borrowing owned `String` keys into `HashMap::get` through `MutexGuard`.
Product `wj-sync` `shared_map_get` emits `g.get(key)` → E0308. Isolate
`mutex_guard_hashmap_string_key_must_borrow` (P3.576) tip RED.

| Gate | Status |
|------|--------|
| `mutex_guard_hashmap_string_key_must_borrow` | ✅ tip GREEN — `g.get(&key)` / `contains_key(&key)` |
| `sync_shared_map_get_must_borrow_key_product` | ✅ tip GREEN — product companion |

**Root cause layer:** signature — `Ok(g) => g.get(key)` with `rt=None` resolved to
`Vec::get` (Owned usize) after P3.649–651; HashMap bridge must treat that homonym
as poisoned for wrapper/unknown receivers (keep real `Vec::get` when receiver is `Vec`).

**Why this is a new class:**
- P3.649–651 are Copy `usize`/`i64` indices that must NOT borrow.
- P3.660 is owned `String` HashMap keys that must STILL borrow (`get(&key)`).

**What became unnecessary:** SharedMap-name peels alone without fixing Vec::get homonym.

**Gates:**
- `cargo test --release --test all --features integration_tests,codegen_tests -- mutex_guard_hashmap_string_key_must_borrow toml_hashmap_get_demoted_str_key_must_not_double_borrow_product sync_shared_map_get_must_borrow_key_product` → GREEN

**Do not steal:** P3.576/P3.288, P3.649–659 (filed).

## P3.659 (2026-10-04) — TDD WDB-445 (DB agent; no compiler src)

Copy `f32` formal into `let` must not `.clone()`; product emits
`let mut x = start_x.clone()` in vegetation scatter.

| Gate | Status |
|------|--------|
| WDB-445 MultiFile | ✅ isolate GREEN — `let mut x = start_x` (no `.clone()`) |
| WDB-445 tip-out | ✅ tip GREEN after tip-out sweep (P3.704/P3.707)

**Root cause layer:** copy / f32-formal-let — Copy `f32` formals bound into locals must not auto-clone
(product tip-out lag / multipass path; isolate already correct).

**Why this is a new class:**
- WDB-437 is i32 **formal** into let.
- WDB-442 is u32 **local** into let.
- WDB-440 is f32 into **struct lit**.

**What became unnecessary:** `start_x.clone()` in terrain vegetation scatter loops.

**Gates:** `CARGO_TARGET_DIR=…/agent-tdd-wdb435` → `wdb445_` — **1 passed / 1 failed**
(isolate GREEN, tip RED; 2026-10-04).

**Do not steal:** WDB-406/408/411/444–445, P3.508–P3.659, WDB-412–445 (filed).

## P3.658 (2026-10-04) — demoted `&str` HashMap key must not `get(&key)`

Product `wj-toml` `get`: tip demotes `key` to `&str` then emits `map.get(&key)`
→ E0277 (`String: Borrow<&str>`). Need `get(key)` when already `&str`, or keep
owned `String` + single borrow.

| Gate | Status |
|------|--------|
| `toml_hashmap_get_demoted_str_key_must_not_double_borrow_product` | ✅ tip GREEN — product 17 + cargo 1/1 (2026-10-04 tip 17:41) |

**Root cause layer:** map-key borrow + demotion — do not add `&` when the key
formal/local is already `&str`.

**Why this is a new class:**
- P3.654–657 are owned String into demoted `&str` stdlib formals (missing borrow).
- P3.658 is demoted `&str` key into HashMap::get with an *extra* borrow.

**What became unnecessary:** reshaping toml `get` / avoiding demotion.

**Gates:** tip dogfood wj-toml GREEN.
- `cargo test --release --test all --features integration_tests,codegen_tests -- toml_hashmap_get_demoted_str_key_must_not_double_borrow_product`

**Do not steal:** P3.654–657 (filed).

## P3.657 (2026-10-04) — product wj-mime owned args into mime must auto-borrow

Product `wj-mime` `is_text`/`is_image`/`is_audio`/`is_video`: tip keeps owned
`String` formals and emits bare `mime::is_*(mime_type)` → E0308. `from_*` may
already demote.

| Gate | Status |
|------|--------|
| `mime_stdlib_owned_args_must_auto_borrow_product` | ✅ tip GREEN — product 12 + cargo 1/1 (2026-10-04 tip 18:24) |

**Root cause layer:** call-site coercion — owned String into demoted mime `&str`
formals must auto-borrow (signature-driven).

**Why this is a new class:**
- P3.654 crypto / P3.655 regex — same layer, different runtime modules.
- P3.657 is mime predicate thin-wraps (product).

**What became unnecessary:** `.as_str()` / reshape mime package.

**Gates:** tip dogfood wj-mime GREEN.
- `cargo test --release --test all --features integration_tests,codegen_tests -- mime_stdlib_owned_args_must_auto_borrow_product`

**Do not steal:** P3.654–656 (filed).

## P3.656 (2026-10-04) — TDD WDB-444 (DB agent; no compiler src)

Copy unit-enum formals into field assign / struct lit must not `.clone()`;
product emits `weather.clone()` / `intensity.clone()` in weather_system.

| Gate | Status |
|------|--------|
| WDB-444 MultiFile | ✅ isolate GREEN — bare `weather` / `intensity` (no `.clone()`) |
| WDB-444 tip-out | ✅ tip GREEN (P3.693) — tip-out regen + Copy finalize/forwarder skip |

**Root cause layer:** copy / unit-enum — Copy unit enums as formals/locals must not auto-clone into assigns
(fixed P3.693: registry + finalize early-return + forwarder Copy skip).

**Why this is a new class:**
- WDB-384/392 are `Direction::Variant.clone()` **path** exprs.
- WDB-440 is f32 into **struct lit**.
- WDB-437 is i32 formal into **let**.

**What became unnecessary:** `weather.clone()` / `intensity.clone()` in WeatherSystem.

**Gates:** `CARGO_TARGET_DIR=target-agent-tip-p3693` → `wdb444_` — **2 passed** (P3.693).

**Do not steal:** WDB-429/432 tip-out (isolate GREEN).

## P3.655 (2026-10-04) — product wj-regex owned args into regex must auto-borrow

Product `wj-regex` thin wrappers: tip keeps `pattern: String, text: String` and
emits bare `regex::is_match(pattern, text)` (also find/find_all/split) while
runtime formals are `&str` → E0308. `replace`/`escape` may already demote.

| Gate | Status |
|------|--------|
| `regex_stdlib_owned_args_must_auto_borrow_product` | ✅ tip GREEN — product 9 + cargo 1/1 (2026-10-04 tip 18:24) |

**Root cause layer:** call-site coercion — owned String formals into demoted
stdlib/runtime `&str` formals must auto-borrow (signature-driven).

**Why this is a new class:**
- P3.645 is csv `parse`/`write`.
- P3.654 is crypto `verify_password`.
- P3.655 is regex module multi-fn thin-wrap product.

**What became unnecessary:** `.as_str()` / reshape regex package.

**Gates:** tip dogfood wj-regex GREEN.
- `cargo test --release --test all --features integration_tests,codegen_tests -- regex_stdlib_owned_args_must_auto_borrow_product`

**Do not steal:** P3.645–654 (filed).

## P3.654 (2026-10-04) — product wj-hash owned args into crypto must auto-borrow

Product `wj-hash` `verify_password`: tip keeps owned `String` formals and emits
`crypto::verify_password(password, hash)` while runtime wants `&str` → E0308.
`hash_password` may already demote.

| Gate | Status |
|------|--------|
| `hash_crypto_owned_args_must_auto_borrow_product` | ✅ tip GREEN — `crypto::verify_password(&password, &hash)` (2026-10-04) |
| `crypto_verify_password_module_method_must_auto_borrow` | ✅ isolate GREEN |

**Root cause layer:** call-site coercion — owned String into demoted crypto
`&str` formals must auto-borrow (signature-driven). IR `call_sites` skips finalize
ownership match; layered WJ owned `string` stubs + peel/strip dropped `&`.
Terminal module `::` re-apply after strip restores runtime-std borrows; do not
treat inferred-borrowed owned `String` as `identifier_already_ref`.

**Why this is a new class:**
- P3.645 is csv stdlib.
- P3.654 is crypto bcrypt verify (product thin-wrap inconsistency vs hash_password).

**What became unnecessary:** `.as_str()` / reshape hash package.

**Gates:** tip dogfood wj-hash GREEN.
- `cargo test --release --test all --features integration_tests,codegen_tests -- hash_crypto_owned_args_must_auto_borrow_product`
- `cargo test --release --test all --features integration_tests,codegen_tests -- crypto_verify_password_module_method`

**Do not steal:** P3.645–653 (filed).

## P3.653 (2026-10-04) — TDD WDB-443 (DB agent; no compiler src)

Copy `i32` formals into HashMap key tuple must not `.clone()`; product emits
`g_score.insert((start_x.clone(), start_y.clone()), …)`.

| Gate | Status |
|------|--------|
| WDB-443 MultiFile | ✅ isolate GREEN — `insert((start_x, start_y), …)` (no `.clone()`) |
| WDB-443 tip-out | ✅ tip GREEN after tip-out sweep (P3.704/P3.707)

**Root cause layer:** copy / hashmap-key-tuple — Copy formals as HashMap key tuple elems must not auto-clone
(product tip-out lag / multipass path; isolate already correct).

**Why this is a new class:**
- WDB-438 is tuple lit into **Vec::push**.
- WDB-437 is formal into **let**.
- WDB-441 is i64 into **index assign**.

**What became unnecessary:** `start_x.clone()` / `nx.clone()` in astar g_score keys.

**Gates:** `CARGO_TARGET_DIR=…/agent-tdd-wdb435` → `wdb443_` — **1 passed / 1 failed**
(isolate GREEN, tip RED; 2026-10-04).

**Do not steal:** WDB-406/408/411/442–443, P3.508–P3.653, WDB-412–443 (filed).

## P3.652 (2026-10-04) — TDD WDB-442 (DB agent; no compiler src)

Copy `u32` local into `let` must not `.clone()`; product emits
`let mut step = budget.clone()` in async_loader pump.
(Prior P3.646 slot collision with HashMap::values — renumbered.)

| Gate | Status |
|------|--------|
| WDB-442 MultiFile | ✅ isolate GREEN — `let mut step = budget` (no `.clone()`) |
| WDB-442 tip-out | ✅ tip GREEN after tip-out sweep (P3.704/P3.707)

**Root cause layer:** copy / local-let — Copy `u32` locals bound into locals must not auto-clone
(product tip-out lag / multipass path; isolate already correct).

**Why this is a new class:**
- WDB-437 is i32 **formal** into let.
- WDB-441 is i64 into **index assign**.
- WDB-440 is f32 into **struct lit**.

**What became unnecessary:** `budget.clone()` in world async_loader pump.

**Gates:** `CARGO_TARGET_DIR=…/agent-tdd-wdb435` → `wdb442_` — **1 passed / 1 failed**
(isolate GREEN, tip RED; 2026-10-04).

**Do not steal:** WDB-406/408/411/441–442, P3.508–P3.651, WDB-412–442 (filed).

## P3.651 (2026-10-04) — owned i64 method formal must not receive `&99`

Product `wj-notes-api` `store.get(99)`: tip emits `get(&99_i64)` while formal is
`id: i64` → E0308.

| Gate | Status |
|------|--------|
| `owned_i64_method_formal_must_not_receive_ref_literal` | ✅ tip GREEN — cargo 3/3 with P3.649–650 (2026-10-04 tip e6aeefa3) |

**Root cause layer:** call-site ownership — Copy integer literals into owned
integer formals must not auto-borrow. P3.635 map-key belt treated every `get`
as `HashMap::get` (`&K`), including user `Store::get(id: i64)`.

**Fix:** Do not replace owned-Copy `get` formals / non-map receivers with
`HashMap::get`; peel `&N` into owned Copy slots on Call(FieldAccess).

**Why this is a new class:**
- P3.649/650 are `&usize` into slice get / Vec::remove.
- P3.651 is `&i64` literal into owned `i64` method formal (notes Store::get).

**What became unnecessary:** reshaping notes `store.get(99)` tests.

**Gates:** tip dogfood GREEN; WDB-134 still `get(&label)`.
- `cargo test --release --test all --features integration_tests,codegen_tests -- owned_i64_method_formal_must_not_receive_ref_literal`

**Do not steal:** P3.646–650 (filed).

## P3.650 (2026-10-04) — `Vec::remove(0)` literal must not borrow

Product `wj-proxy` `self.logs.remove(0)` tip emits `remove(&0_usize)` → E0308.
Isolate named-local remove gate can false-GREEN.

| Gate | Status |
|------|--------|
| `vec_remove_usize_literal_must_not_borrow` | ✅ tip GREEN — cargo + product proxy 25 (2026-10-04 tip e6aeefa3) |

**Root cause layer:** call-site ownership — `Vec::remove` takes owned `usize`;
integer literals must not be borrowed. Same P3.635 over-broad map-key belt.

**Fix:** Gate map-key borrow / HashMap::get bridge on map/set/wrapper receivers;
peel `&0_usize` on non-map `remove`.

**Why this is a new class:**
- `bug_vec_remove_usize_no_ref` is named local `pos`.
- P3.650 is literal `0` → `&0_usize` on product proxy.

**What became unnecessary:** casting / reshape ring-buffer trim in proxy.

**Gates:** tip dogfood proxy GREEN (`remove(0_usize)`).
- `cargo test --release --test all --features integration_tests,codegen_tests -- vec_remove_usize_literal_must_not_borrow`

**Do not steal:** P3.646–649 (filed).

## P3.649 (2026-10-04) — slice `.get(N)` literal must not borrow `&N_usize`

Product `wj-cron` `parts.get(0)` … tip emits `parts.get(&0_usize)` → E0277.
Blocks scheduler (path-dep). Distinct from P3.644 (variable `idx as usize` in sync).

| Gate | Status |
|------|--------|
| `slice_get_usize_literal_must_not_borrow` | ✅ tip GREEN — cargo + product cron 30 (2026-10-04 tip e6aeefa3) |

**Root cause layer:** call-site ownership — `slice::get` takes owned `usize`;
Call(FieldAccess) match scrutinees hit P3.635 map-key belt that forced `&K`
via `HashMap::get` for every `get` spelling.

**Fix:** Narrow Call(FieldAccess)/MethodCall map-key borrow to map/set/wrapper
receivers; peel `&N_usize` into owned Copy formals; keep WDB-134 `get(&label)`.

**Why this is a new class:**
- P3.644 is variable cast index `&(idx as usize)` (sync pool).
- P3.649 is integer **literal** `&0_usize` (cron parse).

**What became unnecessary:** reshaping cron field splits / scheduler deps.

**Gates:** tip dogfood cron GREEN (`get(0_usize)` …); WDB-134 no-reg.
- `cargo test --release --test all --features integration_tests,codegen_tests -- slice_get_usize_literal_must_not_borrow`

**Do not steal:** P3.646–648 (filed).

## P3.646 (2026-10-04) — HashMap::values() `&Copy` into owned add / cast

Product `EventBus::listener_count`: `for count in self.subscriber_counts.values()`
tip emitted `total += count as usize` → E0606 (`&usize` as `usize`). Correct is
`*count`.

| Gate | Status |
|------|--------|
| unit `p3646_hashmap_values_usize_sum_must_deref` | ✅ GREEN (`total += *count`) |
| MultiFile `hashmap_values_copy_elem_must_deref_into_usize_add` | ✅ GREEN |
| tip product `event/bus` listener_count | ⏳ tip rebuild |

**Root cause layer:** `values()`/`keys()` were collapsed to the map receiver so
`extract_iterator_element_type` yielded `(K, V)`; bare registry `Iterator` beat
stdlib `Iterator<&V>`. Fix: stdlib `values`/`keys` signatures + prefer
parameterized `Iterator<item>` over bare `Iterator`.

**Do not steal:** P3.638 Map::get shared-ref (GREEN).

## P3.647 (2026-10-04) — reused owned `Vec` formal gets `&clips` at recursive calls

`BlendTree::evaluate_node(clips: Vec<…>)` stays owned; recursive sites missed
`.clone()` (E0382) because auto_clone treated args as Read. P3.647b: product also
had `forwarding_borrow` forcing `&clips` into owned formals.

| Gate | Status |
|------|--------|
| unit `p3647_recursive_owned_vec_param_needs_clone` | ✅ GREEN |
| unit `p3647_parsed_source_needs_clone_on_first_recursive_clips` | ✅ GREEN |
| MultiFile `reused_owned_vec_formal_must_not_reborrow_recursive` | ✅ tip GREEN — `clips.clone()` |
| isolate `owned_vec_formal_with_forwarding_borrow_must_not_reborrow` | ✅ tip GREEN |
| product tip `blend_tree` evaluate_node (P3.647b/c) | ✅ tip GREEN — `clips.clone()`; metadata Self+4 (narrow re-borrow skip hold 2026-10-05) |
| tip `animation/blend_tree` evaluate_node | ✅ tip GREEN (same) |

**Root cause layer:** signature (P3.647c metadata Self+N duplicated by
`refresh_method_registry` append-on-missing-MethodSignature) + coercion
(`call_site_needs_shared_ref` / finalize re-borrow after peel). Analyzer
Borrowed + `forwarding_borrow` while codegen emits owned `Vec`
(`emitted_rust_ref_params=false`); dual-oracle raw `forwarding_borrow` and a
post-peel “owned collection local → `&`” loop re-introduced `&clips`.

**What became unnecessary:** raw `|| forwarding_borrow` dual-oracle in
`reconcile_multipass_demoted_*`; finalize re-borrow of owned Vec formals after
IR peel. No new method-name lists.

**Gates:** `cargo test --release --test all -- owned_vec_formal` → **7 passed**
(incl. tip product metadata + no-reborrow); lib
`forwarding_borrow_must_not_borrow_codegen_owned_vec_when_analyzer_still_borrowed`.

**Do not steal:** P3.557/559 Vec demotion gates; P3.668 json multipass.

## P3.645 (2026-10-04) — product wj-csv owned args into stdlib must auto-borrow

Product `wj-csv` thin wrappers: `csv.parse(text)` / `csv.write(rows)` tip emits
bare owned → E0308 (`&str` / `&[Vec<String>]`). Isolate write gate can false-GREEN.

| Gate | Status |
|------|--------|
| `csv_stdlib_owned_args_must_auto_borrow_product` | ✅ tip GREEN — product 6 + cargo 1/1 (2026-10-04 tip 18:24) |

**Root cause layer:** call-site coercion — owned String / Vec into demoted stdlib
formals must auto-borrow (signature-driven), including homonym `fn write`.

**Why this is a new class:**
- P3.644 is slice `.get` `&usize` (sync pool).
- P3.645 is stdlib `csv::parse`/`write` product thin-wrap borrow.

**What became unnecessary:** renaming wrappers / `.as_str()` / `&rows` in wj-csv.

**Gates:** tip probe 12:02 — `csv::parse(text)` / `csv::write(rows)` RED.
- `cargo test --release --test all --features integration_tests,codegen_tests -- csv_stdlib_owned_args_must_auto_borrow_product`

**Do not steal:** WDB-441, P3.508–P3.644 (filed).

## P3.644 (2026-10-04) — slice/Vec `.get(usize)` must not borrow index

Product `wj-sync` `pool.wj`: `job_txs.get(idx)` tip emits
`job_txs.get(&(idx as usize))` → E0277 (`SliceIndex` not for `&usize`).

| Gate | Status |
|------|--------|
| `slice_get_usize_index_must_not_borrow` | ✅ tip GREEN — sync pool get(idx as usize) (2026-10-04 tip e6aeefa3) |

**Root cause layer:** call-site ownership — `slice::get` / `Vec::get` takes owned
`usize` (Copy), must not prefix `&` on the cast index.

**Why this is a new class:**
- `bug_vec_remove_usize_no_ref` is `.remove` owned usize.
- P3.644 is `.get` on Vec of mpsc Senders with `int as usize` index.

**What became unnecessary:** `*` deref / reshape pool index access in wj-sync.

**Gates:** tip probe 12:02 — `get(&(idx as usize))` RED.
- `cargo test --release --test all --features integration_tests,codegen_tests -- slice_get_usize_index_must_not_borrow`

**Do not steal:** WDB-441, P3.508–P3.643 (filed).

## P3.643 (2026-10-04) — TDD WDB-441 (DB agent; no compiler src)

Copy `i64` local into indexed assign must not `.clone()`; product emits
`self.entities[i] = swapped_entity.clone()`.

| Gate | Status |
|------|--------|
| WDB-441 MultiFile | ✅ isolate GREEN — `entities[i] = swapped_entity` (no `.clone()`) |
| WDB-441 tip-out | ✅ tip GREEN after tip-out sweep (P3.704/P3.707)

**Root cause layer:** copy / index-assign — Copy `i64` locals assigned into `Vec` slots must not auto-clone
(product tip-out lag / multipass path; isolate already correct).

**Why this is a new class:**
- WDB-437 is formal into **let**.
- WDB-440 is f32 into **struct lit**.
- WDB-436 is loop counter into **cast** assign.

**What became unnecessary:** `swapped_entity.clone()` in ECS sparse swap-remove.

**Gates:** `CARGO_TARGET_DIR=…/agent-tdd-wdb435` → `wdb441_` — **1 passed / 1 failed**
(isolate GREEN, tip RED; 2026-10-04).

**Do not steal:** WDB-406/408/411/440–441, P3.508–P3.643, WDB-412–441 (filed).

## P3.642 (2026-10-04) — reused owned String into owned formal must clone first use

Product `wj-todo-cli` `stats` / `export`: `let snapshot = encode_store(store)` then
`decode_store(snapshot)` (move) and later `decode_store(snapshot.clone())` → E0382.
Tip clones only the second call; first move invalidates reuse.

| Gate | Status |
|------|--------|
| `reused_owned_string_into_owned_formal_must_clone_first_use` | ✅ tip GREEN |

**Root cause layer:** constraint/reuse (match-arm binding walk) —
`match_arm_body_uses_binding` / auto_clone `statement_uses_binding` skipped nested
`Statement::Match` / `Let` / `If`, so export’s deeper `decode_store(snapshot)` under
`match todos_to_json` never counted as arm reuse; first match-scrutinee call moved
while a later site still got `.clone()`.

**What became unnecessary:** todo-cli manual first-use clones; no new `ir_call_site` peel.

**Gates (2026-10-04 tip):**
- `… -- reused_owned_string_into_owned_formal_must_clone_first_use` → **1 passed**
- related `reused_owned_string` / `match_arm_owned_binding_reuse` / spawn / mpsc → **10 passed**
- export: first `decode_store(snapshot.clone())`; stats keeps last-use move on second.

**Do not steal:** WDB tip-out Copy-clone lag; other tip-live REDs.

## P3.641 (2026-10-04) — TDD WDB-440 (DB agent; no compiler src)

Copy `f32` local into struct literal field must not `.clone()`; product emits
`mse: mse.clone()` in `ComparisonResult { … }`.

| Gate | Status |
|------|--------|
| WDB-440 MultiFile | ✅ isolate GREEN — `ComparisonResult { … mse, … }` (no `mse.clone()`) |
| WDB-440 tip-out | ✅ tip GREEN after tip-out sweep (P3.704/P3.707)

**Root cause layer:** copy / struct-lit — Copy `f32` locals as struct fields must not auto-clone
(product tip-out lag / multipass path; isolate already correct).

**Why this is a new class:**
- WDB-437 is formal into **let**.
- WDB-428 is Copy into **println**.
- WDB-347 is Copy f32 **match** binding.

**What became unnecessary:** `mse.clone()` in SSIM comparison result construction.

**Gates:** `CARGO_TARGET_DIR=…/agent-tdd-wdb435` → `wdb440_` — isolate codegen GREEN / tip RED
(cargo-check timed out under contention; emit has bare `mse`; 2026-10-04).

**Do not steal:** WDB-406/408/411/439–440, P3.508–P3.641, WDB-412–440 (filed).

## P3.640 (2026-10-04) — TDD WDB-439 (DB agent; no compiler src)

Copy `i32` from indexed tuple field `.0` / `.1` must not `.clone()` on return;
product emits `return self.nodes[index as usize].0.clone()`.

| Gate | Status |
|------|--------|
| WDB-439 MultiFile | ✅ isolate GREEN — `return self.nodes[index as usize].0` (no `.clone()`) |
| WDB-439 tip-out | ✅ tip GREEN after tip-out sweep (P3.704/P3.707)

**Root cause layer:** copy / index-tuple-field — Copy `i32` via `nodes[i].0` must not auto-clone
(product tip-out lag / multipass path; isolate already correct).

**Why this is a new class:**
- WDB-363 is `].clone().field` (clone element then field).
- WDB-423 is destructure of indexed tuple element.
- WDB-438 is Copy formals in tuple **literal** into `push`.

**What became unnecessary:** `.0.clone()` / `.1.clone()` on astar node accessors.

**Gates:** `CARGO_TARGET_DIR=…/agent-tdd-wdb435` → `wdb439_` — **1 passed / 1 failed**
(isolate GREEN, tip RED; 2026-10-04).

**Do not steal:** WDB-406/408/411/438–439, P3.508–P3.640, WDB-412–439 (filed).

## P3.639 (2026-10-04) — owned String formal must not receive `&String` at adapter

Product `wj-auth-api` adapter: `handle_http(…, authorization: String, …)` but tip
emits `handle_http(…, &meta.2, …)` → E0308. Sibling `meta.0`/`meta.1` move.

| Gate | Status |
|------|--------|
| `owned_string_formal_must_not_receive_ref_at_adapter_call` | ✅ tip GREEN — `meta.2` moves |
| product auth-api adapter cargo-check | ✅ GREEN (with P3.637) |

**Root cause layer:** call-site ownership — owned String method formal must move
tuple field, not over-borrow one slot among several.

**Why this is a new class:**
- P3.637 is owned locals into demoted `&str` sibling methods.
- P3.638 is HashMap get `&String` payload into `Option<String>`.
- P3.639 is adapter over-borrow of one owned formal among mixed args.

**What became unnecessary:** `.clone()` / reshaping `meta.2` in auth adapter.

**Gates:** `$CARGO_TARGET_DIR=/tmp/wj-tdd-p3635` — 0 passed / 3 failed with P3.636–637 (2026-10-04).
- `cargo test --release --test all --features integration_tests,codegen_tests -- owned_string_formal_must_not_receive_ref_at_adapter_call`

**Do not steal:** WDB-134/438, P3.508–P3.638 (filed).

## P3.638 (2026-10-04) — HashMap/`Map::get` `&String` payload into owned `Option<String>`

Product-shaped `Event::get_data_string` with `use std::map::Map`: project analysis
registers `Map::get -> Option<V>` (owned key). Match emitted
`Some(EventDataValue::String(value)) => Some(value)` (`&String` → E0308).
Single-file / bare `Map` without std/map pull still cloned (HashMap meta).

| Gate | Status |
|------|--------|
| unit `wj_map_get_option_v_still_shared_get_via_hashmap_contract` | ✅ GREEN |
| `module_file_map_get_string_payload_must_clone` | ✅ tip GREEN |
| `single_file_hashmap_get_string_payload_must_clone` | ✅ tip GREEN |
| tip product `src/event` module-file | ✅ `value.clone()` |
| WDB-347 Copy match payload (no noise on owned Copy) | ✅ narrowed to `copy_match_payload_binding` |

**Root cause layer:** stdlib trait classification — `is_map_shared_get_call` required
`Option<&V>` on the looked-up sig; WJ `Map::get` Option\<V\> + Owned key failed closed,
so match bindings were not borrowed and WDB-347 blanket-skipped clone.

**Fix:** Honor HashMap/BTreeMap meta shared-ref contract for Map\* receivers; narrow
identifier early-return to `copy_match_payload_binding` (P3.574 intent) with `*` for
`&Copy` into owned context.

**What became unnecessary:** surgical gen patches / app-side `.clone()` on Event getters.

**Gates:** `CARGO_TARGET_DIR=…/windjammer-game/.cargo-target-wj`
- `cargo test --release -p windjammer --lib wj_map_get_option_v_still_shared_get`
- `cargo test --release --test bug_hashmap_get_match_string_payload_must_clone_into_owned_test --features integration_tests,codegen_tests`

**Do not steal:** WDB-134/438, P3.508–P3.637 (filed).

## P3.637 (2026-10-04) — owned String locals into demoted `&str` method formals

Product `wj-auth-api`: `find_user(username: &str)` / `verify_user(…: &str, …: &str)`
but call sites pass owned `username` / `password` → E0308. Distinct from greened
P3.621 (`&method_label(…)` call-expr borrow).

| Gate | Status |
|------|--------|
| `owned_string_into_demoted_str_method_formal_must_auto_borrow` | ✅ tip GREEN — auth-api 48 (2026-10-04 shared tip 16:41) |
| product auth-api cargo-check | ✅ GREEN |

**Root cause layer:** call-site coercion — owned String **locals** into demoted
`&str` method formals must auto-borrow (signature-driven), including last-use
second args on methods (IR free-fn-only shared-borrow reapply missed methods).

**Fix:** Method-arg post-IR force `&` for Identifier into `&str` / demoted shared
text formals; widen IR reconcile shared-borrow reapply beyond free-fn-only.

**Why this is a new class:**
- P3.621 is owned **call-expr return** into `&str` (webhook, tip GREEN).
- P3.637 is owned **locals** into sibling demoted `&str` methods (auth).

**What became unnecessary:** `.as_str()` / reshaping find_user/verify_user in auth.

**Gates:** `$CARGO_TARGET_DIR=/tmp/wj-agent-test-p3636` — **3 passed** with P3.636 + WDB-134 (2026-10-04).
- `cargo test --release --test all --features integration_tests,codegen_tests -- owned_string_into_demoted_str_method_formal_must_auto_borrow`

**Do not steal:** WDB-134/438, P3.508–P3.636 (filed).

## P3.636 (2026-10-04) — `&mut self` field into owned `json::to_string` must clone

Product `wj-webhook` `list_events`: tip demotes to `&mut self` then emits
`json::to_string(self.events)` → E0507. P3.619–621 greened on tip 07:32; this is
the remaining webhook blocker.

| Gate | Status |
|------|--------|
| `mut_self_field_into_owned_json_formal_must_clone` | ✅ tip GREEN — `self.events.clone()` |
| product webhook cargo-check | ✅ GREEN |

**Root cause layer:** field access — non-Copy field behind `&mut self` into owned
formal must clone; method-path `json.to_string` was resolving MutBorrowed
`to_string` homonyms instead of runtime `json::to_string(value: T)` owned.

**Fix:** Prefer stdlib `json::to_string` owned contract on method sites; treat
`self.field` as behind-ref when current method emits borrowed self (upgrades /
registry), then `.clone()` into owned slots.

**Why this is a new class:**
- P3.620 was `&mut EventBus` into owned emit (tip GREEN).
- P3.636 is `&mut self.events` move into owned `json::to_string`.

**What became unnecessary:** manual `.clone()` reshape in webhook `list_events`.

**Gates:** `$CARGO_TARGET_DIR=/tmp/wj-agent-test-p3636` — **3 passed** with P3.637 + WDB-134 (2026-10-04).
- `cargo test --release --test all --features integration_tests,codegen_tests -- mut_self_field_into_owned_json_formal_must_clone`

**Do not steal:** WDB-134/438, P3.508–P3.635 (filed).

## P3.634 (2026-10-04) — TDD WDB-438 (DB agent; no compiler src)

Copy `i32` formal into tuple literal must not `.clone()`; product emits
`result.push((x.clone(), y + 1_i32, …))` in astar_grid neighbors.

| Gate | Status |
|------|--------|
| WDB-438 MultiFile | ✅ isolate GREEN — product-shaped `CostGrid::get_neighbors` emits bare `push((x, y + 1, self.get_cost(…)))` |
| WDB-438 tip-out | ✅ tip GREEN after tip-out sweep (P3.704/P3.707)

**Root cause layer:** codegen identifier auto-clone — `ident_skips_auto_clone_as_copy` missed Copy pass-by-value formals when inference was thin; tuple literals set `in_owned_value_context`, so analysis `.clone()` stuck before tuple slot cleanup.

**Fix:** `binding_is_copy_pass_by_value_scalar` early in `ident_skips_auto_clone_as_copy` (type-classification; not method heuristics).

**Why this is a new class:**
- WDB-437 is formal into **let**.
- WDB-434 is const into **call** formals.
- WDB-423 is **indexed** tuple element.

**What became unnecessary:** `x.clone()` / `y.clone()` in neighbor tuple construction.

**Gates:** `CARGO_TARGET_DIR=$HOME/Library/Caches/windjammer/cargo-target/shared` →
`cargo test --release --test all -- wdb438_` — **1 passed / 1 failed** post-fix
`9cb2896a` (`wdb438_module` ✅, tip-out ❌ until product regen; 2026-10-04).

**Do not steal:** WDB-406/408/411/437–438, P3.508–P3.641, WDB-412–440 (filed).

## P3.633 (2026-10-04) — multipass homonym `draw_text`: module-qualified call-site sig

When `draw::draw_text` and `hud::draw_text` both exist, `game.rs` emitted
`draw::draw_text(&label, …)` — wrong homonym borrow from bare `draw_text` refresh
challengers while single-module multipass still moved `label`.

| Gate | Status |
|------|--------|
| `test_multipass_module_qualified_autoborrow` | ✅ GREEN |
| `test_multipass_no_name_collision_different_modules` | ✅ GREEN |
| spawn / mpsc / notes empty-lit / `owned_string_for_loop_load` | ✅ no-reg |

**Root cause layer:** call-site signature refresh (`qualified_callee_skips_bare_homonym_lookup` + `has_ownership_collision_for_call`) — user `draw::fn` paths did not skip bare leaf registry keys, so `prefer_shared_text_ref_signature` upgraded owned FFI-forward formals from a sibling homonym; simple-name ownership collision also flagged module-qualified calls.

**Fix:** Treat `is_lowercase_user_module_qualified_call` like runtime-std/type-qualified paths (no bare homonym challengers); only apply simple-name explicit ownership collision when `ownership_collision_blocks_autoborrow(func_name)`.

**What became unnecessary:** Peels/heuristics to strip `&label` on collision — registry key discipline fixes emit at source.

**Gates:** `CARGO_TARGET_DIR=~/Library/Caches/windjammer/cargo-target/shared`
`cargo test --release --test all -- test_multipass_module_qualified`
`cargo test --release --lib -- user_module_qualified_call_skips_bare_homonym_refresh`
`cargo test --release --test all -- owned_string_for_loop_load spawn mpsc empty_lit`

## P3.632 (2026-10-04) — tip-truth gates: module_qualified move + class3 demoted `&str`

Stale assertions expected `&label` auto-borrow / call-site `.to_string("Metal")`
while tip correctly:
- keeps owned `draw_text` (FFI forward) and **moves** last-use `label`;
- demotes `set_name` to `name: &str` with assign-site `name.to_string()`.

| Gate | Status |
|------|--------|
| `test_multipass_module_qualified_autoborrow` | ✅ tip GREEN (move, not `&label`) |
| `test_multipass_no_name_collision_different_modules` | ✅ GREEN (P3.633 compiler fix) |
| `class3_string_literal_to_owned_param` | ✅ tip GREEN (owned coerce **or** demoted `&str`) |

**Root cause layer:** n/a (gate alignment) — tip ownership already correct; tests
were false RED against evolved string demotion / FFI-forward owned formals.

**What became unnecessary:** chasing peels to force `&label` / call-site
`.to_string()` when tip already emits valid Rust.

**Gates:** `cargo test --release --test all -- test_multipass_module_qualified class3_string_literal_to_owned_param`

## P3.631 (2026-10-04) — notes-api mixed demoted `&str` + owned `String` call args

Product `handle_request` → `App::handle`: after `method` demotes to `&str`, owned
`path`/`body` must move (not `&path` / `&body.into()`). Empty lits already
`"".to_string()`; sibling demotion must not re-borrow other owned String slots.
Forwarder `handle` → `inner` must pass `origin`/`accept_encoding`/`client_key` by
move into emitted `String` formals.

| Gate | Status |
|------|--------|
| `demoted_method_then_owned_empty_lits_must_own` | ✅ tip GREEN |
| `handle_forward_empty_lits_must_own` | ✅ tip GREEN |
| `handle_request_empty_lits_hex_app_must_own` | ✅ tip GREEN |
| `owned_string_for_loop_load_must_not_borrow_path` / `path_bytes_wal_layout` / spawn / mpsc | ✅ no-reg |

**Root cause layer:** coerce/reconcile — `local_should_move_into_owned_text_formal`
returned `false` for caller outer formals (only locals/`local_var_types` counted),
so shared-borrow reapply and forwarder IR paths prefixed `&` on owned `path`/`body`
and on mixed-forward `origin` despite callee owned `String` emission.

**Fix:** When callee slot is owned WJ text, caller **outer** formals use
`caller_keeps_owned_outer_formal` (move); demoted outer formals stay on borrow path.

**What became unnecessary:** Extra peels on path/body without signature+move gate;
method-name heuristics unchanged (still banned).

**Gates:** `CARGO_TARGET_DIR=~/Library/Caches/windjammer/cargo-target/shared`
`cargo test --release --test all -- demoted_method_then_owned_empty_lits_must_own handle_forward_empty_lits_must_own handle_request_empty_lits_hex_app_must_own owned_string_for_loop_load_must_not_borrow_path path_bytes_wal_layout spawn mpsc`.

## P3.629 (2026-10-04) — trait impl owned formal `mut` (E0053 + E0596)

| Gate | Status |
|------|--------|
| `trait_impl_owned_param_not_reborrowed_when_mutated` | ✅ GREEN — `mut ctx: App`, not `&mut App` |
| spawn / mpsc / `dogfood_store_has_key` no-reg | ✅ GREEN |

**Root cause layer:** codegen formal emission — E0053 early return for trait impl
owned non-`self` formals skipped the owned-formal `auto_needs_mut` path, so
`ctx.record_resource(...)` emitted with immutable `ctx: App` (rustc E0596).

**Fix:** `format_trait_impl_owned_formal` — keep owned AST type (never `&mut T`
for trait contract) but apply the same body-driven `mut` prefix as regular
owned formals (`variable_needs_mut` + analyzer mutation sets).

**What became unnecessary:** No ir_call_site peel or method-name ownership
heuristics; trait ownership still wins over analyzer MutBorrowed demotion.

**Gates:** `CARGO_TARGET_DIR=~/Library/Caches/windjammer/cargo-target/shared`
`cargo test --release --test all -- trait_impl_owned_param_not_reborrowed_when_mutated dogfood_store_has_key_forward_ref_borrows_owned_key bug_thread_spawn_closure_must_not_be_ref_test bug_mpsc_sync_channel_boundary_signature_test`.

## P3.625 (2026-10-04) — full suite post P3.622–628 (no wdb-layers hang)

**wdb-layers dogfood no longer hangs** (P3.622). Re-run on `3c7dd5f1`:
`cargo test --release --test all` → **5620 passed / 123 failed** (~1739s, EXIT:101).
Log `/tmp/wj-full-suite-p3628.log`; fails `/tmp/wj-suite-fails-p3628.txt`.

| Bucket | Count | Notes |
|--------|------:|-------|
| tip-out / gen / product scanners | ~85 | stale `.agent-wip/rel_tip_out` + game-core `gen/` — regen with tip `wj` |
| tip-live / assertion | ~38 | triage next (e0507 + WDB-209 + WAL cleared by P3.626–628) |

**Cleared tip-live:**
- ~~`dogfood_store_has_key_forward_ref_borrows_owned_key`~~ ✅ P3.626
- ~~`dogfood_wal_segment_cross_crate_append_put_borrows_vec_literal`~~ ✅ P3.627
- ~~`e0507` multi-loop borrow pair~~ ✅ P3.628

**Cleared tip-live this session:** P3.629 trait `mut`, P3.630 FfiString + load_batch,
P3.631 notes-api owned slots, P3.632 module_qualified/class3 gate alignment.

**Do not steal:** tip-out mass regen is product work; prefer tip-live signature/solver fixes.

## P3.628 (2026-10-04) — multi-loop iterable reuse (`entity_components` E0382)

| Gate | Status |
|------|--------|
| `test_for_loop_param_used_multiple_times_borrows` | ✅ GREEN — `for comp in &entity_components` |
| `test_param_used_in_multiple_nested_loops_borrows` | ✅ GREEN |
| WAL / store / spawn no-reg | ✅ GREEN (`dogfood_store_has_key`, `dogfood_wal_segment`, `spawn`, `mpsc`) |

**Root cause layer:** codegen formal reconcile — `for_loop_borrow_needed` demoted
params to `&Vec` formals, which suppressed use-site `&` in
`should_borrow_for_iteration` (`inferred_borrowed_params` / `emitted_rust_ref_formals`).
Tests (and E0382 fix intent) expect owned formals + borrowed iteration at the `for` site.

**Fix:** Drop formal demotion branch for `for_loop_borrow_needed`; keep
`precompute_for_loop_borrows` + `should_borrow_for_iteration` use-site borrow.

**What became unnecessary:** Formal `&Vec` demotion solely for sequential/nested
`for` iterable reuse; IR/solver unchanged (reuse was already marked in
`for_loop_borrow_needed`).

**Gates:** `CARGO_TARGET_DIR=~/Library/Caches/windjammer/cargo-target/shared`
`cargo test --release --test all -- test_for_loop_param_used_multiple_times_borrows test_param_used_in_multiple_nested_loops_borrows dogfood_store_has_key dogfood_wal_segment spawn mpsc`.

## P3.626 (2026-10-04) — WDB-209 `latest.has_key(key)` last-use move (tip-live)

| Gate | Status |
|------|--------|
| `dogfood_store_has_key_forward_ref_borrows_owned_key` | ✅ GREEN — `latest.has_key(key)` moves |
| WDB-209 multipass + demotion regression bundle | ✅ GREEN (8 tests) |

**Root cause layer:** codegen prepare (`param_has_owning_method_use`) — stale
readonly convergence on sibling `get(&key)` made
`method_call_arg_emits_shared_rust_ref` run before WJ AST owned `Key` on
`BasePart::has_key`, so `non_self_owned_forward` demoted `key_in_latest_base`
to `&Key` + `.clone()` at the call site.

**Fix:** Check `method_call_arg_formal_is_owned_non_copy` before shared-ref
skips in owning-use detection (`expression_has_owning_method_use` /
`expression_has_owning_method_call_arg_use` / `method_call_sibling_ast_expects_owned_arg`).

**What became unnecessary:** `key.clone()` at `latest.has_key` when formal stays
owned; no ir_call_site peel / method-name heuristics.

**Gates:** `CARGO_TARGET_DIR=~/Library/Caches/windjammer/cargo-target/shared`
`cargo test --release --test all -- wdb209_multipass_catalog_push_column_col_must_stay_owned dogfood_store_has_key_forward_ref_borrows_owned_key bug_demoted_vec_param_into_owned_vec_callee_must_clone_test bug_wdb124_module_file_demoted_vec_i64_formal_must_borrow_clone_call_sites_test bug_wdb125_module_file_demoted_struct_formal_must_borrow_clone_call_sites_test bug_mut_param_passthrough_no_shared_amp_test` → **8 passed**.

## P3.627 (2026-10-04) — WAL cross-crate `append_put` forwarding borrow (P3.626 regression)

After P3.589 owned-Vec denial in `call_site_needs_shared_ref_at_emit`,
`maybe_borrow_vec_or_helper` added `&vec![…]` from metadata
`forwarding_borrow_params`, then owned-formal terminal peel converted to
`.clone()` → RED `dogfood_wal_segment_cross_crate_append_put_borrows_vec_literal`.

| Gate | Status |
|------|--------|
| `dogfood_wal_segment_cross_crate_append_put_borrows_vec_literal` | ✅ GREEN — `&vec![…]` / `&encode_int64(…)` |
| `owned_vec_u8_literal_must_not_borrow_at_call_site` | ✅ no-reg |
| `dogfood_store_has_key_forward_ref_borrows_owned_key` | ✅ no-reg |
| `forwarding_borrow_beats_emitted_owned_vec_contract` | ✅ lib GREEN (metadata oracle) |

**Root cause layer:** ir_call_site terminal reconcile — owned-emission peel after
`maybe_borrow_vec_or_helper` ignored defining-crate `forwarding_borrow_params`.

**Fix:** `forwarding_borrow_metadata_requests_call_site_borrow` in signature
bridge; skip `coerce_borrowed_arg_to_owned` when metadata marks forwarding borrow;
P3.589 bridge denial unchanged for stale owned-Vec flags.

**What became unnecessary:** cross-crate `.clone()` on vec literals / helper
returns into `WalSegment::append_put` when dependency metadata already records
forwarding borrow.

**Gates:** `CARGO_TARGET_DIR=~/Library/Caches/windjammer/cargo-target/shared`
`cargo test --release --test all -- dogfood_wal_segment_cross_crate_append_put_borrows_vec_literal owned_vec_u8_literal_must_not_borrow dogfood_store_has_key_forward_ref`
→ **3 passed**; `cargo test --release -p windjammer --lib forwarding_borrow` → **3 passed**.

## P3.624 (2026-10-04) — WDB-435 generic `ObjectPool<T>` Copy usize field return

Product `object_pool` emitted `self.capacity.clone()` / `self.in_use.clone()` on
`&self` getters while non-generic isolate was GREEN.

| Gate | Status |
|------|--------|
| WDB-435 MultiFile (`ObjectPool<T>` + private `Vec<T>` fields) | ✅ GREEN |
| WDB-435 tip-out (`rel_tip_out` + `gen/object_pool/…/object_pool.rs`) | ✅ GREEN after tip regen |

**Root cause layer:** struct field registry / impl context — `impl ObjectPool<T>`
left `current_struct_fields` empty (map keyed `ObjectPool`, impl type
`ObjectPool<T>`), so Copy field type was unknown and borrowed-`self` lowering
added `.clone()`. `lookup_struct_field_types` now strips type args; explicit
`return self.field` skips clone when field type is Copy.

**What became unnecessary:** `.clone()` on Copy `usize` field returns / struct-lit
fields for generic impl parents; redundant return-statement clone when Copy is
known from inference.

**Gates:** `cargo test --release --test all -- wdb435_`; related
`wdb427_`/`wdb433_`/`wdb436_`/`wdb437_` isolates GREEN (tip-outs for 427/433
still stale product); `cargo test --release -p windjammer --lib struct_decl_base_name`.

## P3.623 (2026-10-04) — cross-file Borrowed bare Vec must `&walls`

`test_cross_file_borrowed_param_gets_ampersand`: analyzer Borrowed `walls: Vec`
with no emission flags must emit `check_collisions(&walls)`. Tip forced Owned
expected (ast_bare_vec override + `bare_formal_is_vec_or_map`).

| Gate | Status |
|------|--------|
| `test_cross_file_borrowed_param_gets_ampersand` | ✅ tip GREEN |
| `borrowed_bare_vec_expects_shared_ref_at_call_site` | ✅ unit GREEN |
| WDB-281 tip-out contains owned Vec | ✅ still GREEN |

**Root cause layer:** constraint/solver (`safety_type_from_signature_param`) —
analysis-only Borrowed bare Vec is Ref; only `emitted=false` stays Owned.

**What became unnecessary:** mistaking analysis-only Borrowed Vec for owned
container without emission confirmation.

**Gates:** `cargo test --release --test all -- test_cross_file_borrowed_param bug_thread_spawn_closure bug_mpsc_sync_channel owned_string_return_into_demoted_str cross_crate_owned_struct_formal cross_crate_owned_bus_formal wdb281`

## P3.622 (2026-10-04) — wdb-layers multipass hang (trait-sig + custom-formal restore)

Full `cargo test --release --test all` stuck on tip `wj build`
`windjammerdb/crates/wdb-layers/src/mod.wj` (CPU > 0, growing RSS — not UE).

Two stacked patho scans:

1. `trait_definition_sigs_for_method` walked all registry sigs per method call.
2. After (1), hang moved to `restore_pub_owned_non_copy_api_formals` →
   `programs_declare_pub_free_fn_owned_custom_formal_at` rescanning every program
   per registry×param Custom formal (P3.585 indexed Vec path but missed Custom).

| Gate | Status |
|------|--------|
| `restore_pub_owned_scales_with_program_lookup_index` | ✅ GREEN |
| `restore_pub_owned_custom_formals_uses_program_lookup` | ✅ GREEN |
| tip `wj build` wdb-layers `--library --module-file --no-cargo` | ✅ completes ~275s (EXIT:0; was hung in Custom-formal restore) |
| Full suite through wdb-layers dogfood | ✅ completes on `d61ee6ab`/`cd639652` — 5622 passed / 121 failed in ~1961s (no hang; codegen through 1025 files) |

**Root cause layer:** signature resolution + bare-pass restore performance
(index completeness — not ownership heuristics).

**What became unnecessary:** full-registry trait-key scan; per-slot
`programs_declare_pub_free_fn_owned_custom_formal_at` / vec formal rescans on
hot restore paths (`ProgramLookup::declares_pub_*`).

**Do not steal:** P3.619–621 landed; tip-out RED cluster (product regen).

## P3.590c (2026-10-04) — stale metadata must not `&source_dir` into owned validate

Product `BuildFingerprint::validate(source_dir: String)` — tip without metadata
moves; with stale `metadata.json` (`emitted_rust_ref_params[1]=true`,
`Reference(str)`) call sites emit `validate(&source_dir)` → E0308.

| Gate | Status |
|------|--------|
| `owned_string_param_must_move_into_owned_method_arg` | ✅ MultiFile GREEN |
| `tip_out_build_fingerprint_must_not_borrow_source_dir` | ✅ tip-out GREEN (metadata patched) |
| `stale_metadata_emitted_ref_must_not_borrow_owned_string_formal` | ✅ unit GREEN |

**Root cause layer:** stale engine metadata.json poisoned call-site borrow;
patched `BuildFingerprint::validate`/`generate` entries. Systemic: codegen
refresh must beat stale emitted_ref=true on owned String (follow-up if recurrence).

**Do not steal:** P3.590b, full engine transpile hang, remaining E0308 clusters.

## P3.590b (2026-10-04) — `load_batch` moves owned path

Product `AssetLoader::load_batch` reuses `name` in Err → `name.clone()`, but
`path` must move into owned `String` formal (`self.load(..., path, size)`).

| Gate | Status |
|------|--------|
| `owned_string_for_loop_load_must_not_borrow_path` | ✅ tip GREEN |
| `tip_out_loader_load_batch_must_not_borrow_path` | ✅ tip-out (regen when product picks up tip) |
| spawn / mpsc no-reg | ✅ GREEN |

**Root cause layer:** signature/solver — stale converged `&str` metadata on
owned WJ `string` formals re-applied `&` in `enforce_call_site_ownership_contract`,
`reconcile_post_ir_*`, and method-arg reuse-after. **Fix:** `local_should_move_into_owned_text_formal`
( AST `struct_method_ast_formal_param_types` + call arg index ) gates shared-borrow
reapply; terminal peel in method `arguments.rs`.

**Became unnecessary:** method-name / temporary-only peels without signature+AST formal.

**Do not steal:** remaining engine E0308 clusters (mesh_renderer, blend_tree, …).

## P3.630 (2026-10-04) — WAL FFI owned `FfiString` (no `&string_to_ffi`)

| Gate | Status |
|------|--------|
| `path_bytes_wal_layout_rustc_cargo_check` | ✅ tip GREEN |
| spawn / mpsc no-reg | ✅ GREEN |

**Root cause layer:** coerce/terminal — `rust_shared_borrow` must not prefix args
already wrapped in `string_to_ffi(...)` (owned extern formal).

**Do not steal:** P3.590b follow-ups, engine E0308 clusters.

## P3.621 (2026-10-04) — owned String return into demoted `&str` method formal

Product `wj-webhook` adapter: `app.handle(method_label(req.method), …)` where
`handle` demotes `method: string` → `method: &str` and `method_label` returns
`String` → E0308. Tip must auto-borrow (`&method_label(…)`) or keep owned formal.

| Gate | Status |
|------|--------|
| `owned_string_return_into_demoted_str_method_formal_must_auto_borrow` | ✅ tip GREEN |
| `multipass_http_hexagonal_method_label_into_demoted_str_must_auto_borrow` | ✅ tip GREEN |

**Root cause layer:** constraint/solver (match binding types) + signature resolution
(MutexGuard peel → defining `WebhookApp::handle` demoted `&str`) + coercion
(`owned_text_into_str_ref` keeps Borrow for `String` → `&str`).

**Why this stayed RED:** `Ok(mut app)` parsed as `Tuple([MutBinding])`, so match
binding inference never typed `app` → IR used bare `handle` Owned stub
(`emitted=None`) instead of demoted `&str`.

**What became unnecessary:** webhook `.as_str()` / reshape `handle`; reconcile peel
was never the right fix once expected ownership is Ref.

**Gates:** `$CARGO_TARGET_DIR=/tmp/wj-tdd-p3619-debug` — GREEN (2026-10-04).
- `cargo test --release --test all -- owned_string_return_into_demoted_str_method_formal multipass_http_hexagonal_method_label owned_helper_into_demoted_str`

**Do not steal:** WDB-406/408/411/436–437, P3.508–P3.620, WDB-412–437 (filed).

## P3.620 (2026-10-04) — owned EventBus formal must not receive `&mut bus`

Product `wj-webhook` `queue_via_event_bus`: `let bus = emit(bus, …)` where
`emit(bus: EventBus, …) -> EventBus`. Tip emits `emit(&mut bus, …)` → E0308.

| Gate | Status |
|------|--------|
| `cross_crate_owned_bus_formal_must_not_receive_mut_ref` | ✅ tip GREEN |

**Root cause layer:** signature boundary — path-dep Owned + `emitted_rust_ref_params=false`
must not bare-pass-demote to MutBorrowed; pick prefers owned Custom over stale MutBorrowed.

**Why this is a new class:**
- P3.619 is `&mut CronExpr` into owned CronExpr (match `Ok` binding).
- P3.620 is `&mut EventBus` into owned EventBus (reassign `let bus = emit(bus, …)`).

**What became unnecessary:** `.clone()` / reshaping emit calls in webhook; importer
MutBorrowed demotion for path-dep owned Custom emission slots.

**Gates:** `$CARGO_TARGET_DIR=/tmp/wj-tdd-p3619-debug` — GREEN (2026-10-04).
- `cargo test --release --test all -- cross_crate_owned_bus_formal_must_not_receive_mut_ref`

**Do not steal:** WDB-406/408/411/436–437, P3.508–P3.619, WDB-412–437 (filed).

## P3.619 (2026-10-04) — owned CronExpr formal must not receive `&mut cron`

Product `wj-scheduler` → `wj_cron::matches_cron(expr: CronExpr, …)` /
`next_run(expr: CronExpr, …)`: tip emits `matches_cron(&mut cron, …)` /
`next_run(&mut cron, …)` → E0308. Source uses bare `matches_cron(cron, …)`.

| Gate | Status |
|------|--------|
| `cross_crate_owned_struct_formal_must_not_receive_mut_ref` | ✅ tip GREEN |

**Root cause layer:** signature boundary — path-dep Owned + emit-false skips
Borrowed **and** MutBorrowed bare-pass demotion; signature pick prefers owned Custom.

**Why this is a new class:**
- P3.574 is package formal should demote to `&CronExpr` (field reads / multi-call).
- P3.619 is **app call site** wrongly emitting `&mut cron` into still-owned formal.

**What became unnecessary:** `.clone()` / reshaping scheduler cron calls.

**Gates:** `$CARGO_TARGET_DIR=/tmp/wj-tdd-p3619-debug` — GREEN (2026-10-04).
- `cargo test --release --test all -- cross_crate_owned_struct_formal_must_not_receive_mut_ref`

**Do not steal:** WDB-406/408/411/436–437, P3.508–P3.618, WDB-412–437 (filed).

## P3.618 (2026-10-04) — TDD WDB-437 (DB agent; no compiler src)

Copy `i32` formal into local `let` must not `.clone()`; product emits
`let mut r = radius.clone()` in station_geometry capsules.

| Gate | Status |
|------|--------|
| WDB-437 MultiFile | ✅ GREEN @ `c2b9db42` — isolate `let mut r = radius` no `.clone()` |
| WDB-437 tip-out | ✅ GREEN after tip regen (`d61ee6ab`) — tip already correct; stale `rel_tip_out`/gen cleared |

**Root cause layer:** copy / formal — Copy formals bound into locals must not auto-clone
(tip already GREEN; tip-out was product lag).

**Why this is a new class:**
- WDB-427 is Copy **field** into let (`chunk.size.clone()`).
- WDB-428 is **local** in println.
- WDB-434 is const into **call** formals.

**What became unnecessary:** `radius.clone()` when initializing `r` from an i32 formal.

**Gates:** `cargo test --release --test all -- wdb437_` → 2 passed (2026-10-04).

**Do not steal:** WDB-406/408/411/436, P3.508–P3.618, WDB-412–437 (filed).

## P3.617 (2026-10-03) — TDD WDB-436 (DB agent; no compiler src)

`usize` loop counter assigned to `i32` with cast must not `.clone()`; product emits
`before_idx = i.clone() as i32` in animation/clip.

| Gate | Status |
|------|--------|
| WDB-436 MultiFile | ✅ GREEN @ `70f7225b` — isolate `before_idx = i` no `.clone()` |
| WDB-436 tip-out | ✅ GREEN after tip regen (`d61ee6ab`) — tip already correct; stale `rel_tip_out`/gen cleared |

**Root cause layer:** copy / loop — usize counter cast-assign to i32 must not auto-clone
(tip already GREEN; tip-out was product lag).

**Why this is a new class:**
- WDB-394 is assign **without** cast (`best_idx = i.clone()`).
- WDB-432 is **indexed** element `.clone() as`.
- WDB-433 is struct **field** `.clone() as`.

**What became unnecessary:** `i.clone()` when assigning loop counter across int widths.

**Gates:** `CARGO_TARGET_DIR=…/agent-tdd-wdb435` → `wdb436_` — 1 passed / 1 failed (isolate GREEN, tip RED).

**Do not steal:** WDB-406/408/411/435, P3.508–P3.617, WDB-412–436 (filed).

## P3.616 (2026-10-03) — reused owned string must not emit `.into().clone()`

Product `wj-auth-api` `handle`: after `own(path)` / sibling Into paint, tip emits
`emit_access_log(…, path.into().clone(), …)` / `origin.into().clone()` → E0282.
Also blocked behind P3.614 E0507 on headers for-in.

| Gate | Status |
|------|--------|
| `reused_owned_string_must_not_emit_into_clone` | ✅ tip GREEN (verified 2026-10-04) |

**Root cause layer:** formal encoding — reused owned locals/formals must clone as
`String` (or borrow), never `.into().clone()` after ambiguous Into paint.

**Why this is a new class:**
- P3.615 is proxy `client_key.into()` **move** (E0382) before second use.
- P3.616 is `.into().clone()` **inference** failure (E0282) on reused path/origin.

**What became unnecessary:** dropping path reuse in auth access-log calls.

**Gates:** `$CARGO_TARGET_DIR=/tmp/wj-tdd-p3616` — GREEN (2026-10-04).
- `cargo test --release --test all -- reused_owned_string_must_not_emit_into_clone`

**Do not steal:** WDB-406/408/411/430–435, P3.508–P3.615, WDB-412–435 (filed).

## P3.615 (2026-10-03) — reused `client_key` must not `impl Into` + `.into()` move

Product `wj-proxy` `complete_proxy(…, client_key: string, …)`: tip emits
`client_key: impl Into<String>` then `own(client_key.into())` /
`check_rate(client_key.into(), …)` before later `client_key` in `RequestLogEntry`
→ E0382. Flat isolates false-GREEN (`String` + `&client_key`).

| Gate | Status |
|------|--------|
| `reused_owned_string_method_formal_must_not_into_move_before_second_use` | ✅ tip GREEN |

**Root cause layer:** formal encoding — reused owned string method formals (≥2
identifier uses) must not take the pub-builder `impl Into<String>` upgrade;
consuming `.into()` before second use is illegal.

**What became unnecessary:** manual `client_key.clone()` rewrites in proxy domain;
`client_key: impl Into<String>` + `check_rate(client_key.into())` before log reuse.

**Gates:** tip `.cargo-target-wj` 2026-10-04 — proxy emit `client_key: String` +
`check_rate(client_key.clone())`;
`cargo test --release --test all -- reused_owned_string_method_formal_must_not_into_move_before_second_use`.

**Do not steal:** WDB-406/408/411/430–435, P3.508–P3.614, WDB-412–435 (filed).

## P3.614 (2026-10-03) — borrowed for-in tuple field must `.clone()` into owned

Product notes-api `headers_meta` / proxy `client_key_from_headers`: `for pair in
headers` over demoted `&Vec<(String,String)>` then `pair.0` / `return pair.1`
moves out of borrowed tuple → E0507; product also emits `*pair.0 == "…"` (E0277).

| Gate | Status |
|------|--------|
| `borrowed_for_in_tuple_field_must_clone_into_owned` | ✅ tip GREEN |
| borrow-tracking prerequisite (`049fd5f9`) | ✅ landed — demoted `&Vec` marked borrowed |

**Root cause layer:** constraint/prepare + bare-pass — `pair.N` field moves count as
consuming the for-in element so `Vec` stays Owned (not bare-pass/`&Vec` demotion);
when borrow iteration is still required, whole binding types as `&(K,V)` and field
extract uses move-out Copy check (`.clone()`), not `is_type_copy(&T)`.

**What became unnecessary:** demoting headers to `&Vec` then star-deref/`pair.N`
moves; treating whole-tuple for-in bindings as `(&K,&V)` (Map-only shape).

**Gates:** tip `.cargo-target-wj` 2026-10-04 — emit Owned Vec + cargo-check;
`cargo test --release --test all -- borrowed_for_in_tuple_field_must_clone_into_owned`.

**Do not steal:** WDB tip-outs / P3.508–P3.613 / WDB-435 (filed).

## P3.589 (2026-10-03) — owned `Vec<u8>` formal must not `&vec![…]` at call site

Product tip `gen/ecs/component_storage.rs` emitted
`registry.add(..., &vec![0_u8; 36])` while `add(..., data: Vec<u8>)` is owned
→ E0308 (~20 leftovers). `forwarding_borrow` from readonly body use beat
`emitted_owned_arg_contract` (same class as P3.588 owned String).

| Gate | Status |
|------|--------|
| `forwarding_borrow_must_not_borrow_owned_vec_emit` | ✅ unit GREEN |
| `owned_vec_u8_literal_must_not_borrow_at_call_site` | ✅ tip GREEN |

**Root cause layer:** signature_bridge — exempt owned Vec/Map emit from
forwarding_borrow share-ref (mirror P3.588 text exemption).

**Gates:** tip `.cargo-target-wj` — unit + MultiFile cargo-check.

**Do not steal:** WDB-406/408/411/328, P3.508–P3.588, WDB-412–429 (filed).

## P3.613 (2026-10-03) — TDD WDB-435 (DB agent; no compiler src)

Superseded by **P3.624** (generic `ObjectPool<T>` repro + struct-field-map fix).
Original non-generic isolate was GREEN; product/tip stayed RED until P3.624.

**Do not steal:** WDB-406/408/411/434, P3.508–P3.615, WDB-412–435 (filed).

## P3.612 (2026-10-03) — TDD WDB-434 (DB agent; no compiler src)

Copy `const i32` into owned call formals must not `.clone()`; product emits
`VoxelGrid::new(GRID.clone(), GRID.clone(), GRID.clone())`.

| Gate | Status |
|------|--------|
| WDB-434 MultiFile | ✅ GREEN @ `61b36a54` — isolate `VoxelGrid::new(GRID, GRID, GRID)` no `.clone()` |
| WDB-434 tip-out | ❌ RED @ `61b36a54` — `GRID.clone()` in `rel_tip_out/scene/station_builder.rs` |

**Root cause layer:** copy / const — Copy consts into owned formals must not auto-clone.

**Why this is a new class:**
- WDB-426 is const on **assign** (`LEAF_FLAG.clone()`).
- WDB-428 is **local** in println.
- WDB-385 is `f32::MAX` assoc path.

**What became unnecessary:** `GRID.clone()` in station VoxelGrid construction.

**Gates:** `CARGO_TARGET_DIR=…/agent-tdd-wdb407` → `wdb434_` — 1 passed / 1 failed (isolate GREEN, tip RED).

**Do not steal:** WDB-406/408/411/433, P3.508–P3.612, WDB-412–434 (filed).

## P3.611 (2026-10-03) — if-without-else trailing `match` must `return match`

`wj-todo-cli` `parse_edit_branch`: trailing `match parse_id(...) { Ok => Ok, Err => Err }`
inside `if verb == "edit"` (no else) emitted a value `match` → E0308 (`if` expects `()`).

| Gate | Status |
|------|--------|
| `if_block_match_err_arm_must_compile_as_function_return` | ✅ tip GREEN — `return match` |
| `if_block_match_err_arm_multipass_must_cargo_check` | ✅ tip GREEN — multipass cargo-check |

**Root cause layer:** coercion/encoding (statement emit) — void-block value `match`/`if let`
that unifies with the function return type must be an explicit `return match`.

**What became unnecessary:** expect_err RED gate; rustc “you might have meant to return”
as the only path for if-without-else Result unify.

**Gates:**
```bash
export CARGO_TARGET_DIR=…/.agent-wip/cargo-target-p3610
cargo test --release --test all -- bug_if_block_match_err_arm_must_return_test
```
→ 2 GREEN.

## P3.610 (2026-10-03) — blackboard `set_bool` must demote with `find_index(&str)`

Same-impl AST still said `find_index(key: string)` (Owned) after codegen emitted
`find_index(key: &str)`, so `set_bool` kept Owned and call sites got
`"__cond_alive".to_string()`.

| Gate | Status |
|------|--------|
| `test_blackboard_set_bool_literal_not_to_string` | ✅ tip GREEN — `set_bool(&str)`, bare lit |
| `test_hashmap_cast_key_auto_borrow` | ✅ tip GREEN (paren-tolerant + idiomatic `"Unknown"`) |
| `test_no_spurious_deref_on_for_loop_tuple` | ✅ tip GREEN — tip-truth `dirty.push(*pos)` under `&map` |

**Root cause layer:** signature / formal encoding — emitted shared-ref
(`emitted_rust_ref_params` / `callee_emits_shared_rust_ref_param`) beats same-impl
AST Owned for sibling owning-use and `pub_module_api_keeps_owned_string_formal`.

**What became unnecessary:** treating same-impl AST Owned as owning when the sibling
already emits `&str`; false RED on hashmap cast paren form / W0006 `.to_string()`.

**Gates:**
```bash
export CARGO_TARGET_DIR=…/.agent-wip/cargo-target-p3610
cargo test --release --test all -- test_blackboard_set_bool_literal_not_to_string \
  test_hashmap_cast_key_auto_borrow test_no_spurious_deref_on_for_loop_tuple \
  bug_if_block_match_err_arm_must_return_test
```
→ 5 GREEN (with peers).

## P3.609 (2026-10-03) — discard-only `Vec<u8>` formal must stay Owned

`Registry::add(data: Vec<u8>) { let _ = data }` demoted to `data: &Vec<u8>` then
forced `add(..., &vec![…])` (engine component_storage E0308 when formal stayed Owned).

| Gate | Status |
|------|--------|
| `owned_vec_u8_literal_must_not_borrow_at_call_site` | ✅ tip GREEN — `data: Vec<u8>`, bare `vec![…]` |

**Root cause layer:** signature / formal encoding — discard-only Vec stubs are FFI/
API contracts (same class as unused Vec keep-Owned); do not treat `let _ = data` as
readonly `.len()` demotion.

**What became unnecessary:** call-site `&vec![…]` peels for Owned Vec formals that only
suppress unused warnings.

**Gates:**
```bash
export CARGO_TARGET_DIR=…/.agent-wip/cargo-target-p3597
cargo test --release --test all -- owned_vec_u8_literal_must_not_borrow
```
→ 1 GREEN.

## P3.608 (2026-10-03) — flaky `codepoints.remove(&pos)` vs `remove(pos)` (homonym)

`let mut codepoints = ….collect::<Vec<char>>()` never recorded a receiver type
(turbofish ignored in `infer_expression_type`), so `codepoints.remove` took the
no-receiver `pick_codegen_refreshed` path over HashMap-ordered `*::remove`
candidates — tip flaked Owned usize vs Borrowed `&K`.

| Gate | Status |
|------|--------|
| `test_vec_remove_with_expression_no_ref` | ✅ tip GREEN — stable `remove(pos)` (30/30 tip emits) |
| `test_vec_remove_usize_no_ref` / `codegen_vec_remove_usize` | ✅ tip GREEN |
| lib `map_key_lookup_with_non_map_receiver_type_name` | ✅ unknown-receiver `remove` not map-key |
| P3.599 `int_formal_must_not_upgrade…` | ✅ tip GREEN (`use std::strings` fixture) |

**Root cause layer:** signature / constraint — turbofish → local binding type;
fail-closed no-receiver pick when first-arg ownership conflicts; map-key
consensus must not fire for conflicting methods with unknown receiver.

**What became unnecessary:** nondeterministic HashMap scan of `*::remove` for
call-site ownership; treating tip `remove(&pos)` as intermittent flake.

**Gates:**
```bash
export CARGO_TARGET_DIR=…/.agent-wip/cargo-target-p3597
cargo test --release --test all -- test_vec_remove_with_expression_no_ref \
  test_vec_remove_usize_no_ref int_formal_must_not_upgrade
```
→ 4 GREEN.

## P3.607 (2026-10-03) — explicit `let ri = &i` → usize must `*ri as usize`

`let ri = &i; compute(v, ri)` emitted `ri as usize` because later usize formals
overwrote `local_var_types["ri"]` with `usize`. Blanket Ref→Owned Deref also
broke for-loop Copy keys (`dirty.push(*pos)`).

| Gate | Status |
|------|--------|
| `test_mixed_coercion_multiple_args` | ✅ tip GREEN — `*ri as usize` |
| `test_fn_arg_string_literal_to_borrowed` | ✅ tip GREEN — tip-truth Owned unused pub free-fn `string` |
| `test_no_spurious_deref_on_for_loop_tuple` | ✅ tip GREEN (P3.610 tip-truth: `*pos` under `&self`+`&map`) |
| lib `borrowed_copy_tuple_to_owned_strips_or_derefs` | ✅ GREEN — StripBorrow (emit-path deref for explicit ref lets) |

**Root cause layer:** constraint/solver (actual SafetyType) + coercion/encoding —
skip use-site int-width overwrite for ref RHS; track `explicit_ref_let_bindings`;
usize cast deref only when `identifier_already_ref` (explicit ref lets).

**What became unnecessary:** `ri as usize` without deref on explicit `&T` lets;
blanket compute_coercion Deref (reverted — for-loop Copy false positives).

**Gates:**
```bash
export WJ_BINARY=$CARGO_TARGET_DIR/release/wj
cargo test --release --test all -- test_mixed_coercion_multiple_args \
  test_fn_arg_string_literal_to_borrowed test_vec_remove_with_expression_no_ref \
  library_multipass_owned_string_to_string_method_must_borrow \
  test_passthrough_collision_preserves_mut
```
→ 5 GREEN (vec_remove flake closed in P3.608).

## P3.606 (2026-10-03) — pub method string → borrowed-text callee must demote `&str`

`AssetLoader::load(path)` with only `strings::len(path)` kept Owned `String`
because `pub_module_api_keeps_owned_string_formal` treated any call-arg +
non-text return as keep-owned. Multipass callers then `.clone()` reused paths.

| Gate | Status |
|------|--------|
| `test_library_multipass_owned_string_to_string_method_must_borrow` | ✅ tip GREEN — `path: &str`, no clone |
| logger / bare lit / custom method string peers | ✅ still GREEN |

**Root cause layer:** signature (pub keep-owned guard) — exclusive forwards into
borrowed-text / shared-ref callees must demote.

**What became unnecessary:** Owned `load(path: String)` + `path.clone()` when
the only use is `strings::len(&path)`.

**Gates:** `WJ_BINARY=tip cargo test --release --test all -- library_multipass_owned_string_to_string_method_must_borrow regression_logger_owned_passthrough test_custom_method_string_param bare_string_literal_into_owned_string_method` → 4 GREEN.

## P3.605 (2026-10-03) — pure-forward MutBorrowed passthrough must emit `&mut T`

`wrapper(grid)` → `do_clear(grid: &mut VoxelGrid)` analyzer-correct MutBorrowed was
emitted as shared `grid: &VoxelGrid` because the pure-forwarding-delegate formal
path always used `borrowed_formal_rust_type_for_param` (shared `&T`) whenever the
callee was any borrow, including MutBorrowed.

| Gate | Status |
|------|--------|
| `test_passthrough_collision_preserves_mut` | ✅ tip GREEN — `wrapper(grid: &mut VoxelGrid)` |
| `test_method_collision_does_not_suppress_mutation_inference` | ✅ still GREEN |
| `passthrough_qualified_call_*` / `bug_mut_reborrow_*` (related) | ✅ still GREEN |

**Root cause layer:** coercion/encoding (formal emit) — pure-forward borrow emit
must honor MutBorrowed / mut-borrowing callee signatures, not force shared `&T`.

**What became unnecessary:** shared-`&T` demotion of mut-passthrough wrappers that
already had analyzer MutBorrowed + MutBorrowed callee formals.

**Gates:**
```bash
export CARGO_TARGET_DIR=…/.agent-wip/cargo-target-p3597
export WJ_BINARY=$CARGO_TARGET_DIR/release/wj
cargo test --release --test all -- method_collision_mutation passthrough_qualified bug_mut_reborrow
```
→ 6 GREEN.

**Do not steal:** WDB tip-outs / notes_api mut-query product tip-outs.

## P3.604 (2026-10-03) — tip-truth: push lit String::from / contains `&String`

| Gate | Status |
|------|--------|
| `test_push_string_literal` | ✅ tip GREEN — `String::from("hello")` or `.to_string()` |
| `test_passthrough_to_string_ref_function` | ✅ tip GREEN — `id: &String` for `Vec<String>::contains` |

**Root cause layer:** none in compiler — gate truth catch-up.

**Gates:** `cargo test --release --test all -- test_push_string_literal test_passthrough_to_string_ref_function` → 2 GREEN.

## P3.603 (2026-10-03) — MutBorrowed self must not freeze analysis-only leaf

`Logger::info("a")` (passthrough → `log` → `push`) stayed bare `"a"` into
`message: String` for out-of-tree / fixture `build_project` (TempDir). IR owned
`String::from("a")`, then reconcile refresh picked analysis-only
`info` Borrowed + `Reference(str)` because `pick_codegen_refreshed_signature`
froze the first MutBorrowed-self candidate and skipped later emit-[false,false]
Owned refresh.

| Gate | Status |
|------|--------|
| `regression_logger_owned_passthrough` | ✅ tip GREEN — `"a".to_string()` / `String::from` |
| out-of-tree passthrough `logger.info("a")` | ✅ tip GREEN |
| `test_custom_method_string_param` / bare lit / string_param peers | ✅ still GREEN |
| spawn / mpsc | ✅ still GREEN |
| lib `pick_mut_self_codegen_owned_string_beats_analysis_only_borrowed_leaf` | ✅ GREEN |

**Root cause layer:** signature — `pick_codegen_refreshed_signature` MutBorrowed
bucket must upgrade when a later candidate has codegen refresh / owned emission.

**What became unnecessary:** reconcile peels of `String::from("a")` driven by a
stale analysis-only Borrowed leaf selected over emit-owned `Logger::info`.

**Gates:**
```bash
export CARGO_TARGET_DIR=…/.agent-wip/cargo-target-p3597
cargo test --release --lib -- pick_mut_self_codegen_owned_string_beats_analysis_only_borrowed_leaf
cargo test --release --test all -- regression_logger_owned_passthrough \
  test_custom_method_string_param bare_string_literal_into_owned_string_method \
  string_param_passed_to_owned_method bug_thread_spawn_closure_must_not_be_ref \
  bug_mpsc_sync_channel_boundary
```
→ lib + 8 integration GREEN.

**Do not steal:** WDB tip-outs / notes_api mut-query product tip-outs.

## P3.602 (2026-10-03) — codegen-refreshed Owned beats analysis-only bare leaf

`Logger::info("a")` / builder string-lit call sites peeled `String::from("a")`
back to bare `"a"` when `mc_select` / `contract_sig` preferred an analysis-only
bare leaf (`info` Borrowed + `Reference(str)`, no emit flags) over the
codegen-refreshed `Logger::info` Owned String formal.

| Gate | Status |
|------|--------|
| multipass `Logger::info("a")` → `"a".to_string()` | ✅ tip GREEN |
| `test_custom_method_string_param` | ✅ tip GREEN |
| `bare_string_literal_into_owned_string_method_must_auto_own` | ✅ tip GREEN — Into or owned |
| `string_param_passed_to_owned_method_should_compile` | ✅ tip GREEN — Into + `.into()` |
| `test_stored_param_infers_owned` | ✅ tip GREEN — String or Into |
| builder / pretty / dispatch / spawn / mpsc peers | ✅ still GREEN |

**Root cause layer:** signature — prefer codegen-refreshed method registry /
`fallback_sig` over analysis-only bare-leaf stubs; `mc_select` must not let
`prefer_converged_over_stub` / `converged_has_reference_params_over_bare` pick
stale Borrowed `Reference(str)` when local has emit flags + Owned.

**What became unnecessary:** dual-oracle `contract_sig` rebuild that overwrote
IR/`mc_resolve` with bare `resolve_method_function_signature`; global upgrade
paths that ignored `codegen_refreshed_beats_analysis_only` /
`emitted_owned_beats_stale_global_borrow`.

**Gates:**
```bash
export CARGO_TARGET_DIR=…/.agent-wip/cargo-target-p3597
cargo test --release --test all -- bare_string_literal_into_owned_string_method \
  test_stored_param_infers_owned test_custom_method_string_param \
  string_param_passed_to_owned_method multipass_owned_builder_string_formals \
  cross_crate_pretty_body_without_own reused_owned_string_second_callee \
  notes_api_dispatch_from_mut_handle json_tostring_note_must_not_mut_borrow \
  ui_builder_string_formal
```
→ 12 GREEN.

**Do not steal:** WDB tip-outs / notes_api mut-query product tip-outs.

## P3.601 (2026-10-03) — pub free-fn builder forward must emit `impl Into<String>`

`render_grid(left_html, …)` → `Tile::value_html(left_html)` stayed `String` so
`render_grid("$1", …)` failed cargo-check. Gaps + scope:

1. Nested `if ok { a + b }` made `param_all_call_sites_are_method_or_call_args`
   return false (unused nested blocks reset `saw`).
2. Text AST formals were excluded from owned-method detection.
3. `keep_concrete` blocked Into even when builder-forward applied; non-text
   `dispatch(note: Note)` must not take the Into path.
4. Builder-forward Into requires owning **MethodCall** args only — free-fn
   passthrough (`format_body` → `pretty`, `twice` → `consume`) stays `String`.

| Gate | Status |
|------|--------|
| `multipass_owned_builder_string_formals_must_not_be_ref_string` | ✅ tip GREEN — `impl Into<String>` + `.into()` |
| `notes_api_dispatch_from_mut_handle_must_not_be_owned` | ✅ tip GREEN — `note: Note` (not Into) |
| `cross_crate_pretty_body_without_own` / `json_tostring_note` / `twice` | ✅ tip GREEN — no over-Into |
| `ui_builder_string_formal` / todo_cli validate | ✅ still GREEN |
| spawn / mpsc | ✅ still GREEN |

**Root cause layer:** signature / formal encoding — pub free-fn Into forward via
AST-owned **method** builder slots + vacuous nested-block walk; text gate;
MethodCall-only owning for builder-forward.

**What became unnecessary:** keep_concrete blocking true builder-forward Into;
text exclusion from owned AST formals; false-negative `all_sites` on unused
`if` arms; free-fn Call ownership driving pub Into.

**Gates:**
```bash
export CARGO_TARGET_DIR=…/.agent-wip/cargo-target-p3597
cargo test --release --test all -- multipass_owned_builder_string_formals_must_not_be_ref_string \
  notes_api_dispatch_from_mut_handle_must_not_be_owned \
  cross_crate_pretty_body_without_own json_tostring_note_must_not_mut_borrow_query \
  reused_owned_string_second_callee_must_not_borrow \
  todo_cli_cross_crate_validate_field_must_auto_borrow \
  ui_builder_string_formal_must_emit_impl_into_string
```
→ 8 GREEN.

## P3.600 (2026-10-03) — `retain`/`filter` Copy compare must deref `&T` param

`Vec<i64>::retain(|id| id != entity_id)` emits `id != entity_id` (E0277). Params
are marked in `borrowed_iterator_vars`, but XOR Copy deref required
`side_is_copy(left)` and infer often misses untyped closure params.

| Gate | Status |
|------|--------|
| `test_retain_closure_deref_i64` | ✅ tip GREEN — `*id != entity_id` |
| `test_retain_closure_deref_string` | ✅ still GREEN |
| `hashmap_i64_for_in_key_eq_owned_must_auto_deref` | ✅ still GREEN |

**Root cause layer:** coercion/encoding — comparison XOR also trusts Copy *peer*
when the borrowed closure param lacks a typed local.

**What became unnecessary:** product `*id` rewrites in hierarchy/scene retain.

**Gates:**
```bash
cargo test --release --test all -- test_retain_closure_deref_i64 \
  test_retain_closure_deref_string hashmap_i64_for_in_key_eq_owned
```
→ 3 GREEN.

**Do not steal:** WDB tip-outs / notes_api mut-query product tip-outs.

## P3.594 (2026-10-03) — `Arc<AtomicI64>` Counter must auto-derive Clone

Product `wj-sync` `Counter { inner: Arc<AtomicI64> }` skipped `#[derive(Clone)]`
because Arc peel only exempted `Mutex`/`RwLock`, not atomics — bare `AtomicI64`
correctly blocks Clone, but `Arc<Atomic*>` is always Clone. Product tests reuse
`c` → codegen `.clone()` → E0599.

| Gate | Status |
|------|--------|
| `sync_counter_inc_must_borrow_or_be_clone` | ✅ tip GREEN — `#[derive(Debug, Clone)]` on Counter |
| `arc_atomic_i64_struct_must_derive_clone` | ✅ tip GREEN |
| `atomic_i64_struct_must_not_derive_clone` | ✅ still GREEN (bare AtomicI64) |

**Root cause layer:** signature / auto-derive — `type_precludes_auto_debug_clone`
Arc payload must skip atomic leaves (same class as Mutex under Arc).

**What became unnecessary:** product Arc-handle churn rewrites; Clone-name heuristics.

**Gates:**
```bash
export CARGO_TARGET_DIR=…/.agent-wip/cargo-target-p3597
cargo test --release --test all -- arc_atomic_i64_struct_must_derive_clone \
  atomic_i64_struct_must_not_derive_clone sync_counter_inc_must_borrow_or_be_clone
```
→ 3 GREEN.

**Do not steal:** WDB tip-outs / WDB-430–433 / P3.595 TDD.

## P3.599 (2026-10-03) — private free-fn must not emit `impl Into<String>` builder forward

Cross-crate `check_nonempty` → `require_nonempty(&field, value)` failed because private
`check_nonempty(field, value)` emitted `impl Into<String>` via
`param_pub_free_string_builder_forward` (no `is_pub` gate). Call-site
`into_string_formal_params` then painted shadowed locals as `value.into()`.

| Gate | Status |
|------|--------|
| `todo_cli_cross_crate_validate_field_must_auto_borrow` | ✅ tip GREEN — `require_nonempty(&field, value)` |
| `ui_builder_string_formal` (3) | ✅ still GREEN (pub builders keep Into) |
| `handle_forward` / `handle_request` empty-lits | ✅ still GREEN |
| `wdb228` push_str Into | ✅ still GREEN |

**Root cause layer:** signature / formal encoding — Into builder-forward is a **pub**
Rust boundary encoding only.

**What became unnecessary:** peels for `value.into()` on private own→validate forwards;
name-heuristic ownership for `require_nonempty`.

**Gates:**
```bash
export CARGO_TARGET_DIR=…/.agent-wip/cargo-target-p3597
cargo test --release --test all -- todo_cli_cross_crate_validate_field_must_auto_borrow \
  ui_builder_string_formal handle_forward_empty_lits handle_request_empty_lits wdb228
```
→ 8 GREEN.

**Do not steal:** WDB tip-outs / WDB-430–433 / P3.594–595 TDD.

## P3.598 (2026-10-02) — StatusChip builder `label` must emit `impl Into<String>`

windjammer-ui `StatusChip::new` / `.label` owned `string` formals must accept Rust
`&str` via `impl Into<String>`. Tip already Into'd `new` (struct-init payload) but
`label` early-demoted to `&str` because bare text field assign skips `payload_stored`
and counts as readonly use — returning before the late Into upgrade.

| Gate | Status |
|------|--------|
| `ui_builder_string_formal_must_emit_impl_into_string` | ✅ tip GREEN — `label: impl Into<String>` (`new` concrete String per P3.710) |
| `hexagonal_ui_builder_string_formal_must_emit_impl_into_string` | ✅ tip GREEN |
| `ui_builder_string_formal_rust_str_call_site_must_cargo_check` | ✅ tip GREEN — `.new("paid".to_string()).label("Paid")` |
| notes-api `handle_forward` / `handle_request` empty-lits | ✅ tip GREEN (no free-fn Into on `&str` forwards) |
| WDB-357 MultiFile | ✅ tip GREEN (P3.710) |
| WDB-357 tip-out/gen | ✅ tip GREEN (P3.710 tip-out regen) |

**Root cause layer:** formal encoding — Into eligibility for Self-returning builders
with bare text field assign; emit before early `&str` demotion. Free-fn Into only via
`param_pub_free_string_builder_forward` (owned forward sites).

**What became unnecessary:** hand-patched `generated/authfetch.rs` Into formals for
StatusChip-style builders once tip regen lands; broad free-fn “any call-arg → Into”
branch (would regress `handle_request(method)` → `handle(method: &str)`).

**Also lands (same tip):** P3.590 `expression_has_concrete_wj_int_i64_peer` + P3.591
usize while-compare demotion guard (product `$WJ test` GREEN on tip p3589c).

**Gates:**
```bash
export CARGO_TARGET_DIR=…/.agent-wip/cargo-target-p3589c
cargo test --release --test all -- ui_builder_string_formal i64_bitops_into_u8_push \
  usize_loop_counter_init_zero wdb357_module_file_owned_string handle_forward_empty_lits \
  handle_request_empty_lits wdb110_tip_isolate wdb110_same_file add_condition_string_literal \
  codegen_component_library_regen
```
→ ui_builder 3/3, i64/usize, handle_*, wdb110, add_condition, component regen 6/6 GREEN;
WDB-357 tip-out still RED (pre-existing gen lag).

**Do not steal:** WDB tip-outs / WDB-430–433 / P3.595–597 TDD.

## P3.597 (2026-10-02) — HashMap i64 for-in key `==` owned int must auto-deref

Product `wj-todo-cli`: `for (k, v) in self.items { if k == id }` with
`HashMap<int, Todo>` → E0277 (`&i64 == i64`). Rust for-in yields `&K`.

| Gate | Status |
|------|--------|
| `hashmap_i64_for_in_key_eq_owned_must_auto_deref` | ✅ tip GREEN — `*k == id` |

**Root cause layer:** codegen / for-in — map `Parameterized` extract yields `(K, V)`;
borrowed for-in wraps as `(&K, &V)` not `&(K, V)` so XOR Copy-key deref applies.

**What became unnecessary:** rewriting todo complete/remove with `*k` or get-only APIs.

**Gates:** tip emit `for (k, v) in &store.items` + `*k == id`; cargo-check GREEN.
- `cargo test --test all --features integration_tests,codegen_tests -- hashmap_i64_for_in_key_eq_owned_must_auto_deref` → GREEN

**Do not steal:** WDB-406/408/411/430–433, P3.508–P3.596, WDB-412–433 (filed).

## P3.596 (2026-10-02) — ambient `u16` must not paint int into `ServerResponse::new` i64

Product `wj-proxy` `base_response(status: u16, …)`: `let code = 404` /
`status as int` / `let status = 500` stay `u16` into `ServerResponse::new(…: i64)`
→ E0308. Runtime formal is `i64` (not u16).

| Gate | Status |
|------|--------|
| `u16_ambient_int_lit_into_server_response_new_must_be_i64` | ✅ tip GREEN — multipass `404_i64` / `500_i64` |
| `server_response_new_int_literal_must_coerce_to_u16` | ✅ still GREEN (bare int → int formal) |
| `if_int_status_into_u16_formal_must_coerce` | ✅ still GREEN |
| `server_response_new_status_is_i64` (lib) | ✅ tip GREEN — runtime registry i64 |

**Root cause layer:** signature — WJ `std/http.wj` `ServerResponse::new`/`error`/`binary`
were `status: u16` while runtime takes `i64`. Multipass preferred the WJ stub, so
`let_binding_int_width_from_later_call_formals` painted `_u16`. Flat isolate already
saw runtime i64 (false-GREEN).

**What became unnecessary:** product proxy status-type rewrites; ambient-u16 peels
once WJ stubs match runtime `int` formals (`status as u16` into the u16 field).

**Gates:**
```bash
export CARGO_TARGET_DIR=…/.agent-wip/cargo-target-p3597
cargo test --release --test all -- u16_ambient_int_lit_into_server_response \
  server_response_new_int_literal_must_coerce if_int_status_into_u16_formal
cargo test --release -p windjammer --lib server_response_new_status_is_i64
```
→ 3+1 GREEN.

**Do not steal:** WDB tip-outs / WDB-430–433 / P3.594–595 TDD.

## P3.595 (2026-10-02) — TDD WDB-433 (DB agent; no compiler src)

Copy `u64` **field** before cast/arith must not `.clone()`; product emits
`total += node.mesh_id.clone() as u64`.

| Gate | Status |
|------|--------|
| WDB-433 MultiFile | ✅ GREEN @ `6b295089` — isolate `total + node.mesh_id` no `.clone()` |
| WDB-433 tip-out | ❌ RED @ `6b295089` — `mesh_id.clone()` in scene_graph_state |

**Root cause layer:** copy / field — Copy fields in arith/cast must not auto-clone.

**Why this is a new class:**
- WDB-431 is field into **insert/push**.
- WDB-432 is **indexed** Copy + cast.
- WDB-423 is indexed Copy into arith (`offsets[i]`).

**What became unnecessary:** `node.mesh_id.clone() as u64` in mesh instance totals.

**Gates:** TDD ran 2026-10-02 — MultiFile GREEN / tip RED; `CARGO_TARGET_DIR=…/agent-tdd-wdb407`

**Do not steal:** WDB-406/408/411/432, P3.508–P3.594, WDB-412–432 (filed).

## P3.594 (2026-10-02) — `counter_inc` must borrow `Counter` (or derive Clone)

Product `wj-sync`: `counter_inc(c: Counter)` stays owned while tests reuse `c`
(codegen inserts `.clone()`). `Counter { Arc<AtomicI64> }` lacked `Clone` →
E0599. Flat Arc-Counter isolates demote `&Counter` + Clone; product multipass
keeps owned formals but now derives Clone (see tip entry above).

| Gate | Status |
|------|--------|
| `sync_counter_inc_must_borrow_or_be_clone` | ✅ tip GREEN (2026-10-03) — Arc Atomic Clone derive |

**Root cause layer:** signature / auto-derive — Arc peel skipped atomics.

**What became unnecessary:** rewriting sync tests with manual handle churn.

**Gates:** see tip P3.594 entry (2026-10-03) → 3 GREEN.

**Do not steal:** WDB tip-outs / WDB-430–433 / P3.595 TDD.

## P3.593 (2026-10-02) — TDD WDB-432 (DB agent; no compiler src)

Indexed Copy element + cast must not `.clone()`; product emits
`children_copy[c].clone() as u64`.

| Gate | Status |
|------|--------|
| WDB-432 MultiFile | ✅ GREEN @ `2c7d248c` — isolate `children[i] as u64` no `.clone()` |
| WDB-432 tip-out | ❌ RED @ `2c7d248c` — `children_copy[c].clone() as u64` in scene_graph_state |

**Root cause layer:** copy / index — indexed Copy before cast must not auto-clone.

**Why this is a new class:**
- WDB-423 is indexed Copy into arith (`offsets[i].clone()`).
- WDB-370 is indexed then `.field.clone()`.
- WDB-431 is struct **field** `material_id.clone()`.

**What became unnecessary:** `children_copy[c].clone() as u64` in scene graph walk.

**Gates:** TDD ran 2026-10-02 — MultiFile GREEN / tip RED; `CARGO_TARGET_DIR=…/agent-tdd-wdb407`

**Do not steal:** WDB-406/408/411/431, P3.508–P3.592, WDB-412–431 (filed).

## P3.592 (2026-10-02) — TDD WDB-431 (DB agent; no compiler src)

Copy `u64` field into insert/push owned formal must not `.clone()`; product emits
`materials.insert(node.material_id.clone())`.

| Gate | Status |
|------|--------|
| WDB-431 MultiFile | ✅ GREEN @ `9dcb583b` — isolate `ids.push(node.material_id)` no `.clone()` |
| WDB-431 tip-out | ❌ RED @ `9dcb583b` — `material_id.clone()` in scene_graph_state |

**Root cause layer:** copy / field — Copy integer fields into owned formals must not auto-clone.

**Why this is a new class:**
- WDB-393 is formal **assign** `x.clone()`.
- WDB-423 is **indexed** `offsets[i].clone()`.
- WDB-430 is Copy **enum** field `binding.binding_type.clone()`.

**What became unnecessary:** `node.material_id.clone()` in material collection.

**Gates:** TDD ran 2026-10-02 — MultiFile GREEN / tip RED; `CARGO_TARGET_DIR=…/agent-tdd-wdb407`

**Do not steal:** WDB-406/408/411/430, P3.508–P3.591, WDB-412–430 / P3.509–P3.573 (filed).

## P3.591 (2026-10-02) — `let mut i` as usize index must not init `0_i32`

Product `wj-find` / `wj-form-parse` emit `let mut i: usize = 0_i32` for
`let mut i = 0` compared to `.len()` → E0308.

| Gate | Status |
|------|--------|
| `usize_loop_counter_init_zero_must_not_be_i32` | ✅ tip GREEN (p3589c product `$WJ test`) — product `0_i32` |

**Root cause layer:** encoding / int unify — usize-bound loop counters must
emit `0` / `0_usize`, not `0_i32`.

**What became unnecessary:** rewriting find/form loops with explicit casts.

**Gates:** tip p3587 product `wj-find` build → `usize = 0_i32`.
- `cargo test --test all --features integration_tests,codegen_tests -- usize_loop_counter_init_zero_must_not_be_i32`

**Do not steal:** WDB-406/408/411/427–430, P3.508–P3.590, WDB-412–429 (filed).

## P3.590 (2026-10-02) — `int` bitops into `push(… as u8)` must keep i64 masks

Product `wj-uuid` `v1_bytes`: `((clock_seq >> 8) & 0x3F) | 0x80` with later
`out.push(clock_hi as u8)`. Tip paints masks `_i32` while `clock_seq: i64` →
E0277. Same bitops without the `u8` push false-GREEN (`_i64`).

| Gate | Status |
|------|--------|
| `i64_bitops_into_u8_push_must_unify_i64` | ✅ tip GREEN (p3589c product `$WJ test`) — `_i32` masks |

**Root cause layer:** encoding / call-arg int context — `Vec<u8>::push` / `as u8`
must not force bitop peers of WJ `int` to i32.

**What became unnecessary:** rewriting uuid v1 masks as explicit `as int`.

**Gates:** tip p3587 isolate + product `$WJ test` wj-uuid → E0277.
- `cargo test --test all --features integration_tests,codegen_tests -- i64_bitops_into_u8_push_must_unify_i64`

**Do not steal:** WDB-406/408/411/427–430, P3.508–P3.589, WDB-412–429 (filed).

## P3.583b (2026-10-02) — field-move demotion: value vs place position

P3.583 call-arg field-move recursion treated method/index **places**
(`grid.cells.is_empty()`, `vals[i]`) as moves, blocking bare-pass demotion and
forcing `grid.clone()` / `view.clone()` / owned `Vec` formals. Field moves only
in **value** position (return/let/call arg); receivers and index bases borrow.

| Gate | Status |
|------|--------|
| `test_static_readonly_voxelgrid_param_no_clone_*` | ✅ tip GREEN — `grid: &VoxelGrid` |
| `test_library_multipass_graph_csr_view_loop_must_borrow_not_clone` | ✅ tip GREEN |
| `demoted_vec_call_arg_must_not_to_string` | ✅ tip GREEN |
| `borrowed_dense_csr_*` | ✅ tip GREEN |
| P3.583 wal / for_loop / owned_custom gates | ✅ still GREEN |

**Root cause layer:** constraint/demotion write-back — place vs value in
`expr_has_field_move_from_param` (multipass + prepare + passthrough).

**What became unnecessary:** Owned restore / `.clone()` for readonly field method
receivers and index projections.

**Gates:** `CARGO_TARGET_DIR=…/cargo-target-p3583`
- `cargo test --release --test all --` (voxelgrid, csr view loop, demoted_vec,
  dense_csr, path_bytes wal, for_loop match_self, owned_custom, match_binding) → **14 passed**

## P3.583 (2026-10-02) — `&self` + field into Owned Vec/String must `.clone()`

Call-arg field moves (`decode_records(self.bytes)`) blocked bare-pass demotion of
`self` (keep/restore Owned or emit `&self` + clone). IR Clone upgrade for
`self.field` into Owned formals failed closed only for `String`/`Custom` when
field-type inference missed — `Vec<u8>` stayed Identity → E0507 move from `&self`.
Also: `match self` Copy payloads bind as `&i32` (match ergonomics) → peel `*amount`.

| Gate | Status |
|------|--------|
| `path_bytes_wal_layout_rustc_cargo_check` | ✅ tip GREEN — `self.bytes.clone()` into Owned Vec |
| `path_bytes_segment_replay_borrows_bytes_field` | ✅ tip GREEN |
| `dogfood_wal_replay_and_recovered_map_rustc_check` | ✅ tip GREEN |
| `dogfood_wal_replay_to_lsn_call_and_body_copy_lsn` | ✅ tip GREEN — tip-truth: `&str` borrow **or** `String`+`.clone()` |
| `test_for_loop_match_self_enum_borrows_correctly` | ✅ tip GREEN — `*amount > 0_i32` |
| `library_multipass_owned_custom_*` | ✅ tip GREEN (P3.582 + call-arg field-move) |
| `bare_pass_skips_custom_field_call_arg_move_p3583` | ✅ lib GREEN |

**Root cause layer:** coercion/encoding — IR `elem_needs_clone` fail-closed via
`!is_copy_base(expected)`; owned-emission beats stale shared flags; post-IR
`maybe_clone_borrowed_field_for_owned_param` for non-extern Owned formals.
Constraint/demotion: call-arg field-move recursion (multipass + prepare +
passthrough). Match: Copy arm bindings under `match self` stay borrowed → peel.

**What became unnecessary:** bare `self.bytes` / `self.path` moves from `&self`
into Owned Vec/String (E0507).

**Gates:** `CARGO_TARGET_DIR=…/cargo-target-p3583`
- `cargo test --release --test all -- path_bytes dogfood_wal_replay test_for_loop_match_self library_multipass_owned_custom` → **16 passed**

## P3.582 (2026-10-02) — returning `csr.neighbors` must keep owned DenseCsr`

Bare-pass + readonly field-projection demotion treated `fn consume(csr) { csr.neighbors }`
as borrow-only, emitting `csr: &DenseCsr` + `.clone()`. Field **moves** (return /
expression / let) must stay Owned through multipass skip/restore and prepare.

| Gate | Status |
|------|--------|
| `library_multipass_owned_custom_wrapper_keeps_owned_formal` | ✅ tip GREEN |
| `library_multipass_owned_custom_forward_with_field_reads_keeps_owned_formal` | ✅ tip GREEN |
| `library_multipass_owned_custom_self_field_must_clone_not_borrow` | ✅ tip GREEN |
| `bare_pass_skips_custom_field_return_move_p3582` | ✅ lib GREEN |

**Root cause layer:** constraint/demotion write-back — field-move detection now
covers Return/Expression (not only Let); bare-pass skip + prepare
`field_proj_readonly` exclude moves.

**What became unnecessary:** demote+`.clone()` for consuming Custom field returns.

**Gates:** `CARGO_TARGET_DIR=…/cargo-target-p3581`
- `cargo test --release --test all --features integration_tests,codegen_tests -- library_multipass_owned_custom` → **4 passed**

## P3.581 (2026-10-02) — `NoteStore::get(id)` must not inherit HashMap::get `&K`

Multipass `store.get(&id)` into `NoteStore::get(id: i64)` E0308. Field site
`self.notes.get(&id)` was already correct; consumer files treated unregistered
`NoteStore` as map-key consensus.

| Gate | Status |
|------|--------|
| `hashmap_field_get_i64_key_must_auto_borrow` | ✅ tip GREEN — `&id` in store, bare `id` at lookup |
| `map_key_lookup_with_non_map_receiver_type_name` | ✅ lib GREEN — NoteStore + MemoryEngine + wrappers |

**Root cause layer:** signature — `method_is_map_key_qualified` no longer falls
through to HashMap consensus for concrete non-wrapper user types lacking a
registered `Type::get`.

**What became unnecessary:** borrowing Copy `i64` into Owned user `get` formals.

**Gates:** `CARGO_TARGET_DIR=…/cargo-target-p3580b`
- `cargo test --release -p windjammer --lib -- map_key_lookup_with_non_map_receiver`
- `cargo test --release --test all --features integration_tests,codegen_tests -- hashmap_field_get_i64_key_must_auto_borrow`
- `test_hashmap_remove_auto_borrows_key` → tip-truth: allow demoted `&TimerId` + `remove(id)`

## P3.580b (2026-10-02) — runtime scan: `F: Fn(...)` → FunctionPointer (`Server::serve`)

Scanner registered `handler: F` as `Custom("F")` and dropped multi-line `where`
clauses, so `call_arg_expected_type` stayed `None` at serve closures (P3.580
fail-open). Accumulate fn headers through `{` and rewrite Fn/FnMut bounds to
`FunctionPointer` (`FnOnce` → `Custom("FnOnce")`).

| Gate | Status |
|------|--------|
| `parse_serve_fn_bound_handler_as_function_pointer` | ✅ lib GREEN |
| `scanned_runtime_server_serve_handler_is_fn_pointer` | ✅ lib GREEN |
| `server_serve_runtime_signature_handler_is_function_pointer` | ✅ tip GREEN |
| `serve_closure_passes_request_to_handler_without_double_borrow` | ✅ still GREEN |

**Root cause layer:** signature (runtime boundary registry)

**What became unnecessary:** relying solely on P3.580 fail-open when serve's
enclosing formal type was missing (`Custom(F)`).

**Gates:** `CARGO_TARGET_DIR=…/cargo-target-p3580b`
- `cargo test --release -p windjammer --lib -- parse_serve_fn_bound scanned_runtime_server_serve`
- `cargo test --release --test all --features integration_tests,codegen_tests -- serve_closure server_serve_runtime_signature`

## P3.580 (2026-10-02) — `Server::serve` Fn closure: clone outer Owned captures

`serve(|request| handle_request(request, deps))` emitted `move |request| handle_request(request, deps)` → E0507 (`Fn` may invoke many times). `thread::spawn` stays FnOnce (move OK).

| Gate | Status |
|------|--------|
| `serve_closure_passes_request_to_handler_without_double_borrow` | ✅ tip GREEN — `deps.clone()` |
| `bug_thread_spawn_closure_must_not_be_ref` / spawn move / mpsc sync_channel | ✅ still GREEN |

**Root cause layer:** coercion/encoding — `compute_coercion` Identity Owned→Owned upgraded to Clone when the enclosing formal is multi-invoke (`Fn`/`FnMut`/fn-ptr). Flag from `call_arg_expected_type` via `formal_is_multi_invoke_closure_trait` (not callee name). When enclosing formal type is missing at the closure site, fail open to multi-invoke if the closure captures outer vars (safer than move into `Fn`); FnOnce sites register FnOnce/fn-ptr.

**What became unnecessary:** bare move of non-Copy outer captures into Owned formals inside `Fn` handlers.

**Follow-up (signature):** `Server::serve` still leaves `call_arg_expected_type=None` at the closure arg in this fixture — wire FunctionPointer from std `http.wj` so the fail-open path is unused.

**Gates:** `CARGO_TARGET_DIR=…/cargo-target-p3580`
- `cargo test --release --test all --features integration_tests,codegen_tests -- serve_closure_passes_request_to_handler_without_double_borrow bug_thread_spawn_closure_must_not_be_ref bug_mpsc_sync_channel_boundary_signature thread_spawn_move_keyword_must_be_preserved` → **7 passed**

**Do not steal:** WDB tip-outs / WDB-430.

## P3.579 (2026-10-02) — tip-truth: read-only Custom / &str demote is consistent

Suite FAILs required owned `MultipartBody` / `VoxelGrid` formals and
`"renamed".to_string()` into demoted `with_label(&str)` while tip correctly
demotes read-only formals with matching call sites.

| Gate | Status |
|------|--------|
| `owned_struct_arg_must_not_emit_ampersand_at_call` | ✅ tip GREEN — allow consistent `&` demote |
| `test_owned_voxelgrid_param_not_auto_borrowed` | ✅ tip GREEN — allow consistent `&` demote |
| `test_string_literal_in_chained_method_call` | ✅ tip GREEN — bare lit into demoted `&str` |
| `string_literal_method_coercion_test` (3) | ✅ tip GREEN — setter demote to `&str` |

**Root cause layer:** none in compiler — gate truth catch-up.

**What became unnecessary:** treating consistent read-only demotion as RED.

**Gates:** `CARGO_TARGET_DIR=…/cargo-target-p3577`
- `cargo test --release --test all -- owned_struct_arg_must_not_emit_ampersand test_owned_voxelgrid_param_not_auto_borrowed test_string_literal_in_chained_method_call string_literal_method_coercion_test`

## P3.578 (2026-10-02) — u32 `while i < half` after `half = n / 2` (P3.348 paint)

`let half = n / 2` with `n: u32` was recorded as WJ `int` (untyped lit peer), so
`let mut i = 0` stayed `_i64` while `half` emitted `_u32` → E0308. Frame
`pixel_count() -> u32` peer already worked.

| Gate | Status |
|------|--------|
| `u32_while_counter_vs_bound_must_not_cast_bound_as_i64` | ✅ tip GREEN — paint + frame |
| P3.577 usize start-before-i / auth int_len / int_while_le | ✅ still GREEN |

**Root cause layer:** constraint/type write-back — Binary `u32 / lit` keeps u32
in let + expression inference; while-compare peer resolves Identifier bounds
through let RHS / params for u32 width.

**What became unnecessary:** casting `half as i64` / leaving `i` as i64 against
u32 bounds.

**Gates:** `CARGO_TARGET_DIR=…/cargo-target-p3577`
- `cargo test --release --test all -- u32_while_counter_vs_bound` → GREEN

**Do not steal:** WDB tip-outs / WDB-430.

## P3.577 (2026-10-02) — `start` before `i`: `start = i + 1` must stay usize (`wj-toml`)

P3.326 gate declared `i` before `start` (GREEN). Eco `split_on_commas` declares
`start` first → `0_i64` + `start = i + 1_usize as i64` (E0308 / E0277). Prepass
only back-propagated bare `dst = src` identifier assigns into `usize_variables`.

| Gate | Status |
|------|--------|
| `module_file_usize_i_plus_one_assign_must_stay_usize` | ✅ tip GREEN (P3.326) |
| `module_file_usize_start_before_i_plus_one_assign_must_stay_usize` | ✅ tip GREEN — eco order |
| eco `wj-toml` `split_on_commas` tip cargo-check | ✅ `let mut start: usize` / `start = i + 1_usize` |
| `auth_json_string_field_int_len` / `int_while_le_vec_len` / usize eq-zero | ✅ still GREEN |

**Root cause layer:** constraint/type write-back — prepass marks `start = i + 1`
(usize counter ± non-neg lit) into `usize_variables` regardless of declaration
order. Encoding: `maybe_cast_usize_to_int_target` parenthesizes non-atomic RHSes
so a residual cast cannot become `i + 1_usize as i64`.

**What became unnecessary:** declaration-order dependence for index peers;
unparenthesized `as i64` peel on binary usize assigns.

**Gates:** `CARGO_TARGET_DIR=…/cargo-target-p3577`
- `cargo test --release --test all -- module_file_usize_i_plus_one_assign module_file_usize_start_before_i` → 2 passed
- `… -- auth_json_string_field_int_len_must_unify_i64 int_while_le_vec_len module_file_usize_index_eq_zero` → 3 passed

**Do not steal:** WDB tip-outs / WDB-430 / P3.573 TDD.

## P3.576 (2026-10-02) — MutexGuard HashMap get key borrow (wj-sync SharedMapSI)

`match g.get(key)` after `Mutex::lock` parses as `Call(FieldAccess)`, not
`MethodCall`. Suffix/`*::get` scramble picked an Owned homonym →
`g.get(key.to_string())`. Multipass often leaves `g` untyped so
`contains_key` (map-only, no Vec conflict) also missed the HashMap bridge.

| Gate | Status |
|------|--------|
| `mutex_guard_hashmap_string_key_must_borrow` | ✅ tip GREEN — multipass SharedMap + `g.get(&key)` / `contains_key(&key)` |
| eco `wj-sync` SharedMapSI tip transpile | ✅ `get(&key)` + `contains_key(&key)` |
| `hashmap_get_through_mutex_guard_*` / `module_file_shared_map_get_*` | ✅ still GREEN |

**Root cause layer:** signature bridge — prefer stdlib `HashMap::{get,contains_key}`
(Borrowed `&K`) for MutexGuard / untyped lock guards / SharedMap handles; shared
post-IR collection-key finalize for Call(FieldAccess) + MethodCall.

**What became unnecessary:** MethodCall-only HashMap fallback; bare `::get`
suffix scramble on map Deref wrappers; Owned `contains_key(key.clone())` on
untyped guards.

**Gates:** `CARGO_TARGET_DIR=…/cargo-target-p3575`
- `cargo test --release --test all -- mutex_guard_hashmap_string_key` → GREEN
- eco tip `wj-sync` shared.rs → `get(&key)` + `contains_key(&key)`

**Do not steal:** WDB-406/408/411/429–430, P3.573 TDD, tip-outs.

## P3.575 (2026-10-02) — Vec index reuse into owned let must `.clone()` (wj-csv)

`let headers = rows[0]` then later `rows[i]` / `Ok((headers, data))` emitted
`let headers: Vec<String> = &rows[0]` (E0308). (1) Tuple/Array uses were not
`Moved`. (2) Match-arm expression blocks never scoped `current_function_body`
/ `current_block_local_idx`, so analysis only saw the outer `match`.

| Gate | Status |
|------|--------|
| `vec_index_reuse_must_clone_into_owned_let` | ✅ tip GREEN — param shape |
| `vec_index_reuse_in_match_ok_arm_must_clone_into_owned_let` | ✅ tip GREEN — eco `Ok(rows)` shape |
| eco `wj-csv` `parse_with_headers` | ✅ `rows[0].clone()` |

**Root cause layer:** constraint/usage classification + block scoping for
match-arm bodies (`generate_block_expr` aligned with `generate_block`).

**What became unnecessary:** false “field-only” borrow let for indexed Vec rows
inside `Ok(rows) => { … }` (wj-csv).

**Gates:** `CARGO_TARGET_DIR=…/cargo-target-p3575`
- `cargo test --release --test all -- vec_index_reuse` → 2 passed
- eco tip `wj-csv` lib.rs → `let headers: Vec<String> = rows[0].clone()`

## P3.574 (2026-10-02) — match-arm owned payload reuse must `.clone()` (wj-cron E0382)

`Ok(expr) => { matches_cron(expr); matches_cron(expr) }` moved `CronExpr` twice
without clone. AutoClone analysis registered `Ok(expr)` and scheduled a site,
but codegen blanket-skipped `match_arm_bindings` in `maybe_auto_clone` /
`maybe_auto_clone_call_arg` / `append_clone_for_owned_non_copy_binding`.

| Gate | Status |
|------|--------|
| unit `match_ok_binding_reuse_in_arm_block_needs_clone` | ✅ GREEN |
| `match_arm_owned_binding_reuse_must_clone` | ✅ isolate GREEN (`expr.clone()` + cargo check) |
| WDB-347 Copy f32 match binding (no noise `.clone()`) | ✅ still GREEN |
| spawn / mpsc sync_channel | ✅ still GREEN |

**Root cause layer:** constraint/solver write-back path — AutoClone sites for
pattern defs were correct; codegen honor path was the gap (not a new peel).

**What became unnecessary:** blanket `match_arm_bindings` skip on owned reuse
clone. Narrowed to `copy_match_payload_binding` only (WDB-347 Copy payloads).

**Gates:** `CARGO_TARGET_DIR=…/cargo-target-p3574`
- `cargo test --release --test all -- match_arm_owned_binding_reuse_must_clone` → GREEN
- `… -- match_arm_owned_binding_reuse_must_clone wdb347 bug_thread_spawn_closure_must_not_be_ref bug_mpsc_sync_channel_boundary` → 7 passed

**Do not steal:** WDB-406/408/411/429–430, P3.508–P3.573, tip-outs (filed).

## P3.573 (2026-10-02) — TDD WDB-430 (DB agent; no compiler src)

Copy unit-enum **field** of a local/loop binding must not `.clone()` into an
owned formal; product emits `is_storage_write(binding.binding_type.clone())`.

| Gate | Status |
|------|--------|
| WDB-430 MultiFile | ✅ GREEN @ `80c86faf` — isolate `is_write(binding.binding_type)` no `.clone()` |
| WDB-430 tip-out | ❌ RED @ `80c86faf` — `binding.binding_type.clone()` in shader_graph_builder/compiler |

**Root cause layer:** copy / field — Copy enum fields into owned formals must not auto-clone.

**Why this is a new class:**
- WDB-375 is nested **index clone chain** `].clone().bindings[j].clone().binding_type.clone()`.
- WDB-402 is unit-variant **path** `HostType::F32.clone()`.
- WDB-392 is `Direction::PosX.clone()`.

**What became unnecessary:** `binding.binding_type.clone()` in shader graph scheduling.

**Gates:** TDD ran 2026-10-02 — MultiFile GREEN / tip RED; `CARGO_TARGET_DIR=…/agent-tdd-wdb407`

**Do not steal:** WDB-406/408/411/429, P3.508–P3.572, WDB-412–429 / P3.509–P3.569 (filed).

## P3.572 (2026-10-01) — module-file must emit inline types from `mod.wj`

`wj build path/to/mod.wj --module-file` regenerates `mod.rs` as only
`pub mod` / `pub use` and **drops** inline `pub struct` / `impl` bodies that
lived in the source `mod.wj`. Product: ports `Host.tick_playable` vanished from
`gen/ports/mod.rs` after ports-only tip transpile.

Also re-check P3.562: tip still harvested `/tmp/check_json_len_sig.rs` etc. into
ports-only `--output /tmp/wj_ports_out` (parent `/tmp` siblings).

| Gate | Status |
|------|--------|
| `module_file_mod_wj_inline_struct_must_emit` | ✅ isolate GREEN |
| unit `extract_mod_wj_code_section_keeps_inline_struct` | ✅ GREEN |
| `module_file_output_must_not_import_tmp_sibling_rs` | ✅ tip GREEN (P3.562) |

**Root cause layer:** multipass module-file — `--module-file` ran
`generate_mod_file` twice; first pass merged `_mod_items.rs` then deleted it;
second pass rewrote declarations-only. Preserve `// Code from mod.wj` when
`_mod_items` is absent.

**What became unnecessary:** product move of Host to `ports/host.wj` solely to
survive tip transpile (composition may still prefer a sibling file).

**Gates:**
- `cargo test --release --lib -- extract_mod_wj_code_section_keeps_inline_struct` → GREEN
- `cargo test --release --test all --features integration_tests,codegen_tests -- module_file_mod_wj_inline_struct_must_emit` → GREEN
- `… -- module_file_output_must_not_import_tmp_sibling` → GREEN

**Do not steal:** WDB-406/408/411/427–429, P3.508–P3.571, WDB-412–428 (filed).

## P3.571 (2026-10-01) — MutBorrowed Copy: `x = x + 1` rewrite path must `*x += 1`

`5b4ccf51` fixed signature_bridge (bare Copy early-return skipped MutBorrowed)
and the AST `CompoundOp` emit path. The rewrite of `x = x + 1` → `+=` still
emitted bare `x += 1` on `&mut i64` → E0368. Same deref set as compound formals.

| Gate | Status |
|------|--------|
| `mut_borrowed_bare_copy_int_expects_mut_ref_at_call_site` | ✅ unit GREEN |
| `test_ownership_inference_mut_borrowed` | ✅ tip GREEN (`increment(&mut counter)` + `*x += 1`) |
| rustc on fixture emit | ✅ clean |

**Root cause layer:** signature (prior) + encoding (compound rewrite deref).

**What became unnecessary:** post-IR peels inventing `&mut *counter`; gate false
REDs that accepted broken `&mut *` / missing formal `&mut`.

**Gates:** `CARGO_TARGET_DIR=…/cargo-target-p3569`
- `cargo test --release -p windjammer --lib mut_borrowed_bare_copy_int` → 1 passed
- `cargo test --release --test all --features integration_tests,codegen_tests -- test_ownership_inference_mut_borrowed` → 1 passed
- tip `wj build` mut_borrowed.wj → rustc clean

**Do not steal:** WDB-406/408/411/427–429, P3.508–P3.570, WDB-412–428 (filed).

## P3.588 (2026-10-02) — forwarding_borrow must not force `&String` into owned String emit

P3.586 made `forwarding_borrow_params` beat `emitted_owned_arg_contract` for WAL
Vec facades. AsRef-owned `string` APIs (`run_parquet_load` → `strings::is_empty`)
also set `forwarding_borrow=true` while emitting `String` + `emitted_rust_ref=false`
→ cross-file call sites emitted `&li_path` into `String` (WDB-110/111/112 E0308).

| Gate | Status |
|------|--------|
| `wdb110_tip_isolate_owned_string_clone_must_not_borrow_at_call_site` | ✅ GREEN — `li_path.clone()` |
| `wdb111_multipass_cross_module_owned_string_clone` | ✅ GREEN |
| `wdb112_full_library_multipass_demoted_str` | ✅ GREEN |
| `forwarding_borrow_must_not_borrow_owned_string_emit` | ✅ lib GREEN |
| `dogfood_wal_segment_cross_crate_append_put_borrows_vec_literal` | ✅ no-reg — Vec facade still borrows |
| `handle_forward_empty_lits_must_own` | ✅ no-reg |

**Root cause layer:** signature bridge — `call_site_needs_shared_ref_at_emit`
skips forwarding_borrow when owned WJ text emits `String`.

**What became unnecessary:** `&li_path` / `&path.clone()` into owned String formals
when metadata already records Owned + `emitted_rust_ref=false`.

**Gates:**
- `cargo test --release -p windjammer --lib forwarding_borrow` → **2 passed**
- `cargo test --release --test all --features integration_tests,codegen_tests -- wdb110_ wdb111_ wdb112_ handle_forward_empty dogfood_wal_segment_cross_crate_append_put` → **6 passed**

## P3.587 (2026-10-02) — notes-api `handle` empty-lit owned formals (emitted-owned beats stale borrow)

`App::handle` forwards `origin`/`accept_encoding`/`client_key` into
`inner(…: String)` while registry metadata still reported shared-ref
(`only_borrow=true` / `expect_borrow=true` despite `emitted_owned=true`). Keep-owned
and `pub_module_api` demoted outer formals to `&str` while call sites emitted
`String::new()` → E0308.

| Gate | Status |
|------|--------|
| `handle_forward_empty_lits_must_own` | ✅ isolate GREEN — `origin: String` + `String::new()` |
| `handle_request_empty_lits_hex_app_must_own` | ✅ isolate GREEN |
| `demoted_method_then_owned_empty_lits_must_own` | ✅ isolate GREEN |
| `notes_api_product_remaining_e0308_must_not_emit` | ✅ tip GREEN after P3.587 — empty lits owned; `query.clone()` |
| `test_add_condition_string_literal_not_to_string_for_str_param` | ✅ no-reg — unused method `&str` demote kept |

**Root cause layer:** signature / emission-contract classification —
`param_only_forwards_to_borrowed_text_callees` and `param_call_sites_expect_borrow`
must defer to `param_only_forwards_to_emitted_owned_callees`; pub method APIs
participate in `pub_module_api_keeps_owned_string_formal` for *used* forwards
(unused method formals still demote for literal keys).

**What became unnecessary:** demoting pub method string formals that only forward
into emitted-owned sibling slots while call sites still own empty lits; free-fn-only
`pub_module_api` gate that left `handle` formals as `&str`.

**Gates:**
- `cargo test --release --test all --features integration_tests,codegen_tests -- handle_forward_empty_lits_must_own handle_request_empty_lits_hex demoted_method_then_owned_empty add_condition_string_literal` → **4 passed**
- related filter `handle_forward empty_lits demoted_method string_literal spawn_closure mpsc_sync_channel notes_api_product add_condition` → **78 passed / 0 failed**
- `notes_api_product_remaining_e0308` + WDB-201/203–217 tip cluster + todo_cli validate → **GREEN on tip**

## P3.586 (2026-10-02) — WAL cross-crate `forwarding_borrow` + `replay_all` tip-truth

`WalSegment::append_put` metadata sets `forwarding_borrow_params=[false,true,true]`
with owned Vec emission, but `call_site_needs_shared_ref_at_emit` denied via
`emitted_owned_arg_contract` / bare-Vec before consulting forwarding flags →
`append_put(vec![].clone(), …)` cross-crate. Also: MultiFile `replay_all` with
unused `path` stayed Owned (WDB-152); fixture now matches product readonly use.

| Gate | Status |
|------|--------|
| `dogfood_wal_segment_cross_crate_append_put_borrows_vec_literal` | ✅ GREEN — `&vec![…]` / borrow helper |
| `dogfood_wal_writer_replay_all_records_branch_borrows_path` | ✅ GREEN — `replay_all(path: &str)` + `&self.path` |
| `forwarding_borrow_beats_emitted_owned_vec_contract` | ✅ lib GREEN |

**Root cause layer:** signature bridge — `forwarding_borrow_params` before owned
emission denial in `call_site_needs_shared_ref_at_emit`.

**What became unnecessary:** cross-crate `.clone()` into append_put Vec facades
when metadata already marks forwarding borrow; unused-path Owned fixture mismatch.

**Gates:**
- `cargo test --release -p windjammer --lib forwarding_borrow_beats_emitted_owned`
- `cargo test --release --test all -- dogfood_wal_segment_cross_crate_append_put dogfood_wal_writer_replay_all_records_branch` → 2 passed

## P3.585 (2026-10-02) — index bare-pass restore (wdb-layers hang)

`restore_pub_owned_non_copy_api_formals` rescanned every program body for each
registry×param slot → CPU-bound hang on windjammerdb `wdb-layers` (~1k files,
~16k sigs) during `cargo test --test all` dogfood. Precompute `ProgramLookup`
(free fns / methods / multi-bare targets) once per restore.

| Gate | Status |
|------|--------|
| tip `wj build` wdb-layers `--library --module-file --no-cargo` | ✅ completes ~250s (was hung 40m+ in restore) |
| `multipass_bare_pass_demotion` lib tests | ✅ 15 passed |
| `wdb175_` / `wdb190_` / `wdb216_module_file_owned_ffi` | ✅ GREEN |

**Root cause layer:** constraint/demotion write-back performance (algorithmic index).

**What became unnecessary:** O(sigs×params×programs×bodies) rescans in
`programs_have_multi_callee_bare_probe_for_target` /
`find_function_body_for_registry_key` inside the pub-owned restore loop.

**Gates:**
- `cargo test --release -p windjammer --lib multipass_bare_pass` → 15 passed
- `/usr/bin/time wj build …/wdb-layers/src/mod.wj --library --module-file --no-cargo` → real ~250s

## P3.584 (2026-10-02) — notes-api `dispatch` `&mut self` + Clone; keep WDB-414 owned move

P3.570 tip-truth was RED again: `dispatch(mut self)` from field→Owned consumes
while `handle_method(&mut self)` calls it → E0507. Fixing only Clone under
`&mut self` without a partial-move guard re-broke WDB-414 (`self.scene.clone()`).

| Gate | Status |
|------|--------|
| `notes_api_dispatch_from_mut_handle_must_not_be_owned` | ✅ isolate GREEN |
| `wdb414_module_file_ctor_must_move_self_field` | ✅ isolate GREEN (`initialize(mut self)` moves `self.scene`) |
| WDB-414 tip-out | ❌ stale tip-out / product gen — regen separately |
| `check_rate_field_replace_must_not_move_self` | ✅ GREEN |

**Root cause layer:** constraint/solver + receiver emit (not reconcile peel) —
(1) `function_partial_moves_self_field_then_assigns_other` → owned `mut self`
(WDB-414); other mutating+field-move methods → `&mut self` + Clone into Owned
formals; (2) Owned+consumes early return must sync `inferred_mut_borrowed_params`
when emitting `&mut self`; (3) `record_self_receiver_upgrade("mut self")` must
be Owned, not MutBorrowed; (4) impl pre-pass must not MutBorrowed-upgrade
partial-move methods; (5) IR Clone for nested `self.config.public_base_url` /
field-extract demote must not strip Clone behind `&mut self`.

**What became unnecessary:** forcing owned `dispatch` from field consumes;
`mut self`↔MutBorrowed collapse in the upgrade table; pre-pass `&mut` on
WDB-414-shaped methods.

**Gates:**
- `cargo test --release --test all -- notes_api_dispatch_from_mut_handle wdb414_module_file_ctor_must_move_self_field check_rate_field_replace` → 4 isolate GREEN (tip-out still RED)
- related: `for_loop_match_self library_multipass_owned_custom path_bytes serve_closure` → GREEN

## P3.570 (2026-10-01) — notes-api `dispatch` must not stay owned when called from `&mut handle_method`

Superseded by **P3.584** (receiver + Clone sync). Isolate remains GREEN.

| Gate | Status |
|------|--------|
| `notes_api_dispatch_from_mut_handle_must_not_be_owned` | ✅ isolate GREEN (product-shaped: store.create + `public_base_url`) |

**Do not steal:** WDB-406/408/411/427–429, P3.508–P3.583 tip-outs, WDB-412–428 tip-outs (filed).

## P3.569 (2026-10-01) — TDD WDB-429 (DB agent; no compiler src)

Copy match-arm payloads (`i32`/`f32`/`bool`) must not `.clone()`; product emits
`Some(StateDataValue::Int(value)) => Some(value.clone())` (Float/Bool too).

| Gate | Status |
|------|--------|
| WDB-429 MultiFile | ✅ GREEN @ `9bc7623a` — isolate `Some(value)` no `.clone()` |
| WDB-429 tip-out | ❌ RED @ `9bc7623a` — `value.clone()` in state_machine/state.rs (+ gen) |

**Root cause layer:** copy / match — Copy enum payloads in match arms must copy/deref, not auto-clone.

**Why this is a new class:**
- WDB-393 is formal **field assign** `x.clone()`.
- WDB-367/380 is `None.clone()`.
- WDB-416 is `a.clone().as_float()`.
- WDB-417 is owned **struct** `chunk.clone()` in match/call.

**What became unnecessary:** `value.clone()` on Int/Float/Bool StateDataValue getters.

**Gates:** TDD ran 2026-10-01 — MultiFile GREEN / tip RED; `CARGO_TARGET_DIR=…/agent-tdd-wdb407`

**Do not steal:** WDB-406/408/411/428, P3.508–P3.567, WDB-412–428 / P3.509–P3.567 (filed).

## P3.568 (2026-10-01) — auth `json_string_field`: `strings.len` must unify with `int` counters

Product `wj-auth-api` `tests/auth_test.wj` emits `i = 0_i64` then compares to
`n = strings::len(body)` (usize) and passes bare `j: i64` into `substring` →
E0308. Isolate mirrors the helper (no app reshape).

| Gate | Status |
|------|--------|
| `auth_json_string_field_int_len_must_unify_i64` | ✅ GREEN |

**Root cause layer:** coercion/encoding + let-binding width sync —
(1) `comparison_other_side_needs_len_as_i64` recurses into arithmetic Binary
before usize early-outs so `i + marker_len <= n` casts `n`;
(2) Binary let-typing prefers WJ `int` over usize peers; Identifier lets prefer
int RHS over while-prepass usize marks; (3) `reconcile_ambiguous_int_local_after_let`
keeps i64 when emit has `as i64` / int+usize Binary (was repainting `start`/`j`
as usize after correct Int registration).

**What became unnecessary:** auth-side `as int` / reshape of
`while i + marker_len <= n`; no new ir_call_site peel.

**Gates:**
- `cargo test --release --test all -- auth_json_string_field_int_len_must_unify_i64` → GREEN
- tip emit: `while i + (marker_len as i64) <= (n as i64)` / `j < ((n as i64))` /
  `substring(..., j as usize, ...)`

**Do not steal:** WDB-406/408/411/427–428, P3.508–P3.567, WDB-412–427 (filed).

## P3.567 (2026-10-01) — TDD WDB-428 (DB agent; no compiler src)

Copy local `i32` in println/format args must not `.clone()`; product emits
`println!(..., max_size.clone())`.

| Gate | Status |
|------|--------|
| WDB-428 MultiFile | ✅ GREEN @ `1b97798b` — isolate `println(..., max_size)` no `.clone()` |
| WDB-428 tip-out | ❌ RED @ `1b97798b` — `max_size.clone()` in `.agent-wip/rel_tip_out/voxel/svo64_convert.rs` |

**Root cause layer:** copy / format — Copy locals in format args must not auto-clone.

**Why this is a new class:**
- WDB-393 is local **assign** `x.clone()`.
- WDB-427 is **field** `chunk.size.clone()` into `let`.
- WDB-426 is **const** `u32`.

**What became unnecessary:** `max_size.clone()` in SVO build logs.

**Gates:** TDD ran 2026-10-01 — MultiFile GREEN / tip RED; `CARGO_TARGET_DIR=…/agent-tdd-wdb407`

**Do not steal:** WDB-406/408/411/427, P3.508–P3.566, WDB-412–427 / P3.509–P3.561 (filed).

## P3.566 (2026-10-01) — crate-root `lib.wj` types must not auto-import as `super::lib::`

Multipass `--module-file` with crate-root `lib.wj` inlines types at the crate
root, but siblings that *infer* those types (no explicit `use crate::Item`) get
`use super::lib::Item` → E0432. Product: `wj-migrate` `db_status` /
`Migration`. Explicit `use crate::Item` greened.

| Gate | Status |
|------|--------|
| `rust_use_path_sibling_of_crate_root_lib_wj_uses_crate` | ✅ unit GREEN |
| `module_file_crate_root_type_must_not_import_super_lib` | ✅ isolate GREEN (`e1329d72`) |
| product `wj-migrate` `$WJ test` | ✅ tip GREEN — 18/18 (tip-p3557 21:23) |

**Root cause layer:** boundary / import path — `wj_file_to_module_path` maps
`lib.wj` → `["lib"]`; `rust_use_path_from_module_to_type` must treat that as
crate root (`crate::Type`), not a `lib` submodule.

**Fix (`e1329d72`):** when defining module is exactly `lib`/`main`, emit
`crate::{Type}`.

**What became unnecessary:** adding explicit `use crate::Migration` in
`db_status.wj` solely to dodge bad auto-import paths.

**Ran (2026-10-01):** tip RED `use super::lib::Item` + E0432; after fix
`use crate::Item` + cargo-check GREEN; official isolate **1 passed**.

**Gates:** `CARGO_TARGET_DIR=/tmp/wj-p3566-target`
- `cargo test -p windjammer --lib rust_use_path_sibling_of_crate_root_lib_wj_uses_crate`
- `cargo test --release --test all --features integration_tests,codegen_tests -- module_file_crate_root_type_must_not_import_super_lib`

**Do not steal:** WDB-406/408/411/427–429, P3.508–P3.565/P3.567–P3.569, WDB-412–428 (filed).


## P3.565 (2026-10-01) — runtime std AsRef methods must not force `&String` formals

`strings.substring` / `strings.len` are AsRef/`&str`. Treating them as
`&String` formals blocked adapter `find_char` demotion when the compare was
inline in an `if` (P3.524 product notes/auth).

| Gate | Status |
|------|--------|
| `adapter_inline_substring_eq_find_char_must_be_str` | ✅ tip GREEN (`cfb54aa4`) |

**Root cause layer:** signature — runtime-std AsRef text APIs must not invent
`&String` ownership for demotion.

**What became unnecessary:** reshaping adapters to bind-then-compare so
`find_char` demotes.

**Gates:** `CARGO_TARGET_DIR=…/cargo-target` (landed with fix)
- `cargo test --release --test all -- adapter_inline_substring_eq_find_char_must_be_str`

**Do not steal:** WDB-406/408/411/427, P3.508–P3.564, WDB-412–426 (filed).

## P3.564 (2026-09-30) — tip-truth gates for array / index own / &str setter / if-else

More suite FAILs were tip-correct: `[1_i32, …]`, `(&parts[1]).to_string()`,
demoted `html: &str` + `.to_string()` in setter, `Some(42_i64)` in if-expr.

| Gate | Status |
|------|--------|
| `test_standalone_array_uses_fixed_syntax` | ✅ tip GREEN |
| `test_vec_string_index_in_struct_init` | ✅ tip GREEN |
| `string_param_passed_to_owned_method_should_compile` | ✅ tip GREEN |
| `test_if_else_expression_in_assignment` | ✅ tip GREEN |

**Root cause layer:** none in compiler — gate truth catch-up.

**What became unnecessary:** requiring untyped array lits / `.clone()` only /
owned `String` formals when tip demotes correctly.

**Gates:** `CARGO_TARGET_DIR=…/cargo-target-tip-p3557`
- `cargo test --release --test all -- test_standalone_array_uses_fixed_syntax test_vec_string_index_in_struct_init string_param_passed_to_owned_method test_if_else_expression_in_assignment`

**Do not steal:** WDB-406/408/411/427, P3.508–P3.563, WDB-412–426 (filed).

## P3.563 (2026-09-30) — tip-truth gate catch-up (cast / &str demote / Vec::remove)

Suite FAILs were tip-correct emits rejected by stale asserts:
`20_i32 as f32`, `map.remove(&((key as usize)))`, `parse_twice(&json)` into
demoted `&str`, concat formal demoted to `&str` + bare lit, Vec::remove
re-cast without `&`.

| Gate | Status |
|------|--------|
| `test_multiple_types_with_new_correct_dispatch` | ✅ tip GREEN |
| `test_hashmap_remove_with_cast` | ✅ tip GREEN |
| `cross_module_match_arm_readonly_concat_demotes_to_str` | ✅ tip GREEN |
| `string_literal_coerces_for_owned_string_formal` | ✅ tip GREEN |
| `codegen_vec_remove_usize_test::test_vec_remove_usize_no_ref` | ✅ tip GREEN |

**Root cause layer:** none in compiler — gate truth catch-up.

**What became unnecessary:** treating tip demote/`_i32 as f32`/`&json` as RED.

**Gates:** `CARGO_TARGET_DIR=…/cargo-target-tip-p3557`
- `cargo test --release --test all -- test_multiple_types_with_new_correct_dispatch test_hashmap_remove_with_cast cross_module_match_arm_readonly_concat string_literal_coerces_for_owned test_vec_remove_usize_no_ref`

**Do not steal:** WDB-406/408/411/427, P3.508–P3.562, WDB-412–426 (filed).

## P3.561 (2026-09-29) — TDD WDB-427 (DB agent; no compiler src)

Copy `i32` struct field into local must not `.clone()`; product emits
`let size = chunk.size.clone()`.

| Gate | Status |
|------|--------|
| WDB-427 MultiFile | ✅ isolate GREEN — `let size = chunk.size` (no `.clone()`) |
| WDB-427 tip-out | ❌ product RED — `chunk.size.clone()` in mesh_generator |

**Root cause layer:** copy / field — Copy `i32` field must copy by value into `let`.

**Why this is a new class:**
- WDB-393 is **local** assign `x.clone()`.
- WDB-346 is nested **color.r** field.
- WDB-426 is **const** `u32`.
- WDB-370 is **indexed** Copy field.

**What became unnecessary:** `chunk.size.clone()` in mesh extent loops.

**Gates:** `CARGO_TARGET_DIR=…/agent-tdd-wdb407` (2026-10-01)

**Ran (DB agent 2026-10-01):** worktree `…/wdb407-tdd` @ `cfb54aa4`; `CARGO_TARGET_DIR=…/agent-tdd-wdb407`
- `cargo test --release --test all --features integration_tests,codegen_tests -- wdb427_` → **1 passed / 1 failed**

**Do not steal:** WDB-406/408/411, P3.508–P3.560, WDB-412–426 / P3.509–P3.559 (filed).

## P3.562 (2026-09-29) — module-file must not harvest temp-root sibling `*.rs`

`--output` under the OS temp dir mined `output.parent()` for hand-written `*.rs`
(leftover harness files → `pub mod p3522-test-copy` / E0432). Refuse harvest from
ephemeral temp roots (and gen/build/generated).

| Gate | Status |
|------|--------|
| `module_file_output_must_not_import_tmp_sibling_rs` | ✅ tip GREEN |

**Root cause layer:** boundary / build layout — sibling FFI copy allowlist.

**What became unnecessary:** treating every non-`gen` parent as an FFI source,
including `/tmp` / macOS `…/T`.

**Gates:** `CARGO_TARGET_DIR=…/cargo-target-tip-p3557`
- `cargo test --release --test all -- module_file_output_must_not_import_tmp`

**Do not steal:** WDB-406/408/411/427, P3.508–P3.561, WDB-412–426 (filed).

## P3.560 (2026-09-29) — explicit-clone keep-owned formals get `mut` for `&mut` peel

WDB-161 multipass: `resolve_column(resolver: &mut T)` + `resolve_select` keeps
owned `resolver` via explicit `.clone()`, but call sites peel to `&mut resolver`
(E0596). Explicit-clone keep-owned early-return skipped `auto_needs_mut`;
`param_explicit_clone_targets_mut_borrow_callee` also missed `while` bodies.

| Gate | Status |
|------|--------|
| `wdb161_module_file_clone_method_receiver_must_not_emit_as_ref` | ✅ tip GREEN — `mut resolver` + cargo-check |

**Root cause layer:** constraint/solver write-back — emitted `&mut` slots +
`variable_needs_mut` / clone→mut-borrow walk (not a call-site peel heuristic).

**What became unnecessary:** relying only on the late `auto_needs_mut` path;
clone keep-owned early returns now share the same mut decision. Mut-borrow
clone walk covers While/If/For (aligned with clone detection).

**Gates:** `CARGO_TARGET_DIR=…/cargo-target-tip-p3557`
- `cargo test --release --test all -- wdb161_module_file_clone_method_receiver` → 1 passed

**Do not steal:** WDB-408/411, P3.508–P3.559, WDB-412–426 (filed).

## P3.559 (2026-09-29) — field writeback gate accepts demoted `&Vec` + `&self.queue`

Tip demotes read-only `pop_ready(queue: Vec)` → `&Vec` and passes `&self.queue`
(no `.clone()`, no `mem::take`). Gate still required take/bare-owned — false RED.
Let-extract writeback still emits `mem::take` (no regress).

| Gate | Status |
|------|--------|
| `owned_field_call_writeback_must_mem_take_not_clone` | ✅ tip GREEN — demoted borrow ok |
| `owned_field_let_extract_writeback_still_mem_take` | ✅ tip GREEN — `mem::take` retained |

**Root cause layer:** none in compiler — gate truth catch-up to Vec demotion.

**What became unnecessary:** requiring `mem::take` when the callee formal is demoted.

**Gates:** tip emit verified; suite filter `owned_field_call_writeback` after tip6.

**Do not steal:** WDB-408/411, P3.508–P3.558, WDB-412–426 (filed).

## P3.558 (2026-09-29) — associated `::new` Copy gate accepts `super::extmetric` import

Tip emits `use super::extmetric::Metric` for path-dep UI crates; the gate only
rewrote `crate::extmetric` → cargo-check E0432 (false RED). Call sites already
pass Copy by value (`Metric::new(days)` / `Gauge::new(pct as f64)`).

| Gate | Status |
|------|--------|
| `cross_crate_associated_new_copy_int_must_not_borrow` | ✅ tip GREEN |
| `cross_crate_associated_new_copy_float_cast_must_not_borrow` | ✅ tip GREEN |

**Root cause layer:** none in compiler — gate import rewrite catch-up.

**What became unnecessary:** treating `super::` path-dep import as a borrow regression.

**Gates:** `CARGO_TARGET_DIR=…/cargo-target-tip-p3557`
- `cargo test --release --test all --features integration_tests,codegen_tests -- cross_crate_associated_new_copy`

**Do not steal:** WDB-408/411, P3.508–P3.557, WDB-412–426 (filed).

## P3.557 (2026-09-29) — bare Vec field-assign stays owned; text demotion gates catch up

P3.346/548 made bare `field = param` never count as payload so `string` can demote
to `&str` + `.to_string()`. That also demoted `Vec<String>` (`with_items(items:
&Vec)` + `node.items = items` → E0308). Non-text bare assigns force Owned again;
text bare assigns still skip payload.

| Gate | Status |
|------|--------|
| `test_vec_param_assigned_to_field_stays_owned` | ✅ tip GREEN — `items: Vec<String>` |
| `test_owned_param_in_struct_literal_stays_owned` | ✅ tip GREEN |
| `module_file_demoted_str_field_assign_must_to_string` | ✅ tip GREEN (text demotion retained) |
| `wdb325_module_file_owned_string_sql_exec_*` isolate | ✅ tip GREEN — concat-only `&str` ok |
| `struct_field_into_owned_string_formal_must_not_borrow` | ✅ tip GREEN — demoted `&str` ok |
| `test_param_passed_to_owned_function_stays_owned` | ✅ tip GREEN — wrapper owned or `&Vec` |

**Root cause layer:** constraint/formal demotion — bare assign payload for non-text only.

**What became unnecessary:** `&Vec` demotion on field-store formals that cannot
`.to_string()`-coerce; treating concat-only `&str` / replace-chain demotion as RED.

**Gates:** `CARGO_TARGET_DIR=…/cargo-target-tip-p3557`
- `cargo test --release --test all --features integration_tests,codegen_tests -- test_vec_param_assigned_to_field test_owned_param_in_struct_literal module_file_demoted_str_field_assign wdb325_module_file_owned_string struct_field_into_owned_string test_param_passed_to_owned_function`

**Do not steal:** WDB-408/411, P3.508–P3.556, WDB-412–426 (filed).

## P3.556 (2026-09-29) — index-cast / shadow-name must not poison signed compares to usize

`let d = self.depths[node_idx as usize]` marked `d` as usize because the emitted
RHS substring contained ` as usize` (index cast). Function-wide `usize_variables`
from a later shadowed `let mut ci = 0` + `.len()` also poisoned earlier
`let ci = find_index(...) -> i32` zero sentinels (`ci < 0_usize`).

| Gate | Status |
|------|--------|
| `wdb406_module_file_i32_field_compare_must_not_emit_usize` | ✅ tip GREEN — `d < self.max_depth` |
| `wdb395_module_file_i32_i64_compare_zero_must_not_emit_usize` | ✅ tip GREEN — `ci < 0_i32` |
| `wdb327_module_file_*` (isolates) | ✅ tip GREEN (no regress) |
| `i32_while_len` / `wdb328` isolates / `module_file_timefmt` | ✅ tip GREEN (no regress) |
| `wdb395_tip_out_*` / `wdb328_tip_out_*` / npc tip-out | ❌ stale product gen |

**Root cause layer:** constraint/codegen — emit-width heuristics (not signature / not ir_call_site peel).

**What became unnecessary:** `max_depth as usize` / `0_usize` sentinels from index-cast
substring poison and same-name later usize loop shadows; no new reconcile peel.

**Gates:** `CARGO_TARGET_DIR=…/cargo-target-tip-p3557`
- `cargo test --release --test all --features integration_tests,codegen_tests -- wdb406_module_file_i32_field_compare wdb395_module_file_i32_i64_compare_zero wdb327_module_file i32_while_len wdb328_module_file module_file_timefmt` → **10 passed / 3 failed** (tip-out only)

**Do not steal:** WDB-408/411, P3.508–P3.555, WDB-412–426 (filed). Isolate WDB-406/395 tip GREEN here.

## P3.555 (2026-09-29) — format!/with_capacity gates accept tip FFI + redundant usize cast

Tip wraps `format!` into `string_to_ffi(...)` (no temp extract) and may emit
`capacity as usize` into `Vec::with_capacity` when the formal is already usize.
Gates still required old temp-extract / bare-capacity shapes — false REDs.
Original bugs (`as usize.clone()`, bare format into &str extern) stay forbidden.

| Gate | Status |
|------|--------|
| `test_format_as_function_argument_extracts_to_variable` | ✅ tip GREEN — FFI path ok |
| `test_format_in_method_call_extracts_to_variable` | ✅ tip GREEN |
| `test_multiple_format_calls_in_same_function` | ✅ tip GREEN |
| `test_usize_with_capacity_no_cast_clone` | ✅ tip GREEN — no `.clone()` on cast |

**Root cause layer:** none in compiler — gate truth catch-up.

**What became unnecessary:** requiring temp extract when FFI owns the format String;
rejecting harmless `as usize` without clone.

**Gates:** bare `cargo test --release --test all -- test_format_as_function_argument test_usize_with_capacity` (no feature filter — these use `cfg(not(any(features…)))`).

**Do not steal:** WDB-406/408/411, P3.508–P3.554, WDB-412–426 (filed).

## P3.554 (2026-09-29) — P3.316 fixture: `copy_bytes` must be `impl Pool` method

Isolate gate put `pub fn copy_bytes(self, …)` as a free function → tip emitted
`&mut self` outside `impl` (E0424-class cargo-check fail). Product shape is
`impl Pool { … }`. After wrap: `let mut i: usize` + plain `i += 1` (GREEN).

| Gate | Status |
|------|--------|
| `usize_field_loop_counter_increment_width` | ✅ tip GREEN — impl method + usize loop |

**Root cause layer:** none in compiler — fixture missing `impl`; tip int width OK.

**What became unnecessary:** treating free-fn `self` emit as a usize-loop codegen bug.

**Gates:** `CARGO_TARGET_DIR=…/cargo-target-tip-load-twice`
- `cargo test --release --test all --features integration_tests,codegen_tests -- usize_field_loop_counter_increment_width`

**Do not steal:** WDB-406/408/411, P3.508–P3.553, WDB-412–426 (filed).

## P3.553 (2026-09-29) — accept demotion in owned-Vec / string-formal gates

Tip demotes read-only `Vec` formals to `&Vec` and field-store `string` formals
to `&str` + `.to_string()` (P3.346/P3.548). Legacy gates still required owned
formals + `.clone()` / `"lit".to_string()` — false REDs when emit cargo-checks.

| Gate | Status |
|------|--------|
| `owned_vec_reuse_into_owned_callee_must_clone_not_reborrow` | ✅ tip GREEN — demoted `&Vec` + `&items` ok |
| `string_const_into_owned_string_formal_must_auto_own` | ✅ tip GREEN — demoted `&str` ok |
| `bare_string_literal_into_owned_string_method_must_auto_own` | ✅ tip GREEN — demoted `&str` + bare lit |
| `domain_push_owned_string_literal_via_signature_not_name` | ✅ tip GREEN — demoted `&str` ok |

**Root cause layer:** none in compiler — gate truth catch-up to formal demotion.

**What became unnecessary:** treating demoted `&Vec` / `&str` formals as missing
clone / `.to_string()` peels.

**Gates:** `CARGO_TARGET_DIR=…/cargo-target-tip-load-twice`
- `cargo test --release --test all --features integration_tests,codegen_tests -- owned_vec_reuse string_const_into_owned bare_string_literal_into_owned domain_push_owned_string`

**Do not steal:** WDB-406/408/411, P3.508–P3.552, WDB-412–426 (filed).

## P3.552 (2026-09-29) — WDB-328 neg-init loops stay i64 (not i32 literal peers)

P3.549 small-literal while-peer and an inverted WDB-328 block forced
`let mut i = -1` → `_i32` while bounds stayed `_i64` (or mixed `1_i32` peers).
Negative literal inits must stay WJ `int`; while/compare lits widen to i64.

| Gate | Status |
|------|--------|
| `wdb328_module_file_i64_neg_init_loop_must_not_take_i32_lit_peers` | ✅ tip GREEN |
| `wdb328_module_file_search_state_neg_init_must_not_mix_i64_i32_loop_lits` | ✅ tip GREEN |
| `i32_neg_while_literal_peers_must_not_widen_i64` (MultiFile) | ✅ tip GREEN |
| `i32_while_len_and_literal_bound_must_not_emit_i64` | ✅ tip GREEN (no regress) |
| `module_file_timefmt_product_must_not_mix_i32_month_or_ref_string` | ✅ tip GREEN |
| `wdb328_tip_out_*` / `tip_out_game_core_npc_*` | ❌ stale product gen |

**Root cause layer:** constraint/codegen — neg-init width + while-peer int promotion
(not signature / not ir_call_site peel).

**What became unnecessary:** forcing `-1` → `_i32` in `function_prefers_i32_coord_locals`
builders; i32 while-peer on negative-init counters.

**Gates:** `CARGO_TARGET_DIR=…/cargo-target-tip-load-twice`
- `cargo test --release --test all --features integration_tests,codegen_tests -- wdb328 i32_while_len module_file_timefmt i32_neg_while_literal` → **5 passed / 2 failed** (tip-out only)

**Do not steal:** WDB-406/408/411, P3.508–P3.551, WDB-412–426 (filed).

## P3.551 (2026-09-29) — accept &str demotion in legacy string/self gates

Tip correctly demotes field-assign `string` formals to `&str` + `.to_string()`
(P3.346/P3.548). Legacy analyzer/self/`auto_to_string` gates still required
`String` formal / `"hello".to_string()` at call sites — false REDs.

| Gate | Status |
|------|--------|
| `test_string_param_assigned_to_string_field` | ✅ tip GREEN — String or `&str`+`.to_string()` |
| `test_auto_infer_immutable_self` | ✅ tip GREEN — `mut self` + `&str` ok |
| `test_auto_infer_owned_self` | ✅ tip GREEN |
| `test_chained_method_calls` | ✅ tip GREEN — literal ok for `&str` formal |

**Root cause layer:** none in compiler — gate truth catch-up to text demotion.

**What became unnecessary:** treating demoted `&str` field-assign formals as
regressions vs owned `String`.

**Gates:** `CARGO_TARGET_DIR=…/cargo-target-tip-load-twice`
- `cargo test --release --test all -- test_string_param_assigned_to_string_field test_auto_infer_immutable_self test_auto_infer_owned_self test_chained_method_calls`

**Do not steal:** WDB-406/408/411, P3.508–P3.550, WDB-412–426 (filed).

## P3.550 (2026-09-29) — while literal i32 peer yields to WJ int call formals

P3.549's small-literal → i32 while-peer made `from_epoch_secs` emit
`let mut month = 1_i32; while month <= 12_i32 as i32` despite `days_in_month(year, month: int)`.
Prefer the later int/i64 call-formal width over a literal-driven i32 peer.

| Gate | Status |
|------|--------|
| `module_file_timefmt_product_must_not_mix_i32_month_or_ref_string` | ✅ tip GREEN — `1_i64` / `12_i64` |
| `i32_while_len_and_literal_bound_must_not_emit_i64` | ✅ tip GREEN (P3.350 retained) |
| `i32_loop_arith_and_len_compare_must_not_emit_i64` | ✅ tip GREEN |
| `json_get_index_owned_value_multipass_must_cargo_check` | ✅ tip GREEN (no regress) |

**Root cause layer:** constraint/codegen — signature-driven formal width beats
literal-bound heuristic for while-peer int width.

**What became unnecessary:** i32 month loop + `12_i32 as i32` / `(month as i64)`
into `days_in_month` when an int formal already peers the counter.

**Gates:** `CARGO_TARGET_DIR=…/cargo-target-tip-load-twice`
- `cargo test --release --test all --features integration_tests,codegen_tests -- module_file_timefmt_product i32_while_len_and_literal i32_loop_arith json_get_index_owned_value_multipass` → **4 passed**

**Do not steal:** WDB-406/408/411, P3.508–P3.549, WDB-412–426 (filed).

## P3.549 (2026-09-29) — `json::len` → usize + i32 while-peer loop width

`std/json.wj` declared `len -> int` while runtime returns `usize`, so
`let n = json.len(root)` typed as WJ int / i64 while `get_index` promoted `i` to
`usize` → `while ((i as i64)) < n` (E0308). Signature alignment fixes the compare
without a new reconcile peel. Separately, while-compare peer scan only saw u32, so
`while i < 512` / `while i < len - 1` after `len as i32` kept `0_i64`.

| Gate | Status |
|------|--------|
| `json_get_index_owned_value_multipass_must_cargo_check` | ✅ tip GREEN — `while i < n` (both usize) |
| `json_is_array_len_owned_value_multipass_must_cargo_check` | ✅ tip GREEN |
| `json_get_owned_value_multipass_must_cargo_check` | ✅ tip GREEN |
| `i32_while_len_and_literal_bound_must_not_emit_i64` | ✅ tip GREEN (P3.350) |
| `i32_loop_arith_and_len_compare_must_not_emit_i64` | ✅ tip GREEN (P3.338) |
| `i32_while_compare_must_not_cast_rhs_to_i64` | ✅ tip GREEN |
| `i32_neg_while_literal_peers_must_not_widen_i64` | ✅ tip GREEN (MultiFile) |
| `tip_out_game_core_npc_behavior_neg_while_must_not_split_i64_i32` | ❌ tip-out stale gen (MultiFile GREEN; regen game-core) |

**Root cause layer:** signature (`std/json.wj` `len -> usize`) + constraint/codegen
(while-compare i32 peer → `assignment_int_target_type` / literal-init promote).

**What became unnecessary:** `while ((i as i64)) < n` when `n` from `json::len`;
u32-only while-peer path that blocked i32 loop counters.

**Gates:** `CARGO_TARGET_DIR=…/cargo-target-tip-load-twice`
- `cargo test --release --test all --features integration_tests,codegen_tests -- json_get_index_owned_value_multipass json_is_array_len_owned_value json_get_owned_value_multipass` → **4 passed**
- `cargo test --release --test all --features integration_tests,codegen_tests -- i32_while_len_and_literal_bound i32_loop_arith_and_len_compare i32_while_compare_must_not_cast i32_neg_while_literal` → **5 passed / 1 failed** (tip-out only)

**Do not steal:** WDB-406/408/411, P3.508–P3.548, WDB-412–426 (filed).

## P3.548 (2026-09-29) — demoted `&str` field assign + fail-closed missing set_if

Bare `self.search_query = query` was counted as owned payload store, locking
`query: String` + `.clone()` instead of `&str` + `.to_string()` (P3.346 regression).
Missing cross-crate `set_if` metadata correctly fail-closed; gate updated to expect
`compile_error!` (no invent-MutBorrowed).

| Gate | Status |
|------|--------|
| `module_file_demoted_str_field_assign_must_to_string` | ✅ tip GREEN — `query: &str` + `.to_string()` |
| `test_cross_crate_set_if_mut_borrow_with_split_voxelgrid_import` | ✅ tip GREEN (signature present) |
| `test_cross_crate_set_if_mut_borrow_when_callee_missing_from_metadata` | ✅ tip GREEN — fail-closed |

**Root cause layer:** constraint/formal demotion — bare field assign must not alone
force Owned when the binding is also borrowed; missing registry → fail closed.

**What became unnecessary:** invent-MutBorrowed when `set_if` absent from metadata;
treating bare field-assign as payload lock for text demotion.

**Gates:** `CARGO_TARGET_DIR=…/cargo-target-tip-load-twice`
- `cargo test --release --test all -- module_file_demoted_str_field_assign_must_to_string test_cross_crate_set_if_mut_borrow` → **3 passed**

**Do not steal:** WDB-406/408/411, P3.508–P3.547, WDB-412–426 (filed).

## P3.547 (2026-09-29) — multipass clone gates need non-Copy payloads

`test_match_borrow_break_ref_binding_clones_non_copy_in_tuple` and
`test_tuple_insert_then_reuse_clones_at_insert` were ❌ because tip correctly
skipped `.clone()` for Copy `AssetError` / `SaveData { bytes: i32 }`.

| Gate | Status |
|------|--------|
| `test_match_borrow_break_ref_binding_clones_non_copy_in_tuple` | ✅ tip GREEN — `AssetError::Io(string)` non-Copy |
| `test_tuple_insert_then_reuse_clones_at_insert` | ✅ tip GREEN — `SaveData.label: string` non-Copy |

**Root cause layer:** none in compiler — false RED from Copy fixtures (same class as P3.545).

**What became unnecessary:** treating Copy Err/`SaveData` reuse as a missing clone peel.

**Gates:** `CARGO_TARGET_DIR=…/cargo-target-tip-load-twice`
- `cargo test --release --test all -- test_match_borrow_break_ref_binding_clones_non_copy_in_tuple test_tuple_insert_then_reuse_clones_at_insert` → **2 passed**

**Do not steal:** WDB-406/408/411, P3.508–P3.546, WDB-412–426 (filed).

## P3.546 (2026-09-29) — WDB-081 CSR view loop must borrow, not `view.clone()`

`graph_csr_local_out_degree` / `neighbor_at` take pub Custom `GraphAdjacencyView`,
read fields, and bare-forward into demoted `vertex_index(&view)`. Pub-owned lock
(`is_public_owned_non_copy_formal_api` + `callee_pub_owned_formal_skip_bare_pass`)
kept them Owned → runners emitted `view.clone()` each loop iteration.

| Gate | Status |
|------|--------|
| `test_library_multipass_graph_csr_view_loop_must_borrow_not_clone` | ✅ tip GREEN — `&view` / no `view.clone()` |
| `test_library_multipass_hashmap_i64_set_contains_key_in_triangle_loop` | ✅ tip GREEN |
| WDB-174/178/192 pub Custom owned restore | ✅ tip GREEN (no false demotion) |

**Root cause layer:** signature / multipass ownership — allow demotion when call
sites expect shared-ref and non-call uses are field/index only; registry shared-ref
beats AST Custom-as-owned forward heuristic.

**What became unnecessary:** call-site `view.clone()` peels for readonly CSR helpers
that only field-read + borrow-passthrough into demoted siblings.

**Gates:** `CARGO_TARGET_DIR=…/cargo-target-tip-load-twice`
- `cargo test --release --test all -- library_multipass_wdb_csr` → **2 passed**
- `cargo test --release --test all -- … wdb174 wdb178 wdb192 … csr …` → **12 passed**

**Do not steal:** WDB-406/408/411, P3.508–P3.545, WDB-412–426 (filed).

## P3.545 (2026-09-29) — let-alias auto-clone gate needs non-Copy Message

`test_let_param_alias_then_reuse_clones_at_alias` was ❌ because `Message`
auto-derived `Copy` (enum-only fields), so tip correctly skipped `.clone()`.
Fixture now includes `label: string` so the move/E0382 auto-clone path is tested.

| Gate | Status |
|------|--------|
| `test_let_param_alias_then_reuse_clones_at_alias` | ✅ tip GREEN — `let msg_copy = message.clone()` |

**Root cause layer:** none in compiler — false RED from Copy fixture. Auto-clone
analysis + emit already correct for non-Copy (unit `test_param_let_alias_then_reuse_needs_clone_at_alias`).

**What became unnecessary:** treating Copy Message alias reuse as a missing clone peel.

**Gates:** `CARGO_TARGET_DIR=…/cargo-target-tip-load-twice`
- `cargo test --release --test all -- test_let_param_alias_then_reuse_clones_at_alias` → **1 passed**
- spawn/mpsc → **4 passed** (re-verified)

**Do not steal:** WDB-406/408/411, P3.508–P3.544, WDB-412–426 (filed).

## P3.544 (2026-09-29) — extern `string_to_ffi` must clone String fields

`audio_play_music(self.current_track)` emitted
`string_to_ffi(self.current_track.to_string())` instead of `.clone()`.

| Gate | Status |
|------|--------|
| `test_extern_string_to_ffi_clones_self_field_without_move` | ✅ tip GREEN |

**Root cause layer:** temporary reconcile (FFI wrap) — narrowed always-`.to_string()`
for extern text args so FieldAccess String places use `.clone()`; demoted `&str`
Identifiers still `.to_string()`.

**What became unnecessary:** blanket `.to_string()` on every non-literal FFI string arg.

**Gates:** `CARGO_TARGET_DIR=…/cargo-target-tip-load-twice`
- `cargo test --release --test all -- test_extern_string_to_ffi_clones_self_field_without_move library_multipass_owned_string_to_string_method_must_borrow` → **2 passed**

**Do not steal:** WDB-406/408/411, P3.508–P3.543, WDB-412–426 (filed).

## P3.543 (2026-09-29) — load_twice `path: &mut String` from invent-MutBorrowed

`load_twice(path)` with `loader.load(path)` twice emitted `path: &mut String` +
`load(path)` instead of Borrowed/`&str` or owned + `load(&path)`.

| Gate | Status |
|------|--------|
| `test_library_multipass_owned_string_to_string_method_must_borrow` | ✅ tip GREEN |

**Root cause layer:** constraint/mutation detection — missing-signature fallback
invented MutBorrowed for any lowercase Identifier receiver (`loader` treated as a
module). Also replaced bare `lookup_method(method)` MutBorrowed checks with
`callable_arg_expects_mut_borrow` (typed/unique only).

**What became unnecessary:** invent-MutBorrowed `else if is_lowercase_module…`
branch in `arg_passed_to_mut_borrowed_callee`; bare `registry.lookup_method(method)`
homonym path for method-arg MutBorrowed.

**Gates:** `CARGO_TARGET_DIR=…/cargo-target-tip-load-twice`
- `cargo test --release --test all -- library_multipass_owned_string_to_string_method_must_borrow` → **1 passed**

**Do not steal:** WDB-406/408/411, P3.508/P3.511/P3.513–P3.514/P3.516/P3.518/P3.520/P3.522/P3.524/P3.526–P3.528/P3.530/P3.532–P3.542, WDB-412–426 / P3.509–P3.542 (filed).

## P3.542 (2026-09-29) — e0308 Copy field into owned i32 must not keep `&`

`inv.has_item(item.item_id, item.quantity)` with `quantity: i32` emitted
`has_item(&item.item_id, &item.quantity)` → E0308 (`expected i32, found &i32`).

| Gate | Status |
|------|--------|
| `bug_e0308_borrowed_struct_field_test` | ✅ tip GREEN — `has_item(&item.item_id, item.quantity)` |
| IR/`apply_ir` for quantity | ✅ already Owned/`item.quantity` before finalize |

**Root cause layer:** signature + finalize gate — method-level `method_is_map_key_qualified`
is true when the *first* user arg is Borrowed (Phase-2 demoted `&str` on `has_item`),
so finalize ran string-key normalize on **every** arg and prefixed `&` onto Copy
FieldAccess. Bridge already treated bare Copy formals as Owned.

**What became unnecessary:** false collection-key re-`&` on later Copy field args;
no new reconcile peel (IR path was already correct).

**Gates:** `CARGO_TARGET_DIR=…/cargo-target-tip-p3524`
- `cargo test --release --test all -- bug_e0308_borrowed_struct_field_test auto_to_string_test` → **9 passed**
- `library_multipass_owned_string_to_string_method_must_borrow` → fixed in P3.543

**Do not steal:** WDB-406/408/411, P3.508/P3.511/P3.513–P3.514/P3.516/P3.518/P3.520/P3.522/P3.524/P3.526–P3.528/P3.530/P3.532–P3.541, WDB-412–426 / P3.509–P3.541 (filed).

## P3.541 (2026-09-28) — TDD WDB-426 (compiler tip fix)

Copy `const` `u32` must not `.clone()` on assign; product emitted
`nodes[node_index] = LEAF_FLAG.clone()`.

| Gate | Status |
|------|--------|
| WDB-426 MultiFile | ✅ isolate GREEN — `nodes[idx] = LEAF_FLAG` (no clone) |
| WDB-426 tip-out | ✅ tip-out GREEN — svo64_convert uses bare `LEAF_FLAG` |

**Root cause layer:** copy / identity — `ident_skips_auto_clone_as_copy` now consults
`module_const_type_for_binding` (same WDB-343 path used for call args) so Copy module
consts never auto-clone on assign/reuse.

**What became unnecessary:** `LEAF_FLAG.clone()` when auto_clone marked the const reused.

**Gates:** `CARGO_TARGET_DIR=…/cargo-target-tip-p3524` (2026-09-29)
- `cargo test --release --test all --features integration_tests,codegen_tests -- wdb426_` → **2 passed**

**Do not steal:** WDB-406/408/411, P3.508/P3.511/P3.513–P3.514/P3.516/P3.518/P3.520/P3.522/P3.524/P3.526–P3.528/P3.530/P3.532–P3.540, WDB-412–425 / P3.509–P3.539 (filed).

## P3.539 (2026-09-28) — TDD WDB-425 (DB agent; no compiler src)

Copy `i32` loop var in abs else-branch must not `.clone()`; product emits
`if dx < 0 { -dx } else { dx.clone() }`.

| Gate | Status |
|------|--------|
| WDB-425 MultiFile | ✅ isolate GREEN — `else { dx }` (no `.clone()`) |
| WDB-425 tip-out | ❌ product RED — `dx.clone()` / `dz.clone()` in component_viewer_controls |

**Root cause layer:** copy / if-else — Copy loop `i32` in else of abs must not auto-clone.

**Why this is a new class:**
- WDB-393 is **assignment** `cursor_x = x.clone()`.
- WDB-422 is Copy **f32** index arith.
- This is **if/else abs** of a for-loop Copy binding.

**What became unnecessary:** `dx.clone()` / `dz.clone()` in station pillar ribs.

**Gates:** `CARGO_TARGET_DIR=…/agent-tdd-wdb407` (2026-09-28)

**Ran (DB agent 2026-09-28):** worktree `…/wdb407-tdd` @ `df61b11a`; `CARGO_TARGET_DIR=…/agent-tdd-wdb407`
- `cargo test --release --test all --features integration_tests,codegen_tests -- wdb425_` → **1 passed / 1 failed**

**Do not steal:** WDB-406/408/411, P3.508/P3.511/P3.513–P3.514/P3.516/P3.518/P3.520/P3.522/P3.524/P3.526–P3.528/P3.530/P3.532–P3.538, WDB-412–424 / P3.509–P3.536 (filed).

## P3.538 (2026-09-28) — migrate `prev = -1` must unify with `version: i64`

Product `wj-migrate` `validate_unique_versions`: `let mut prev = -1` then
`prev = m.version` emits `prev = -1_i32` + `version: i64` → E0308.
Compare site casts (`prev as i64`); assignment does not. P3.527 greened
string-scan `i == 0_usize`; this is **negative sentinel vs struct int field**.

| Gate | Status |
|------|--------|
| `migrate_prev_sentinel_must_unify_with_version_i64` | ✅ isolate GREEN — `prev = -1_i64` + `prev = m.version` |
| product `wj-migrate` `$WJ test` | ❌ tip-out pending regen (+ P3.528 move `applied`) |

**Why this is a new class:**
- P3.527 is **eq zero** (`i == 0`) with usize loop counter.
- This is **assign** of `m.version: i64` into a `-1` binding left as `i32`.

**Root cause layer:** int unify / emit-truth — later assign `prev = m.version` (WJ int field) was invisible to mut-local peer scan (u32/i32 only), so `-1` stayed coordinate i32.

**What became unnecessary:** i32 sentinel default when a later field assign peers WJ `int`/`i64` (no reconcile peel).

**Fix:** `mut_int_local_peer_width_from_later_assigns` recognizes WJ int field RHS; let emit uses that peer for `_i64` and clears i32 binding marks.

**Ran (2026-09-28):** tip p3520 —
- tip emit `prev = -1_i64` / `prev >= 0_i64`
- `cargo test --release --test all --features integration_tests,codegen_tests -- migrate_prev_sentinel_must_unify_with_version_i64` → **1 passed**

**Gates:** `CARGO_TARGET_DIR=…/cargo-target-tip-p3520`
- `cargo test --release --test all --features integration_tests,codegen_tests -- migrate_prev_sentinel_must_unify_with_version_i64`

**Do not steal:** WDB-406/408/411, P3.518/P3.522/P3.524/P3.526–P3.528/P3.530/P3.532–P3.537, WDB-412–424 / P3.509–P3.536.

## P3.537 (2026-09-28) — cookie `/build` path-dep: `HashMap.get` lit must stay borrowed

Product-shaped path-dep (`wj_cookie = { path = "…/build" }`, generated `lib.rs`,
**no** `--metadata`): after `parse_cookie_header` → `HashMap`,
`map.get("access_token")` emits `get(String::from("access_token"))` (E0308).
`--metadata` library isolates false-GREEN with bare `get("access_token")`.
Full `wj-auth-api` tip-out: bare get (P3.537) + owned `dispatch(req)` /
resolve_token demote chain (P3.532). Leftover: adapter `find_char(&String)` (P3.524).

| Gate | Status |
|------|--------|
| `--metadata` cookie library isolate | ⚠️ false-GREEN bare `get("access_token")` |
| `cookie_build_path_dep_map_get_must_not_own_key` | ✅ isolate GREEN — bare `get("access_token")` |
| product `wj-auth-api` cookie get | ✅ bare `get("access_token")` on tip p3520 |
| product leftover | ❌ `find_char(&String)` only (P3.524); P3.532 dispatch/resolve GREEN |

**Why this is a new class:**
- Not P3.532 product-only gate — this is the **build/-path-dep without metadata** ABI.
- Same-crate / `--metadata` HashMap.get isolates stay borrowed.
- Distinct from notes-api `qs_get` lit `String::from` (path-dep demoted `&str` formal).

**Root cause layer:** signature — generated-`lib.rs` path-dep recovery omitted `->` return types, so `Ok(map)` stayed untyped and unresolved `map.get("lit")` auto-owned the key.

**What became unnecessary:** unresolved-instance auto-own of HashMap.get lits after metadata-less cookie path-deps (no new reconcile peel / no get-name heuristic).

**Fix:** recover `return_type` from generated Rust (`-> Result<HashMap<…>, …>`) and parse `HashMap`/`BTreeMap` as `Parameterized` so match bindings type as HashMap and stdlib `HashMap::get` Borrowed `&Q` applies.

**Ran (2026-09-28):** tip p3520 —
- `cargo test --release -p windjammer --lib recovers_parse_cookie_header` → **1 passed**
- `cargo test --release --test all --features integration_tests,codegen_tests -- cookie_build_path_dep_map_get_must_not_own_key` → **1 passed**

**Gates:** `CARGO_TARGET_DIR=…/cargo-target-tip-p3520`
- `cargo test --release --test all --features integration_tests,codegen_tests -- cookie_build_path_dep_map_get_must_not_own_key`

**Do not steal:** WDB-406/408/411, P3.518/P3.522/P3.524/P3.526–P3.528/P3.530/P3.532–P3.536, WDB-412–424 / P3.509–P3.535.

## P3.536 (2026-09-27) — TDD WDB-424 (compiler tip fix)

Repeated `match` on a non-Copy field must borrow, not `.clone()`; product emitted
`match body.shape.clone()` twice (`ShapeType` includes `ConvexHull(Vec<Vec3>)`).

| Gate | Status |
|------|--------|
| WDB-424 MultiFile | ✅ isolate GREEN — `match &body.shape` twice; Copy payload `*v` |
| WDB-424 tip-out | ✅ tip-out GREEN — jolt `world.rs` uses `match &body.shape` |

**Root cause layer:** match codegen (`let x = match` block path) — `generate_block_expr`
mirrored `generate_match_statement`: suppress field auto-clone on scrutinees and prefer
`match &place` when the root is borrowed or auto_clone marks the field path reused.
Statement `Match` path also strips `.clone()` → `&place` for the same cases.

**What became unnecessary:** `body.shape.clone()` / leaving clone when `&` was intended
in the `let … = match` expression path (was skipping `&` once `.clone()` was already
appended).

**Gates:** `CARGO_TARGET_DIR=…/cargo-target-tip-p3524` (2026-09-29)
- `cargo test --release --test all --features integration_tests,codegen_tests -- wdb424_` → **2 passed**
- `… -- wdb424_ wdb347_module_file wdb372_module_file_index_enum_match` → isolate GREEN;
  WDB-372 tip-out still stale product (pre-existing), not regressed by this fix

**Do not steal:** WDB-406/408/411, P3.508/P3.511/P3.513–P3.514/P3.516/P3.518/P3.520/P3.522/P3.524/P3.526–P3.528/P3.530/P3.532–P3.535, WDB-412–423 / P3.509–P3.534 (filed).

## P3.535 (2026-09-27) — `form_parse("?a=1")` must yield key `a`, not `?a`

`STDLIB_FORM_HANDOFF.md` says `form_parse` accepts an optional leading `?`.
Runtime previously called `url::form_urlencoded::parse` on raw bytes → key `"?a"`.
Fix: `strip_prefix('?')` once in `encoding::form_parse` (ccc610ed). Do not restore
package `strip_question` (P3.526 `starts_with` moves `t`).

| Gate | Status |
|------|--------|
| runtime `form_parse_strips_optional_leading_question` | ✅ GREEN (p3535) |
| `form_parse_must_strip_leading_question` | ✅ isolate GREEN after rebuild |
| product `wj-querystring` `$WJ test` | ✅ 14/14 |

**Why this is a new class:**
- P3.463 is **wiring** (`encoding::form_parse` symbol).
- P3.526 is **`starts_with` move** of a local `t`.
- This is **semantic parity** of the std form parser vs the handoff/`?` contract.

**Root cause layer:** runtime `encoding::form_parse` — strip a single leading `?` before `form_urlencoded::parse`.

**Ran (2026-09-28):** `.agent-wip/cargo-target-p3535` — unit RED→GREEN after
`strip_prefix('?')`; isolate RED on pre-fix tip (22:27), GREEN after rebuild.
Earlier tip p3515: runtime `[("?a", "1")]` RED.

**Gates:** `CARGO_TARGET_DIR=…/.agent-wip/cargo-target-p3535`
- `cargo test -p windjammer-runtime --lib encoding::tests::form_parse_strips_optional_leading_question -- --exact`
- `cargo test --release --test all --features integration_tests,codegen_tests -- form_parse_must_strip_leading_question`

**Do not steal:** WDB-406/408/411, P3.518/P3.522/P3.524/P3.526–P3.528/P3.530/P3.532–P3.534/P3.536–P3.541, WDB-412–426 / P3.509–P3.540.

## P3.534 (2026-09-27) — TDD WDB-423 (DB agent; no compiler src)

Copy `(f32, f32, f32)` array index destructure must not `.clone()`; product emits
`offsets[(i) as usize].clone()`.

| Gate | Status |
|------|--------|
| WDB-423 MultiFile | ✅ isolate GREEN — `let (ox, oy, oz) = offsets[(i as usize)]` (no `.clone()`) |
| WDB-423 tip-out | ❌ product RED — stale `offsets[(i) as usize].clone()` until regen |

**Root cause layer:** constraint / type inference — `[(1.0, 0.0, 0.0), …]` did not infer `[ (f32,f32,f32); N ]`, so index codegen treated the element as unknown and cloned.

**Why this is a new class:**
- WDB-363 is indexed tuple **field** (`planes[N].clone().normal`).
- WDB-422 is Copy **f32** index in arithmetic (`view_proj[i].clone()`).
- WDB-393 is **local i32** `y.clone()`.

**What became unnecessary:** `.clone()` on Copy tuple array index destructure.

**Gates:** `CARGO_TARGET_DIR=…/.agent-wip/cargo-target-tip-p3520` (2026-09-27)
- `cargo test --release --test all --features integration_tests,codegen_tests -- wdb423_module_file_copy_tuple_index_must_not_clone wdb422_module_file_copy_f32_index_arith_must_not_clone`
  → isolates **ok**; tip-out product RED

**Ran (DB agent 2026-09-27):** worktree `…/wdb407-tdd` @ `7c62e550`; `CARGO_TARGET_DIR=…/agent-tdd-wdb407`
- `cargo test --release --test all --features integration_tests,codegen_tests -- wdb423_` → **0 passed / 2 failed**

**Do not steal:** WDB-406/408/411, P3.508/P3.511/P3.513–P3.514/P3.516/P3.518/P3.520/P3.522/P3.524/P3.526–P3.528/P3.530/P3.532–P3.533/P3.535, WDB-412–422 / P3.509–P3.531 (filed).

## P3.533 (2026-09-27) — `use std::url` must not import runtime `Url` over a local `Url`

P3.530 greened the path-prefixed literal. `use std::url` still imported
`windjammer_runtime::url::Url`, shadowing the local struct (E0255 / E0560 / E0308).
Keep `join_url` → `url::join`. Do not rename the package `Url`.

| Gate | Status |
|------|--------|
| `local_url_struct_must_not_emit_runtime_url` | ✅ official cargo GREEN on p3505 — bare `Url {`, no runtime type import |
| `local_url_must_not_import_runtime_url_type` | ✅ official cargo GREEN on p3505 — no `use windjammer_runtime::url::Url;` |

**Why this is a new class:**
- P3.530 was the **path-prefixed literal**. This is the **use-import shadow**.
- `use windjammer_runtime::url;` (module) is fine for `url::join`.

**Root cause layer:** name resolution — collect local struct/enum names before emitting `use std::url` type imports; skip stdlib qualification for those names.

**What became unnecessary:** `use windjammer_runtime::url::Url` when this unit declares `struct Url`.

**Ran (2026-09-27):** `.agent-wip/cargo-target-p3505/release/wj`
- `… -- local_url_struct_must_not_emit_runtime_url` → **ok**
- `… -- local_url_must_not_import_runtime_url_type` → **ok**

**Gates:** `CARGO_TARGET_DIR=…/.agent-wip/cargo-target-p3505`
- `cargo test --release --test all --features integration_tests,codegen_tests -- local_url_must_not_import_runtime_url_type local_url_struct_must_not_emit_runtime_url`

**Do not steal:** WDB-406/408/411, P3.518/P3.522/P3.524/P3.526–P3.528/P3.530/P3.532–P3.533, WDB-412–422 / P3.509–P3.531.

## P3.532 (2026-09-27) — `wj-auth-api` leftover: `&mut req`, `get(lit.to_string())`, resolve_token

Free `dispatch(req: ServerRequest, …)` was demoted to `&mut` when a same-crate
method `App::dispatch(&mut self, …)` shared the bare name in the signature
registry. Cookie get greened via P3.537; find_char remains P3.524.

| Gate | Status |
|------|--------|
| `free_dispatch_must_not_inherit_method_mut_self` | ✅ isolate GREEN |
| `owned_method_param_into_demoted_resolve_token_must_borrow` | ✅ isolate GREEN (both demote `&str` after free-fn multipass) |
| `auth_api_product_dispatch_must_not_mut_req` | ✅ product GREEN — `dispatch(req)`, no `&mut req`; profile/resolve_token both `&str` |
| HashMap.get path-dep | ✅ P3.537 |
| find_char | ❌ P3.524 |

**Why this is a new class:**
- Notes-api stays green because `NotesApp::dispatch` is owned `mut self` (collided slot Owned).
- Auth method is MutBorrowed self → free formal inherited `&mut ServerRequest`.
- resolve_token: single-pass free-fn preregister left wrappers Owned while leaf helpers demoted; multipass converges before method bodies emit.

**Root cause layer:** signature registry — (1) do not register inherent methods under
the bare name; free-fn lookup must ignore method homonyms with self receivers;
(2) multipass free-function formal preregistration so demotion chains converge;
(3) AST-owned stubs must not beat preregistered demoted `&str` at call sites.

**What became unnecessary:** post-IR peels for `&mut req` / spawn-name style heuristics — fixed at registry + preregister convergence.

**Fix:** skip bare-name method registration; filter free-fn signature lookup; 4-pass free-fn formal preregister; `free_function_ast_arg_is_owned_wj_formal` defers to preregistered borrow; enforce_call_site prefers preregistered demotion over stale Owned contract.

**Ran (2026-09-29):** `.agent-wip/cargo-target-tip-p3532`
- `cargo test --release --test all --features integration_tests,codegen_tests -- free_dispatch_must_not_inherit_method_mut_self owned_method_param_into_demoted_resolve_token_must_borrow auth_api_product_dispatch_must_not_mut_req`

**Gates:** `CARGO_TARGET_DIR=…/.agent-wip/cargo-target-tip-p3532`
- `cargo test --release --test all --features integration_tests,codegen_tests -- free_dispatch_must_not_inherit_method_mut_self`
- `cargo test --release --test all --features integration_tests,codegen_tests -- owned_method_param_into_demoted_resolve_token_must_borrow`
- `cargo test --release --test all --features integration_tests,codegen_tests -- auth_api_product_dispatch_must_not_mut_req`

**Do not steal:** WDB-406/408/411, P3.518/P3.522/P3.524/P3.526–P3.528/P3.530/P3.536–P3.541, WDB-412–426 / P3.509–P3.540.

## P3.531 (2026-09-27) — TDD WDB-422 (DB agent; no compiler src)

Copy `f32` array index in arithmetic must not `.clone()`; product emits `view_proj[3].clone() + view_proj[0]`.

| Gate | Status |
|------|--------|
| WDB-422 MultiFile | ✅ isolate GREEN — `view_proj[3] + view_proj[0]` (no `.clone()`) |
| WDB-422 tip-out | ❌ product RED — `view_proj[i].clone()` in `frustum_culling.rs` |

**Root cause layer:** copy / index — `[f32; 16]` elements are Copy; binary `+`/`-` must not auto-clone.

**Why this is a new class:**
- WDB-393 is **local i32** `y.clone()`.
- WDB-346 is **field** `color.r.clone()`.
- WDB-355 is **Copy Vec3**.

**What became unnecessary:** `view_proj[3].clone()` in frustum plane extract.

**Gates:** `CARGO_TARGET_DIR=…/agent-tdd-wdb407` (2026-09-27)

**Ran (DB agent 2026-09-27):** worktree `…/wdb407-tdd` @ `b04ed61b`; `CARGO_TARGET_DIR=…/agent-tdd-wdb407`
- `cargo test --release --test all --features integration_tests,codegen_tests -- wdb422_` → **1 passed / 1 failed**

**Do not steal:** WDB-406/408/411, P3.508/P3.511/P3.513–P3.514/P3.516/P3.518/P3.520/P3.522/P3.524/P3.526–P3.528/P3.530, WDB-412–421 / P3.509–P3.529 (filed).

## P3.530 (2026-09-27) — local `Url` must not emit `windjammer_runtime::url::Url`

`wj-url` `use std::url` + package `struct Url { port, query: string }` rewrites
`Url { … }` to `windjammer_runtime::url::Url` (no `port`; `query: Option<String>`)
→ E0560 / E0308. Isolates without `use std::url` GREEN (17/17). Do not rename
the local struct or drop the `join_url` → `url.join` thin-wrap.

| Gate | Status |
|------|--------|
| `local_url_struct_must_not_emit_runtime_url` | ✅ official cargo GREEN on p3505 — leftover import was P3.533 (now GREEN) |
| product `wj-url` `$WJ test` after wrap | ⚠️ isolate GREEN; product regen pending |

**Why this is a new class:**
- Not P3.526 (`starts_with` move).
- `use std::url` aliases `Url` for literals; local struct must keep identity.

**Root cause layer:** name resolution / codegen — local struct literal after `use std::url`.

**Ran (2026-09-27):** tip p3520 21:03. Isolate emit confirmed. `url.join` alone cargo-checks.

**Gates:** `CARGO_TARGET_DIR=$HOME/Library/Caches/windjammer/cargo-target/agent-tdd-p3530-eco`
- `cargo test --release --test all --features integration_tests,codegen_tests -- local_url_struct_must_not_emit_runtime_url`

**Do not steal:** WDB-406/408/411, P3.518/P3.522/P3.524/P3.526–P3.528, WDB-412–421 / P3.509–P3.529.

## P3.529 (2026-09-27) — TDD WDB-421 (DB agent; no compiler src)

Last-use of an owned `Vec` formal into a callee must move; product emits `indices.clone()`.

| Gate | Status |
|------|--------|
| WDB-421 MultiFile | ✅ isolate GREEN — `finish(n, indices)` after `indices[i]` (no `indices.clone()`) |
| WDB-421 tip-out | ❌ product RED — `indices.clone()` in `rel_tip_out` `uv_unwrap_algorithm.rs` |

**Root cause layer:** last-use — WJ indexes `indices` then passes it once into `per_vertex_average_uv`. That last use must move.

**Why this is a new class:**
- WDB-418 is **early-return of formal** into `empty_result`.
- WDB-420 is **return of a local** `result.clone()`.
- WDB-415 is **if-then move of a formal**.

**What became unnecessary:** `indices.clone()` on last use into the UV average helper.

**Gates:** `CARGO_TARGET_DIR=…/agent-tdd-wdb407` (2026-09-27)

**Ran (DB agent 2026-09-27):** worktree `…/wdb407-tdd` @ `0e01f24e`; `CARGO_TARGET_DIR=…/agent-tdd-wdb407`
- `cargo test --release --test all --features integration_tests,codegen_tests -- wdb421_` → **1 passed / 1 failed**

**Do not steal:** WDB-406/408/411 (compiler), P3.508/P3.511/P3.513–P3.514/P3.516/P3.518/P3.520/P3.522/P3.524/P3.526–P3.528, WDB-412–420 / P3.509–P3.525 (filed).

## P3.528 (2026-09-27) — `apply_conn` must not move `applied` Vec each loop

Product `wj-migrate` `db_apply.rs` (tip p3520 20:49):
`if version_applied(applied, item.migration.version)` inside `for item in located`
→ rustc E0382 move in previous iteration. `version_applied` takes `Vec<int>`.

`pending()` already emits `version_applied(&applied, …)`. Isolates emit
`applied.clone()` and cargo-check (false-GREEN). Do not reshape the package.

| Gate | Status |
|------|--------|
| isolate `pending_count` + `version_applied(applied, v)` in loop | ⚠️ false-GREEN `applied.clone()` + cargo-check |
| `product_applied_vec_loop_must_not_move` | ❌ product RED — `version_applied(applied, …)` |

**Why this is a new class:**
- Not WDB-418 / P3.525 (early-return / last-use `return result.clone()`).
- Not P3.526 / P3.527 (string / usize).
- Same-crate `pending` already borrows; `apply_conn` does not.

**Root cause layer:** last-use / loop reuse — owned Vec formal reused each iteration must borrow or clone.

**Ran (2026-09-27):** tip p3520 20:49. Isolate cargo-check GREEN. Product `db_apply.rs:156` moves.

**Gates:** `CARGO_TARGET_DIR=$HOME/Library/Caches/windjammer/cargo-target/agent-tdd-p3526-eco`
- `cargo test --release --test all --features integration_tests,codegen_tests -- product_applied_vec_loop_must_not_move`

**Do not steal:** WDB-406/408/411 (compiler), P3.508/P3.511/P3.513–P3.514/P3.516/P3.518/P3.520/P3.522/P3.524/P3.526–P3.527, WDB-412–420 / P3.509–P3.525 (filed).

## P3.527 (2026-09-27) — string-scan `i == 0` must not mix usize / `0_i64`

`split_version_name`: `let mut i = 0` + `while i < strings.len(stem)` emits
`i: usize`, then `if i == 0` → `i == 0_i64` (E0308 / E0277).

Distinct from P3.516 (`while k <= vec.len()`). Isolates match product.

| Gate | Status |
|------|--------|
| `string_scan_index_eq_zero_must_unify` | ✅ isolate GREEN — `if i == 0_usize` |
| product `wj-migrate` `lib.rs` | ❌ tip-out pending regen |

**Root cause layer:** emit-truth / int unify — `signed_peer_for_zero_sentinel` re-inferred AST `let i = 0` as signed `Int` and forced `0_i64` even when the binding emits as `usize`.

**What became unnecessary:** signed zero-sentinel peer path for bindings already in `usize_variables` / `local_var_types` usize (no new reconcile peel).

**Fix:** `signed_peer_for_zero_sentinel` returns `None` when the identifier is emit-truth usize so comparison peer keeps `_usize`.

**Ran (2026-09-28):** tip p3520 — `cargo test --release --test all --features integration_tests,codegen_tests -- string_scan_index_eq_zero_must_unify` → **1 passed** (`if i == 0_usize`).

**Gates:** `… -- string_scan_index_eq_zero_must_unify`

**Do not steal:** WDB-406/408/411, P3.516/P3.518/P3.520/P3.522/P3.524/P3.526, WDB-412–420 / P3.509–P3.525.

## P3.526 (2026-09-27) — `starts_with(t)` must borrow when `t` is reused

`wj-querystring` `strip_question`: `strings.starts_with(t, "?")` then
`substring(t, …)` / return `t`. Runtime `starts_with<S: AsRef<str>>` takes
`t` by value → E0382. Isolates match product. Blocks form_* thin-wrap.

| Gate | Status |
|------|--------|
| `starts_with_must_borrow_then_reuse` | ✅ isolate GREEN — `starts_with(&t, "?")` + `substring(&t` / `len(&t)` |
| product `wj-querystring` `$WJ test` | ❌ tip-out pending regen |

**Root cause layer:** emit-truth — owned `let t = strings::trim(text)` stayed in
`inferred_borrowed_params`, so call-site reconcile peeled `&t` into a move
despite runtime `starts_with` already being Borrowed + `emitted_rust_ref_params`.

**What became unnecessary:** a `starts_with` name heuristic; WJ stub last-write
was already restored for needles (WDB-144). Haystack needed owned-local emit-truth
so IR Borrow is not peeled.

**Ran (2026-09-28):** tip p3520. Isolate GREEN `starts_with(&t, "?")`.
Product tip-out still stale gen.

**Gates:** `… -- starts_with_must_borrow_then_reuse`

**Do not steal:** WDB-406/408/411, P3.518/P3.520/P3.522/P3.524, WDB-412–420 / P3.509–P3.525.

## P3.525 (2026-09-27) — TDD WDB-420 (DB agent; no compiler src)

Last-use `return` of a local `Vec` must move; product emits `return result.clone()`.

| Gate | Status |
|------|--------|
| WDB-420 MultiFile | ✅ isolate GREEN — `return result` after `push` (no `result.clone()`) |
| WDB-420 tip-out | ❌ product RED — `return result.clone()` / `return path.clone()` in svo + astar |

**Root cause layer:** last-use — WJ `let mut result = Vec::new(); result.push(node); return result`. The local is exclusive last-use.

**Why this is a new class:**
- WDB-418 is **early-return of an owned formal** into `empty_result`.
- WDB-415 is **if-then move of a formal**.
- WDB-407 is **`Vec` `new` demote**.

**What became unnecessary:** `return result.clone()` / `return path.clone()` of a local Vec.

**Gates:** `CARGO_TARGET_DIR=…/agent-tdd-wdb407` (2026-09-27)

**Ran (DB agent 2026-09-27):** worktree `…/wdb407-tdd` @ `4af83ee2`; `CARGO_TARGET_DIR=…/agent-tdd-wdb407`
- `cargo test --release --test all --features integration_tests,codegen_tests -- wdb420_` → **1 passed / 1 failed**

**Do not steal:** WDB-406/408/411 (compiler), P3.508/P3.511/P3.513–P3.514/P3.516/P3.518/P3.520/P3.522/P3.524 (notes-api / wj-glob), WDB-412–419 / P3.509–P3.523 (filed).

## P3.524 (2026-09-27) — adapter `find_char` must take `text: &str`, not `&String` ✅

Product `wj-notes-api` `adapters/http_server.rs`:
`find_char(text: &String, …)` while callers held `&str` → rustc E0308.

**Root cause layer:** signature — WJ `std/strings.wj` declaration stubs aliased
`substring` → `strings::substring` as owned/`Reference(String)`, clobbering scanned
runtime AsRef/`&str`. Nested module-file multipass then treated
`strings.substring(text, …)` as a `&String` formal callee. Analyzer bare
`lookup_method("substring")` reinforced the false `&String` need.

**Fix:**
- `restore_runtime_borrowed_strings_signatures` also restores when local is
  `Reference(String)` / owned string vs runtime `Reference(str)` / AsRef emit flags
- `register_module_aliases` refuses to clobber runtime `strings::*` AsRef with WJ stubs
- `param_needs_string_ref` MethodCall path uses `{module}::{method}` for runtime
  modules (no bare `substring` homonym)

**What became unnecessary:** peels / name heuristics for `substring`; product reshape.

| Gate | Status |
|------|--------|
| `nested_module_find_char_must_demote_text_to_str_not_string_ref` | ✅ GREEN `text: &str` |
| `product_adapter_find_char_text_must_not_be_string_ref` | ✅ GREEN `text: &str` |
| `restore_runtime_strings_beats_reference_string_haystack` | ✅ |
| `register_module_aliases_does_not_clobber_runtime_strings_asref` | ✅ |

**Ran (2026-09-29):** tip `.agent-wip/cargo-target-tip-p3524/release/wj`
- `cargo test --release -p windjammer --lib restore_runtime_strings_beats`
- `cargo test --release -p windjammer --lib register_module_aliases_does_not_clobber`
- `cargo test --release --test all -- nested_module_find_char_must_demote product_adapter_find_char_text_must_not_be_string_ref`

**Do not steal:** WDB-406/408/411 (compiler), P3.508/P3.511/P3.513–P3.514/P3.516/P3.518/P3.520/P3.522 (notes-api / wj-glob), WDB-412–419 / P3.509–P3.523 (filed).

## P3.523 (2026-09-27) — TDD WDB-419 (DB agent; no compiler src)

Reused owned `string` into several owned helpers must not `parse_flag(line.clone())` at each callsite.

| Gate | Status |
|------|--------|
| WDB-419 MultiFile | ✅ isolate GREEN — sequential `parse_flag(line, …)` emits no `line.clone()` |
| WDB-419 tip-out | ❌ product RED — `parse_flag(line.clone(), "forward")` in `agent_playtest_protocol.rs` |

**Root cause layer:** signature / last-use — WJ reuses `line` across owned `parse_flag(line, key)` calls. Callees should demote to borrow; callsites must not clone each time.

**Why this is a new class:**
- WDB-413 is **`string_len(line.clone())` in one comparison**.
- WDB-409 is **stored `set(name)` must stay owned**.
- WDB-418 is **early-return Vec move**.

**What became unnecessary:** `parse_flag(line.clone(), "forward")` / `parse_flag(line.clone(), "back")`.

**Gates:** `CARGO_TARGET_DIR=…/agent-tdd-wdb407` (2026-09-27)

**Ran (DB agent 2026-09-27):** worktree `…/wdb407-tdd` @ `0f7b527b`; `CARGO_TARGET_DIR=…/agent-tdd-wdb407`
- `cargo test --release --test all --features integration_tests,codegen_tests -- wdb419_` → **1 passed / 1 failed**

**Do not steal:** WDB-406/408/411 (compiler), P3.508/P3.511/P3.513–P3.514/P3.516/P3.518/P3.520/P3.522 (notes-api / wj-glob), WDB-412–418 / P3.509–P3.521 (filed).

## P3.522 (2026-09-27) — `handle_method` / `handle_http` must be `&mut self`

P3.520 isolate GREENS `check_rate(&mut self)`. Product still emits `handle_method(self)` / `handle_http(self)` then `self.check_rate` (E0596) and `MutexGuard.handle_http` (E0507).

| Gate | Status |
|------|--------|
| `check_rate_field_replace_must_not_move_self` | ✅ P3.520 isolate GREEN — `check_rate(&mut self)` |
| `handle_method_must_mut_self_for_check_rate` MultiFile | ✅ isolate GREEN — `handle_method(&mut self)` / `handle_http(&mut self)`, no `self.clone().check_rate` |
| `handle_method_must_mut_self_for_check_rate` product | ✅ product GREEN — `handle_method(&mut self)` / `handle_http(&mut self)` |
| `notes_api_product_remaining_e0308_must_not_emit` | ❌ `&mut query` still on tip p3520 |

**Why this is a new class:**
- P3.520 same-field writeback fixed **check_rate** only. Callers defined *above* the writeback callee (and callers that first read `self.config`) stayed `&self` + `self.clone()` or owned `self`.
- Adapter `state.lock()` → `app.handle_http(...)` cannot move `NotesApp`.

**Root cause layer:** constraint / self-mode — `if`/`match` conditions were not walked for mut-self; writeback callees were classified as consuming so the impl pre-pass skipped them; Inferred `Owned` + `body_modifies` forced `mut self` instead of `owned_self_receiver`.

**What became unnecessary:** `self.clone().check_rate` on `&self` callers; owned `mut self` / `self` on handle_http/handle_method.

**Ran (compiler 2026-09-27):** `CARGO_TARGET_DIR=…/.agent-wip/cargo-target-tip-p3520`
- `… -- bug_handle_method_must_mut_self_for_check_rate_test::handle_method_must_mut_self_for_check_rate` → **ok**
- `… -- bug_check_rate_field_replace_must_not_move_self_test::check_rate_field_replace_must_not_move_self` → **ok**
- `… -- wdb414_module_file_ctor_must_move_self_field` → isolate **ok** (tip-out product RED)
- `… -- bug_notes_api_handle_method_must_mut_self_for_check_rate_test::handle_method_must_mut_self_for_check_rate` → **ok**

**Gates:** `CARGO_TARGET_DIR=…/.agent-wip/cargo-target-tip-p3520`
- `cargo test --release --test all --features integration_tests,codegen_tests -- handle_method_must_mut_self_for_check_rate check_rate_field_replace_must_not_move_self`

**Do not steal:** WDB-406/408/411 (compiler), P3.508/P3.511/P3.513–P3.514/P3.516/P3.518/P3.520 (notes-api / wj-glob), WDB-412–418 / P3.509–P3.521 (filed).

## P3.521 (2026-09-27) — TDD WDB-418 (DB agent; no compiler src)

Early-return of an owned `Vec` formal after `.len()` must move; product emits `empty_result(positions.clone())`.

| Gate | Status |
|------|--------|
| WDB-418 MultiFile | ✅ isolate GREEN — `return empty_result(positions)` (no clone), including loop-internal return |
| WDB-418 tip-out | ❌ product RED — stale `empty_result(positions.clone())` until regen |

**Root cause layer:** constraint / reuse — auto_clone treated later sibling-path indexes as reachable after a diverging `if { return }`, and `in_loop` forced clone on function-exiting return. Exclusive early-return is the same class as exclusive match arms (P3.332 / WDB-417).

**Why this is a new class:**
- WDB-415 is **if-then** `materials_to_palette(materials)` (no early return + later sibling uses).
- WDB-417 is **match-arm** `generate_culled_mesh(chunk)`.
- WDB-407 is **`Vec` `new` demote**.

**What became unnecessary:** `positions.clone()` on exclusive early-return into `empty_result` (top-level and `while` inner return).

**Gates:** `CARGO_TARGET_DIR=…/.agent-wip/cargo-target-tip-p3520` (2026-09-27)
- `… -- wdb418_module_file_early_return_must_move_owned_vec` → isolate **ok**; tip-out product RED
- `… -- wdb415_module_file_owned_vec_formal_inside_if_must_not_clone` → isolate **ok**

**Ran (compiler 2026-09-27):** tip p3520 after exclusive early-return reuse + `in_loop` exempt when `in_diverging_early_return`.
- Isolate emit: `return empty_result(positions);` (both the top-level empty/short check and the `while` bounds check)

**Do not steal:** WDB-406/408/411 (compiler), P3.508/P3.511/P3.513–P3.514/P3.516/P3.518/P3.520 (notes-api / wj-glob), WDB-412–417 / P3.509–P3.519 (filed).

## P3.520 (2026-09-27) — `check_rate` field replace must be `&mut self`

`wj-notes-api` `check_rate` assigns `self.buckets = buckets_from_limit(...)` after a helper takes `self.buckets`. Caller `handle_method` still `self.dispatch(...)`.

| Gate | Status |
|------|--------|
| `check_rate_field_replace_must_not_move_self` | ✅ isolate GREEN on tip p3520 (20:32) — `fn check_rate(&mut self)` |
| product `wj-notes-api` `$WJ test` | ⚠️ check_rate `&mut self`; caller still owned `self` (P3.522) |

**Why this is a new class:**
- P3.518 last-use `&mut query` is **GREEN** (move `query`).
- Insert into `self.buckets` greened `&mut self`. **Replace** after a consuming helper infers owned `mut self`.
- Caller continues after `None`. Do not edit `windjammer/src/`. Do not reshape notes-api.

**Root cause layer:** signature / self-mode — field replace + later use of `self` must stay `&mut self`.

**What became unnecessary:** further `&mut query` isolates.

**Ran (2026-09-27):** tip `.agent-wip/cargo-target-p3505/release/wj` 0.50.0 (20:11). `$WJ build --library --module-file` + `cargo check`.
- Isolate: `fn check_rate(mut self, …)` + `handle_method(&mut self)` → rustc move.
- Product GET-one: `note_get_reply(&mut note, if_none_match, query)` (no `&mut query`).

**Gates:** `CARGO_TARGET_DIR=$HOME/Library/Caches/windjammer/cargo-target/agent-tdd-p3520-eco`
- `cargo test --release --test all --features integration_tests,codegen_tests -- check_rate_field_replace_must_not_move_self`

**Do not steal:** WDB-406/408/411 (compiler), P3.508/P3.511/P3.513–P3.514/P3.516/P3.518 (notes-api / wj-glob), WDB-412–417 / P3.509–P3.519 (filed).

## P3.519 (2026-09-27) — TDD WDB-417 (DB agent; no compiler src)

Exclusive match arms must move an owned formal; product emits `generate_culled_mesh(chunk.clone())`.

| Gate | Status |
|------|--------|
| WDB-417 MultiFile | ✅ isolate GREEN — exclusive arms move / borrow; no `chunk.clone()` |
| WDB-417 getter / cross-module | ✅ isolate GREEN — `get_local` + `should_face` emit `&Chunk` workers; `generate(chunk: Chunk)` + `&chunk` |
| WDB-417 tip-out | ❌ product RED — stale `generate_chunk_mesh(chunk: &mut VoxelChunk)` + `generate_culled_mesh(chunk.clone())` |

**Root cause layer:** signature — WJ `generate_chunk_mesh(chunk: VoxelChunk, strategy)` moves `chunk` into one exclusive arm. Tip isolates already emit owned `generate` + borrowed workers. Product demote/`clone` is stale gen (regen), not a remaining isolate gap.

**Why this is a new class:**
- WDB-407 is **`Vec` `new` demote** (`new(&joints)`).
- WDB-415 is **owned Vec clone inside `if`**.
- WDB-414 is **`new(self.scene)`**.

**What became unnecessary:** `chunk.clone()` in exclusive match arms when WJ moves `chunk`.

**Gates:** `CARGO_TARGET_DIR=…/.agent-wip/cargo-target-tip-p3520` (2026-09-27)
- `… -- wdb417_module_file_match_arm_must_move_owned_chunk` → isolate **ok**; tip-out **FAILED** (stale product)
- `… -- wdb417_module_file_readonly_getter_must_not_mut_then_clone` → **ok** (`generate(chunk: &Chunk)`, no clone)
- `… -- wdb417_module_file_cross_module_getter_must_not_mut_then_clone` → **ok** (`generate(chunk: Chunk)` + `gen_*( &chunk )`)

**Ran (DB agent 2026-09-27):** worktree `…/wdb407-tdd` @ `edfea5ab`; `CARGO_TARGET_DIR=…/agent-tdd-wdb407`
- `cargo test --release --test all --features integration_tests,codegen_tests -- wdb417_` → **3 passed / 1 failed**

**Do not steal:** WDB-406/408/411 (compiler), P3.508/P3.511/P3.513–P3.514/P3.516/P3.518 (notes-api / wj-glob), WDB-412–416 / P3.509–P3.517 (filed).

## P3.518 (2026-09-27) — product GET-one still `&mut query` into `query: String`

P3.513/P3.514 nested `Some(mut note)` is GREEN. Product `dispatch` still last-uses `&mut query`.

| Gate | Status |
|------|--------|
| `fetch_note_store_then_query_must_not_mut` | ✅ P3.514 isolate GREEN — `Some(mut note)` + `query.clone()` / move |
| `interp_query_then_get_must_not_mut_query` | ✅ P3.513 isolate GREEN — same mut-bind |
| `split_query_list_then_get_must_not_mut_query` | ✅ P3.511 isolate GREEN |
| `notes_api_product_remaining_e0308_must_not_emit` | ✅ product query GREEN on tip p3505 (20:11) — `note_get_reply(&mut note, …, query)` |

**Why this is a new class:**
- Mut-bind E0596 is closed. Remaining is last-use / auto-ref of `query: String`.
- Isolates with nested `/`+`/health` ifs, `HttpReply`, two list-arm `query.clone()`, and `"${query}"` still last-use `query.clone()`. Product-only `&mut query`.

**Root cause layer:** last-use / call-site auto-ref — `query: String` must receive `query` / `query.clone()`, not `&mut query`. Do not reshape notes-api.

**What became unnecessary:** further E0596 `Some(note)` isolates.

**Ran (2026-09-27):** `.agent-wip/cargo-target-p3505/release/wj` 0.50.0 — last-writer `apply_callee_mut_borrow_to_call_args` treats WJ `string` as owned (official cargo):
- `cargo test --release --test all --features integration_tests,codegen_tests -- notes_api_product_remaining_e0308_must_not_emit interp_query_then_get_must_not_mut_query split_query_list_then_get_must_not_mut_query fetch_note_store_then_query_must_not_mut` → **4 passed / 0 failed** (14.39s).
- Product emit: `Some(mut note) => note_get_reply(&mut note, if_none_match, query)` (no `&mut query`).

**Gates:** `CARGO_TARGET_DIR=$HOME/Library/Caches/windjammer/cargo-target/agent-tdd-p3518`
- product remaining E0308 query slot **GREEN**. Next rustc: P3.520 `check_rate` owned `mut self`.

**Do not steal:** WDB-406/408/411/414–416, P3.516 (`<=` len unify).

## P3.517 (2026-09-27) — TDD WDB-416 (DB agent; no compiler src)

Owned enum formal must not `a.clone().as_float()` when WJ is `a.as_float()`.

| Gate | Status |
|------|--------|
| WDB-416 MultiFile | ✅ isolate GREEN — `a.as_float() + b.as_float()` (no clone) |
| WDB-416 tip-out | ❌ product RED — `a.clone().as_float()` in `rel_tip_out` + `gen` `visual_scripting/runtime.rs` |

**Root cause layer:** signature — isolate already moves. Product `evaluate` match on `BuiltinFn` still clones owned `Value` before `as_float(self)`. Regen or match product match-arms.

**Why this is a new class:**
- WDB-358 is **`self.clone().method()`**.
- WDB-362 is **indexed `].clone().method()`**.
- WDB-409 is **`set(name: &str)`** demote.

**What became unnecessary:** `a.clone()` before a one-shot owned-self method.

**Ran (2026-09-27):** worktree `…/wdb407-tdd` @ `6d9dcd2c`; `CARGO_TARGET_DIR=…/agent-tdd-wdb407`
- `cargo test --release --test all --features integration_tests,codegen_tests -- wdb416_` → **1 passed / 1 failed**

**Do not steal:** WDB-406/408/411 (compiler), P3.508/P3.511/P3.513–P3.514/P3.516/P3.518 (notes-api / wj-glob), WDB-412–415 / P3.509–P3.515 (filed).

## P3.516 (2026-09-27) — `while k <= vec.len()` must unify i64 like `>=`

`wj-glob` `match_segs`: `>= pats.len()` / `ti >= texts.len()` emit `(len() as i64)`, but `while k <= texts.len()` leaves raw `usize`.

| Gate | Status |
|------|--------|
| `int_while_le_vec_len_must_unify_i64` | ✅ isolate GREEN — `07e2ec99` (`usize_variables` no longer beats i64 on `<=`) |
| product `wj-glob` `$WJ test` | ✅ 14 passed on tip p3515 (19:38) |

**Why this is a new class:**
- `bug_int_index_while_len_must_not_emit_usize_add_test` only asserts `ti >=` lines and treats **transpile-ok** as success (false-GREEN).
- `>=` already casts. `<=` in `while` does not. Do not edit `windjammer/src/`. Do not reshape `wj-glob`.

**Root cause layer:** encoding / compare — `<=` vs `>=` len unify is not symmetric.

**What became unnecessary:** casting `.len()` in `wj-glob`; package `is_match` now thin-wraps `std::path.glob_match` (14 tests + `wj-find` 4/4).

**Ran (2026-09-27):** tip `.agent-wip/cargo-target-p3505/release/wj` 0.50.0 (18:50) RED; tip `.agent-wip/cargo-target-tip-p3515/release/wj` 0.50.0 (19:38) after `07e2ec99` GREEN.
- Product `$WJ test`: **14 passed**.

**Gates:** `CARGO_TARGET_DIR=$HOME/Library/Caches/windjammer/cargo-target/agent-tdd-p3516-eco`
- `cargo test --release --test all --features integration_tests,codegen_tests -- int_while_le_vec_len_must_unify_i64`

**Do not steal:** WDB-406/408/411 (compiler), P3.508/P3.511/P3.513/P3.514 (notes-api), WDB-412–415 / P3.509–P3.515 (filed).

## P3.515 (2026-09-27) — TDD WDB-415 (DB agent; no compiler src)

Owned `Vec` formal moved into a callee inside `if` must not `.clone()`.

| Gate | Status |
|------|--------|
| WDB-415 MultiFile | ✅ isolate GREEN — `materials_to_palette(materials)` inside `if` (no clone) |
| WDB-415 tip-out | ❌ product RED — `materials_to_palette(materials.clone())` in `gen/rendering/unified_renderer.rs` |

**Root cause layer:** signature — isolate already moves the owned Vec. Product `upload_materials(&mut self, materials: Vec<MaterialData>)` still clones into `UnifiedRenderer::materials_to_palette` (method + assoc path, not isolate free fn). Regen or match product shape.

**Why this is a new class:**
- WDB-412 is **field.clone()** into a **for-loop** Vec formal.
- WDB-414 is **`new(self.scene)`** (constructor field).
- WDB-407 is **`Vec` `new` demote** (`new(&joints)`). Same dual status (isolate GREEN / tip RED).

**What became unnecessary:** `materials.clone()` when WJ passes `materials` once.

**Ran (2026-09-27):** worktree `…/wdb407-tdd` @ `ca9ca3b8`; `CARGO_TARGET_DIR=…/agent-tdd-wdb407`
- `cargo test --release --test all --features integration_tests,codegen_tests -- wdb415_` → **1 passed / 1 failed**

**Do not steal:** WDB-406/408/411 (compiler), P3.508/P3.511/P3.513/P3.514 (notes-api), WDB-412–414 / P3.509–P3.512 (filed).

## P3.514 (2026-09-27) — `fetch_note(self.store, id)` GET-one must mut-bind note

| Gate | Status |
|------|--------|
| `fetch_note_store_then_query_must_not_mut` | ✅ isolate E0596 GREEN on tip p3505 (18:50) — `Some(mut note)` + last-use `query` (move) |
| `interp_query_then_get_must_not_mut_query` | ✅ P3.513 same mut-bind on p3505 (18:50) |
| `notes_api_product_remaining_e0308_must_not_emit` | ❌ product RED — `&mut query` into `query: String` (`let mut query`) |

**Why this is a new class:**
- P3.513 GET-one is **`self.store.fetch(id)`**. Product is **`fetch_note(self.store, id)`** while POST/PUT still call `self.store.create` / `update` / `delete`.
- Isolates last-use `query` / `query.clone()`. Product last use remains `&mut query` after `query.clone()` in the list arm.

**Root cause layer:** last-use / auto-ref — product `dispatch` still emits `let mut query` + `note_get_reply(&mut note, …, &mut query)` into `query: String`. Mut-bind for `Note` greened. Do not edit `windjammer/src/`. Do not reshape notes-api. Path-dep `qs_get` + nested `/`/`/health` ifs + `wj_url` still last-use move in isolates (not this E0308).

**What became unnecessary:** treating method-call `store.fetch` as the only missing nest; filing another E0596 isolate.

**Ran (2026-09-27):** tip `.agent-wip/cargo-target-p3505/release/wj` 0.50.0 (18:50). Product `$WJ build --module-file`.
- Product GET-one: `Some(mut note) => note_get_reply(&mut note, if_none_match, &mut query)`.
- Isolates with path-dep `qs_get` + nested if + `join_url`: `Some(mut note)` + `query` move.

**Gates:** `CARGO_TARGET_DIR=$HOME/Library/Caches/windjammer/cargo-target/agent-tdd-p3511-eco`
- `cargo test --release --test all --features integration_tests,codegen_tests -- notes_api_product_remaining_e0308_must_not_emit` → still the RED product gate.

## P3.513 (2026-09-27) — nested route_match GET-one must mut-bind note (not `&mut query` yet)

| Gate | Status |
|------|--------|
| `interp_query_then_get_must_not_mut_query` | ✅ isolate GREEN — `generate_block_expr` now mut-binds `Some(mut note)` (official cargo 18:50+) |
| `split_query_list_then_get_must_not_mut_query` | ✅ isolate GREEN on tip p3505 (17:47) — shallow `match fetch` now `Some(mut note)` + `query.clone()` |
| `notes_api_product_remaining_e0308_must_not_emit` | ❌ product RED — `Some(note) => note_get_reply(&mut note, if_none_match, &mut query)` (`let mut query = split.1`) |

**Why this is a new class:**
- P3.511 shallow `NotesApp` + `store.fetch` greened `Some(mut note)`.
- Product dispatch is **nested** `route_match` / `params.get` / `parse_positive_int` / `method` / `fetch`. That graph still emits immutable `Some(note)` + `&mut note`.
- Isolate last use is still `query.clone()`; product last use is `&mut query`. Separate remaining E0308.

**Root cause layer:** codegen / match-binding — mut-bind for `&mut Note` after `json.to_string` does not apply inside nested Option/enum matches. Do not edit `windjammer/src/`. Do not reshape notes-api.

**What became unnecessary:** refiling P3.511 shallow fetch; treating `"${query}"` interpolation as the `&mut query` trigger.

**Ran (2026-09-27):** tip `.agent-wip/cargo-target-p3505/release/wj` 0.50.0 (17:47). Isolate `$WJ build --module-file` + `cargo check`.
- Nested emit: `Some(note) => note_get_reply(&mut note, if_none_match, query.clone())` — E0596.
- Product emit unchanged: `let mut query` + `&mut query`.

**Gates:** `CARGO_TARGET_DIR=$HOME/Library/Caches/windjammer/cargo-target/agent-tdd-p3512-eco`
- `cargo test --release --test all --features integration_tests,codegen_tests -- interp_query_then_get_must_not_mut_query` → **0 passed / 1 failed** (0.54s after 1016s compile; TDD RED).

## P3.512 (2026-09-27) — TDD WDB-414 (isolate GREEN on tip; tip-out regen)

`Type::new(self.scene)` must move the field; product emitted `CsgVoxelizer::new(self.scene.clone())`.

| Gate | Status |
|------|--------|
| WDB-414 MultiFile | ✅ isolate GREEN — `initialize(mut self)` moves `self.scene` |
| WDB-414 tip-out | ❌ tip-out stale — `CsgVoxelizer::new(self.scene.clone())` until product regen |

**Root cause layer:** ownership demotion — tip keeps owned `mut self` for partial field move + later field write (no longer clones).

**What became unnecessary:** `self.scene.clone()` at `Vox::new` / `CsgVoxelizer::new` when WJ moves `self.scene`.

**Gates (2026-09-29 tip p3524):** `… -- wdb414_module_file_ctor` → isolate **ok**; tip-out product RED (regen).

## P3.511 (2026-09-27) — split query list-then-get must not `&mut note` / `&mut query`

| Gate | Status |
|------|--------|
| `split_query_list_then_get_must_not_mut_query` | ✅ isolate GREEN (re-verified 2026-10-02 tip p3574) |
| `notes_api_product_remaining_e0308_must_not_emit` | ❌ product RED — `note_get_reply(&mut note, if_none_match, &mut query)` into `query: String` (E0308) |

**Why this is a new class:**
- Two-arg / if-return query isolates emit `query.clone()` and cargo-check (P3.489 / P3.499).
- This hexagonal dispatch matches product: store `list` + `fetch`, `query` from `split_path_query`, list arm clones query, GET-one still demotes `note` to `&mut Note` without a `mut` binding.
- Product still uses `&mut query` (not reproduced by the isolate clone). Separate remaining E0308.

**Root cause layer:** codegen / match-binding — `json.to_string(note)` demotes the formal to `&mut Note`; `Some(note)` stays immutable. Do not edit `windjammer/src/`. Do not reshape notes-api.

**What became unnecessary:** treating HashMap/`&mut self` as required to reproduce `&mut note`; refiling empty-lit P3.508 (tip GREEN).

**Ran (2026-09-27):** tip `.agent-wip/cargo-target-tip-p3510/release/wj` 0.50.0 (17:28). Isolate `$WJ build --module-file` + `cargo check`.
- Emit: `Some(note) => note_get_reply(&mut note, if_none_match, query.clone())` — E0596 cannot borrow `note` as mutable.
- Not `&mut query` (clone). Product `$WJ test`: **1** E0308 `&mut query`.
- Eco: `wj-fetch` 31/31; notes-api still blocked.

**Gates:** `CARGO_TARGET_DIR=$HOME/Library/Caches/windjammer/cargo-target/agent-tdd-p3511-eco`
- `cargo test --release --test all --features integration_tests,codegen_tests -- split_query_list_then_get_must_not_mut_query` → **0 passed / 1 failed** (2.83s after 829s compile; TDD RED).

## P3.510 (2026-09-27) — TDD WDB-413 (DB agent; no compiler src)

Read-only reuse of `line` across `start(line, key) < slen(line)` must not emit `line.clone()`.

| Gate | Status |
|------|--------|
| WDB-413 MultiFile | ✅ isolate GREEN (re-verified 2026-09-29 / P3.543 tip) — no `line.clone()` |
| WDB-413 tip-out | ❌ product RED — `gpu::string_len(line.clone())` in `rel_tip_out` + `gen` until regen |

**Root cause layer:** signature — unused/unread `line` formal on `start` stays owned `String`, so `present` clones before the second read. Product `gpu::string_len` also stays owned, so tip emits `string_len(line.clone())` after `key_value_start(&line, key)`. WJ is `start(line, key) < slen(line)` / `gpu::string_len(line)` with no `.clone()`.

**Why this is a new class:**
- WDB-412 is **read-only `for` Vec formal** forcing `field.clone()`. This is **string reuse** across two callees forcing **`line.clone()`**.
- WDB-106 is the opposite: explicit `.clone()` on sequential owned calls **must stay**.
- WDB-143 is reuse after an owned **consume** that **must** clone to compile.

**What became unnecessary:** treating an unused `line` formal as owned consume; call-site `line.clone()` when WJ reuses `line`.

**Ran (2026-09-27):** worktree `…/wdb407-tdd` @ `0513a4a3`; `CARGO_TARGET_DIR=…/agent-tdd-wdb407`
- `cargo test --release --test all --features integration_tests,codegen_tests -- wdb413_` → **0 passed / 2 failed**

**Do not steal:** WDB-406/408/411 (compiler), P3.508/P3.511 (notes-api), WDB-412 / P3.509 (filed).

## P3.509 (2026-09-27) — WDB-412 read-only `for` Vec formal must borrow

| Gate | Status |
|------|--------|
| WDB-412 MultiFile | ✅ isolate GREEN — `contains_name(&pb.metas, "w")`; `metas: &Vec<Meta>` |
| WDB-412 tip-out | ⚠️ product — stale `rel_tip_out` / `gen` still has `pb.binding_metas.clone()` (regen, not isolate) |

**Root cause layer:** signature — `for m in metas { m.name == needle }` was classified as a Vec consume (`param_consumed_as_for_loop_iterable` / For-arm read operand). Field/method-only loop-var reads are borrowed iteration, so the Vec formal demotes to `&Vec` and call sites borrow the field twice without `.clone()`.

**Why this is a new class:**
- WDB-410 is **owned-self wither reconstruct**. This is a **read-only `for` Vec formal** that stayed owned, forcing **`pb.binding_metas.clone()` at the call site**.
- WDB-407 is `Vec` `new`. WDB-378 is indexed `src[i].clone().field`.
- regression-006 still consumes `for x in items` when the element is moved.

**What became unnecessary:** treating field-read `for m in metas` as consume; call-site `field.clone()` into an owned Vec formal.

**Gates:** `CARGO_TARGET_DIR=…/.agent-wip/cargo-target-tip-p3510`
- `cargo test --release --test all --features integration_tests,codegen_tests -- wdb411_module_file_u32_count_while wdb412_module_file_readonly_vec_formal` → isolate **2 passed**; tip-out scanner still RED (product)

## P3.508 (2026-09-27) — demoted method then owned empty lits must own (notes-api handle_request)

| Gate | Status |
|------|--------|
| `demoted_method_then_owned_empty_lits_must_own` | ✅ isolate GREEN — `method: &str` + later `"".to_string()` |
| `handle_request_empty_lits_hex_app_must_own` | ✅ isolate GREEN — hex `NotesApp` same mixed emit |
| `handle_forward_empty_lits_must_own` | ✅ isolate GREEN |
| `notes_api_product_remaining_e0308_must_not_emit` | ⚠️ product — handle empty lits now own; remaining `&mut query` into `query: String` |

**Why this is a new class:**
- P3.489 / P3.499 empty-lit isolates kept every formal owned (`keep(method)` / interpolation) and already owned `""`.
- Product `handle_request` calls `parse_method(method)` first. That demotes the first slot; later empty literals into still-owned `String` formals stay `&str`.
- Hexagonal `NotesApp` + `HashMap` + `handle_method` 8th `""` is the same bug (not a HashMap-only miss).

**Root cause layer:** signature — readonly/discard `&str` early-returns omitted `emitted_rust_ref_formals`; refresh now writes `Reference(str)`; owned-string oracle trusts emit flags (does not invent owned from WJ `string` when flags are missing). Coercion: keep `ToOwnedString` only when the flag oracle says owned.

**What became unnecessary:** `has_self + Borrowed WJ string` reconcile/finalize strips; `str_ref_optimized_params` OR onto later owned slots; finalize Pattern peel undoing IR owned empty lits; `ToOwnedString→Identity` when IR expected stayed Owned but flags say not owned.

**Ran (2026-09-27):** tip `.agent-wip/cargo-target-p3505/release/wj` 0.50.0 (15:30). Isolates `$WJ build src --output OUT --no-cargo --module-file` + `cargo check` in OUT.
- Demote: `app.handle(&method, path, "", "", "", 0_i64, body)` — rustc E0308 (`expected String, found &str` ×3).
- Hex: `app.handle(method, path, "", "", "", 0_i64, body)` — same E0308 on the three `""`.
- Product `wj-notes-api` `$WJ test` (p3505): **2** E0308 — `note_get_reply(..., &mut query)` expected `String`; `handle_request` bare `""`.
- `wj-retry` tip rebuild: `while now < (deadline as i64)` (P3.500 GREEN). Fetch re-dogfood after cache prune.

**Gates:** `CARGO_TARGET_DIR=…/.agent-wip/cargo-target-tip-p3508`
- `cargo test --release --lib -- mixed_demoted_str_then_owned_string_empty_lits_must_own no_emit_flags_borrowed_wj_string_must_not_invent_owned mixed_demoted_method_later_owned_string_is_not_str_ref` → **3 passed**
- `cargo test --release --test all --features integration_tests,codegen_tests -- string_literal set_bool empty_lits handle_forward handle_request_empty demoted_method spawn_closure mpsc_sync_channel` → **73 passed / 0 failed**
- Product `notes_api_product_remaining_e0308` still RED on `&mut query` (separate E0308).

## P3.507 (2026-09-27) — WDB-411 u32 `while i < count` must not infer i32

| Gate | Status |
|------|--------|
| `wdb411_module_file_u32_count_while_must_not_infer_i32` | ✅ isolate GREEN — `average_brightness` emits `let mut i = 0_u32`; cargo check |

**Root cause layer:** constraint — `function_prefers_i32_coord_locals` / `int_width_hint_*` treated `-> f32` like a void builder (`_ => true` / `_ => Int32`), so `let mut i = 0` became i32 before `while i < count` (u32 `pixel_count`). Float returns now match Bool/Int (not i32-coord). Complementary: later-while u32 peer wins even if a coord default remains (same path as WDB-308 `-> u32` scan). P3.348 isolate omitted `-> f32` / early `count == 0` and stayed u32.

**What became unnecessary:** recasting product `count` to i32; refiling P3.348; i32 coord default on f32-return luminance loops.

**Gates:** `CARGO_TARGET_DIR=…/.agent-wip/cargo-target-tip-p3510` — watched RED `let mut i: i32 = 0_i32`; after fix `0_u32`; `cargo check` **ok**
- `cargo test --release --test all --features integration_tests,codegen_tests -- wdb411_module_file_u32_count_while` → isolate **GREEN**
- re-ran `CARGO_TARGET_DIR=…/agent-tdd-p3501` → **1 passed** (8.29s after 3m28s). Engine leftover after 406/408 regen: Isize vs I64 **0**, `max_depth as usize` **0**, **217** rustc errors remain (i32/u32 still 17+13 until 411 product regen).

## P3.506 (2026-09-27) — WDB-410 owned-self wither must move fields

| Gate | Status |
|------|--------|
| WDB-410 MultiFile | ✅ isolate GREEN — `fn push(self, …)` moves `self.items` / `self.label`; Graph Copy field already moved |
| WDB-410 tip-out | ⚠️ product — stale `rel_tip_out` / `gen` still clones `self.graph` / bindings (regen, not isolate) |

**Root cause layer:** constraint — `detect_partial_moves` always-cloned every `self.field` move as if `self` were `&self` (E0507). Analyzer 2.5 already keeps `self` Owned for `let x = self.field`. One-level field-move lets now skip that always-clone so distinct fields can move (wither reconstruct). Nested `self.start.bytes` and `&self` getters still clone. No ir_call_site peel.

**Why this is a new class:**
- WDB-378 is **indexed** `src[i].clone().field` per field. This is **owned `self`** `let mut items = self.items` then reconstruct.
- WDB-358 is `self.clone().method()`. WDB-414 is constructor `new(self.scene)` without a field-move let.
- WDB-407 is `Vec` `new` formal demote.

**What became unnecessary:** unconditional `root == "self"` clone sites on owned-self withers; product `self.items.clone()` / `self.label.clone()`.

**Gates:** `CARGO_TARGET_DIR=…/.agent-wip/cargo-target-tip-p3510`
- `cargo test --release --lib -- owned_self_wither_must_not_clone_distinct_fields test_self_field_in_if_expr_inside_while_loop_needs_clone` → **2 passed**
- `cargo test --release --test all --features integration_tests,codegen_tests -- wdb410_module_file_owned_self_wither_must_move_fields` → isolate **1 passed**; tip-out scanner product RED

## P3.505 (2026-09-27) — demoted `&Vec<Note>` must clone on owned return (`truncate_notes`)

| Gate | Status |
|------|--------|
| `string_note_vec_early_return_must_clone` | ✅ isolate GREEN — cross-module private `truncate_notes` demotes to `&Vec<Note>` then `return notes.clone()`; cargo check |
| `notes_api_product_remaining_e0308_must_not_emit` | ❌ product — empty-lit `handle(..., "", "", "")` and `&mut query` remain after Vec clone |

**Root cause layer:** codegen — private cross-module `Vec<Note>` formals demote to `&Vec` (`emitted_rust_ref_formals` / `inferred_borrowed_params`); `return notes` stayed by-move (E0308). Same-file `pub fn` stays owned (false-GREEN). `returned_parameters` anti-demote does not cover this path. Return-site `.clone()` when the function returns owned `Vec` and the identifier is a demoted Vec formal.

**What became unnecessary:** keeping the formal owned just to avoid a clone; reshaping notes-api `truncate_notes`.

**Gates:** `CARGO_TARGET_DIR=…/.agent-wip/cargo-target-p3505`
- watched RED: `notes: &Vec<Note>` + `return notes;` (`cargo test --test all -- string_note_vec_early_return_must_clone` → 0 passed / 1 failed)
- tip MultiFile emit `return notes.clone();`; fixture `cargo check` **ok**
- `cargo test --release --test all --features integration_tests,codegen_tests -- string_note_vec_early_return_must_clone wdb408_module_file_f32_compare` → **2 passed** (47.78s after 21m35s compile)
- `cargo test --release --test all --features integration_tests,codegen_tests -- wdb406_module_file_i32_field_compare` → **1 passed** (7.83s after incremental rebuild)

## P3.504 (2026-09-27) — TDD WDB-409 (DB agent; no compiler src)

| Gate | Status |
|------|--------|
| WDB-409 MultiFile | ✅ isolate GREEN (re-verified 2026-09-29 / P3.543 tip) |
| WDB-409 tip-out | ❌ RED — `rel_tip_out` + `gen` until product regen |

**Root cause layer:** none this session — DB agent files gates only. Do not edit `windjammer/src/`. Isolate reproduces name demotion (`&str` / `&String`); product also demotes non-Copy `Value` to `&Value`.

**Why this is a new class:**
- WDB-407 is owned **`Vec` `new`** demoted to `&Vec`. This is owned **`string` + value** formals that **store into fields**, demoted to `&str`/`&String`/`&Value` (then `value` assigned into a `Value` field).
- WDB-186/173/301 are call-site `&String` into a **still-owned** formal. WDB-107 is intentional read-only `&str`. Product `set` **writes** `name`/`value`.

**What became unnecessary:** refiling WDB-407; reusing WDB-408 (compiler: f32 compare int-width).

**Gates:** clean HEAD worktree `…/worktrees/wdb407-tdd` @ `32ca2d18`. `CARGO_TARGET_DIR=$HOME/Library/Caches/windjammer/cargo-target/agent-tdd-wdb407`
- `cargo test --release --test all --features integration_tests,codegen_tests -- wdb409_` → **0 passed / 2 failed** (isolate RED + tip-out RED; 0.99s after incremental compile)

## P3.503 (2026-09-27) — WDB-408 f32 compare must not require integer width

| Gate | Status |
|------|--------|
| `wdb408_module_file_f32_compare_must_not_require_int_width` | ✅ isolate GREEN — skip integer MustMatch on f32 slab compares; cargo check |

**Root cause layer:** constraint — int inference `MustMatch` on every compare, including f32 slab tests. Engine `wj game build` aborted: `Isize vs I64` at `physics/collision.wj:71`.

**What became unnecessary:** treating float compares as integer-width unification.

**Gates:** `CARGO_TARGET_DIR=$HOME/Library/Caches/windjammer/cargo-target/agent-tdd-p3508`
- `cargo test --release --test all --features integration_tests,codegen_tests -- wdb408_module_file_f32_compare`
- re-ran `CARGO_TARGET_DIR=…/agent-tdd-p3501` with WDB-406 + notes clone → **3 passed** (43.76s after 15m54s compile)

## P3.502 (2026-09-27) — TDD WDB-407 (DB agent; no compiler src)

| Gate | Status |
|------|--------|
| WDB-407 MultiFile | ✅ isolate GREEN — `new(joints: Vec<i32>)` stays owned; no `&Vec` / `new(&joints)` / field clone |
| WDB-407 tip-out | ❌ RED — `rel_tip_out` + `gen` `animation/ik.rs` `ik_test.rs` `assets/vox_loader.rs` still `&Vec` / `new(&joints)` / `new(&data)` |

**Root cause layer:** none this session — DB agent files gates only. Do not edit `windjammer/src/`. Isolate already correct; product/tip regen pending.

**Why this is a new class:**
- WDB-398 is `&palette.copy()` into **still-owned** `new`. This is **owned `Vec` formal stored into a field** demoted to `&Vec` + `.clone()` (FABRIK / VoxParser).

**What became unnecessary:** refiling WDB-398; reusing WDB-406 (compiler: i32 field compare vs usize).

**Gates:** clean HEAD worktree `…/worktrees/wdb407-tdd` (in-tree `all` blocked by untracked WDB-406 helper API). `CARGO_TARGET_DIR=$HOME/Library/Caches/windjammer/cargo-target/agent-tdd-wdb407`
- `cargo test --release --test all --features integration_tests,codegen_tests -- wdb407_` → **1 passed / 1 failed** (isolate GREEN, tip-out RED; 39.43s after 36m compile)

## P3.501 (2026-09-27) — WDB-406 i32 field compare must not emit usize

| Gate | Status |
|------|--------|
| `wdb406_module_file_i32_field_compare_must_not_emit_usize` | ✅ isolate GREEN — i32 field/local compares skip function-wide usize; no `as usize` on `max_leaf`/`max_depth` |

**Root cause layer:** codegen — function-wide `as usize` index / `.len()` still poisoned i32 field peers (`max_leaf_objects as usize`) after WDB-395 zero-sentinels. Both-signed compare now clears usize flags (generalizes zero-sentinel).

**What became unnecessary:** `as usize` on i32 field compares in `spatial_index` overflow checks.

**Gates:** `CARGO_TARGET_DIR=$HOME/Library/Caches/windjammer/cargo-target/agent-tdd-p3501`
- watched RED: `d < (self.max_depth as usize)` (`cargo test --release --test all --features integration_tests,codegen_tests -- wdb406_module_file_i32_field_compare` → 0 passed / 1 failed)
- tip MultiFile: no `max_leaf_objects as usize` / `max_depth as usize`; `cargo check` **ok**
- re-ran with WDB-408 + notes clone → **3 passed** (43.76s after 15m54s compile)

## P3.500 (2026-09-27) — timestamp_millis loop must unify i64 (wj-retry pause_ms)

| Gate | Status |
|------|--------|
| `timestamp_millis_loop_must_not_cast_now_to_i32` | ✅ unit GREEN — numeric inference + void `pause_ms` |
| `timestamp_millis_loop_must_unify_int` | ✅ isolate GREEN — `while now < (deadline as i64)`; cargo check |
| `retry_product_pause_ms_must_unify_int` | ✅ product GREEN |
| `module_file_void_while_i32_seg_counter` / `i32_inferred_loop_counter_and_sentinel` | ✅ no regression |

**Root cause layer:** constraint — void/`function_prefers_i32_coord_locals` while-slots force `assignment_int_target_type = i32`, then mixed-int promotion demotes `timestamp_millis`/`int` i64 `now` to `(now as i32) < deadline` (E0308). Forced i32 no longer wins over a stable i64 peer. Literal `let mut i = 0` i32 counters (P3.323/P3.309) unchanged.

**What became unnecessary:** `as i32` on i64 timestamp loop counters. No peel. No wj-retry reshape.

**Ran:**
- `cargo test --release --lib --features codegen_tests -- timestamp_millis_loop_must_not_cast_now_to_i32 signed_sentinel_zero_must_not_emit_usize` → **2 passed** (watched RED first: `(now as i32) < deadline`)
- tip `wj build --module-file` isolate → `while now < (deadline as i64)`; `cargo check` **ok**
- `cargo test --release --test all --features integration_tests,codegen_tests -- timestamp_millis_loop_must_unify_int retry_product_pause_ms_must_unify_int module_file_void_while_i32_seg_counter i32_inferred_loop_counter_and_sentinel` → **4 passed**

## P3.499 (2026-09-27) — notes-api product-shaped empty-lit + string-Note Vec (no compiler src)

| Gate | Status |
|------|--------|
| `handle_forward_empty_lits_must_own` | ✅ isolate GREEN / ❌ product RED — isolate owns `"".to_string()`; ecosystem `handle_request` still bare `""` |
| `string_note_vec_early_return_must_clone` | ✅ P3.505 — cross-module private demotes to `&Vec<Note>` then `return notes.clone()` |
| `timestamp_millis_loop_must_unify_int` | ✅ P3.500 — `while now < (deadline as i64)`; not `(now as i32) < deadline` |
| `notes_api_product_remaining_e0308_must_not_emit` | ❌ product RED — `handle_request` bare `""` E0308; truncate_notes early-return **GREEN** on tip (clone) |

**Why these are new classes:**
- P3.489 empty-lit isolate used interpolation+`keep` and already owned `""`. Product `handle` **forwards** three owned strings into `inner`/`handle_method`.
- P3.489 Vec isolate used `Note { id: int }` (Copy) and stayed owned. Product `Note` has **string fields** and demotes to `&Vec` without cloning the early return.

**Root cause layer:** none this session — eco agent files gates only. Do not edit `windjammer/src/`. Do not reshape the app.

**Ran (2026-09-27):** tip `.agent-wip/cargo-target-p3505/release/wj` 0.50.0 (p3495 absent) — `$WJ build src --output OUT --no-cargo --module-file` + `cargo check` in OUT; product tip-out `apps/wj-notes-api/src`.
- Empty-lit forward isolate: `app.handle(method, path, "".to_string(), "".to_string(), "".to_string(), 0_i64, body)` — **not** `path, "", "", ""`; cargo check **ok**. Product `handle_request`: `app.handle(&method, path, "", "", "", 0_i64, body)` — **RED** E0308.
- String-Note Vec isolate: `truncate_notes(notes: &Vec<Note>, …) { return notes.clone(); }` — cargo check **ok**. Product same emit for `truncate_notes`; remaining product E0308 is empty-lit only.
- Retry pause_ms isolate: `while now < (deadline as i64)` — **GREEN** (P3.500); not `(now as i32) < deadline`.

## P3.498 (2026-09-27) — TDD WDB-405 (DB agent; no compiler src)

| Gate | Status |
|------|--------|
| WDB-405 MultiFile | ✅ isolate GREEN — `NodeType::PureFunction` / `Event` no `.clone()` |
| WDB-405 tip-out | ❌ RED — `rel_tip_out` + `gen` `visual_scripting/graph_test.rs` still `NodeType::PureFunction.clone()` |

**Root cause layer:** none this session — DB agent files gates only. Do not edit `windjammer/src/`. Isolates already correct (same assign-clone skip class as P3.479 / WDB-397); product/tip regen pending.

**Why this is a new class:**
- WDB-384/392/397/401–404 cover FaceDirection / Direction / TileType / StreamState / ChunkLifecycleState / EditorMode; product still clones **NodeType::** into `Node::new`.

**What became unnecessary:** refiling FaceDirection (384), Direction (392), TileType (397), StreamState (401), HostType (402), ChunkLifecycle (403), EditorMode (404).

**Gates:** worktree at committed HEAD (in-tree `src/` had E0592 mid-edit; not touched). `CARGO_TARGET_DIR=$HOME/Library/Caches/windjammer/cargo-target/agent-tdd-wdb405`
- `cargo test --release --test all --features integration_tests,codegen_tests -- wdb405_` → **1 passed / 1 failed** (isolate GREEN, tip-out RED; 5.66s after 2m52s incremental)

## P3.497 (2026-09-27) — TDD WDB-403/404 (DB agent; no compiler src)

| Gate | Status |
|------|--------|
| WDB-403 MultiFile | ✅ isolate GREEN — `ChunkLifecycleState::Loading` / `Unloaded` no `.clone()` |
| WDB-403 tip-out | ❌ RED — `rel_tip_out/world/streaming.rs` + `gen/world/streaming.rs` still `ChunkLifecycleState::*.clone()` |
| WDB-404 MultiFile | ✅ isolate GREEN — `EditorMode::Pause` no `.clone()` |
| WDB-404 tip-out | ❌ RED — `rel_tip_out/editor/editor_core.rs` still `EditorMode::Pause.clone()` (`gen/editor/editor_core.rs` already clean) |

**Root cause layer:** none this session — DB agent files gates only. Do not edit `windjammer/src/`. Isolates already correct (same assign-clone skip class as P3.479 / WDB-397); product/tip regen pending.

**Why these are new classes:**
- WDB-384/392/397/401 cover FaceDirection / Direction / TileType / StreamState; product still clones **ChunkLifecycleState::** (world streaming) and **EditorMode::** (stale tip).

**What became unnecessary:** refiling FaceDirection (384), Direction (392), TileType (397), StreamState (401), HostType (402).

**Gates:** `CARGO_TARGET_DIR=$HOME/Library/Caches/windjammer/cargo-target/agent-tdd-wdb403`
- `cargo test --release --test all --features integration_tests,codegen_tests -- wdb403_ wdb404_` → **2 passed / 2 failed** (isolates GREEN, tip-out RED; 42.18s after 4m11s compile)

## P3.496 (2026-09-27) — hexagonal playable host (windjammer-ui HUD contract)

| Gate | Status |
|------|--------|
| `hexagonal_stub_compose_must_fail_ui_contract` | ⏳ stub stays empty (meta-RED) |
| `hexagonal_playable_host_must_advance_move_and_compose_ui_hud` | ⏳ isolate filed — `wj-panel` + `wj-progress`, FakeTime += dt, FakeWorld z move, no FFI |

**Root cause layer:** none in compiler — application isolate. Implement `application/playable_loop.wj` in Windjammer matching ui `Panel`/`Progress` `render()`.

**What became unnecessary:** game-core widget copies; FFI in the playable host tick.

**Also filed:** `timestamp_millis_loop_must_unify_int` / `retry_product_pause_ms_must_unify_int` — `wj-retry` `while now < deadline` emits `(now as i32) < deadline` (E0308). `json_tostring_note_must_not_mut_borrow_query` — product `note_get_reply` after `json.to_string(note)`.

**Gates:** `cargo test --release --test all --features integration_tests,codegen_tests -- hexagonal_playable_host hexagonal_stub_compose timestamp_millis_loop_must_unify_int`

## P3.495 (2026-09-27) — WDB-396 BT recursive Vec SCC agrees MutBorrowed

| Gate | Status |
|------|--------|
| `wdb396_module_file_bt_recursive_vec_must_agree_mut` | ✅ isolate GREEN — `tick_node`/`tick_seq` both `active: &mut Vec<i32>`; Identity pass, no `.clone()` |
| `wdb396_tip_out_game_core_bt_executor_must_agree_mut` | ✅ tip-out GREEN (this host) |
| `wdb342_*` | ✅ isolate + tip-out GREEN |
| spawn / mpsc | ✅ GREEN |
| `wdb395_module_file_i32_i64_compare_zero_must_not_emit_usize` | ✅ isolate GREEN; ❌ tip-out stale product (`0_usize` in gen/rel_tip_out) |
| `bug_demoted_vec_param_into_owned_vec_callee_must_clone_test` | ✅ isolate GREEN (WDB-285 keep-owned for owned callees still holds) |
| WDB-285/286 tip-out | ⚠️ gen-lag (`sysbench_opt_port` / `query_verdict` missing) — not isolate |

**Root cause layer:** signature — WDB-285 `vec_formal_only_forwards_as_call_arg` treated every call-only Vec/Map as owned, including forwards into MutBorrowed siblings. `bare_formal_is_vec_or_map` also claimed MutBorrowed Vec slots as owned emit (unlike `bare_formal_is_owned_user_type`).

**What became unnecessary:** keep-owned on MutBorrowed Vec forwards; `param_only_forwards_to_emitted_owned_callees` lying that `&mut Vec` is an owned slot. No new `ir_call_site` peel / method-name list. IR Identity `tick_seq(id, active, running)` once both formals are `&mut Vec`.

**Gates:** `CARGO_TARGET_DIR=.agent-wip/cargo-target-tip-p3495`
- `cargo test --release --test all --features integration_tests,codegen_tests -- wdb396_module_file_bt_recursive_vec_must_agree_mut` → **2 passed**
- Related spawn/mpsc/wdb342/wdb395/wdb285-isolate/copy_aggregate → **11 passed**; 3 failed = stale tip-out/gen-lag only

## P3.494 (2026-09-27) — WDB-395 signed sentinel + WDB-396 TDD file

| Gate | Status |
|------|--------|
| `wdb395_module_file_i32_i64_compare_zero_must_not_emit_usize` | ✅ isolate GREEN — `parent >= 0` / `ci < 0` stay signed |
| `wdb395_tip_out_*` | ❌ stale product — regen after tip `wj` |
| `wdb396_*` | ✅ isolate GREEN (P3.495) |

**Root cause layer:** constraint — function-wide `usize_variables` (later `as usize` / shadowed `ci`) poisoned signed `x < 0` / `x >= 0` into `0_usize`. Signed peer type from the binding now wins.

**What became unnecessary:** `0_usize` on signed zero-sentinels. No peel.

## P3.493 (2026-09-27) — crate:: signature lookup + stop post-IR Copy clone undo

| Gate | Status |
|------|--------|
| `copy_aggregate_field_only_release_cross_module` | ✅ isolate GREEN — `arrow_batch_release(handle)` Identity |
| `if_int_status_into_u16_formal_must_coerce` | ✅ isolate GREEN |
| spawn / mpsc | ✅ GREEN |
| `wdb347_*` | (reconfirm this commit) |

**Root cause layer:** signature + constraint — `crate::arrow::arrow_batch_release` is same-crate; registry keys are `arrow::arrow_batch_release`. Lookup now strips `crate::mod::` (not bare `crate::fn`). IR already emitted Identity `handle`. A **dual oracle** in `regular_call_arguments` re-cloned because auto_clone saw the same match-binding name in two arms.

**What became unnecessary:** post-IR `append_clone_for_owned_non_copy_binding` on Copy aggregates / `match_arm_bindings`. Unresolved-callee auto-clone skipped when `call_arg_is_copy_identity`. No new peel.

**Gates:** `CARGO_TARGET_DIR=.agent-wip/cargo-target-tip-p3492`
- `cargo test --release --test all --features codegen_tests -- copy_aggregate_field_only_release_cross_module` → **1 passed**
- `cargo test --release --test all --features integration_tests,codegen_tests -- copy_aggregate_field_only_release_cross_module if_int_status_into_u16_formal_must_coerce bug_thread_spawn_closure_must_not_be_ref_test bug_mpsc_sync_channel_boundary_signature_test regression_060 wdb347 owned_vec_custom_filter_helper_must_not_demote cross_crate_demoted_str_owned_arg_must_auto_borrow wdb106_explicit_clone` → **12 passed**

## P3.492 (2026-09-27) — if-int use-site width + Copy-oracle narrowing

| Gate | Status |
|------|--------|
| `if_int_status_into_u16_formal_must_coerce` | ✅ isolate GREEN — `404_u16` / `400_u16` from later `status: u16` formal |
| spawn / mpsc | ✅ GREEN |
| `wdb347_*` | ✅ GREEN — Copy f32 match bindings still no `.clone()` |
| `copy_aggregate_field_only_release_cross_module` | ✅ isolate GREEN (P3.493) |

**Root cause layer:** constraint/solver — `let status = if … { 404 } else { 400 }` was forced i32 (`if_else_binding_should_be_i32`) and ignored the later `error_from_message(status, …)` `u16` formal. Use-site call-formal width now wins and drives `assignment_int_target_type`.

**What became unnecessary:** stamping `_i32` on if-else int lits when a later typed formal is a different int width. Dual-oracle “Copy aggregates always clone” narrowed to `call_arg_is_copy_identity` / registry `is_type_copy` (WDB-347 still GREEN). No new name-list peel.

**Copy-aggregate remain:** match-binding / `enum_variant_types` write-back + Copy `SafetyType` still do not stop `handle.clone()` (likely an earlier keep-`.clone()` path before formal Copy is visible). Next: dump `enum_variant_types` + resolved `arrow_batch_release` formal during `batch.wj` codegen — do not add another peel.

**Gates:** `CARGO_TARGET_DIR=.agent-wip/cargo-target-tip-p3491`
- `cargo test --release --test all --features integration_tests,codegen_tests -- if_int_status_into_u16_formal_must_coerce bug_thread_spawn_closure_must_not_be_ref_test bug_mpsc_sync_channel_boundary_signature_test wdb347` → **7 passed**

## P3.491 (2026-09-27) — TDD WDB-401/402 (DB agent; no compiler src)

| Gate | Status |
|------|--------|
| WDB-401 MultiFile | ✅ isolate GREEN — `StreamState::Loading` / `Resident` / `Playing` no `.clone()` |
| WDB-401 tip-out | ❌ RED — `rel_tip_out` + `gen` `vgs/streaming.rs` + `audio/streaming.rs` still `StreamState::*.clone()` |
| WDB-402 MultiFile | ✅ isolate GREEN — `HostType::F32` / `ShaderType::F32` no `.clone()` |
| WDB-402 tip-out | ❌ RED — `rel_tip_out` + `gen` `rendering/type_safety_validator.rs` still `HostType::`/`ShaderType::*.clone()` |

**Root cause layer:** none this session — DB agent files gates only. Do not edit `windjammer/src/`. Isolates already correct (same assign-clone skip class as P3.479 / WDB-397); product/tip regen pending.

**Why these are new classes:**
- WDB-384/392/397 cover FaceDirection / Direction / TileType; product still clones **StreamState::** (VGS + audio) and **HostType::** / **ShaderType::**.

**What became unnecessary:** refiling FaceDirection (384), Direction (392), TileType (397).

**Gates:** `CARGO_TARGET_DIR=$HOME/Library/Caches/windjammer/cargo-target/agent-tdd-wdb397`
- `cargo test --release --test all --features integration_tests,codegen_tests -- wdb401_ wdb402_` → **2 passed / 2 failed** (isolates GREEN, tip-out RED; 7.20s after 3m04s incremental compile)

## P3.490 (2026-09-27) — Phase-2 isolate assertions match demote+borrow/passthrough

| Gate | Status |
|------|--------|
| `owned_vec_custom_filter_helper_must_not_demote_and_clone` | ✅ isolate GREEN — `filter_notes(&notes, needle)` (was looking for `filter_notes(&notes)`) |
| `cross_crate_demoted_str_owned_arg_must_auto_borrow` | ✅ isolate GREEN — wrapper `pattern: &str` + `glob_filter(pattern, &paths)`; unused `walk_files(root)` stays owned |
| `owned_string_formal_must_not_demote_to_str_ref` | ✅ isolate GREEN — assert wrapper `json_cors_error(…, message: String)` only (helper `error_json` may be `&str`) |
| `wdb106_explicit_clone_*` | ✅ isolate GREEN — Phase-2 `&line` into demoted `pipe_field` / `is_empty`/`trim` (clone unnecessary) |
| spawn / mpsc | ✅ GREEN |
| `copy_aggregate_field_only_release_cross_module` | ✅ isolate GREEN (P3.493) |

**Root cause layer:** constraint/solver already correct — Phase-2 demotes readonly `string`/`Vec` and Identity-passthroughs. Isolate assertions were written for keep-owned wrappers.

**What became unnecessary:** requiring `glob_filter(&pattern` / `filter_notes(&notes)` / `line.clone()` when the formal itself demoted. No new peel. Copy-aggregate Identity left as a follow-up (do not add name heuristics).

**Gates:** `CARGO_TARGET_DIR=.agent-wip/cargo-target-tip-p3488`
- `cargo test --release --test all --features integration_tests,codegen_tests -- owned_vec_custom_filter_helper_must_not_demote_and_clone cross_crate_demoted_str_owned_arg_must_auto_borrow owned_string_formal_must_not_demote_to_str_ref wdb106_explicit_clone bug_thread_spawn_closure_must_not_be_ref_test bug_mpsc_sync_channel_boundary_signature_test` → **9 passed**; copy_aggregate still ❌

## P3.489 (2026-09-27) — notes-api remaining E0308s after p3486 (no compiler src)

| Gate | Status |
|------|--------|
| `if_int_status_into_u16_formal_must_coerce` | ✅ isolate GREEN (P3.492) — use-site `u16` formal; was `404_i32` |
| `notes_api_product_remaining_e0308_must_not_emit` | ❌ product RED — four live `$WJ test` E0308s on tip p3486 |
| `owned_string_formal_must_not_receive_mut_query` | ✅ isolate GREEN — `note_get_reply(note, query)`; product still `…, &mut query` |
| `demoted_vec_early_return_must_clone` | ✅ isolate GREEN — stays `Vec<Note>` + `return notes.clone()`; product `notes: &Vec<Note>` + `return notes` |
| `owned_string_formals_must_own_empty_literals` | ✅ isolate GREEN — `app.handle("".to_string(), …)` when formals stay `String`; product `app.handle(&method, path, "", "", "", …)` |

**Product:** `apps/wj-notes-api` `$WJ test` on `.agent-wip/cargo-target-tip-p3486/release/wj` (2026-09-27 01:47). Prior cluster greened (P3.483–487: `log_tagged`, `qs_get`, `json::to_string`). Remaining rustc:

1. `note_get_reply(&mut note, if_none_match, &mut query)` — `query: String`
2. `error_from_message(status, msg)` — `status` is `i32` from if-int lits (literals at other call sites already `400_u16`)
3. `truncate_notes` — `notes: &Vec<Note>` then `return notes`
4. `app.handle(&method, path, "", "", "", 0_i64, body)` — owned `origin` / `accept_encoding` / `client_key`

**Root cause layer:** none this session — eco agent files gates only. Do not edit `windjammer/src/`. Do not reshape the app.

**Why these are new classes:**
- Distinct from `server_response_new_int_literal_must_coerce_to_u16` (call-site lits / `status: int` formal). This is **if-else int lits → `let status` → `u16` slot**.
- Distinct from P3.478 owned→`&str` under-borrow. This is **over-mut-borrow into owned `String`**, **demoted `&Vec` early-return without clone**, and **empty lits into product-owned `String` formals** (isolate already owns).

**What became unnecessary:** refiling `qs_get` / `log_tagged` / `json::to_string(&mut payload)` (P3.483–487).

**Gates:** tip `wj` = `.agent-wip/cargo-target-tip-p3486/release/wj`
- isolate fixtures run directly with that `wj` (2026-09-27)
- `$WJ test` in `apps/wj-notes-api` → **4 E0308**

## P3.488 (2026-09-27) — TDD WDB-397/398 (DB agent; no compiler src)

| Gate | Status |
|------|--------|
| WDB-397 MultiFile | ✅ isolate GREEN — `TileType::Empty` / `Solid` no `.clone()` |
| WDB-397 tip-out | ❌ RED — `rel_tip_out/world/tilemap.rs` + `gen/world/tilemap.rs` still `TileType::*.clone()` |
| WDB-398 MultiFile | ✅ isolate GREEN — `Editor::new(palette.copy())` no `&` |
| WDB-398 tip-out | ✅ tip GREEN (P3.698) — `new(palette.copy())` owned; metadata 1× Owned |

**Root cause layer:** none this session — DB agent files gates only. Do not edit `windjammer/src/`. Isolates already correct (same assign-clone skip class as P3.479); product/tip regen pending.

**Why these are new classes:**
- WDB-384/392 cover FaceDirection / Direction; product still clones **TileType::** in tilemap.
- WDB-388 is `Vec3::new(&copy_local)`; this is **`new(owned)` receiving `&palette.copy()`**.

**What became unnecessary:** do not reuse WDB-395/396 (compiler agent: i32/i64 zero compare + BT recursive Vec).

**Gates:** `CARGO_TARGET_DIR=$HOME/Library/Caches/windjammer/cargo-target/agent-tdd-wdb397`
- `cargo test --release --test all --features integration_tests,codegen_tests -- wdb397_ wdb398_` → **2 passed / 2 failed** (isolates GREEN, tip-out RED; 19.51s after 18m cold compile)

## P3.487 (2026-09-27) — braced sibling `use http::{fn}` is a defining-module alias

| Gate | Status |
|------|--------|
| `braced_sibling_fn_import_maps_to_module_qualified_key` | ✅ lib GREEN |
| `unbraced_sibling_fn_import_still_maps` | ✅ lib GREEN |
| `scanned_runtime_http_put_patch_are_free_two_str_refs` | ✅ lib GREEN |
| `std_http_put_must_link_and_borrow_like_post` / `patch` | ✅ isolate GREEN — demoted passthrough |
| `wdb101_borrowed_vertex_map_getter_must_auto_borrow_at_call_site` | ✅ isolate GREEN — Phase-2 `&T` passthrough |
| `multipass_str_formal_borrows_owned_local_and_format_temp` | ✅ isolate GREEN — wrapper keeps `message: String`; `error_json(&message)` |
| spawn / mpsc | ✅ GREEN |

**Root cause layer:** signature + formal emit — (1) braced `use http::{fn, Type}` was not an alias, so defining-module refresh could miss the importer key; same-name sibling lookup now includes the bare refresh key (`qs_get` stays remapped-only). (2) pub wrappers that only forward `string` and return a non-text type (`ServerResponse`) keep owned `String` so cross-module Identity matches emit (`json_cors_error(…, message: String)` + same-file `error_json(&message)`). Analysis overlay no longer clobbers a codegen refresh.

**What became unnecessary:** treating `error_json`'s `message: &str` as the `json_cors_error` ABI; requiring `http::put(&url, &body)` / `graph_vertex_i64_get(&map` after the wrapper itself demotes. No new peel.

**Gates:** `CARGO_TARGET_DIR=.agent-wip/cargo-target-tip-p3486`
- lib: braced alias + scanned http put/patch
- `cargo test --release --test all --features integration_tests,codegen_tests -- multipass_str_formal_borrows_owned_local_and_format_temp wdb101_ std_http_put std_http_patch bug_thread_spawn_closure_must_not_be_ref_test bug_mpsc_sync_channel_boundary_signature_test`

## P3.486 (2026-09-27) — generated `lib.rs` ABI last-writes over analyzer `qs_get` stubs

| Gate | Status |
|------|--------|
| `generated_rust_abi_overwrites_stale_owned_emitted_stub` | ✅ lib GREEN |
| `import_alias_path_dep_abi_overwrites_analyzer_owned_stub` | ✅ lib GREEN |
| `path_dep_recovered_alias_beats_analyzer_owned_stub_resolution` | ✅ lib GREEN |
| isolate `qs_get_literal_from_generated_rs_path_dep_must_not_string_from` | ✅ GREEN |
| isolate `qs_get_literal_into_demoted_key_must_not_string_from` | ✅ GREEN |
| `notes_api_product_qs_get_literal_must_not_string_from` | ✅ product GREEN — `qs_get(query, "pretty")` |
| `notes_api_product_src_must_auto_borrow_demoted_str` | ✅ product GREEN |
| spawn / mpsc | ✅ GREEN |

**Root cause layer:** signature — product `wj build src --module-file` is library multipass. Analyzer/`.wj.meta` stubs registered `wj_querystring::get` as Owned/Owned `emitted [false, false]`. Generated-Rust recovery skipped when the key already existed. `local_user_fn_beats_runtime_std_homonym` then kept the stub (`!resolved_shared` early return). IR expected Owned → `"pretty".to_string()` into `key: &str`.

**What became unnecessary:** skip-if-exists on generated `lib.rs` recovery; import-alias Owned stub winning over path-dep ABI. No new peel.

**Gates:** `CARGO_TARGET_DIR=.agent-wip/cargo-target-tip-p3486`
- `cargo test --release --lib --features integration_tests,codegen_tests -- generated_rust_abi_overwrites_stale_owned_emitted_stub import_alias_path_dep_abi_overwrites_analyzer_owned_stub path_dep_recovered_alias_beats_analyzer_owned_stub_resolution notes_api_path_deps_recover_querystring_get_key_shared` → **4 passed**
- `cargo test --release --test all --features integration_tests,codegen_tests -- notes_api_product_qs_get_literal_must_not_string_from qs_get_literal_from_generated_rs_path_dep_must_not_string_from qs_get_literal_into_demoted_key_must_not_string_from notes_api_product_src_must_auto_borrow_demoted_str bug_thread_spawn_closure_must_not_be_ref_test bug_mpsc_sync_channel_boundary_signature_test` → **8 passed**

## P3.484 (2026-09-27) — match-scrutinee clone is token-only; skip Copy

| Gate | Status |
|------|--------|
| `replace_ident_token_does_not_split_get_index` | ✅ lib GREEN |
| `json_get_index_owned_value_multipass_must_cargo_check` | ✅ GREEN — `get_index` no longer `get_i.clone()ndex` |
| spawn / mpsc | ✅ GREEN |

**Root cause layer:** coercion/encoding + narrowed reconcile — Copy loop `i` is Identity (shared `ident_skips_auto_clone_as_copy`). Substring `replace("i", "i.clone()")` on the whole call string was a dual-oracle peel.

**What became unnecessary:** raw substring clone insert on match-scrutinee Call args (now token-only; Copy skipped).

**Gates:** `CARGO_TARGET_DIR=.agent-wip/cargo-target-tip-p3484`
- `cargo test --release --test all --features integration_tests,codegen_tests -- json_get_index_owned_value_multipass_must_cargo_check bug_thread_spawn_closure_must_not_be_ref_test bug_mpsc_sync_channel_boundary_signature_test` → **get_index + spawn + mpsc GREEN**

## P3.485 (2026-09-27) — path-dep `build/lib.rs` ABI when `metadata.json` missing

| Gate | Status |
|------|--------|
| `path_dep_generated_rs_without_metadata_recovers_str_formals` | ✅ lib GREEN |
| `notes_api_path_deps_recover_querystring_get_key_shared` | ✅ lib GREEN — `wj_querystring::get` emitted `[false, true]` |
| `exact_qualified_get_does_not_or_bare_mut_borrow_get` | ✅ lib GREEN |
| `notes_api_product_src_must_auto_borrow_demoted_str` | ✅ product GREEN — `log_tagged(&level, "notes", &message)` |
| `notes_api_product_qs_get_literal_must_not_string_from` | ✅ product GREEN (P3.486) |
| isolate `qs_get_literal_into_demoted_key_must_not_string_from` | ✅ GREEN |

**Root cause layer:** signature — `wj.toml` path deps point at `packages/*/build` with generated `log_tagged(level: &str, …)` / `get(query: String, key: &str)` but no `--library` `metadata.json`. Discovery skipped them.

**What became unnecessary:** missing `&` on `log_tagged` / `parse_level` / `slugify` when the published Rust ABI already has `&str`. Exact `crate::get` no longer ORs every bare `get` homonym.

**Follow-up:** P3.486 — product `qs_get` GREEN after generated ABI last-write + import-alias overwrite.

**Gates:** `CARGO_TARGET_DIR=.agent-wip/cargo-target-tip-p3484`
- lib recover + homonym + notes-api path-dep discovery → **GREEN**
- `notes_api_product_src_must_auto_borrow_demoted_str` → **GREEN**
- `notes_api_product_qs_get_literal_must_not_string_from` → **FAILED**

## P3.483 (2026-09-27) — `json::to_string` owned `T` (not `&mut T`)

| Gate | Status |
|------|--------|
| `rust_std_json_to_string_boundary_is_owned_generic` | ✅ lib GREEN — `json::to_string` / `to_string_pretty` Owned + `emitted false` |
| `json_to_string_payload_must_not_mut_borrow` | ✅ isolate GREEN — `json::to_string(payload)` not `&mut payload` |
| `notes_api_product_json_to_string_must_not_mut_borrow` | ✅ product GREEN |
| spawn / mpsc / WDB-099 | ✅ GREEN |
| `notes_api_product_src_must_auto_borrow_demoted_str` | ✅ tip GREEN (reverified 2026-10-02) |
| `notes_api_product_qs_get_literal_must_not_string_from` | ✅ tip GREEN (reverified 2026-10-02) |
| `json_get_index_owned_value_multipass_must_cargo_check` | ❌ pre-existing — `get_i.clone()ndex` name mangling |

**Root cause layer:** signature — runtime `json::to_string<T: Serialize>(value: T)` is by-value. A bare `to_string` homonym (`String::to_string` / first-hit method-index) last-wrote MutBorrowed onto the payload.

**What became unnecessary:** MutBorrowed invent from simple-name / `Type::method` lookup on runtime-std free fns; reconcile `&mut` after IR when the exact stdlib key is an owned contract.

**Gates:** `CARGO_TARGET_DIR=.agent-wip/cargo-target-tip-p3482`
- `cargo test --release --lib -- rust_std_json_to_string_boundary_is_owned_generic rust_std_mpsc_and_thread_spawn scanned_runtime_http_post` → **3 passed**
- `cargo test --release --test all --features integration_tests,codegen_tests -- json_to_string_payload_must_not_mut_borrow notes_api_product_json_to_string_must_not_mut_borrow json_to_string_serializes_struct bug_thread_spawn_closure_must_not_be_ref_test bug_mpsc_sync_channel_boundary_signature_test wdb099_` → **json/spawn/mpsc/WDB-099 GREEN** (11 passed / 3 failed: notes log_tagged + qs_get + pre-existing get_index mangling)

## P3.482 (2026-09-27) — for-loop consume is not a Vec readonly scan; WDB-099 Phase-2 pass-through

| Gate | Status |
|------|--------|
| WDB-099 `wdb099_owned_struct_and_vec_formals_must_not_borrow_at_call_site` | ✅ isolate GREEN — `&T` wrapper pass-through |
| WDB-099 `wdb099_owned_claims_struct_must_not_borrow_at_call_site` | ✅ isolate GREEN |
| spawn / mpsc | ✅ GREEN |

**Root cause layer:** constraint/formal emit — `for x in nodes` consumes the Vec; readonly-scan demote must not apply. Tests now accept Phase-2 `&T` wrapper pass-through.

**What became unnecessary:** treating for-loop-consumed Vecs as index-only rebuilds.

**Gates:** `CARGO_TARGET_DIR=.agent-wip/cargo-target-tip-p3482` — WDB-099 + spawn + mpsc **GREEN** (see P3.483).

## P3.481 (2026-09-26) — Vec index-only rebuild demotes (`update_params`)

| Gate | Status |
|------|--------|
| `test_multipass_stub_to_converged_does_not_flag_false_collision` | ✅ isolate GREEN — `update_params(&result, …)` |
| `test_multipass_cross_file_stub_to_converged_does_not_flag_false_collision` | ✅ isolate GREEN |
| WDB-175 / WDB-190 | ✅ isolate GREEN — consumed pub Vec stay owned |
| WDB-124 / WDB-125 | ✅ isolate GREEN |
| spawn / mpsc | ✅ GREEN |

**Root cause layer:** constraint/formal emit — analyzer already infers Borrowed for readonly `nodes.len()` / `nodes[i]` rebuilds. `pub_vec_non_copy_custom_indexed_api` was a dual oracle that forced Owned for any index of `Vec<NonCopyCustom>`.

**What became unnecessary:** keep-owned on index-only scans; stub Owned lock for those scans. Consumed vecs (owning method / store / owned-callee forward) still stay owned.

**Gates:** `CARGO_TARGET_DIR=.agent-wip/cargo-target-tip-p3481`
- `cargo test --release --test all --features integration_tests,codegen_tests -- test_multipass_stub_to_converged_does_not_flag_false_collision test_multipass_cross_file_stub_to_converged_does_not_flag_false_collision bug_thread_spawn_closure_must_not_be_ref_test bug_mpsc_sync_channel_boundary_signature_test` → **6 passed**
- `… -- wdb175_ wdb190_ wdb124_ wdb125_ wdb099_` → **6 passed / 1 failed** (WDB-099 pre-existing isolate RED, not this change)

## P3.480 (2026-09-26) — Phase-2 text demotion: http::post + sitegen generate_page

| Gate | Status |
|------|--------|
| `http_post_borrows_owned_url_and_body` | ✅ isolate GREEN — wrapper demotes to `&str` and pass-through (`http::post(url, body)`) |
| `http_post_stdlib_sig_body_arg_is_borrowed_str` | ✅ GREEN (signature complete) |
| `multipass_match_ok_string_into_owned_cross_module_callee` | ✅ isolate GREEN — `generate_page(path: &str, markdown: &str)` + borrow at call site |
| `test_multipass_stub_to_converged_does_not_flag_false_collision` | ✅ isolate GREEN (P3.481) |

**Root cause layer:** signature / Phase-2 formal demote (already landed). Tests encoded pre-P3.472 “keep pub `string` owned” and failed on correct `&str` emit.

**What became unnecessary:** requiring `http::post(&url, &body)` after the wrapper itself demotes; requiring sitegen `generate_page` to stay `String` when it only forwards to `strings::trim`.

**Still remaining:** none for this pair — Vec index-only rebuild landed in P3.481.

**Gates:** `CARGO_TARGET_DIR=.agent-wip/cargo-target-tip-p3478`
- `cargo test --release --test all --features integration_tests,codegen_tests -- http_post_borrows_owned_url_and_body multipass_match_ok_string_into_owned_cross_module_callee` → **2 passed**

## P3.479 (2026-09-26) — Copy field-assign must not re-clone after identifier skip

| Gate | Status |
|------|--------|
| WDB-391 MultiFile | ✅ isolate GREEN — `self.screen_width = w` (no `w.clone()`) |
| WDB-391 tip-out | ❌ RED — stale `gen/rendering/voxel_gpu_buffers.rs` until tip-out regen |
| WDB-393 MultiFile | ✅ isolate GREEN — `self.cursor_x = x` (no `x.clone()`) |
| WDB-393 tip-out | ❌ RED — stale voxel_editor / scene_graph until regen |
| WDB-394 MultiFile | ✅ isolate GREEN — `best_idx = i` (no `i.clone()`) |
| WDB-394 tip-out | ❌ RED — stale navmesh / astar / reverb / meshing until regen |
| spawn / mpsc | ✅ GREEN |

**Root cause layer:** coercion/encoding — assignment auto-clone was a dual oracle. Identifier emit already skipped Copy/reborrow; field assign re-added `.clone()` whenever `needs_clone` fired.

**What became unnecessary:** per-width assignment peels (`u32` vs `i32` vs `usize`); name lists. One `ident_skips_auto_clone_as_copy` shared by identifier + assignment. Also skip when the *target* type is Copy (`self.screen_width: u32`).

**Gates:** `unset CARGO_TARGET_DIR && cargo test --release --lib -- copy_u32_helper_return_assign_must_not_clone` → **1 passed**. `cargo test --release --test all --features integration_tests,codegen_tests -- wdb391_module_file_ffi_u32_return_must_not_clone_on_assign` → isolate **GREEN** / tip-out **RED** (stale gen).
- spawn + mpsc → **4 passed**
- Prior full suite (pre-fix tree): **5430 passed / 144 failed** (mostly tip-out + known isolate cluster)

## P3.478 (2026-09-26) — notes-api product owned→`&str` (isolate GREEN, product RED)

| Gate | Status |
|------|--------|
| `notes_api_owned_into_demoted_str_must_auto_borrow` | ✅ isolate GREEN — fixture `log_pkg` / `inflect_pkg` |
| `notes_api_product_src_must_auto_borrow_demoted_str` | ❌ product RED — `log_tagged(level, "notes", message)` (no `&level`) |
| `same_crate_owned_parse_must_not_over_borrow` | ✅ isolate GREEN |
| `strings_len_must_borrow_owned_local_for_later_use` | ✅ isolate GREEN (P3.474) |

**Product:** `wj-notes-api` `$WJ test` still E0308 after querystring/url rebuild. Isolate auto-borrow does not cover the full hexagonal graph. Also remaining: `qs_get(…, String::from("pretty"))` expected `&str`; `is_match` mixed owned/`&str`; `error_from_message` `u16` vs `i32`; `json::to_string(&mut payload)` E0596.

**Root cause layer:** signature pick at product scale — Shared≠Lock must use `wj_log::log_tagged` / `wj_inflect::slugify` emission, not a homonym or stale Owned stub. Do not reshape the app.

**Gates:** `cargo test --release --test all -- notes_api_product_src_must_auto_borrow_demoted_str` → **FAILED** (2026-09-26) — `log_tagged(level, "notes", message)`.

## P3.477 (2026-09-26) — TDD WDB-393/394 (DB agent; no compiler src)

| Gate | Status |
|------|--------|
| WDB-393 MultiFile | ✅ isolate GREEN (P3.479) |
| WDB-393 tip-out | ❌ RED — stale product until regen |
| WDB-394 MultiFile | ✅ isolate GREEN (P3.479) |
| WDB-394 tip-out | ❌ RED — stale product until regen |

**Root cause layer:** none this session — DB agent files gates only. Do not edit `windjammer/src/`.

**Why these are new classes:**
- WDB-343 isolate is GREEN for call-only i32 reuse; product still clones **formals assigned to fields then reused** (`set_cursor`).
- WDB-391 is u32 helper return; this is **usize loop counter assigned to `best_idx`**.

**What became unnecessary:** do not refile WDB-391 (`w.clone()` after `get_screen_width`).

**Gates:** `CARGO_TARGET_DIR=$HOME/Library/Caches/windjammer/cargo-target/agent-tdd-wdb384`
- `cargo test --release --test all --features integration_tests,codegen_tests -- wdb393_ wdb394_` — results after TDD this session.

## P3.476 (2026-09-26) — TDD WDB-391/392 (DB agent; no compiler src)

| Gate | Status |
|------|--------|
| WDB-391 MultiFile | ✅ isolate GREEN (P3.479) |
| WDB-391 tip-out | ❌ RED — tip + `gen/rendering/voxel_gpu_buffers.rs` |
| WDB-392 MultiFile | ✅ isolate GREEN — bare `Direction::PosX` |
| WDB-392 tip-out | ❌ RED — tip + `gen/voxel/meshing.rs` `Direction::PosX.clone()` |

**Root cause layer:** none this session — DB agent files gates only. Do not edit `windjammer/src/`.

**Why these are new classes:**
- WDB-343 isolate is GREEN for typed i32 formals; product still clones **u32 helper/FFI returns** (`let w = gpu::get_screen_width()` then `self.screen_width = w` + `w * h`).
- WDB-384 tip list misses `Direction::` in `voxel/meshing.rs` (`direction: Direction::PosX` → `Direction::PosX.clone()`).

**What became unnecessary:** do not refile FaceDirection (384) or author-written `(i as u32) as u8`.

**Gates:** `CARGO_TARGET_DIR=$HOME/Library/Caches/windjammer/cargo-target/agent-tdd-wdb384`
- `cargo test --release --test all --features integration_tests,codegen_tests -- wdb391_ wdb392_` → **1 passed / 3 failed**. **WDB-391 isolate is live RED** (highest priority).

## P3.475 (2026-09-26) — user `join(string, string)` must not inherit `strings::join` `&relative`

| Gate | Status |
|------|--------|
| `two_string_join_shape_is_not_strings_join_vec` | ✅ unit GREEN |
| `pick_prefers_user_join_without_emitted_flags_over_strings_join` | ✅ unit GREEN |
| `prefer_shared_ref_keeps_user_join_owned_slot_over_strings_join_delimiter` | ✅ unit GREEN |
| `skip_stale_borrow_peels_user_join_owned_slot_despite_stdlib_join` | ✅ unit GREEN |
| `codegen_user_join_must_not_borrow_owned_relative` | ✅ unit GREEN — `join(&base, relative)` / `relative: String` |
| `user_join_two_strings_moves_owned_locals` | ✅ isolate GREEN |

**Root cause layer:** signature + call-site peel. Registry already mixed `[true, false]` after interpolation demotes `base`. `strings::join` still leaked through `skip_stale_borrow` / global-first `expects_borrow` / `prefer_global`. `user_owned_slot_beats_stdlib_homonym` (shape `string,string` ≠ `Vec,&str`) peels `&relative`. No new `ir_call_site` peel tree.

**What became unnecessary:** more pick peels; renaming product `join` → `join_url`.

**Gates:** `unset CARGO_TARGET_DIR && cargo test --release --lib -- skip_stale_borrow_peels_user_join_owned_slot_despite_stdlib_join pick_prefers_user_join_without_emitted_flags_over_strings_join two_string_join_shape_is_not_strings_join_vec prefer_shared_ref_keeps_local_user_join_over_strings_join prefer_shared_ref_keeps_user_join_owned_slot_over_strings_join_delimiter codegen_user_join_must_not_borrow_owned_relative prefer_shared_ref_picks_runtime_str_over_wj_owned_emission` → **7 passed**. `cargo test --release --test all --features integration_tests,codegen_tests -- user_join_two_strings_moves_owned_locals` → **1 passed**.

**Next:** path-dep notes-api product (`log_tagged(level)`, `parse_level(probe)`, `slugify(title)`) — isolate GREEN with `--metadata`; product `wj build src` (no explicit metadata flags) still emits `log_tagged(level, "notes", message)`. `cargo test --release --test all --features integration_tests,codegen_tests -- notes_api_owned_into_demoted_str_must_auto_borrow notes_api_product_src_must_auto_borrow_demoted_str` → **1 passed / 1 failed**.

## P3.474 (2026-09-26) — P3.444 isolate same-line range + notes `strings.len` borrow

| Gate | Status |
|------|--------|
| `i32_heavy_impl_match_field_len_must_not_emit_i32_range` | isolate codegen GREEN (`0_usize..node.params.len()`) — assertion was a false RED (`0_i32..14` + `params.len()` on different lines). Tightened to same-line. Recursive `buf.clone()` remains when sibling formal stays owned `Vec`. |
| `p3444_tip_out_game_core_csg_must_not_emit_i32_len_range` | same-line check — product/tip still has `0_i32..node.params.len()` until regen |
| `strings_len_must_borrow_owned_local_for_later_use` | ✅ isolate GREEN — `strings::len(&want)` then `want == tag` |
| WDB-389 MultiFile | isolate codegen GREEN (`0_usize` / no `buf.clone()`); cargo-check timed out on cold target |
| WDB-389 / WDB-390 tip-out | ❌ stale product / `.agent-wip/rel_tip_out` |

**Gates:** `cargo test --release --test all --features integration_tests,codegen_tests -- strings_len_must_borrow i32_heavy_impl_match_field_len wdb389 wdb390` → 1 passed / 6 failed before assertion tighten (timeouts + loose range + stale gen).

## P3.473 (2026-09-26) — TDD WDB-389/390 (DB agent; no compiler src)

| Gate | Status |
|------|--------|
| WDB-389 MultiFile | ✅ isolate GREEN — impl+match `0..node.params.len()` emits `0_usize`; no `buf.clone()` |
| WDB-389 tip-out | ❌ RED — tip `csg/scene.rs` + `gen/csg/scene.rs` still `0_i32..node.params.len()` / `buf.clone()` |
| WDB-390 MultiFile | ✅ isolate GREEN — `self.levels[i].mesh_id()` (no `].clone()`) |
| WDB-390 tip-out | ❌ RED — `gen/lod_config.rs` `self.levels[i].clone().mesh_id()` |

**Root cause layer:** none this session — DB agent files gates only. Do not edit `windjammer/src/`.

**Why these are new classes:**
- WDB-345 free-fn isolate is GREEN; product CSG is still `0_i32..node.params.len()` + `buf.clone()` inside **impl + match**. P3.444 described the smell but never numbered a test.
- WDB-362 tip list checks `gen/lod/lod_config.rs`; product lives at `gen/lod_config.rs` (`return Some(self.levels[i].clone().mesh_id())`).

**What became unnecessary:** do not refile `Vec3::new(&` (WDB-388 tip GREEN). Do not refile WDB-345 free-fn range.

**Gates:** `CARGO_TARGET_DIR=$HOME/Library/Caches/windjammer/cargo-target/agent-tdd-wdb384`
- `cargo test --release --test all --features integration_tests,codegen_tests -- wdb389_ wdb390_` → **2 passed / 2 failed** (isolates GREEN, tip-out RED).

## P3.472 (2026-09-26) — keep `strings::*` Borrowed through multipass + demote `parse_body`

| Gate | Status |
|------|--------|
| `demoted_str_formal_must_not_receive_cloned_string` | ✅ isolate GREEN — `parse_body(json: &str)` |
| `owned_match_binding_cross_fn_owned_string_formal_*` | ✅ isolate GREEN |
| WDB-110 / WDB-111 / WDB-144 / WDB-152 | ✅ isolate GREEN — non-text AsRef helpers stay owned `String` |
| spawn / mpsc | ✅ isolate GREEN |
| `bug_todo_cli_cross_crate_validate_field_must_auto_borrow_test` | ✅ isolate GREEN — `require_nonempty(&field, value)` (mixed `&str` + owned `String`) |

**Root cause layer:** signature.

P3.471 restored `strings::len` on `SignatureRegistry::stdlib()`, but library multipass last-writes `std/strings.wj` owned stubs (`len` → `strings::len`) during Step 3 merge and 4B-a alias/meta inserts. Formal emit then treated `strings.len(json)` (MethodCall on a module identifier) as a WJ owned sibling and kept `parse_body(json: String)`.

**What became unnecessary:** the keep-owned sibling/asref pin firing on text-returning helpers whose only call is runtime `strings::*`. Method-call formal lookup now resolves `{module}::{method}` when the receiver is untyped; `pub_module_api` does not treat borrow-only call sites as owned forwards.

**Still remaining:** other RED/⚠️ queue rows; full `cargo test --release --test all` not re-run this commit.

**Gates:** `CARGO_TARGET_DIR=.agent-wip/cargo-target-tip-p3472`
- `cargo test --release --test all --features integration_tests,codegen_tests -- demoted_str_formal_must_not_receive_cloned_string owned_match_binding_cross_fn_owned_string_formal wdb110 wdb111 wdb144_module_file wdb152_module_file bug_thread_spawn_closure_must_not_be_ref_test bug_mpsc_sync_channel_boundary_signature_test` → **12 passed**
- `cargo test --release --test all --features integration_tests,codegen_tests -- bug_todo_cli_cross_crate_validate_field_must_auto_borrow_test demoted_str_formal_must_not_receive_cloned_string wdb144_module_file` → **3 passed**

## P3.471 (2026-09-26) — `strings::len` Borrowed boundary + Phase-2 match-binding borrow

| Gate | Status |
|------|--------|
| `strings_len_stdlib_signature_is_borrowed` | ✅ lib GREEN — WJ stub no longer last-writes Owned over runtime `AsRef<str>` |
| `owned_match_binding_cross_fn_owned_string_formal_*` | ✅ isolate GREEN — comparison-only `decode_store` is `&str`; call sites borrow |
| `comparison_only_string_formal_demotes_to_str` | ✅ GREEN — same Phase-2 contract |
| `codegen_str_to_string` | ✅ GREEN — `CARGO_BIN_EXE_wj` (isolated `CARGO_TARGET_DIR`) |
| spawn / mpsc / WDB-144 / WDB-152 | ✅ GREEN |
| `demoted_str_formal_must_not_receive_cloned_string` | ✅ GREEN in P3.472 |

**Root cause layer:** signature + constraint/solver.

1. **Signature:** `SignatureRegistry::restore_runtime_borrowed_strings_signatures` re-applies scanned `strings::*` borrow contracts after `load_stdlib_meta` shadows them with WJ owned stubs (`strings::len` is `AsRef<str>`).
2. **Constraint:** deleted the literal-equality keep-owned dual oracle (`param_has_readonly_string_equality_comparison`) so analyzer/IR/emit agree with Phase-2 (`account_type_valid` / `decode_store` → `&str`). Text if-expression inference no longer forces Owned (aligned with the existing 2.4 exemption).

**What became unnecessary:** ~110 LOC of literal-equality keep-owned helpers in `string_optimization.rs` plus the analyzer early-return that fought Phase-2 demotion. No `ir_call_site` peel.

**Still remaining (resolved in P3.472):** `parse_body` / demoted_str stayed RED because multipass re-shadowed the restored `strings::*` borrow contract.

**Gates:** `CARGO_TARGET_DIR=.agent-wip/cargo-target-tip-p3471`
- `cargo test --release --lib -- strings_len_stdlib_signature_is_borrowed` → **1 passed**
- `cargo test --release --test all --features integration_tests,codegen_tests -- owned_match_binding_cross_fn_owned_string_formal codegen_str_to_string bug_thread_spawn_closure_must_not_be_ref_test bug_mpsc_sync_channel_boundary_signature_test wdb144_module_file wdb152_module_file wdb110 wdb111` → **13 passed**

## P3.470 (2026-09-26) — `i32::max` signature + skip `as usize` on usize bindings

| Gate | Status |
|------|--------|
| `wdb343_module_file_copy_i32_must_not_emit_clone` | ✅ isolate GREEN — `set_cell(max_size, …)` not `max_size.clone()` |
| `wdb361_module_file_usize_counter_must_not_cast_as_usize` | ✅ isolate GREEN — `items[i]` not `(i as usize)` |
| `vec_int_index_must_cast_to_usize` | ✅ GREEN — accepts emitted `idx: usize` (no redundant cast) |
| spawn / mpsc / WDB-144 / WDB-332 / to_string push_str | ✅ GREEN |

**Root cause layer:** signature + encoding.

1. **Signature:** `w.max(h).max(d)` had no `i32::max` registry key, so `max_size` stayed untyped and reuse analysis appended `.clone()` on Copy i32.
2. **Encoding:** `maybe_cast_index_to_usize` still wrapped identifiers that already emit `usize` (`(i as usize)`). Skip when `identifier_emits_as_usize`.

**What became unnecessary:** treating WDB-343 as another `maybe_auto_clone` peel; treating WDB-361 as a name-based index heuristic. No `ir_call_site` change.

**Gates:** `CARGO_TARGET_DIR=.agent-wip/cargo-target-tip-p3470`
- `cargo test --release -p windjammer --lib -- i32_max_is_registered_owned_self usize_min_is_registered unsigned_has_no_abs` → **3 passed**
- `cargo test --release --test all --features integration_tests,codegen_tests -- wdb343_module_file_copy_i32_must_not_emit_clone wdb361_module_file_usize_counter_must_not_cast_as_usize vec_int_index_must_cast_to_usize bug_thread_spawn_closure_must_not_be_ref_test bug_mpsc_sync_channel_boundary_signature_test wdb144_module_file wdb332_module_file to_string_on_non_string_field_in_push_str` → **13 passed**

## P3.469 (2026-09-26) — delete `restore_display`; keep ToOwnedString for genuine converts

| Gate | Status |
|------|--------|
| `test_to_string_on_non_string_field_in_push_str` | ✅ isolate GREEN without post-IR restore |
| `test_to_string_on_int_preserved_for_push_str` | ✅ isolate GREEN |
| WDB-144 / WDB-152 / HashMap i64 / spawn / mpsc / WDB-332 | ✅ GREEN — no invent on bare Display |
| spawn / mpsc | ✅ GREEN |

**Root cause layer:** coercion/encoding.

`apply_ir` forced `ToOwnedString` for user-written `.to_string()`, then immediately downgraded it to Identity whenever the formal was a shared text ref. Identity + later Borrow emitted `&self.rows`. `restore_display` was a post-IR patch that put `.to_string()` back.

**What became unnecessary:** `restore_display_to_owned_string_for_text_formal` and its three terminal call sites (`apply_ir`, reconcile, method arguments). Genuine converts now keep `ToOwnedString` through encode.

**Temporary remaining:** `peel_owned_literal_when_stdlib_expects_str_ref` still counters WJ owned stubs vs runtime `&str` (WDB-144). Dual-oracle `apply_owned_string_literal_coercion` still exists.

**Gates:** `CARGO_TARGET_DIR=.agent-wip/cargo-target-tip-p3469-fix`
- `cargo test --release -p windjammer --lib -- display_int_into_str_ref_needs_to_owned_string rust_shared_borrow_skips_string_literals` → **2 passed**
- `cargo test --release --test all --features integration_tests,codegen_tests -- to_string_on_non_string_field_in_push_str to_string_on_int_preserved_for_push_str wdb144_module_file_demoted_str_formal_must_not_receive_owned_string wdb152_module_file_string_lit_into_owned_string_formal_must_to_string bug_thread_spawn_closure_must_not_be_ref_test bug_mpsc_sync_channel_boundary_signature_test hashmap_field_get_i64_key_must_auto_borrow wdb332` → **11 passed**
- HEAD full suite (pre-this-commit binary): `5377 passed / 192 failed` — remaining are almost all tip-out/gen-lag scans.

## P3.468 (2026-09-26) — Display `.to_string()` + `strings::contains` `&str` needle

| Gate | Status |
|------|--------|
| `test_to_string_on_non_string_field_in_push_str` | ✅ isolate GREEN — `push_str(&self.rows.to_string())` |
| `test_to_string_on_int_preserved_for_push_str` | ✅ isolate GREEN |
| `wdb144_module_file_demoted_str_formal_must_not_receive_owned_string` | ✅ isolate GREEN — `strings::contains(&label, "lit")` not `String::from` |
| WDB-152 / HashMap i64 get / spawn / mpsc / WDB-332 isolate | ✅ GREEN — restore is user-written convert only |

**Root cause layer:** signature + coercion/encoding + narrowed reconcile.

1. **Signature:** `apply_owned_string_literal_coercion` re-wrapped IR-peeled `"lit"` using the WJ `std/strings.wj` owned stub. Stdlib last-write `strings::contains` needle is `&str` (`get_signature` first, not fallback). Dual-oracle wrap now skips when that boundary says `&str`.
2. **Coercion/encoding:** `rust_shared_borrow` keeps `.to_string()` (no peel). Language-level convert infers Owned String.
3. **Reconcile (narrowed):** collection-key `.to_string()` peel skips genuine non-literal converts. `restore_display_to_owned_string_for_text_formal` only fires for text formals or a user-written convert — not Display `i64`/`usize` into numeric slots.

**What became unnecessary:** collection-key peel of `self.rows.to_string()`; post-IR `String::from("lit")` into runtime `&str` needles. No new method-name ownership list.

**Temporary remaining:** `restore_display` is still a post-IR safety net for Display→`&str` when apply_ir's early `rust_shared_borrow` skips constraint write-back. Delete path: expected SafetyType for language-level convert is Owned String and `compute_coercion` emits `ToOwnedString` before any shared-ref peel.

**Gates:** `CARGO_TARGET_DIR=.agent-wip/cargo-target-tip-p3468` `cargo test --release --test all --features integration_tests,codegen_tests -- to_string_on_non_string_field_in_push_str to_string_on_int_preserved_for_push_str wdb144_module_file_demoted_str_formal_must_not_receive_owned_string wdb152_module_file_string_lit_into_owned_string_formal_must_to_string bug_thread_spawn_closure_must_not_be_ref_test bug_mpsc_sync_channel_boundary_signature_test hashmap_field_get_i64_key_must_auto_borrow` → **9 passed**.

## P3.467 (2026-09-26) — WDB-203 MultiFile isolate is GREEN (tip-out lag)

| Gate | Status |
|------|--------|
| `wdb203_module_file_owned_sql_into_demoted_str_must_borrow` | ✅ isolate GREEN — owned `let sql = make_sql()` into demoted `&str` borrows; cargo-check |
| Tip-out / gen unified_port | ⚠️ host-lag — not a compiler isolate target |

**Root cause layer:** none in compiler. Owned `String` local → demoted `&str` already Borrow on tip MultiFile (twin WDB-181). Queue ❌ was product gen scan.

**What became unnecessary:** treating WDB-203 as a missing borrow peel. No `ir_call_site` change.

**Gates:** `CARGO_TARGET_DIR=.agent-wip/cargo-target-tip-p3466` `cargo test --release --test all --features integration_tests,codegen_tests -- wdb203_module_file_owned_sql_into_demoted_str_must_borrow` → **1 passed**.

## P3.466 (2026-09-26) — WDB-201 MultiFile isolate is GREEN (tip-out lag)

| Gate | Status |
|------|--------|
| `wdb201_module_file_reused_key_into_owned_entries_push_must_clone` | ✅ isolate GREEN — reused `key` into `Vec<(Key, Value)>::push` clones or stays owned; cargo-check |
| Tip-out / gen `relational_secondary_index_port` | ⚠️ host-lag — not a compiler isolate target |
| spawn / mpsc / WDB-209 | ✅ GREEN (P3.465) |

**Root cause layer:** none in compiler. Product `relational_secondary_index_put` compare-then-push already emits owned/clone on tip MultiFile. Queue ❌ was tip-out/gen string-scan only.

**What became unnecessary:** treating WDB-201 as a missing `method == "push"` heuristic. P3.465 generic peel + `Vec::push` owned T is enough.

**Gates:** `CARGO_TARGET_DIR=.agent-wip/cargo-target-tip-p3465` `cargo test --release --test all --features integration_tests,codegen_tests -- wdb201_module_file_reused_key_into_owned_entries_push_must_clone` → **1 passed**.

## P3.465 (2026-09-26) — peel `Vec<T>::method` so field-receiver `push` is signature-driven

| Gate | Status |
|------|--------|
| `wdb209_multipass_catalog_push_column_col_must_stay_owned` | ✅ isolate GREEN — `push(col)` via `Vec::push` after generic peel |
| `vec_push_borrowed_loop_elem_must_clone_for_owned_push` | ✅ GREEN — borrowed non-Copy still clones |
| WDB-124 / 125 / demoted Vec reuse / has_key / mut_param | ✅ GREEN — no regression |
| spawn / mpsc | ✅ GREEN |

**Root cause layer:** signature. `method_call_signature_for_arg` looked up `Vec<CatalogColumnBinding>::push` and missed the registry key `Vec::push`. Prepare then used a banned `method == "push"` fallback.

**What became unnecessary:** both `method == "push"` prepare fallbacks (`expression_has_owning_method_use` + `expression_for_each_param_call_argument_site`), plus `stdlib_vec_push_value_arg_is_owned` / `expr_is_vec_like_receiver`. Same peel as `resolve_function_signature` Step 3a (`Vec<T>` → `Vec`). No new `ir_call_site` peel.

**Gates:** `CARGO_TARGET_DIR=.agent-wip/cargo-target-tip-p3465` `cargo test --release --test all --features integration_tests,codegen_tests -- wdb209_multipass_catalog_push_column_col_must_stay_owned vec_push_borrowed_loop_elem_must_clone_for_owned_push bug_thread_spawn_closure_must_not_be_ref_test bug_mpsc_sync_channel_boundary_signature_test` → **6 passed**. Related: `dogfood_store_has_key_forward_ref_borrows_owned_key bug_demoted_vec_param_into_owned_vec_callee_must_clone_test bug_wdb124_module_file_demoted_vec_i64_formal_must_borrow_clone_call_sites_test bug_wdb125_module_file_demoted_struct_formal_must_borrow_clone_call_sites_test bug_mut_param_passthrough_no_shared_amp_test` → **9 passed**.

## P3.464 (2026-09-26) — `path::glob_match` boundary signature

| Gate | Status |
|------|--------|
| `bug_std_path_glob_match_wiring_test` | ✅ isolate GREEN — `path::glob_match` scanned from `path.rs` |
| `bug_std_strings_contains_owned_needle_test` | ✅ GREEN — stale adoption-queue ❌; already wired |
| spawn / mpsc | ✅ GREEN |

**Root cause layer:** signature. Missing `glob_match` in `windjammer_runtime::path` (E0425). Added `&str`/`&str` so the scanner registers Borrowed (no method-name list).

**What became unnecessary:** treating glob as a peel/codegen issue. No `ir_call_site` change.

**Gates:** `CARGO_TARGET_DIR=.agent-wip/cargo-target-tip-p3462` `cargo test --release --lib -- scanned_runtime_path_glob_match_is_registered` → **1 passed**. `cargo test --release --test all --features integration_tests,codegen_tests -- bug_std_path_glob_match_wiring_test bug_std_strings_contains_owned_needle_test bug_thread_spawn_closure_must_not_be_ref_test bug_mpsc_sync_channel_boundary_signature_test` → **7 passed**.

## P3.463 (2026-09-26) — `std::url` / `encoding::form_*` signatures + owned HashMap forward

| Gate | Status |
|------|--------|
| `bug_std_url_parse_wiring_test` | ✅ isolate GREEN — `url::parse` / `format` / `join` scanned from `url.rs` |
| `bug_std_encoding_form_urlencoded_wiring_test` | ✅ isolate GREEN — `encoding::form_parse` / `form_stringify` |
| `std_config_resolve_must_wire` | ✅ isolate GREEN — owned HashMap formals forward into `config::resolve` |
| spawn / mpsc | ✅ GREEN |

**Root cause layer:** signature + constraint + coercion.

1. **Signature:** missing runtime/`std` stubs so fail-closed `compile_error!("missing boundary signature for url::…")` / missing `encoding::form_*` symbols. Scanner now registers `url::parse|format|join` (`&str`/`&Url`) and `encoding::form_parse`/`form_stringify`.
2. **Constraint:** `is_public_owned_non_copy_formal_api` / `vec_formal_only_forwards` only treated `Vec`. HashMap-only-forward formals demoted to `&mut HashMap` then Identity-moved into owned `config::resolve` (E0308). Same owned-forward container predicate now covers HashMap/Set.
3. **Coercion:** `MutRef→Owned` was always `StripBorrow` (wrong for non-Copy). Now matches `Ref→Owned` (Clone / ToOwnedString / Copy strip).

**What became unnecessary:** treating missing `url`/`form_*` as a peel problem; HashMap-only `&mut` demote + StripBorrow. No new `ir_call_site` peel or method-name list.

**Temporary remaining:** MutRef→Owned Clone is the safety net when a HashMap formal is still wrongly demoted. Delete path: once every only-forward map/set formal stays owned from the expanded predicate, Clone on those sites should not fire.

**Gates:** `CARGO_TARGET_DIR=.agent-wip/cargo-target-tip-p3462` `cargo test --release --lib -- mut_ref_to_owned_hashmap_needs_clone mut_ref_to_owned_copy_strips scanned_runtime_url_parse_format_join_are_registered scanned_runtime_encoding_form_helpers_are_registered rust_std_mpsc_and_thread_spawn_boundary_signatures_registered` → **5 passed**. `cargo test --release --test all --features integration_tests,codegen_tests -- bug_std_url_parse_wiring_test bug_std_encoding_form_urlencoded_wiring_test bug_std_config_module_test bug_thread_spawn_closure_must_not_be_ref_test bug_mpsc_sync_channel_boundary_signature_test` → **url/form/spawn/mpsc/config 11+4 GREEN**.

## P3.462 (2026-09-26) — borrowed loop elem clone gate is non-Copy (P3.303)

| Gate | Status |
|------|--------|
| `vec_push_borrowed_loop_elem_must_clone_for_owned_push` | ✅ isolate GREEN — `Achievement { id, name: string }`; `push(ach.clone())` |
| spawn / mpsc | ✅ GREEN |
| `bug_todo_cli_cross_crate_validate_field_must_auto_borrow_test` | ✅ GREEN — stale queue ❌; P3.436/437 already landed |

**Root cause layer:** none in compiler — the P3.303 fixture used Copy `Achievement` (`i32` only), so `push(ach)` is Identity. Tip already clones borrowed **non-Copy** `values()` elems into owned `Vec::push`.

**What became unnecessary:** treating Copy Identity as a compiler RED. No new `ir_call_site` peel.

**Gates:** `CARGO_TARGET_DIR=.agent-wip/cargo-target-tip-p3462` `cargo test --release --test all --features integration_tests,codegen_tests -- vec_push_borrowed_loop_elem_must_clone_for_owned_push bug_thread_spawn_closure_must_not_be_ref_test bug_mpsc_sync_channel_boundary_signature_test bug_todo_cli_cross_crate_validate_field_must_auto_borrow_test` → **6 passed** after fixture rewrite (Copy fixture was the only RED).

## P3.461 (2026-09-26) — last-use owned `col` into `Vec::push` moves (WDB-209)

| Gate | Status |
|------|--------|
| `wdb209_multipass_catalog_push_column_col_must_stay_owned` | ✅ isolate GREEN — `push(col)` not `push(col.clone())` |
| `dogfood_store_has_key_forward_ref_borrows_owned_key` | ✅ GREEN — last-use `has_key(key)` moves |
| WDB-124 / 125 / demoted Vec reuse clone | ✅ GREEN — demoted `&T` still clones into owned callees |
| mut_param / WDB-217 / spawn / mpsc | ✅ GREEN — no regression |

**Root cause layer:** constraint/emit-truth + reconcile shrink. IR already computed Owned→Owned Identity for last-use `col` into `Vec::push`. Terminal reconcile then blanket-cloned every non-Copy param into an owned slot when the receiver was not `self`/field (`latest.has_key(key)` heuristic), undoing last-use Identity.

**What became unnecessary:** the post-IR blanket `.clone()` on local-receiver owned slots. Analyzer `Type::Reference` / `inferred_borrowed_params` no longer mark an emitted-owned formal as already-`&T`. `caller_demoted_non_copy_formal_into_owned_callee` now requires emit-truth (owned outer formal wins). Reuse clones stay in `ensure_owned_move_clone_for_reuse`.

**Temporary remaining:** deleted in P3.465 — field-receiver lookup now peels `Vec<T>::push` → `Vec::push`.

**Gates:** `CARGO_TARGET_DIR=.agent-wip/cargo-target-tip-p3461` `cargo test --release --test all --features integration_tests,codegen_tests -- wdb209_multipass_catalog_push_column_col_must_stay_owned dogfood_store_has_key_forward_ref_borrows_owned_key bug_thread_spawn_closure_must_not_be_ref_test bug_mpsc_sync_channel_boundary_signature_test bug_demoted_vec_param_into_owned_vec_callee_must_clone_test bug_wdb124_module_file_demoted_vec_i64_formal_must_borrow_clone_call_sites_test bug_wdb125_module_file_demoted_struct_formal_must_borrow_clone_call_sites_test bug_mut_param_passthrough_no_shared_amp_test bug_wdb217_module_file_owned_csr_clone_into_mut_ref_must_reborrow_test` → **14 passed**.

## P3.460 (2026-09-26) — `Mutex::lock` types `Ok(g)` so `g.data.get` borrows `&K`

| Gate | Status |
|------|--------|
| `mutex_lock_boundary_signature_returns_mutex_guard` | ✅ unit GREEN |
| `module_file_shared_map_get_must_borrow_key` | ✅ isolate GREEN — `g.data.get(&key)` cargo-check |
| `hashmap_get_through_mutex_guard_must_cargo_check` | ✅ isolate GREEN |
| hashmap auto-borrow / remove / get_mut | ✅ GREEN — tests now use `CARGO_BIN_EXE_wj` (not stale `target/release/wj`) |
| spawn / mpsc | ✅ GREEN — no regression |

**Root cause layer:** signature + constraint. `m.inner.lock()` (`Arc<Mutex<MapCell>>`) had no `Mutex::lock` boundary signature, so `Ok(g)` stayed untyped, `g.data.get` did not resolve to `HashMap::get(&K)`, and the owned `key: String` formal was passed by value (`get(key)` / E0308). Not a method-name ownership list.

**What became unnecessary:** failing closed to a bare `get` homonym (owned key / `.to_string()`). No new `ir_call_site` peel. `Arc`/`Rc`/`Box` peel in `registry_method_return_type` so `Mutex::lock` applies; unique registered field type types `g.data` when the guard binding is still untyped; `substitute_stdlib_generics` now walks `Result` / `Parameterized`.

**Temporary remaining:** unique-field fallback is fail-closed on conflicting `data` types. Delete path: once every `lock`/`read`/`write` match binding is typed from the new signatures, the unique-field scan can go.

**Gates:** `CARGO_TARGET_DIR=.agent-wip/cargo-target-tip-p3459` `cargo test --release --lib -- mutex_lock_boundary_signature_returns_mutex_guard hashmap_get_expects_borrowed_key_ref` → **2 passed**. `cargo test --release --test all --features integration_tests,codegen_tests -- bug_module_file_shared_map_get_must_borrow_key_test bug_hashmap_get_through_mutex_guard_must_borrow_key_test bug_hashmap_get_mut_tuple_match_test codegen_hashmap_auto_borrow_test codegen_hashmap_remove_test bug_thread_spawn_closure_must_not_be_ref_test bug_mpsc_sync_channel_boundary_signature_test` → **15 passed**.

## P3.459 (2026-09-25) — user `join(string, string)` owned slot beats `strings::join` homonym

| Gate | Status |
|------|--------|
| `skip_stale_borrow_peels_user_join_owned_slot_despite_stdlib_join` | ✅ unit GREEN |
| `codegen_user_join_must_not_borrow_owned_relative` | ✅ unit GREEN |
| `user_join_two_strings_moves_owned_locals` | ✅ isolate GREEN — `join(base, relative)` not `join(&base, &relative)` |
| spawn / mpsc / mut_param / WDB-217 / demoted Vec | ✅ GREEN — P3.456/458 helpers restored (other-agent WIP had deleted them) |
| P3.444 fixture | closer product `CsgScene` + scan `windjammer-game-core/gen/csg/scene.rs` (tip-out still host-lag) |

**Root cause layer:** signature / lookup boundary. Local user `join` (`(&str, String)`) must win over runtime `strings::join` (`Vec`, delimiter `&str`) at the same bare name. `user_owned_slot_beats_stdlib_homonym` was already the pick; it was not consulted by `skip_stale_borrow`, `preregistered_free_call_arg_expects_borrow`, or regular-call last-writer, so `call_sig`/`global` still froze `&relative`.

**What became unnecessary:** treating a stdlib homonym's shared-ref flags as the user API. Duplicate `user_owned_slot_beats_stdlib_homonym` (other-agent copy) deleted. No new `ir_call_site` peel / method-name list. P3.456 `vec_formal_only_forwards_as_call_arg` and P3.458 MutRef Identity last-writers kept.

**Temporary remaining:** regular-call terminal peel of `&name` when local owned slot beats stdlib homonym. Delete path: once `call_sig` is the local user join (not `strings::join`) through prepare + skip_stale, last-writer `rust_shared_borrow` will not prefix and the peel can go.

**Gates:** `CARGO_TARGET_DIR=.agent-wip/cargo-target-tip-p3459` `cargo test --release --lib -- skip_stale_borrow_peels_user_join_owned_slot_despite_stdlib_join codegen_user_join_must_not_borrow_owned_relative mut_ref_to_shared_ref_is_identity_reborrow` → **3 passed**. `cargo test --release --test all --features integration_tests,codegen_tests -- bug_mut_param_passthrough_no_shared_amp_test bug_thread_spawn_closure_must_not_be_ref_test bug_mpsc_sync_channel_boundary_signature_test bug_wdb217_module_file_owned_csr_clone_into_mut_ref_must_reborrow_test bug_demoted_vec_param_into_owned_vec_callee_must_clone_test user_join_two_strings_moves_owned_locals` → **11 passed**.

## P3.458 (2026-09-25) — `&mut T` / `&T` reborrow stays Identity (`take_in_edges(csr)`)

| Gate | Status |
|------|--------|
| `bug_mut_param_passthrough_no_shared_amp_test` | ✅ isolate GREEN — bare `csr` into `&mut` / `&T` callees |
| WDB-217 codegen + tip-out pagerank | ✅ GREEN — no regression |
| spawn / mpsc | ✅ GREEN — no regression |

**Root cause layer:** constraint/solver + coercion + signature emit-truth.

1. `infer_actual_safety_type` now treats `identifier_already_mut_ref` as `MutRef` (not Owned).
2. `compute_coercion`: `MutRef` → `Ref` is Identity (Rust reborrow), not Borrow (`&csr` / `&&mut T`).
3. Last-writers in `regular_call_arguments` / `function_call_generation` no longer apply `rust_shared_borrow` for mut-expected slots or already-`&mut`/`&T` bindings. Shared-only helper is `callee_arg_expects_shared_borrow_at_call`.
4. Method thin wrappers (`Host::run` → `take_edges`): AST-owned sibling lookup no longer beats field-write / emitted `&mut` (`method_call_arg_formal_is_owned_non_copy`).

**What became unnecessary:** last-writer `callee_arg_expects_borrow_at_call` → `rust_shared_borrow` restack (dual-oracle that undid IR Identity + the existing 1996 peel). No new `ir_call_site` peel / method-name list.

**Temporary remaining:** last-writers still sanitize `&x.clone()` and prefix shared borrow for *owned* non-ref bindings. Delete path: IR apply + 900–923 shared-ref path should own that; then last-writers can drop.

**Gates:** `cargo test --release --lib -- mut_ref_to_shared_ref_is_identity_reborrow mut_ref_to_mut_ref_is_identity` → **2 passed**. `cargo test --release --test all --features integration_tests,codegen_tests -- bug_mut_param_passthrough_no_shared_amp_test bug_thread_spawn_closure_must_not_be_ref_test bug_thread_spawn_move_keyword_must_be_preserved_test bug_mpsc_sync_channel_boundary_signature_test bug_wdb217_module_file_owned_csr_clone_into_mut_ref_must_reborrow_test` → **11 passed**.

## P3.457 (2026-09-25) — owned `FnOnce` closures stay by-value (`spawn(move ||)`)

| Gate | Status |
|------|--------|
| `bug_thread_spawn_move_keyword_must_be_preserved_test` | ✅ isolate GREEN — `spawn(move ||` not `spawn((move ||` |
| spawn-not-ref / mpsc | ✅ GREEN — no regression |

**Root cause layer:** encoding. `rust_shared_borrow` treated `move || …` as a compound expr and wrapped `&(move || …)`. Last-writer then peeled `&` (closure Identity) and left `spawn((move || …))`. `move` was already inferred; the wrap was leftover Borrow encoding.

**What became unnecessary:** `&(closure)` wrap + paren leftover after the closure peel. Closures are Identity in `rust_shared_borrow` (same as `apply_shared_borrow_prefix`). No new method-name list / `ir_call_site` peel.

**Gates:** `cargo test --release --lib -- rust_shared_borrow_keeps_move_closure_by_value` → **1 passed**. `cargo test --release --test all --features integration_tests,codegen_tests -- bug_thread_spawn_move_keyword_must_be_preserved_test bug_thread_spawn_closure_must_not_be_ref_test bug_mpsc_sync_channel_boundary_signature_test` → **6 passed**.

**Follow-up isolate:** `bug_mut_param_passthrough_no_shared_amp_test` → GREEN in P3.458.

## P3.456 (2026-09-25) — only-forward `Vec` formals stay owned (WDB-285 class)

| Gate | Status |
|------|--------|
| `bug_demoted_vec_param_into_owned_vec_callee_must_clone_test` | ✅ isolate GREEN — `workload_verdict(samples: Vec<u64>)` + `samples.clone()` |
| WDB-179 / 185 / 195 / 216 / 124 / 126 / 127 | ✅ isolate GREEN |
| spawn / mpsc | ✅ GREEN — no regression |
| WDB-285 / 286 tip-out/gen | ⚠️ gen-lag (`sysbench_opt_port` / `query_verdict` missing on this host) — not an isolate target |

**Root cause layer:** signature / formal write-back. Analyzer + forwarding-delegate emit cascaded `&Vec` from `median_pair` (readonly `.len()`/`[i]`) up through `workload_verdict` (body is only `median_pair(samples)`), so `claim_cap` reborrowed `&samples` into a demoted sibling. WJ AST `Vec<T>` that is **only** used as a call argument now keeps the owned formal.

**What became unnecessary:** cascading `&Vec` demotion through forwarding wrappers after the next sibling already emitted `&Vec`. No new `ir_call_site` peel / method-name list. Delete path: registry `param_ownership` Owned for those WJ formals should make the prepare/emit skip redundant.

**Gates:** `cargo test --release --test all --features integration_tests,codegen_tests -- bug_demoted_vec_param_into_owned_vec_callee_must_clone_test` → **1 passed**. Related `bug_demoted_vec_param…` + spawn/mpsc + WDB-179/185/195/216/124/126/127 + WDB-285/286 → **15 passed, 2 failed** (tip-out/gen-lag only).

`bug_todo_cli_match_scrutinee_reuse_must_auto_clone_test` was a lexer-false RED (`r#"…\n…"#` two-char escape, not a newline). Fixture now uses a real newline → **GREEN** (no compiler change).

## P3.455 (2026-09-25) — `db::Connection` reuse must borrow, not `.clone()`

| Gate | Status |
|------|--------|
| `bug_db_connection_helper_reuse_invalid_clone_test` | ✅ isolate GREEN — `ensure_schema(&conn)` / `count_rows(&conn)` |
| spawn / mpsc | ✅ GREEN — no regression |

**Root cause layer:** signature (runtime non-`Clone` capability) + narrowed reconcile. Helpers already emit `conn: &Connection`. After IR encoded `ensure_schema(&conn)`, `apply_match_scrutinee_move_clone_if_needed` string-replaced `conn` → `conn.clone()` inside `&conn`, yielding `&conn.clone()` (E0599). Scanner now records runtime structs without `#[derive(Clone)]` (`Connection` yes, `Row` no).

**What became unnecessary:** injecting `.clone()` on match-scrutinee call args that are already `&name` / `&mut name`, shared-ref callees, or scanned non-`Clone` types. `rust_shared_borrow` no longer early-returns on a leading `&` (that kept `&x.clone()`). No new method-name list.

**Temporary reconcile remaining:** the match-scrutinee string rewrite itself (WDB-347 / todo-cli owned `Vec` reuse). Delete path: IR reuse + owned expected type should emit `.clone()` before match lowering so the rewrite can go.

**Gates:** `cargo test --release --test all --features integration_tests,codegen_tests -- bug_db_connection_helper_reuse_invalid_clone_test bug_thread_spawn_closure_must_not_be_ref_test bug_mpsc_sync_channel_boundary_signature_test` → **5 passed**.

## P3.454 (2026-09-25) — `Vec[int]` index cast without undoing sentinel i64

| Gate | Status |
|------|--------|
| `bug_vec_int_index_loop_test` | ✅ isolate GREEN — `lines[(idx as usize)]` |
| `bug_int_loop_assign_end_bound_must_unify_test` | ✅ GREEN — `colon_at` stays `-1_i64`; `colon_at = j as i64` |
| haystack / substring / spawn / mpsc / `for_zero_to_len` / WDB-119 / WDB-121 | ✅ GREEN — no regression |

**Root cause layer:** encoding. Untyped `let mut idx = 0` is recorded as a literal-init WJ `int` counter so index sites still emit `as usize` after `.len()` usize promotion. `reconcile_ambiguous_int_local_after_let` was painting `let mut colon_at = -1_i64` as `usize` (mixed-int inference) *before* the emitted `_i64` suffix could win, so `colon_at = j` skipped the i64 cast.

**What became unnecessary:** stamping *all* `let mut x = 0` as `Type::Int` (that dual-oracle broke `i < strings::len`). Emitted `_i64` / negative-init now wins over mixed-codegen usize in reconcile — no new `ir_call_site` peel.

**Gates:** `cargo test --release --lib -- type_casting::tests` → **6 passed**. `cargo test --release --test all --features integration_tests,codegen_tests -- bug_vec_int_index_loop_test bug_int_loop_assign_end_bound_must_unify_test bug_haystack_contains_substring_int_index_unify_test bug_substring_int_indices_usize_test bug_substring_end_i_plus_one_must_not_emit_i32_into_usize_test bug_thread_spawn_closure_must_not_be_ref_test bug_mpsc_sync_channel_boundary_signature_test for_zero_to_len` → **11 passed**. Broader `bug_int_loop|bug_vec_int|bug_substring|bug_haystack|bug_for_zero|wdb119|wdb121` → **11 passed**.

## P3.453 (2026-09-25) — negative `int` sentinels stay i64 (`colon_at = -1`)

| Gate | Status |
|------|--------|
| `bug_int_loop_assign_end_bound_must_unify_test` | ✅ isolate GREEN — `let mut colon_at = -1_i64`; `colon_at = j as i64` |
| haystack / substring / spawn / mpsc | ✅ GREEN — no regression |

**Root cause layer:** constraint + encoding. `let mut colon_at = -1` (Unary Neg) was back-propagated into `usize_variables` via `colon_at = j`, emitting `-1_usize`. Assignment target lookup also preferred `usize_variables` over WJ `int`.

**What became unnecessary:** treating `usize_variables` as assignment-target width when the local is a signed WJ `int` sentinel. No new `ir_call_site` peel.

**Gates:** `cargo test --release --test all --features integration_tests,codegen_tests -- bug_int_loop_assign_end_bound_must_unify_test bug_haystack_contains_substring_int_index_unify_test bug_substring_end_i_plus_one_must_not_emit_i32_into_usize_test bug_thread_spawn_closure_must_not_be_ref_test bug_mpsc_sync_channel_boundary_signature_test` → **8 passed**.

**Follow-up:** `bug_vec_int_index_loop_test` → GREEN in P3.454.

## P3.452 (2026-09-25) — usize formals wrap mixed i64 + `1_usize` arith

| Gate | Status |
|------|--------|
| `bug_haystack_contains_substring_int_index_unify_test` | ✅ isolate GREEN — `(i + j + 1) as usize` (no `+ 1_usize`) |
| hexagonal haystack `contains.rs` | ✅ isolate GREEN |
| `bug_substring_int_indices_usize_test` / `bug_substring_end_i_plus_one_*` | ✅ GREEN |
| spawn / mpsc | ✅ GREEN — no regression |

**Root cause layer:** encoding (`coerce_arg_str_for_usize_formal`). `strings::substring` formals are already `usize`. Binary emit suffixes only the literal (`i + j + 1_usize`) while `i`/`j` stay i64. Call-site wrap existed but was skipped when `arg_already_usize` was true (`.len()` comparison marked the Binary usize).

**What became unnecessary:** Identifier-only WJ-int `already_usize` override in `apply_post_ir_numeric_formal_casts`; duplicated Identifier/`infer` match in `expression_generation`. Mixed-suffix wrap now runs *before* the already-usize return. No new `ir_call_site` peel.

**Gates:** `cargo test --release --lib -- coerce_usize_formal_wraps_mixed` → **2 passed**. `cargo test --release --test all --features integration_tests,codegen_tests -- bug_haystack_contains_substring_int_index_unify_test bug_substring_int_indices_usize_test bug_substring_end_i_plus_one_must_not_emit_i32_into_usize_test bug_thread_spawn_closure_must_not_be_ref_test bug_mpsc_sync_channel_boundary_signature_test` → **8 passed** (2 haystack + 2 substring + 4 spawn/mpsc).

**Follow-up:** `bug_vec_int_index_loop_test` → GREEN in P3.454. `bug_int_loop_assign_end_bound_must_unify_test` → GREEN in P3.453.

## P3.451 (2026-09-25) — trait AST `string` stays owned at FieldAccess call sites

| Gate | Status |
|------|--------|
| `trait_owned_string_field_and_concat_must_cargo_check` | ✅ isolate GREEN — `authenticate(request.email, request.password)` (no `&`) |
| `trait_method_owned_string_param_accepts_field_access_without_borrow` | ✅ isolate GREEN — `report_lines(demo_tenant().slug)` (no `&`) |
| inherent `DbReportReader::report_lines` body-converged `&str` | ✅ unit GREEN — write-back skips non-impl homonyms |
| spawn / mpsc / user_join / trait_owned_string_call | ✅ GREEN — no regression |

**Root cause layer:** signature / registry write-back. Trait item `string` is Owned (E0053), but (1) impl analysis overwrote `Trait::method` with body-converged `&str`, (2) merge treated that as an Owned→Borrowed "refinement", (3) `method_call_arg_expects_pattern_str` then consulted the impl homonym and prefixed FieldAccess with `&` after IR reconcile.

**What became unnecessary:** `Call(FieldAccess)` Pattern/`&str` re-borrow on non-text receivers when the resolved or trait contract is owned `string`. No new `ir_call_site` peel. Inherent `DbReportReader::report_lines` still body-converges to `&str`.

**Gates:** `cargo test --release --lib -- write_back_restores_trait_impl_not_inherent_homonym apply_trait_owned_string_* merge_refresh_must_not_undo_trait_owned_string_password normalize_preserves_body_converged_borrow_for_instance_methods` → **6 passed**. `cargo test --release --test all --features integration_tests,codegen_tests -- trait_owned_string_field_and_concat_must_cargo_check trait_method_owned_string_param_accepts_field_access_without_borrow bug_thread_spawn_closure_must_not_be_ref_test bug_mpsc_sync_channel_boundary_signature_test user_join_two_strings_moves_owned_locals bug_trait_owned_string_call_must_not_over_borrow_test` → **8 passed**.

## P3.450 (2026-09-25) — WJ `int` module const in i32-coord builders

| Gate | Status |
|------|--------|
| `i32_coord_literal_peers_must_not_emit_i64` | ✅ isolate GREEN — `VIEWER_GRID as i32 / 2_i32` + cargo-check |
| spawn / mpsc / WDB-127 / i32 range/neg-while | ✅ GREEN — no regression |

**Root cause layer:** encoding / mixed-int promotion. `const VIEWER_GRID: int` is i64 in Rust, but i32-coord promotion (`wj_int_coord_builder_operand` / `mixed_arith_should_prefer_i32_over_i64`) only treated *locals*, so `VIEWER_GRID / 2_i32` never got `as i32`. Type-driven: `module_const_types` is WJ `int`/`i64`, not a const-name list.

**What became unnecessary:** no new peel. Existing P3.353 `as i32` path now fires for module consts. No `ir_call_site` change.

**Gates:** `cargo test --release --test all --features integration_tests,codegen_tests -- i32_coord_literal_peers_must_not_emit_i64` → 1 passed. Related cluster (coord + spawn + mpsc + WDB-127 + i32 range/neg-while) → **10 passed**.

## P3.449 (2026-09-25) — unannotated `vec![10, 20]` inherits callee `Vec<u64>`

| Gate | Status |
|------|--------|
| `wdb127_module_file_demoted_vec_formal_must_borrow_bare_local_call_sites` | ✅ isolate GREEN — `vec![10_u64, 20_u64]` + cargo-check |
| encode / spawn / mpsc / WDB-125/126 / int-inference collections | ✅ GREEN — no regression |

**Root cause layer:** constraint + signature write-back + encoding. Three gaps: (1) `vec!` / array element literals were not MustMatched to the collection expr, so Call `MustBe(U64)` on the identifier never reached `10`/`20`; (2) call-site `Vec<u64>` / `&Vec<u64>` was only consulted for float `Vec::new()` refinement, and `infer_let_value_type` early-returned `Vec<int>` from `vec![10]`; (3) `function_prefers_i32_coord_locals` is true for `-> u64`, so unannotated lets stamped `_i32` (Priority 0) and never read the solver. `assignment_int_peer_from_formal` also omitted `u64`/`usize`.

**What became unnecessary:** float-only call-site Vec refine; `Vec(_)` early-return that blocked callee width. i32-coord still paints unannotated `vec!` when the element is default WJ `int` (void builders). No new `ir_call_site` peel.

**Gates:** `cargo test --release --test all --features integration_tests,codegen_tests -- bug_wdb127_module_file_demoted_vec_formal_must_borrow_bare_local_call_sites` → 1 passed. Related: `wdb127_ wdb126_ wdb125_ spawn mpsc int_inference_generic_collections int_inference_vec_element int_inference_assignment struct_field_literal_typing test_cross_file_int_inference` → **30 passed**.

## P3.448 (2026-09-25) — import alias must keep remapped Owned identity

| Gate | Status |
|------|--------|
| `import_alias_must_not_steal_foreign_fn_ownership` | ✅ isolate GREEN — `use owned_pkg::get as query_get` must not emit `query_get(&` |
| `import_alias_owned_get_must_not_merge_foreign_query_get_refresh` | ✅ unit GREEN — pick/merge/prefer reject different simple names |
| encode / spawn / mpsc | ✅ GREEN — no regression |

**Root cause layer:** signature / registry boundary. Two gaps: (1) `--metadata owned_pkg=a,borrowed_pkg=b` was parsed as a single NAME=PATH, so `owned_pkg` was overwritten with a garbage path and `owned_pkg::get` never registered; (2) refresh/merge/`local_sig` treated the alias string `query_get` as `borrowed_pkg::query_get` (method-index / first-shared-ref) even when remapped to `owned_pkg::get`.

**What became unnecessary:** alias-name registry keys in codegen refresh (`codegen_refresh_lookup_keys` uses only the remapped fn); `get_signature(func_name)` steal in `regular_call_arguments`; merge/pick/prefer OR-union of `get` vs `query_get`; bare external-crate keys when a crate alias is present. No new `ir_call_site` peel.

**Gates:** `cargo test --release --lib -- import_alias_owned_get` → 1 passed. `cargo test --release --test all --features integration_tests,codegen_tests -- import_alias_must_not_steal_foreign_fn_ownership bug_thread_spawn_closure_must_not_be_ref_test bug_mpsc_sync_channel_boundary_signature_test bug_cross_crate_owned_encode_named_fn_must_not_borrow_arg_test` → 6 passed.

**WDB-127 leftover:** ✅ GREEN (P3.449) — callee `Vec<u64>` now writes `_u64` into unannotated `vec![10, 20]`.

## P3.447 (2026-09-25) — same-crate `&T` passthrough + for-in-self shared borrow

| Gate | Status |
|------|--------|
| WDB-217 isolate `touch(csr)` | ✅ GREEN — `run(csr: &DenseCsr) { touch(csr) }` not `touch(&csr)` |
| WDB-217 tip-out pagerank | ✅ GREEN — no `csr.clone()` into `&mut DenseCsr` |
| `test_for_in_self_field_borrows_when_self_used_in_body` | ✅ GREEN — `&self.passes` when body only shared-borrows self |
| WDB-307 isolate + tip-out | ✅ GREEN — no regression |
| spawn / mpsc | ✅ GREEN |

**Root cause layer:** signature / lookup boundary + constraint write-back from emitted formals. Same-crate bare `touch` was treated as path-dep because the global/multipass registry also listed it (`cross_crate_import` / `dep_shared`), then a post-IR oracle re-prefixed `&` onto an already-`&DenseCsr` caller formal. `caller_owned_non_copy_formal` ignored preregistered `name: &T` strings when `emitted_rust_ref_formals` lagged. IR actual for emitted `&T` Custom formals is now Ref (Identity). P3.423 clone override only fires when the loop body mutates self / calls `&mut self`.

**What became unnecessary:** same-crate names no longer enter the path-dep re-borrow block; `dep_shared` is false for bare local callees; P3.423 no longer clones `self.field` when the body only shared-borrows self. Existing `caller_formal_emitted_shared_ref` strip is no longer gated on callee registry lookup. No new `ir_call_site` peel.

**Gates:** isolate cluster above → **11 passed**. No-reg: `wdb217_tip_out wdb125_ wdb126_ demoted_encode_line borrowed_dense_csr_cross_file` → **5 passed**. P3.444 tip-out still stale RED if included.

## P3.446 (2026-09-25) — TDD WDB-388 + user-join pick unit

| Gate | Status |
|------|--------|
| WDB-388 MultiFile | ✅ isolate GREEN — `Vec3::new(a, b, c)` must not `&a` |
| WDB-388 tip-out | ✅ product scan GREEN — no `Vec3::new(&` in tip/gen |
| `prefer_shared_ref_keeps_local_user_join_over_strings_join` | ✅ unit GREEN |
| `codegen_user_join_must_not_borrow_owned_relative` | ✅ in-process GREEN |
| `user_join_two_strings_moves_owned_locals` | ✅ isolate GREEN — mixed `(&str, String)`; call no longer `join(&base, &relative)` |

**Root cause layer:** signature / lookup boundary. `use std::strings` rewrote bare `join` → `strings::join` via `imported_runtime_qualified_callee`, so `dep_shared` borrowed the owned `relative` slot. Shape-aware pick/prefer/merge keeps `join(string, string)` distinct from `strings::join(Vec, str)` (no first-shared-ref / OR-union). Local user free-fn shadows imported runtime module.

**What became unnecessary:** post-IR `finalize_borrowed_text` was not the rewriter once `text_sig` stayed user; no new peel. `dep_shared` no longer consults `strings::join` for a same-crate `join`.

**Gates:** `cargo test --release --lib -- codegen_user_join_must_not_borrow_owned_relative prefer_shared_ref_keeps_user_join two_string_join_shape prefer_shared_ref_picks_runtime_str refresh_join_delimiter codegen_strings_join_vec_arg` → 6 passed. `cargo test --release --test all -- user_join_two_strings_moves_owned_locals bug_thread_spawn_closure_must_not_be_ref_test bug_mpsc_sync_channel_boundary_signature_test` → 5 passed. `cargo test --release --test all -- wdb388_module_file_vec3_new_must_not_borrow_copy_local` → 2 passed (isolate + tip-out).

## P3.445 (2026-09-24) — `f32::MAX` associated path is Copy (no `.clone()`)

| Gate | Status |
|------|--------|
| WDB-385 MultiFile | ✅ isolate GREEN — `Vec3::new(f32::MAX, f32::MAX, f32::MAX)` |
| WDB-385 tip-out | ❌ RED — voxel_scene stale product regen |
| `copy_primitive_associated_path_is_that_primitive` | ✅ unit GREEN |

**Root cause layer:** constraint/inference (type classification). Parser folds `f32::MAX` into Identifier `"f32::MAX"`. Auto-clone treated that as a reused move. `copy_primitive_associated_path_type` types any `Primitive::ASSOC` as that Copy primitive so the existing `binding_is_copy_pass_by_value_scalar` skip fires. No MAX/MIN name list; no new `ir_call_site` peel.

**What became unnecessary:** FieldAccess-only guess for `f32.MAX`; extra clone-skip peels in `ir_call_site.rs`.

**Gates:** `cargo test --release --test all -- wdb385_module_file_f32_assoc_const_must_not_clone wdb344_module_file_copy_f32_must_not_emit_clone wdb355_module_file_copy_vec3_must_not_emit_clone` → isolate GREEN (3), tip-out stale RED (3). No-reg: spawn/mpsc/encode_line/WDB-125/Copy f32 formal/WDB-384/386/387 isolates GREEN.

## P3.444 (2026-09-25) — i32-heavy impl `0..node.params.len()` + `buf.clone()` into `&mut Vec`

| Gate | Status |
|------|--------|
| `i32_heavy_impl_match_field_len_must_not_emit_i32_range` | ✅ isolate GREEN — match-bound `node.params.len()` is usize; recursive `buf` reborrows |
| `p3444_tip_out_game_core_csg_must_not_emit_i32_len_range` | ❌ RED — stale product `gen/csg/scene.rs` (needs tip-out regen) |
| `mut_borrowed_bare_vec_is_not_owned_emission` | ✅ unit GREEN |
| P3.359 `for_zero_to_len_must_not_emit_i32_range` | ✅ isolate GREEN — no regression |
| WDB-345 MultiFile | ✅ isolate GREEN — no regression |
| WDB-345 tip-out | ❌ RED — same stale product file |

**Product:** `CsgScene::emit_node_instructions` after `let node = match self.get_node(node_id)`. Engine rustc: `expected i32, found usize` on range end + `expected &mut Vec<f32>, found Vec<f32>` on `buf.clone()`.

**Root cause layer:** constraint/inference + emission contract. (1) `let node = match …` parses as `Block { Match }`; `infer_expression_type` ignored `Statement::Match`, so `node.params` was untyped and `consensus_return_is_usize("len")` was poisoned by `NameLen::len() -> i32`. Typing the match expression from `Some(n) => n` makes `Vec::len` resolve to usize. (2) `sig_arg_confirms_owned_emission` treated MutBorrowed bare `Vec` as owned (WDB-281 shortcut), so recursive calls cloned into `&mut Vec`. MutBorrowed / `MutableReference` now fail closed as not-owned.

**What became unnecessary:** no new `ir_call_site` peel; no `len`/`emit_node_instructions` name heuristic. `bare_formal_is_vec_or_map` still describes AST shape; owned confirmation no longer overrides MutBorrowed.

**Gates:** `cargo test --release --lib -- mut_borrowed_bare_vec_is_not_owned_emission` → 1 passed. `cargo test --release --test all --features integration_tests,codegen_tests -- i32_heavy_impl_match_field_len` → isolate GREEN, tip-out stale RED. No-reg: `for_zero_to_len_must_not_emit_i32_range`, WDB-345 isolate, spawn/mpsc, user_join, WDB-388 → 9 passed (WDB-345 tip-out stale).



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

## P3.443 (2026-09-24) — TDD WDB-384–387 (DB agent)

| Gate | Status |
|------|--------|
| WDB-384 MultiFile | ✅ isolate GREEN — `Face::PosX` |
| WDB-384 tip-out | ❌ RED — mesh_generator / squad / npc / streaming / material_editor |
| WDB-385 MultiFile | ✅ isolate GREEN (P3.445) — Copy primitive associated path |
| WDB-385 tip-out | ❌ RED — voxel_scene stale regen |
| WDB-386 MultiFile | ✅ isolate GREEN |
| WDB-386 tip-out | ❌ RED — shader_graph_compiler / shader_effect_test |
| WDB-387 MultiFile | ✅ isolate GREEN |
| WDB-387 tip-out | ❌ RED — world_partition/streaming |

**Root cause layer:** none this session — DB agent files gates only. **WDB-385 isolate GREEN in P3.445.**

**Gates:** `cargo test --release --test all --features integration_tests,codegen_tests -- bug_wdb384_ bug_wdb385_ bug_wdb386_ bug_wdb387_` → **3 passed / 5 failed**.

## P3.442 (2026-09-24) — local Copy scalar into owned Copy formal must not `&x`

| Gate | Status |
|------|--------|
| `copy_local_into_owned_f32_formal_must_not_borrow` | ✅ MultiFile GREEN (2026-09-24) |
| `cross_module_copy_local_into_vec3_new_must_not_borrow` | ✅ MultiFile GREEN (2026-09-24) — math/vec3 + nav |
| Tip-out `gen/ai/navmesh.rs` | ✅ tip GREEN (2026-09-24 regen) — no `Vec3::new(&x)` / `Triangle::new(&id)` |

**Product:** engine cargo-check E0308 ×437 after P3.424b regen. Source is `Vec3::new(x, y, z)`; first identifier is over-borrowed into owned `f32`/`u32` formals.

**Compiler agent:** peel `&` on Copy locals (and Copy aggregates) when the resolved formal is owned. Do not special-case `Vec3` by name — signature-driven.

## P3.441 (2026-09-24) — Same-file demoted Custom `&T` must borrow owned `.clone()`

| Gate | Status |
|------|--------|
| `owned_custom_clone_text_into_ref_formal_prefixes_borrow` | ✅ GREEN |
| `owned_string_clone_text_into_str_ref_keeps_clone_without_amp` | ✅ GREEN — `String.clone()` still deref-coerces to `&str` |
| `demoted_encode_line_clone_must_auto_borrow` | ✅ GREEN — `encode_line(&item)` / `&item.clone()` into `todo: &Todo` |
| `wdb125_module_file_demoted_struct_formal_must_borrow_clone_call_sites` | ✅ isolate GREEN |
| `wdb126_module_file_demoted_vec_formal_must_borrow_vec_literal_call_sites` | ✅ no regression |
| `wdb169` / `wdb190` owned Call temps | ✅ no regression — `encode_startup()` stays Identity |
| `bug_thread_spawn_closure_must_not_be_ref_test` | ✅ GREEN |
| `bug_mpsc_sync_channel_boundary_signature_test` | ✅ GREEN |

**Root cause layer:** signature + coercion — (1) same-file demotion (`encode_line(todo: Todo)` → `todo: &Todo`) lived only on preregistered emitted formals; refreshed call-site sigs stayed AST-owned so `force_owned` peeled `&`. Write-back via `sync_call_sig_from_preregistered_free_fn_emission` after refresh. (2) `compute_coercion` / contract: `String.clone()` deref-coerces to `&str`; Custom `&T` does not (Rust will not auto-ref a function-arg temp).

**What became unnecessary:** “`.clone()` satisfies `&T` via deref” for Custom (string-only now); rejected extra regular-call / function-call peels that duplicated IR. Temporary: `force_owned` still skips when preregistered expects borrow (delete once registry always carries same-file demotion).

**Gates:** `cargo test --release --lib -- owned_custom_clone_text_into_ref_formal_prefixes_borrow owned_string_clone_text_into_str_ref_keeps_clone_without_amp` → 2 passed. `cargo test --release --test all -- demoted_encode_line_clone_must_auto_borrow demoted_struct_loop wdb125_module_file_demoted_struct_formal_must_borrow_clone_call_sites wdb126_module_file_demoted_vec_formal_must_borrow wdb169_module_file_owned_helper_into_owned_formal_must_not_borrow wdb190_module_file_feed_unified_owned_startup_must_not_borrow bug_thread_spawn_closure_must_not_be_ref_test bug_mpsc_sync_channel_boundary_signature_test` → 12 passed.

**Follow-up (P3.446):** `user_join_two_strings_moves_owned_locals` is now isolate GREEN — local user `join` shadows `use std::strings` lookup; shape-aware pick keeps the owned `relative` slot.

## P3.440 (2026-09-24) — WDB-379 `format!` must not lower to `write!`/`unwrap`

| Gate | Status |
|------|--------|
| `wdb379_module_file_format_must_not_emit_write_unwrap` | ✅ isolate GREEN — let-bound `format!("{}-{}", …)` stays `format!` |
| WDB-379 tip-out / `gen/` | ❌ stale product — needs retranpile with tip `wj` |

**Root cause layer:** codegen — capacity-hint path emitted `String::with_capacity` + `write!(&mut __s).unwrap()` (Rust leakage). Direct `return format!(…)` skipped the hint; `let s = format!(…)` (product shape) hit it.

**What became unnecessary:** rustc `write!` prealloc for `format!`. Hints remain recorded for a future WJ-native path.

**Gates:** `cargo test --release --test all -- wdb379_module_file_format_must_not_emit_write_unwrap` → isolate GREEN / tip-out RED.

## P3.438 (2026-09-24) — Defining demotion beats importer stubs; runtime `&str` still wins

| Gate | Status |
|------|--------|
| `refresh_call_site_prefers_global_bare_pass_demotion_over_importer_stub` | ✅ GREEN |
| `local_user_fn_homonym_keeps_global_bare_pass_when_bare_names_match` | ✅ GREEN |
| `bare_pass_skips_pub_vec_u8_owned_api_wdb175` | ✅ GREEN — fixture now has product `buf_len` + `decode_startup` |
| `prefer_shared_ref_picks_runtime_str_over_wj_owned_emission` | ✅ GREEN |
| `prefer_shared_ref_picks_runtime_connection_query_over_wj_owned_sql` | ✅ GREEN |
| `refresh_join_delimiter_uses_runtime_fallback_from_stdlib` | ✅ GREEN |
| `bug_todo_cli_cross_crate_validate_field_must_auto_borrow_test` | ✅ no regression |
| `bug_thread_spawn_closure_must_not_be_ref_test` | ✅ GREEN (already) |
| `bug_mpsc_sync_channel_boundary_signature_test` | ✅ GREEN (already) |
| WDB-381/382/383 isolates | ✅ isolate GREEN; tip-out ❌ stale product |
| `promote_overlapping_prefers_mixed_owned_string_over_all_ref_importer_stub` | ✅ GREEN |

**Root cause layer:** signature — (1) `owned_user_refresh_beats_stdlib_shared_ref` treated crate-prefix demotion (`sf1_cli::run_parquet_load`) as a stdlib homonym of the importer stub; (2) `local_owned_wj_string_api_beats_borrowed_homonym` treated Borrowed + stale `emitted=false` + `Type::String` formals as an owned user API; (3) P3.437 mixed-beats in `prefer_shared_ref_signature` froze WJ `strings::join` / `Connection::query` owned-string stubs over runtime-scanned `&str`/`AsRef<str>`.

**What became unnecessary:** classifying importer all-false stubs as owned user APIs; mixed-beats blocking runtime-std challengers. No new `ir_call_site` peel.

**Gates:** `cargo test --release --lib -- refresh_call_site_prefers_global_bare_pass_demotion_over_importer_stub local_user_fn_homonym_keeps_global_bare_pass_when_bare_names_match bare_pass_skips_pub_vec_u8_owned_api_wdb175 prefer_shared_ref_picks_runtime refresh_join_delimiter_uses_runtime_fallback_from_stdlib refresh_split pick_prefers_mixed_owned_string_over_all_ref_importer_stub merge_refresh_keeps_mixed_owned_over_importer_all_ref_stub merge_refresh_upgrades_importer_stub_to_defining_mixed registry_refresh_prefers_mixed_over_more_true_flags promote_overlapping_prefers_mixed_owned_string_over_all_ref_importer_stub` → 12 passed. `cargo test --release --test all -- bug_todo_cli_cross_crate_validate_field_must_auto_borrow_test bug_wdb175_module_file_demoted_vec_into_owned_vec_must_clone bug_thread_spawn_closure_must_not_be_ref_test bug_mpsc_sync_channel_boundary_signature_test` → 7 passed. `cargo test --release --lib -- prefer_shared_runtime_tests promote_overlapping_tests multipass_bare_pass_demotion::tests` → 30 passed.

**P3.439 follow-up:** `local_owned_wj_string_api_beats_borrowed_homonym` keep `Owned || emitted_owned || flags==false` for user `join` vs `strings::join`; only refuse when `resolved` is a **non-stdlib** defining-module demotion (`shared_ref_emission_beats`). Full suite on P3.438 binary: **5381 passed, 167 failed** (majority tip-out/gen lag). `user_join_two_strings_moves_owned_locals` ✅ GREEN (P3.446). Trait owned-string field gates ✅ GREEN (P3.451).

## P3.440 (2026-09-24) — TDD WDB-377–383 tip RED cluster (DB agent)

| Gate | Status |
|------|--------|
| WDB-377–379 MultiFile isolates | ✅ GREEN (3/3) — PassId / struct-lit / `format!` |
| WDB-377–379 tip-out / `gen/` | ❌ RED (3/3) — shader_graph_compiler, shader_graph_builder, scene_file/asset_db/executor |
| WDB-380–383 MultiFile isolates | ✅ GREEN (4/4) — remaining `None` / mesh_id / AssetType / u32 index |
| WDB-380–383 tip-out / `gen/` | ❌ RED (4/4) — tilemap/blend/music, pbr+dispatcher, asset_browser, frame_analysis |

**Root cause layer:** none this session — DB agent files gates only. Isolates already emit correctly; product tip-out is stale vs tip `wj`.

**What became unnecessary:** nothing in codegen. WDB-367 tip GREEN was path-list incomplete (missed tilemap/voxel_scene/blend_tree/dialogue/timeline/music).

**Gates:** `cargo test --release --test all --features integration_tests,codegen_tests -- bug_wdb377_ bug_wdb378_ bug_wdb379_` → **3 passed / 3 failed**. `… bug_wdb380_ bug_wdb381_ bug_wdb382_ bug_wdb383_` → **4 passed / 4 failed**.

## P3.437 (2026-09-24) — Mixed defining formals beat all-ref stubs; file WDB-380

| Gate | Status |
|------|--------|
| `merge_refresh_keeps_mixed_owned_over_importer_all_ref_stub` | ✅ GREEN |
| `merge_refresh_upgrades_importer_stub_to_defining_mixed` | ✅ GREEN |
| `registry_refresh_prefers_mixed_over_more_true_flags` | ✅ GREEN |
| `pick_prefers_mixed_owned_string_over_all_ref_importer_stub` | ✅ GREEN |
| `bare_pass_must_not_demote_path_dep_owned_emission_slot` | ✅ GREEN |
| `bug_todo_cli_cross_crate_validate_field_must_auto_borrow_test` | ✅ GREEN — `require_nonempty(&field, value)` |
| `wdb380_module_file_remaining_none_must_not_emit_clone` | ✅ isolate GREEN — bare `None` |
| WDB-380 tip-out / `gen/` | ❌ stale product — needs retranpile with tip `wj` |

**Root cause layer:** signature — `pick_stronger_codegen_refresh` / `merge_codegen_refresh_metadata` keep mixed `[&str, String]` over importer `[true, true]` (count-true-wins and OR-union destroyed owned slots). Bare-pass skips rewriting a path-dep owned-emission slot to `&str`. No extra `ir_call_site` peel required.

**What became unnecessary:** treating last-true-count as stronger than a mixed owned-emission contract; call-site peel for `&value`.

**Gates:** `cargo test --release --lib -- merge_refresh_keeps_mixed_owned_over_importer_all_ref_stub merge_refresh_upgrades_importer_stub_to_defining_mixed registry_refresh_prefers_mixed_over_more_true_flags pick_prefers_mixed_owned_string_over_all_ref_importer_stub bare_pass_must_not_demote_path_dep_owned_emission_slot` → 5 passed. `cargo test --release --test all -- bug_todo_cli_cross_crate_validate_field_must_auto_borrow_test wdb380_module_file` → todo-cli GREEN, isolate GREEN, tip-out RED.

## P3.436 (2026-09-24) — Load path-dep signatures on `--module-file` ModuleCompiler path

| Gate | Status |
|------|--------|
| `path_dep_module_file_load_keeps_mixed_owned_value_formal` | ✅ GREEN — `wj.toml` path-dep `metadata.json` registers mixed formals |
| `promote_overlapping_prefers_mixed_owned_string_over_all_ref_importer_stub` | ✅ GREEN (P3.438) |
| `bug_todo_cli_cross_crate_validate_field_must_auto_borrow_test` | ✅ GREEN (P3.437) |
| `pick_prefers_mixed_owned_string_over_all_ref_importer_stub` | ✅ no regression (P3.435) |

**Root cause layer:** signature — (1) ModuleCompiler `--module-file` never loaded path-dep metadata; (2) `build_library` Step 3 `ownership_changed` + raw `insert` replaced defining `[true, false]` with importer `[true, true]`; Step 4B-b overlay did the same. `promote_overlapping` now restores via `defining_mixed_owned_emission_beats`.

**What became unnecessary:** another `ir_call_site` peel for `&value` (existing peel can fire once Owned is in the local layered registry).

**Gates:** pending this session.

## P3.435 (2026-09-23) — Prefer mixed owned-emission signatures over all-ref stubs

| Gate | Status |
|------|--------|
| `pick_prefers_mixed_owned_string_over_all_ref_importer_stub` | ✅ GREEN |
| `bug_todo_cli_cross_crate_validate_field_must_auto_borrow_test` | ✅ tip GREEN (P3.599) — private Into forward gated on `is_pub` |
| WDB-372 isolate / demoted-str auto-borrow | ✅ no regression |

**Root cause layer:** signature pick/merge — `pick_codegen_refreshed_signature` kept the first `emitted_rust_ref_params` candidate with any `true` flag, so an importer `[true, true]` stub beat defining-module `[true, false]` mixed formals. Merge now refuses to overwrite a stronger owned-emission contract.

**What became unnecessary:** extra regular_call peel for path-dep owned slots (never landed; did not peel `&value`).

**Gates:** `cargo test --release --lib -- pick_prefers_mixed_owned_string_over_all_ref_importer_stub pick_prefers_mut_borrowed_over_ast_owned_custom_stub` → 2 passed. `cargo test --release --test all -- bug_owned_arg_into_demoted_str_formal_must_auto_borrow_test bug_cross_crate_demoted_str_owned_arg_must_auto_borrow_test bug_wdb372_module_file_index_enum_match_must_not_clone_scrutinee_test::wdb372_module_file_index_enum_match_must_not_clone_scrutinee` → 3 passed. Validate still RED.

## P3.434 (2026-09-23) — Match scrutinee must not `.clone()` indexed non-Copy enums

| Gate | Status |
|------|--------|
| `bug_thread_spawn_closure_must_not_be_ref_test` | ✅ GREEN |
| `bug_mpsc_sync_channel_boundary_signature_test` | ✅ GREEN |
| `bug_wdb372_module_file_index_enum_match_must_not_clone_scrutinee` | ✅ isolate GREEN — `Text(string)` non-Copy; `match &self.entries[idx].value` |
| `bug_wdb377` / `bug_wdb378` / `bug_wdb379` isolates | ✅ isolate GREEN (already on tip; tip-out pending regen) |
| `bug_todo_cli_cross_crate_validate_field_must_auto_borrow_test` | ✅ tip GREEN (P3.599) — `require_nonempty(&field, value)` |

**Root cause layer:** constraint/type — `generate_field_access` auto-cloned non-Copy index fields (`Val` with `Text(string)`). Match/if-let now set `suppress_borrowed_clone` and prefix `&` for `&self` field/index places instead of `.clone()`. Nested `self.quality.steps` struct-lit consume no longer treats the leaf name as a `self` field (analyzer).

**What became unnecessary:** auto-clone on match scrutinees (if-let path previously suppressed only for tuple patterns). No new `ir_call_site` peel (path-dep validate peel reverted — it did not peel `&value`).

**Gates:** `cargo test --release --test all -- bug_wdb372_module_file_index_enum_match_must_not_clone_scrutinee_test::wdb372_module_file_index_enum_match_must_not_clone_scrutinee bug_wdb377_module_file_copy_pass_id_must_not_double_clone_test::wdb377_module_file_copy_pass_id_must_not_double_clone bug_wdb378_module_file_index_struct_lit_must_not_clone_element_per_field_test::wdb378_module_file_index_struct_lit_must_not_clone_element_per_field bug_wdb379_module_file_format_must_not_emit_write_unwrap_test::wdb379_module_file_format_must_not_emit_write_unwrap` → 4 isolate passed; validate still RED.

## P3.432 (2026-09-23) — Align stale ownership gates with tip inference

| Gate | Status |
|------|--------|
| `bug_thread_spawn_closure_must_not_be_ref_test` | ✅ GREEN |
| `bug_mpsc_sync_channel_boundary_signature_test` | ✅ GREEN |
| `compiler_tests::test_automatic_reference_insertion` | ✅ GREEN — Copy pass-by-value accepts mixed-int `x as i64`; `greet(&name)` already correct |
| `codegen_multi_use_struct_field_must_auto_clone_gate_test` | ✅ GREEN — identity `own_code(s: string)` (`.replace` demotes to `&str`) |
| `e0507_ownership_inference_test::test_vec_index_method_owned_self_generates_clone` | ✅ GREEN — return bare `self` keeps owned-self; field getters stay `&self` + clone |

**Root cause layer:** none in compiler — the three sample REDs were stale assertions. Field-return methods (`self.name`) are intentionally `&self` (parameter_analysis). `&str`-only helpers demote. Unannotated `let x = 5` is i32 and casts into `int` (i64).

**What became unnecessary:** no new reconcile. Gates now match tip contracts instead of old clone/`double(x)` strings.

**Gates:** `cargo test --release --test all -- bug_thread_spawn_closure_must_not_be_ref_test bug_mpsc_sync_channel_boundary_signature_test compiler_tests::test_automatic_reference_insertion codegen_multi_use_struct_field_must_auto_clone_gate_test::multi_use_struct_field_must_clone_before_owned_formal e0507_ownership_inference_test::test_vec_index_method_owned_self_generates_clone` → spawn/mpsc GREEN; after gate align, 3/3 sample tests GREEN.

## P3.431 (2026-09-23) — MutBorrowed Copy/Custom call sites keep `&mut`

| Gate | Status |
|------|--------|
| `copy_type_passthrough_mut_test::test_copy_type_self_field_passthrough_mut` | ✅ GREEN — `apply_rotation(&mut self.transform)` |
| `reference_handling_test::test_mut_ref_no_double_borrow` | ✅ GREEN — `modify(&mut vec)` |
| `cross_module_self_field_mut_test` | ✅ GREEN (fill_grid / mixed) |
| `codegen_mut_owned_param_moved_test` | ✅ GREEN — AppDeps stays owned `mut deps` |
| `auto_mut_borrow_arg_test` | ✅ GREEN |
| `compiler_tests::test_automatic_reference_insertion` | ✅ GREEN (P3.432) — `greet(&name)`; `double((x as i64))` is pass-by-value |
| `codegen_multi_use_struct_field_must_auto_clone_gate_test` | ✅ GREEN (P3.432) — identity `own_code` keeps owned `String`; field clones |
| `e0507_ownership_inference_test::test_vec_index_method_owned_self_generates_clone` | ✅ GREEN (P3.432) — `into_track(self)` + `.clone().into_track` |

**Root cause layer:** signature — (1) `emitted_owned_arg_contract` / `bare_formal_is_owned_user_type` no longer claim owned for MutBorrowed + bare Custom; (2) `pick_codegen_refreshed_signature` treats live MutBorrowed as mut-borrow refresh without requiring a MutableReference wrap; (3) `sync_call_sig_from_preregistered_free_fn_emission` was classifying `t: &mut T` as owned (because `: &mut` contains `: &`) and unwrapping to bare Custom. Constraint write-back: `wrap_converged_borrow_param_types` now wraps MutBorrowed Copy aggregates.

**What became unnecessary:** MutBorrowed + bare Custom → owned claim in `emitted_owned_arg_contract` (both the `emitted=false` and no-record paths). AppDeps owned-mut is the post-sync **Owned** contract, not MutBorrowed.

**Gates:** `cargo test --release --lib -- pick_prefers_mut_borrowed_over_ast_owned_custom_stub mut_borrowed_bare_custom_expects_mut_ref owned_bare_custom_after_owned_mut mut_borrowed_explicit_mut_ref mut_borrowed_bare_vec` → 5 passed. `cargo test --release --test all -- copy_type_passthrough_mut_test reference_handling_test::test_mut_ref_no_double_borrow cross_module_self_field_mut_test codegen_mut_owned_param_moved_test auto_mut_borrow_arg_test` → 14 passed / 1 pre-existing `compiler_tests` RED (not in this filter).

## P3.430 (2026-09-23) — WDB-374–376 isolates + strengthen 372/373 `self.`

| Gate | Status |
|------|--------|
| `bug_thread_spawn_closure_must_not_be_ref_test` | ✅ GREEN |
| `bug_mpsc_sync_channel_boundary_signature_test` | ✅ GREEN |
| WDB-370–376 MultiFile isolates | ✅ GREEN (`is_type_copy` skip from P3.429; 372/373 now use `self.`) |
| WDB-370–376 tip-out / `gen/` | ❌ stale product — needs retranpile with tip `wj` |

**Root cause layer:** constraint/type — same `is_type_copy` skip as P3.429; no new reconcile. Filed missing TDD isolates for WDB-374/375/376 (queue rows had no tests).

**What became unnecessary:** nothing new in codegen; product `].clone().state` / nested binding / `buffer_id` clones are already wrong vs tip isolate emit.

**Gates:** `cargo test --release --test all -- bug_thread_spawn_closure_must_not_be_ref_test bug_mpsc_sync_channel_boundary_signature_test bug_wdb370_ bug_wdb371_ bug_wdb372_ bug_wdb373_ bug_wdb374_ bug_wdb375_ bug_wdb376_` → 11 passed (2 spawn/mpsc + 7 isolates + extras) / 7 tip-out RED.

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
| P1 | **WDB-101: borrowed map getter call site must auto-`&` owned local** | `wdb101_borrowed_vertex_map_getter_must_auto_borrow_at_call_site` | ✅ tip IR GREEN (P3.487: Phase-2 `&T` wrapper passthrough accepted) |
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
| P1 | **Demoted `&str` formal must not receive `String.clone()` at call site** | `codegen_demoted_str_formal_must_not_receive_owned_clone_gate_test` | ✅ isolate GREEN (P3.472) — multipass re-restores `strings::*` Borrowed; `parse_body(json: &str)` |
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
| P1 | **`Vec` index with Windjammer `int` loop var (`wj-yaml`)** | `bug_vec_int_index_loop_test` (see also `bug_substring_int_indices_usize_test`) | ✅ isolate GREEN (P3.454) — `lines[(idx as usize)]` without stamping all `let mut x = 0` as i64 |
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
| P1 | **`strings.substring` int indices must not emit `i64 + 1_usize`** | `bug_haystack_contains_substring_int_index_unify_test` | ✅ tip GREEN (P3.452) — wrap before `already_usize`; `(i + j + 1) as usize` |
| P1 | **Match `Ok(body)` → comparison-only formal demotes; call site borrows** | `bug_owned_match_binding_cross_fn_owned_string_formal_test` | ✅ tip GREEN (P3.471) — Phase-2 `&str` + `decode_store(&body)` |
| P1 | **LedgerKit request_context UUID/Bearer without empty-concat** | `bug_request_context_uuid_substring_no_plus_empty_test` | ✅ tip GREEN — P3.247 dogfood |
| P1 | **Seed overlay `Ok(body) => body` without empty-concat** | `bug_seed_overlay_read_body_no_plus_empty_test` | ✅ tip GREEN — P3.248 dogfood |
| P1 | **Seed overlay `remember_*` loop/split without empty-concat** | `bug_seed_overlay_remember_no_plus_empty_test` | ✅ tip GREEN — P3.249 dogfood |
| P1 | **Seed overlay `apply_*` BankLineView + module const → owned field** | `bug_seed_overlay_apply_bank_line_no_plus_empty_test` | ✅ tip GREEN — `LINE_STATUS_MATCHED.to_string()` (P3.257) |
| P1 | **Owned helper return → demoted `&str` formal auto-borrow** | `bug_owned_helper_into_demoted_str_formal_must_auto_borrow_test` | ✅ tip GREEN (2026-09-12) |
| P1 | **Cross-crate owned free fn named `encode` must not borrow arg** | `bug_cross_crate_owned_encode_named_fn_must_not_borrow_arg_test` | ✅ tip GREEN (P3.282) — import alias + qualified registry lookup; no bare `encode` homonym borrow |
| P1 | **Import alias must not steal foreign fn ownership metadata** | `bug_import_alias_must_not_steal_foreign_fn_ownership_test` | ✅ tip GREEN (P3.448) — remapped `owned_pkg::get` only; comma `--metadata`; no foreign `query_get` merge |
| P1 | **Owned `Vec<Custom>` filter helper must not demote + clone** | `bug_owned_vec_custom_filter_helper_must_not_demote_and_clone_test` | ✅ tip GREEN (P3.284) — forwarder keeps owned `Vec` when callee preregistered owned |
| P0 | **`std::thread::spawn(\|\| …)` must not wrap closure in `&(move \|\| …)` (E0716/E0525)** | `bug_thread_spawn_closure_must_not_be_ref_test` | ✅ tip GREEN (P3.286) — qualified `thread::spawn` + FnOnce owned peel; no bare `spawn` homonym |
| P1 | **`std::sync::mpsc::sync_channel` missing boundary signature** | `bug_mpsc_sync_channel_boundary_signature_test` | ✅ tip GREEN (P3.287) — `mpsc::sync_channel` aliased from runtime; SyncSender typing is P3.293 |
| P1 | **`mpsc::SyncSender` type for bounded channels (`Sender`≠`SyncSender`)** | `bug_mpsc_sync_sender_type_for_bounded_channel_test` | ✅ tip GREEN (P3.293) — `BoundedIntSender` cargo-checks; `wj-sync` bounded live |
| P1 | **`std::thread::spawn(move \|\| …)` with Arc capture still wraps `&(move \|\|…)`** | `bug_thread_spawn_move_arc_must_not_be_ref_test` | ✅ tip GREEN (P3.294) — parser `move\|\|` closure + FnOnce Identity peel |
| P1 | **`spawn(move \|\|)` must preserve `move` keyword (not emit bare `\|\|`)** | `bug_thread_spawn_move_keyword_must_be_preserved_test` | ✅ isolate GREEN (P3.457) — `spawn(move \|\|` (unwrap leftover `&(…)` wrap) |
| P1 | **Library multipass strips `spawn(move \|\|)` `move` keyword** | `bug_module_file_spawn_move_keyword_must_be_preserved_test` | ✅ tip GREEN (P3.295) — library `--module-file` preserves `move` |
| P1 | **Library multipass strips `spawn(move \|\|)` when closure starts with `while`** | `bug_module_file_spawn_move_in_worker_loop_must_be_preserved_test` | ✅ tip GREEN (2026-09-16) — P3.297 While/Loop capture analysis |
| P1 | **`mut out: Vec<u8>` returned owned must not demote to `&Vec<u8>` (`wj-uuid`)** | `bug_mut_owned_vec_u8_return_must_not_demote_to_ref_test` | ✅ tip GREEN (2026-09-16) — P3.298 returned Vec must not demote |
| P1 | **`int` find-pos `>= 0` must not emit `as usize >= 0_i64` (`wj-timefmt`)** | `bug_int_find_pos_ge_zero_must_not_mix_usize_i64_test` | ✅ tip GREEN (2026-09-16) — binding beats usize_variables; `strings::len`→i64 |
| P1 | **`int` `while n>0` `n % 10` / `digit == 0` must not split i64 vs i32 (LedgerKit)** | `bug_int_mod_literal_zero_compare_must_not_split_i64_i32_test` | ✅ tip GREEN (P3.336); LedgerKit `make api-check` GREEN |
| P1 | **`substring(s, i, i+1)` emits `(i + 1_i32) as usize` (`wj-duration`)** | `bug_substring_end_i_plus_one_must_not_emit_i32_into_usize_test` | ✅ tip GREEN (P3.300 isolate + P3.315 nested) |
| P1 | **`&mut DenseCsr` → owned `distances_to_map` must clone (batch)** | `bug_wdb235_module_file_mut_ref_csr_into_owned_distances_to_map_must_clone_test` | ✅ tip GREEN (P3.316) — tip demotes to `&DenseCsr` |
| P1 | **HashMap String `contains_key`/`get` must borrow key** | `bug_wdb236_module_file_hashmap_string_get_must_borrow_key_test` | ✅ tip GREEN (2026-10-08) — `get(&key)`; `let mut updated = neighbors.clone()` cargo-check |
| P1 | **u64 acc `+= len() as u64 as i64` must stay u64** | `bug_wdb237_module_file_u64_acc_must_not_cast_len_through_i64_test` | ✅ tip GREEN (P3.316) — `len() as u64` |
| P1 | **`"props".to_string()` → demoted `sql_exec` `&str`** | `bug_wdb244_module_file_string_lit_into_demoted_sql_exec_must_not_to_string_test` | ✅ tip GREEN (P3.364); twin module-file gate |
| P1 | **bare `"props"` → owned df table_provider must `.to_string()`** | `bug_wdb245_module_file_string_lit_into_owned_df_table_must_to_string_test` | ✅ tip GREEN (P3.383) — tip demotes `left_table: &str` + bare lit |
| P1 | **WCC `p.clone()` → demoted `&GraphVertexI64Map` get must borrow** | `bug_wdb247_module_file_owned_wcc_map_clone_into_demoted_ref_must_borrow_test` | ✅ tip GREEN (verified 2026-10-04); twin WDB-222 |
| P1 | **analytics `csr.clone()` → demoted `&DenseCsr` multi_source must reborrow** | `bug_wdb248_module_file_owned_csr_clone_into_demoted_ref_analytics_must_reborrow_test` | ✅ tip GREEN (P3.383) — owned DenseCsr + clone accepted |
| P1 | **SSSP `distances.clone()` → demoted `&GraphVertexF64Map` get must borrow** | `bug_wdb249_module_file_owned_sssp_f64_map_clone_into_demoted_ref_must_borrow_test` | ✅ tip GREEN (verified 2026-10-04); twin WDB-223 |
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
| P1 | **sysbench `&samples` → owned workload_verdict must clone** | `bug_wdb285_module_file_demoted_vec_into_owned_sysbench_verdict_must_clone_test` + `bug_demoted_vec_param_into_owned_vec_callee_must_clone_test` | ✅ isolate GREEN (P3.456); tip-out/gen still host-lag |
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
| P1 | **Copy i32 must not emit `.clone()`** | `bug_wdb343_module_file_copy_i32_must_not_emit_clone_test` | ✅ isolate GREEN (P3.470) — `i32::max` signature; no `max_size.clone()` |
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
| P1 | **usize counter must not emit `(i as usize)`** | `bug_wdb361_module_file_usize_counter_must_not_cast_as_usize_test` | ✅ isolate GREEN (P3.470) — skip cast when binding emits `usize` |
| P1 | **indexed `&self` method must not `].clone().mesh_id()`** | `bug_wdb362_module_file_index_method_must_not_clone_element_test` | 🆕 RED / filed (P3.418); MultiFile GREEN; tip RED; twin WDB-359 |
| P1 | **indexed tuple field must not `].clone().rotation`** | `bug_wdb363_module_file_index_tuple_field_must_not_clone_element_test` | 🆕 RED / filed (P3.418); MultiFile GREEN; tip RED; twin WDB-359 |
| P1 | **usize lit must not emit `N_usize as usize`** | `bug_wdb364_module_file_usize_lit_must_not_cast_as_usize_test` | 🆕 RED / filed (P3.419); MultiFile GREEN; tip RED; twin WDB-361/352 |
| P1 | **statement args must not emit `__wj_tmpN` lets** | `bug_wdb365_module_file_must_not_emit_wj_tmp_lets_test` | ✅ tip GREEN (P3.723) — nested self-field split-borrow |
| P1 | **BT tick must not force `tree.clone()`** | `bug_wdb366_module_file_bt_tick_must_not_force_tree_clone_test` | ✅ tip GREEN (P3.721) — hard-owned restore; bare forward ≠ FieldInCallArg/store |
| P1 | **`None` must not emit `None.clone()`** | `bug_wdb367_module_file_none_must_not_emit_clone_test` | ✅ MultiFile + tip GREEN (P3.427) — unit keywords + `Type::None` identifier paths |
| P1 | **`string` into `&str` must not emit `&*ident`** | `bug_wdb368_module_file_string_must_not_emit_star_deref_ref_test` | ✅ tip GREEN (P3.722) — text ref skip Copy `*` path; product match+let |
| P1 | **string field eq must not `.key.clone() ==`** | `bug_wdb369_module_file_string_field_eq_must_not_clone_test` | ✅ MultiFile GREEN (P3.421) — honor `suppress_borrowed_clone` on index-field; tip-out pending regen |
| P1 | **indexed Copy field must not `].clone().coord.clone()`** | `bug_wdb370_module_file_copy_field_must_not_double_clone_test` | ✅ isolate GREEN (P3.429) — `is_type_copy` skip on Copy aggregates; tip-out pending regen |
| P1 | **indexed Copy Vec3 must not `].clone().position.clone()`** | `bug_wdb371_module_file_copy_vec3_field_must_not_double_clone_test` | ✅ isolate GREEN (P3.429); tip-out pending regen; twin WDB-370/355 |
| P1 | **indexed enum match must not `].value.clone()`** | `bug_wdb372_module_file_index_enum_match_must_not_clone_scrutinee_test` | ✅ isolate GREEN (P3.434) — non-Copy `Text(string)`; `suppress_borrowed_clone` + `&` prefix; tip-out pending regen |
| P1 | **indexed String field must not `].clone().path.clone()`** | `bug_wdb373_module_file_index_string_field_must_not_double_clone_test` | ✅ isolate GREEN (P3.429/430) — `self.watches[i].path`; tip-out pending regen |
| P1 | **indexed Copy enum must not `].clone().state.clone()`** | `bug_wdb374_module_file_copy_enum_field_must_not_double_clone_test` | ✅ isolate GREEN (P3.430) — `is_type_copy` skip; tip-out pending regen |
| P1 | **nested index must not `].clone().bindings[j].clone().binding_type.clone()`** | `bug_wdb375_module_file_nested_index_must_not_clone_chain_test` | ✅ isolate GREEN (P3.430) — nested `self.passes[i].bindings[j].binding_type`; tip-out pending regen |
| P1 | **indexed Copy u32 must not `.buffer_id.clone()`** | `bug_wdb376_module_file_copy_u32_index_field_must_not_clone_test` | ✅ isolate GREEN (P3.430) — `self.lifetimes[i].buffer_id`; tip-out pending regen |
| P1 | **indexed Copy PassId must not `].clone().pass_id.clone()`** | `bug_wdb377_module_file_copy_pass_id_must_not_double_clone_test` | ✅ isolate GREEN (P3.433/434/438); tip RED (P3.438 TDD); twin WDB-374 |
| P1 | **index struct lit must not clone element per field** | `bug_wdb378_module_file_index_struct_lit_must_not_clone_element_per_field_test` | ✅ isolate GREEN (P3.433/434/438); tip RED (P3.438 TDD); twin WDB-370/375 |
| P1 | **`format!` must not emit `write!(&mut __s).unwrap()`** | `bug_wdb379_module_file_format_must_not_emit_write_unwrap_test` | ✅ isolate GREEN (P3.440) — let-bound `format!` (capacity hint) no longer `write!`/`unwrap`; tip-out pending regen |
| P1 | **remaining product `None.clone()` (tilemap/blend/music/…)** | `bug_wdb380_module_file_remaining_none_must_not_emit_clone_test` | ✅ isolate GREEN (P3.437/438); tip RED (P3.438 TDD); twin WDB-367 |
| P1 | **indexed String mesh_id must not `].clone().mesh_id.clone()`** | `bug_wdb381_module_file_index_mesh_id_must_not_double_clone_test` | ✅ isolate GREEN (P3.437/438); tip RED (P3.438 TDD); twin WDB-373 |
| P1 | **indexed Copy AssetType must not `].clone().asset_type.clone()`** | `bug_wdb382_module_file_copy_asset_type_must_not_double_clone_test` | ✅ isolate GREEN (P3.437/438); tip RED (P3.438 TDD); twin WDB-374/377 |
| P1 | **u32 index must not emit `as i64 as usize`** | `bug_wdb383_module_file_u32_index_must_not_cast_via_i64_test` | ✅ isolate GREEN (P3.437/438); tip RED (P3.438 TDD); twin WDB-353 |
| P1 | **Copy enum variant must not `FaceDirection::PosX.clone()`** | `bug_wdb384_module_file_copy_enum_variant_must_not_clone_test` | ✅ isolate GREEN (P3.443 TDD); tip RED |
| P1 | **`f32::MAX`/`MIN` must not emit `.clone()`** | `bug_wdb385_module_file_f32_assoc_const_must_not_clone_test` | ✅ isolate GREEN (P3.445) — `copy_primitive_associated_path_type`; tip-out pending regen |
| P1 | **indexed Copy ShaderFile must not `].clone().shader_file`** | `bug_wdb386_module_file_copy_shader_file_must_not_double_clone_test` | ✅ isolate GREEN (P3.443 TDD); tip RED; twin WDB-377 |
| P1 | **indexed `get_id()` must not `].clone().get_id()`** | `bug_wdb387_module_file_index_get_id_must_not_clone_element_test` | ✅ isolate GREEN (P3.443 TDD); tip RED; twin WDB-362 |
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
| P1 | **i32 coord / GPU dim literal peers must not emit `_i64`** | `bug_i32_coord_literal_peers_must_not_emit_i64_test` | ✅ tip GREEN (P3.450) — module `int` const casts `as i32` in coord builders |
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
| P1 | **usize `start = i + 1` emits `1_usize as i32/i64` (`wj-toml`)** | `bug_module_file_usize_i_plus_one_assign_must_stay_usize_test` | ✅ tip GREEN (P3.326 + P3.577 start-before-`i`) |
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
| Gate `bug_module_file_shared_map_get_must_borrow_key_test` | ✅ tip GREEN (P3.460) — `g.data.get(&key)` after `Mutex::lock` types `Ok(g)` |

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
| Gate `bug_import_alias_must_not_steal_foreign_fn_ownership_test` | ✅ tip GREEN (P3.448 restore) |
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
| Gate `bug_mut_param_passthrough_no_shared_amp_test` | ✅ isolate GREEN (P3.458) — `&mut`/`&T` reborrow Identity, no stacked `&csr` |
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
| Tip **WDB-201** `&mut Key` → owned `entries.push` | ✅ isolate GREEN (P3.466); tip-out/gen ⚠️ host-lag |
| Tip **WDB-202** `&mut MulticolState` → owned return | ✅ **GREEN** — tip-out/gen owned `state:` (no demoted `&mut`) |
| Tip **WDB-203** owned `sql` → demoted `&str` simple_query | ✅ isolate GREEN (P3.467); tip-out/gen ⚠️ host-lag |
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
| Tip **WDB-236** HashMap String `contains_key`/`get` | ✅ codegen GREEN (2026-10-08) — `get(&key)` and owned clone of `&Vec` (`neighbors.clone()`); cargo-check |
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
| Tip **WDB-209** `catalog_push_column` `&mut CatalogColumnBinding` | ✅ **GREEN** — P3.461 last-use `push(col)` move; formal owned |
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
| Tip **WDB-203** owned `sql` → demoted `&str` simple_query | ✅ isolate GREEN (P3.467); tip-out/gen ⚠️ host-lag |
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
| MultiFile **WDB-307** `for node in index.graph.nodes` move | ✅ isolate + tip-out GREEN (P3.447 recheck) |
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
| Tip **WDB-365** `__wj_tmpN` statement temps | ✅ tip GREEN (P3.723) |
| Tip **WDB-366** BT `tree.clone()` owned formal force-clone | ✅ tip GREEN (P3.721) |
| Tip **WDB-332** i32 priority `.to_string()` | ✅ tip GREEN (P3.418 affinity) |

**TDD:** `wdb364_ wdb365_ wdb366_ wdb332_tip` — WDB-366 ✅ tip GREEN (P3.721); 364/365 tip-out lag; 332 tip GREEN.

**Compiler agent priority:** tip greens **WDB-365/368** + WDB-462/463 tip-out (+ open wave). No Phase 606+.


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
| `nested_self_field_in_struct_lit_must_not_force_owned_self` | ✅ MultiFile GREEN (2026-09-23) — `fn update_params(&self)` |
| `cross_file_nested_self_field_in_struct_lit_must_not_force_owned_self` | ✅ MultiFile GREEN (2026-09-24) — `fn update_params(&self)` + cargo-check |
| Tip-out `update_raymarch_params(self)` | ✅ tip GREEN (2026-09-24 regen) — `fn update_raymarch_params(&self)` |

**Product:** `update_raymarch_params(self)` (reads only) called from `&mut self` loop → E0507 move. Atmosphere/water update_* already emit `&self`. Engine cargo-check after regen: **288 rustc errors** (E0308×197, E0277×33, E0507×21).

**Fix (same-file):** Struct-literal field values no longer treat nested `self.a.b` as a last-segment lookup on `self`. Direct `self.field` stays conservative when the type is unknown; nested chains use `resolve_self_field_chain_type` (Copy leaf = read).

**Remaining:** unknown direct `self.field` in a split-impl struct literal still defaults to “moves self” when `lookup_field_type_for_self` misses cross-file VoxelGPURenderer fields.

## P3.426 WindjammerDB CQ-C5 — tip REDs WDB-371–373 (2026-09-22)

| Gate | Status |
|------|--------|
| Tip **WDB-371** `].clone().position.clone()` (mesh_ops/half_edge) | 🆕 RED / filed |
| Tip **WDB-372** `match …].value.clone()` (blackboard) | isolate ✅ P3.434; tip-out pending regen |
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

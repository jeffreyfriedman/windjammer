# Ownership / width ratchet residue (correctness stream, 2026-10)

Status of the open `.config/known-test-failures.txt` entries after the correctness stream. Run each test in
its own process (nextest model); under in-process `cargo test` the process-global registries make results
order dependent (see "Test isolation" below).

## 1. While/for counter used as an index keeps a generic width (`bug_while_literal_bound_index_must_not_be_i32_test`)

```windjammer
pub fn pick(offsets: Vec<f32>) -> f32 {
    let mut i = 0
    let mut best = 0.0
    while i < 6 { best = offsets[i]; i = i + 1 }
    best
}
```

emits `let mut i: i32 = 0_i32; while i < 6_i32 { best = offsets[(i as usize)]; ... }`. Valid Rust, but the width is a
generic default plus a cast, not the `usize` the index use implies.

Attempted: in the (legacy) int collector, record `v[ident]` index uses and, after the hard `MustBe`/`MustMatch`
fixpoint, give a fully unconstrained `MustMatch` class that contains an index use `usize` (a *soft default*, never
overriding a typed member). The class is found and resolved correctly (`DBG` showed the literal, the identifier uses and
the `i + 1` step all in one free class), but the Rust backend still printed `i32`: `let_statement_generation.rs`
(~lines 600-690: `prefer_usize_over_i32`, `while_i32_peer`, `codegen_i32_binding_names`,
`sync_i32_coord_binding_after_let`) decides the local's width independently of the numeric solver and re-stamps `: i32`.
Reverted rather than leave an unused constraint kind.

Design: the numeric solver must be the only width owner (ARCHITECTURE_IMPROVEMENT_PLAN WP 8); delete the let/while width
heuristics in `let_statement_generation.rs` / `int_promotion_type_inference.rs`, then add the soft index default to
`ir/numeric_solver` as a `PreferIfUnconstrained(id, usize)` constraint.

## 2. Callee `Vec<u8>` params stored into a struct: borrow vs own (`path_and_bytes_borrow_tests::path_bytes_segment_append_{put,delete}...`, `path_bytes_ffi_vec_return_borrowed_at_from_bytes`)

The tests require `append_put(&self, key: Vec<u8>, value: Vec<u8>)` to *borrow* `key`/`value` at
`WalRecord::put(lsn, key, value)` while `WalRecord::put` stores both into a struct field. Storing requires an owned
`Vec<u8>` (or a clone at the store), so a borrowing formal at the call site cannot type-check unless `put`'s formals
themselves are demoted to `&Vec<u8>` with a clone inside `put`. The current output (`append_put(&self, key: Vec<u8>, ..)`
forwarding by move into the owned `put`) is correct Rust and avoids a clone. These three assertions encode a
different, undocumented ownership model for non-`pub` byte-vector formals; an ADR must decide which model is the
contract before the tests or the compiler change.

## 3. Cross-crate dogfood ownership (`cross_crate_dogfooding_ownership_test::dogfood_lsm_store_apply_patch_asymmetric_coercion`, `dogfood_wal_ffi_snapshot_and_path_borrow_at_call_sites`)

Same family as (2) (engine-crate layouts): asymmetric Key/Value formals and FFI `Vec` snapshot borrow at `from_bytes`.
Needs the converged cross-file registry to be the single source for call sites (see 4).

## 4. Root cause shared by most remaining entries: registries disagree at call sites

Call-site code consults, for the same callee, (a) the per-file codegen registry (refreshed while emitting the callee),
(b) `global_signature_registry` (analysis snapshot, can be one pass stale) and (c) AST formals
(`struct_method_ast_formal_param_types`). Several classifiers (`is_collection_key_lookup_with_project`,
`method_is_map_key_qualified_with_project`, `emitted_owned_arg_contract`, `callee_arg_is_hard_owned`) mix them. The
fixed instance is `fp.validate(&arg)` where (b) said `Borrowed` while the emitted formal (a) was owned. Remaining
entries (`library_multipass_ownership_codegen_test::test_match_borrow_break_*`, `ref_mut_logic_layered_test::
test_if_let_method_return_option`, `svo_bfs_layout_test`, `library_multipass_map_key_codegen_test::
test_library_multipass_graph_bfs_hashmap_compiles`) fail rustc or assertions for shape-specific reasons (match
borrow-break clones, `unsafe` FFI parentheses, ...) and were not root-caused in this stream.

WP 9 (one ownership owner) is the structural fix: the final registry must be produced once, before any codegen, and
call sites must read only it.

## 5. Test isolation

Under in-process `cargo test --test all` ~130 tests fail or flip between runs (process-global `OnceLock`
registries / `thread_local!` state shared by parallel tests, e.g. identical function names such as `pass`, `tag`,
`all` in different tests). `.config/nextest.toml` runs one process per test, which is what the ratchet should be
measured with. WP 3 (state into `CompilationSession`) removes the cause.

## 6. Findings that did not reproduce on this tree (a downstream application dogfood)

* `concat-temp-into-demoted-str-formal`: concat temps (`x + ""`) passed to read-only `string` formals, cross-module,
  mixed owned/borrowed formals: emitted `payload(_temp0, &job.id)` / `payload2(&_temp0, &_temp1, &job.format)`,
  rustc clean. Needs the actual 10-formal `audit_canonical_payload` sources to bisect.
* `loop-var-field-move-reused-collection`: `for item in items { next = item.field; break }` followed by a second
  `for item in items` emits `for item in &items` twice and clones the moved field; no repro.

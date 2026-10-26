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

Under in-process `cargo test --test all` many tests fail or flip between runs (process-global `OnceLock`
registries / `thread_local!` state shared by parallel tests, e.g. identical function names such as `pass`, `tag`,
`all` in different tests). `.config/nextest.toml` runs one process per test, which is what the ratchet should be
measured with. WP 3 (state into `CompilationSession`) removes the cause.

## 6. Findings that did not reproduce on this tree (a downstream application dogfood)

* `concat-temp-into-demoted-str-formal`: concat temps (`x + ""`) passed to read-only `string` formals, cross-module,
  mixed owned/borrowed formals: emitted `payload(_temp0, &job.id)` / `payload2(&_temp0, &_temp1, &job.format)`,
  rustc clean. Needs the actual 10-formal `audit_canonical_payload` sources to bisect.
* `loop-var-field-move-reused-collection`: `for item in items { next = item.field; break }` followed by a second
  `for item in items` emits `for item in &items` twice and clones the moved field; no repro.

## 7. Codegen-correctness stream (2026-10-11): fixed, gated, and still open

Gate: `tests/codegen_correctness_test.rs` (also `cargo test --test codegen_correctness`). Hand-written repros with an
independent `// expect:` oracle live in `tests/codegen_correctness_corpus/`; seeded generated programs per class
(`gate_*`, `WJ_CODEGEN_GATE_SEEDS=n`) run both `wj run --interpret` and `wj build --no-cargo` + `rustc` and shrink a failing
program unit by unit. `// interp-gap:` marks a program the interpreter evaluates wrongly; the compiled output must still equal
the expectation. The verified corpus (`examples/verified`, `// rust: gap`) is the second gate: the gap programs now compile
and match (the header was flipped to `// rust: ok`).

Fixed (each with a corpus program): by-reference closure capture for let-bound closures that never escape
(`closure_escape.rs`); `fn(..)` formals and returns emitted as `impl Fn`; `let .. else`; `"x" + n.to_string()`; inline `mod` +
`use` (functions were emitted twice); hand-written `impl PartialEq` vs the inferred derive and `&Self` formals; `impl Trait for
int`; `map[key]` with a `&str`/`String` key; `map.get(&k)` with an owned `String` local; `get(..).unwrap_or(0)` /
`unwrap_or(&0)` / `lines().next().unwrap_or("0")`; `cmp(&x)`; open slices `s[a..]`; `*tok == "lit"`; `return if ..` in a unit
block; integer widths through `${..}` interpolation, closure bodies, `&x`, and `max/min/abs` on integers; `&mut self` for
`Iterator::next`, `self.helper()?`, and loop variables whose field is mutated (`for i in self.list { i.c.inc() }`);
`o.ok_or(e)?` / `r?` on a parameter (owned); parameters moved into a spawned closure (owned); `JoinHandle` pushed by move;
spawn closure tail values; `let mut i: int = 0` with a literal `while` bound; `--library --module-file` writes `mod.rs` for
directories whose `mod.wj` is their only file; direct adapters on collections (`v.map(..)`, `v.sum()`).

Still open (needs the numeric/ownership convergence, not another local heuristic):
* unconstrained integer literals default to `i32` (`let v = [1, 2, 3]; v.map(..).collect()` into `Vec<int>` needs the result
  type to flow back through iterator adapters); `for x in rx { prev = x }` with `let mut prev = 0`.
* `let c = self.items[i]; c.inc()` clones the element: Windjammer's value semantics versus the interpreter's aliasing is
  undecided (needs an ADR).
* codegen still re-decides loop/for borrow mutability (`iteration_borrow_dispatch.rs`) separately from the analyzer's `&mut self`
  rule: both were extended with the same field-chain walk (`for_loop_over_self_field_triggers_mut_self`); WP 9 should leave one.
* the interpreter copies loop elements (`for i in self.list { i.c.inc() }` mutates nothing); and has no threads/channels.
* `ExprId` is `(line, column)` of the token *after* an expression, so `x`, `x * 2` and an enclosing closure can share an id;
  interpolation sub-expressions are now rebased onto unique columns. Keying by node identity is part of WP 8.

## Cross-module trait field access keeps an owned parameter (`cross_module_ownership_test::cross_module_trait_field_access_inferred_borrowed`)

```windjammer
// composition/login.wj
pub fn login(deps: AppDeps, token: string) -> Result<string, string> {
    let claims = match deps.verifier.verify(token) { Ok(v) => v, Err(m) => return Err(m) }
    deps.issuer.issue(claims.sub, claims.email)
}
```

`deps` is only read (a field access and two `&self` trait-method calls) but is emitted as `deps: AppDeps` instead of
`deps: &AppDeps`. The trait methods are declared in another module (`ports/auth.wj`), so at analysis time the callee's
receiver mode is unknown and the conservative answer is "owned". Same class as the multi-file ownership demotion
entries above: the call-site and defining-module analyses need the trait method receiver modes before parameter
ownership is decided.

## An untyped counter compared with a literal is painted `i32` after an `==` against a declared `int` (`interpreter_differential_test::interpreter_matches_compiled_rust`, `check_build_differential_test::check_agrees_with_the_build_pipeline`)

```windjammer
fn main() {
    let seed: int = 1
    let mut i = 0
    while i < 6 {
        let b = i == seed
        i += 1
    }
}
```

The numeric solver resolves every integer in this program to `i64` (`WJ_DEBUG_INT_CONSTRAINTS=1` shows `4:15 -> I64`),
but the Rust backend emits `while i < (6_i32 as i32)` (rustc: expected `i64`, found `i32`). Without the `==` line it is
`6_i64`. The width is re-decided downstream of the solver by the same let/while heuristics as section 1
(`function_prefers_i32_coord_locals`, `codegen_i32_binding_names`, comparison balancing in `type_balancing.rs`).
Convergence C4 made `int` default to `i64` in the solver; deleting those heuristics so the solver is the only width
owner closes this and section 1 together. The interpreter corpus program `result_chain` hits it (seeds 1 and 4).

## Tuple write-back after a by-value call clones the heap vectors (`library_multipass_map_key_codegen_test::test_library_multipass_tuple_writeback_must_not_clone`)

```windjammer
let pushed = push_pair(costs, verts, i as f64, i as i64)
costs = pushed.0
verts = pushed.1
```

emits `push_pair(costs.clone(), verts.clone(), ..)`. Both bindings are overwritten from the result before they are read
again, so the clones are dead copies (rustc accepts the program). The last-use fact (C6, `ir::liveness`) does not model
"reassigned before the next read" for tuple projections of the call result.

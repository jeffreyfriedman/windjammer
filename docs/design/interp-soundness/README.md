---
status: current
last_verified: 5457283f4156
---
# Interpreter fast paths: soundness notes

Every interpreter fast path (machine-code units, typed stores, lean helpers) has a generic fallback that is always
correct, so a fast path is sound only if its guards make it observationally identical to that fallback. Each note below
argues one path's guards, names the code they live in, and names the test that pins them. The design documents these
notes serve are [interp-struct-vec-loops.md](../interp-struct-vec-loops.md),
[interp-call-unit.md](../interp-call-unit.md) and [interp-nbody-ladder.md](../interp-nbody-ladder.md).

Rules:

1. Any code comment that cites a soundness argument links an in-repo path under `docs/design/interp-soundness/`.
2. Any new fast path lands its soundness note in the same branch as the code, and gets a row here.
3. Every row names a test that fails when the guard is removed. The counter gate is `tests/interpreter_fast_paths_test.rs`
   (it asserts each fast-path counter fires on its programs); parity programs live in `tests/engine_parity/`.
4. Measurements are not typed into these notes: they live in the measurement files on pc-results.

| Note | Invariant it protects | Code it guards | Pinned by | Status |
|---|---|---|---|---|
| [fnunit2.md](fnunit2.md) | A whole function body runs as one unit with the walker's frame, error and locals on every exit | `unit_body` in `src/interpreter/resolved/jit/lower.rs`; `enter_body`, `params_alias` in `src/interpreter/resolved/jit/osr.rs` | `tests/engine_parity/jit_fnunit*.wj`; counter `fnunit`; `function_units_refuse_aliased_parameters` | implemented |
| [fnunit.md](fnunit.md) | Earlier tail-chained loop-unit design; hazards list and test matrix | none (superseded by fnunit2) | none | superseded design |
| [hoist.md](hoist.md) | A hoisted row view is only used while no call can clear or move the rows it names (loop versioning, preheader guard) | `hoist.rs`, `lower.rs` (`SAFE_CALLS`), `lower_rows.rs` in `src/interpreter/resolved/jit/` | `tests/engine_parity/jit_hoist_*.wj`; counters `hoisted`, `hoistfail`; `tests/hoist_traps_test.rs` | implemented |
| [rowcache.md](rowcache.md) | Row views cannot outlive a run: nothing keeps their raw addresses valid across OSR entries | `rows::fill` in `src/interpreter/resolved/jit/rows.rs` (unchanged) | none (no code) | rejected design |
| [p3.md](p3.md) | Inlining a mutating callee would need deopt inside the callee | `inline_call` in `src/interpreter/resolved/jit/lower_struct.rs` (pure callees only) | `tests/engine_parity/jit_loop_inline.wj`; counter `inline` | rejected design |
| [maplean.md](maplean.md) | Lean `map_get`/`map_insert` act on exactly the shapes the general helper would, else fall back | `rt::map_get`, `rt::map_insert` in `src/interpreter/resolved/jit/rt.rs`; `lower_map.rs` | `tests/engine_parity/jit_map_lean.wj`; counters `mapleanget`, `mapleanins`; `the_lean_map_helpers_decline_other_shapes` | implemented |
| [intdense.md](intdense.md) | Typed integer map stores rebuild bit-identical `Value`s and keep the documented hash order | `IntDense` in `src/interpreter/kmap.rs`; `rt.rs` | `tests/engine_parity/kmap_intdense*.wj`; counter `mapleantyped`; `typed_arrays_are_only_for_64_bit_signed_int_values_and_keys_in_range` | implemented |
| [vecinline.md](vecinline.md) | A read view exists only on unique packed storage and every mutation clears it | `lean_push`, `index_get`, read views in `src/interpreter/resolved/jit/rt.rs`; `view_load` in `lower_expr.rs` | `tests/engine_parity/jit_vec_view.wj`; counters `packed`, `packedread` | implemented |
| [pushlen.md](pushlen.md) | An inline push's pending length is flushed before every helper call and at every unit exit, so the vector's own length is never observed stale | `Lo::call`, `effect` in `src/interpreter/resolved/jit/lower.rs`, `lower_expr.rs`; `arm_push`, `Ctx::flush_push` in `rt.rs`; `execute` in `osr.rs`; `pushscan.rs` | `tests/engine_parity/jit_push_inline.wj`; counters `pushinl`, `pushflush` | implemented |
| [pipeline.md](pipeline.md) | The typed scalar pipeline is observationally the boxed drain (order, overflow, effects, errors) | `src/interpreter/scalar_pipe.rs`; `drain_iter` in `src/interpreter/iter.rs` | `tests/engine_parity/scalar_pipe_*.wj`; counters `scalarpipe`, `scalarbail`; `the_scalar_pipeline_takes_and_declines_exactly_these_chains` | implemented |
| [constdiv.md](constdiv.md) | A zero or MIN/-1 guard is skipped only for a literal divisor that cannot trap | `int_arith` in `src/interpreter/resolved/jit/lower_expr.rs` | `tests/engine_parity/jit_constdiv_*.wj` | implemented |
| [nodebugkey.md](nodebugkey.md) | AST Debug text is never a key or identity, so eliding file paths from it changes only cost | `ast_text` in `src/interpreter/resolved/island.rs`; `elided_debug` in `src/source_map.rs` | `tests/interpreter_closure_keys_test.rs` | implemented |
| [vecretag.md](vecretag.md) | A local float-literal vector is retagged `f64` once at a `Vec<f64>` call so binding does not rebuild it per call; the slot gets a new body, never an in-place change | `retag_arg`, `is_vec_f64` in `src/interpreter/resolved/expr.rs`; `retagged_f64` in `src/interpreter/vecbody.rs` | `tests/engine_parity/vec_retag.wj`; counter `vecretag` | implemented |

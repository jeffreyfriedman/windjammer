---
status: current
last_verified: 5457283f4156
---
# perf-jit-vec-inline: invalidation argument (written before the change)

Branch is stacked on perf-jit-map-lean (it reuses the `local_cell` hit table and `lean_value`). Targets: the sieve's
per-element `rt::push` and `rt::index_get` calls plus `local_cell` (costs on pc-results).

## Part A: push. DECISION: a lean helper, NOT inline machine code.
Inlining the push in machine code would write the element into spare capacity and bump the vector's length. `Vec`'s
length lives in the Rust `Vec<bool>` header (unspecified layout), so machine code would have to keep a pending length
in `Ctx` and flush it before every call-out, OSR exit and deopt (every `call` site, `run_stmt`, the unit exit paths):
a new protocol whose omission is invisible on the happy path (a stale `len()`, a lost element after an exit). I cannot
argue that convincingly here, so that part is STOPPED. What is done instead: `rt::push` takes a lean path (`lean_push`)
for a packed body whose packing has exactly the pushed type: same `Rc::make_mut`, same `Packed` append that
`VecBody::push_value` -> `Packed::put` performs, same `cache.take()`, no `Value` boxing, no `with_slot`/`poke` closures,
the cell through the hit table. Everything else (unpacked, empty `Values`, another type, a value the packing cannot
hold, a local not reachable through plain references) goes to the existing code unchanged. The call still clears every
view and row view FIRST, before touching the vector (a push may move packed storage), so views are invalidated exactly
as before and an OSR exit or deopt sees the real `Vec` (it was mutated by the helper before returning; there is no
state held in the unit). Capacity growth is `Vec::push`'s own reallocation inside the helper: nothing outside holds a
pointer to the old buffer because all views were cleared before it.

## Part B: `flags[i]` read through a READ view, inline in machine code
`Ctx::views` (`View{key, ptr, len}`, by `slot % 4`) already lets machine code STORE into a packed vector without a call;
`index_set` fills it after `Rc::make_mut` (so the storage is unique). I add a read-only flavour:
* Built by `index_get` after a successful direct packed read, only when the body is `Packed` with exactly the wanted
  element type (bool / i64 / float: `view_elem`), AND uniquely owned (`Rc::strong_count == 1 && weak_count == 0`).
  Key = `view_key(slot, type) | READ_VIEW`. A store view of the same slot/type already in the bucket is kept (it is
  unique and written through, so it serves reads).
* Inline read (`view_load`): `(key & !READ_VIEW) == view_key(slot, type)` AND `(unsigned) i < len`; else call the
  existing `index_get` (which fails an out-of-bounds index with nothing changed, so the walker's panic comes from the
  closure engine). A negative index is a huge unsigned: miss. Element kind is part of the key: a bool read never hits an
  int view; a width other than the packing's never gets a view (the helper builds it only for the packing's own type).
* A STORE compares the key EXACTLY, so a read view (bit set) never serves `view_store`; the store goes to `index_set`,
  which makes the body unique (`make_mut`) and replaces the view with a store view. A read view is only built on unique
  storage; the flag is a second belt: a store through a view must never skip `make_mut`.

### What can invalidate a view, and who clears it (unchanged set, now also covering reads)
1. `push` clears all views before touching storage (storage may move).
2. `run_stmt` (any closure-run statement: clone, pop, truncate, clear, reassign, a call that mutates through `&mut`, a
   rebinding of a reference) clears all views and the resolved cells.
3. `index_set` that is not served in place (unpack, non-packing store) clears all views.
4. Stores through a view/`index_set` change contents in place: a read view of the SAME storage sees them (no copy).
5. Sharing: a read view exists only on unique storage; the only way to make it shared is a clone/`Rc` copy, and every
   such operation is a closure-run statement (2). Lowered code never clones a vector.
6. Two names for one vector (reference locals): all are the same cell/Rc. A store through the other name is in place
   (unique) so the first name's read view sees it. The copy-on-write case (store when shared) cannot arise with a read
   view alive because it requires sharing (5). Debug mutant `readviewshared` (build the view on a shared body) is killed
   by jit_vec_view.wj's `cow` case.
7. Bucket collisions (`slot % 4`) between two vectors, or a bool vector and an int vector: the key contains the slot and
   the type, so a mismatch is a miss (the helper runs, as today).
8. The unit's exit/OSR deopt: `Ctx` (and its views) die with the unit run; the closure engine then reads the real `Vec`.
   Reads change nothing, so there is no state to write back.
9. Row views are separate (`clear_rows`) and unaffected.

Memory-model note: the pointer is taken from the body while it is borrowed shared and used after the borrow ended; the
existing store views already do this (pointer from `as_mut_ptr`, used by machine code across other borrows) and are only
ever used while nothing else touches the body (the clears above). Same discipline, no new access pattern.

## local_cell once per run
Already done by the hit table in perf-jit-map-lean (an index in `Ctx::cell_hit`, cleared with `cells`); the vector
helpers `index_get`/`index_set` reach it through `with_cell`. After this branch the sieve's inner loop no longer calls
`index_get` at all in steady state, so `local_cell` is called ~once per outer iteration (misses of the view) instead of
per element.

## Tests (tests/engine_parity/jit_vec_view.wj, fast_paths counters `packedread`, `packed`)
sieve; growth by push with reads between; bool+int+float vectors in one loop (bucket/kind keys); a vector mutated by a
call in the loop; a clone taken mid-loop with stores through a reference (`cow`); i32 packing; a Vec<string> (unpacked);
index past the end (panic last). Debug mutants on the corpus gate: `readviewshared`, `viewreadidx` (read at 0).

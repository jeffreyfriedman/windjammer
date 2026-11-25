---
status: current
last_verified: 5457283f4156
---
# perf-jit-map-lean: soundness note (written before the change)

Scope: `rt::map_get` / `rt::map_insert` (src/interpreter/resolved/jit/rt.rs), called from machine code for
`match m.get(&k)` and `m.insert(k, x)` on a map local with an integer key (lower_map.rs). Measured cost at 10x
(see the measurement files on pc-results): the hash work is a small part of each call; the rest is `with_slot` + `peek`/`poke` closures that are not
inlined, `Value` -> bits through `kinds::bits`, `Value` drop glue on the replaced value.

## Design: a per-call shape guard, not an entry-time cache
The task suggests a map-shape guard at loop/OSR entry. I do NOT cache anything across calls. The lean path
re-reads the local's CONTENT on EVERY call and takes the lean branch only when the local is a `Value::Map` held directly
or reached through plain references (`local_cell`: the cell of a `Ref` chain with no projection and no lock guard, resolved
once per unit run and dropped before every closure-run statement, exactly the mechanism Vec accesses already use).
Measured: the benchmark's `m` is such a reference-held local, so a direct-only guard never fired. Any other shape
(projection, guard, `Rc`, a non-map) goes to the existing helper, kept verbatim as the fallback (`map_get_general`,
`map_insert_general`). The guard costs one discriminant compare; the saving comes from removing the
closure/boxing machinery, not from skipping the check, so an entry-time cache buys nothing and would need an
invalidation argument. Because nothing is cached, the questions below have the same answer as for the current helpers.

## What can change the map while machine code runs, and why each is seen
1. Another statement of the same loop mutating the map through an alias (a `&mut HashMap` param passed to a call, a
   reference local): the machine-code loop cannot lower calls to user fns, and such statements run in the closure engine
   via `run_stmt`. After any closure statement the next helper call reads rstack afresh; a map reached through a
   reference is not "direct", so it takes the slow path (as before). A map the local holds directly cannot be aliased
   mutably (the only aliases of a direct local are `Rc` clones, which make `Rc::make_mut` copy: the insert helper keeps
   `Rc::make_mut`, so copy-on-write and the walker's value semantics are unchanged).
2. The map replaced (`m = HashMap::new()`), cleared, removed from, grown: all happen in closure statements or inside
   `KMap` itself; the lean helper holds no pointer, slot index or table reference between calls, so it sees the
   replacement; the KMap representation changes (Dense <-> Table <-> Tree, rehash, resize, `condense`) stay
   internal to `KMap::get_int/set_int/insert`, which are the same functions the old helpers called.
3. Storage moves during the helper itself: `set_int` may resize/convert; no reference into the map is held across
   it (the value is built first, `Rc::make_mut` first, the borrow of `km` ends with the call).
4. The unit's classification of the local (Class::Map(elem), made at OSR entry by `map_elem`) is unchanged; the value
   type is still checked per call: `get` returns the bits only when the stored value is exactly `want` (width equal,
   representable), else it FAILS with nothing changed and the closure engine reruns the statement (mixed widths test).

## local_cell hit table (the only cached state, and its invalidation)
`Ctx::cell_hit[slot % 8] = (slot+1, index into cells | NO_CELL)` answers `local_cell` without scanning `cells`. It is a
pure index into `cells`, filled only from `cells`, and cleared in the same statement as `cells.clear()` (run_stmt, the only
place a reference can be rebound); the debug mutant `cellhitstale` (keep the table) is killed by vp_ref_rebind.wj. The
cell's content (the KMap) is re-read on every call.

## Semantics that must be (and are) unchanged
* iteration order and hashing: KMap untouched except an allocation-free in-place store (below); the helpers still call
  `get_int` / `set_int`; `--hash-seed` ordering sorts at iteration, independent of insertion path.
* missing key: found=0, bits 0, no failure. Duplicate key: replaces the value in place, keeps the key (BTreeMap::insert rule).
* overflow: no arithmetic in the helper (keys via `int_value`, values via `bits`/`value`); overflow checks live in lowered
  arithmetic, unchanged.
* failure protocol: any refusal sets `Ctx::fail = 1` before changing anything; `found` is written on every get.
* `map_insert` order of effects: value type decoded before the map is touched (a bad type code fails with no change).

## In-place store in `KMap::set_int` (the only change outside rt.rs)
When the slot holds `Value::Int` and the new value is `Value::Int`, overwrite both fields instead of drop-and-move
(skips the `Value` drop glue). Identical observable result: same variant, same key kept, `len` unchanged.

## Fuse get + insert on the same key
Decided after the lean helpers are measured; it needs the arm expressions between the two calls to be pure and the map
not mutated between them. If the argument is not clean it is not done (the report is on pc-results).

## Fuse get + insert: NOT DONE
The fusion needs the value expression between the two calls (`c + 1`, arms) to be pure AND to hold the slot/entry across
the arm; with the map cell re-borrowed per call a retained entry reference would be the first cached map state, and a
same-key proof would have to cover aliasing through `&mut` params and Rc COW. Not argued convincingly in this branch: stopped.

## Tests
engine_parity/jit_map_lean.wj: non-int key map (string, float key-in-loop), map behind `&mut` param in the loop,
map mutated by a closure-run statement mid-loop (insert/remove/clear), map replaced mid-loop, missing key, duplicate
key, mixed value widths, i64 extremes, an `Rc`-shared (cloned) map, iteration after. fast_paths counter `mapleanget` /
`mapleanins` assert the lean branch is taken on the int/int program and NOT on the string/&mut ones.
Real mutations per hunk, shown failing.

---
status: current
last_verified: 5457283f4156
---
# SOUNDNESS-hoist: hoisted row views by loop versioning (L1)

Branch `perf-jit-hoist-rows` (base cc-file-transfer/wt-rc3-int 578e40a79). Files: `jit/lower.rs` (analysis, versioning),
`jit/lower_rows.rs` (hoisted access), `jit/lower_expr.rs` (index variable hint), `jit/rt.rs`, `jit/osr.rs` (counters).

## 0. Shape of the change

`lower` runs the existing lowering once (pass A, "analyze"), unchanged in output, and records per `while`/`for` statement
(keyed by the address of its body): `safe` (see 1.4), the rows `(slot, shape)` it accesses through views, the
index-variable candidates (row, variable name, variable walker slot), and the walker slots of every variable DEFINED
inside it (`def`, every `def_var` of a program variable). If some safe loop accesses rows, pass B lowers the unit again
into a fresh function: at such a loop (the outermost one on the way down) it emits

    preheader: build missing views, check every key  ->  all hold: FAST copy of the loop    (counter hoisted)
                                                      -> else:     SLOW copy = today's lowering, unchanged (hoistfail)
    both copies jump to one join block

If pass B fails (`None`/`Err`, or a disqualifying call in the fast copy) pass A's function is used: no behavior change.

## 1. EXACTLY what is hoisted

### 1.1 Per versioned loop L, per row (slot, shape idx) in rows(L), at L's preheader
- `key` check: `ctx.rows[slot % ROWS].key == row_key(slot, idx)` after calling `row_build` if it was not (the build is
  the existing helper with its credit rule; it never fails the unit). All rows are checked AFTER all builds (a later
  build can evict an earlier one that shares `slot % ROWS`): any miss -> the slow copy.
- loaded once into SSA values: `len` and `tbl` (the view's table address). These are VALUES computed at the preheader,
  which dominates the whole fast copy; they are never stored in a variable the loop can change.
Not hoisted: field VALUES (no float or int payload is ever kept in a register across a store), the vec, or element Rc state.

### 1.2 Per loop (L itself, and every nested loop in the fast copy), per candidate (row, index variable v)
`fields_v = *(tbl + 8*v)` (the field-array ADDRESS of element v) and `bad_v = !(v <u len)`, computed at that loop's
preheader (branch: when `bad_v`, `fields_v` is 0 and is never dereferenced). Only when ALL hold:
1. the access's index expression is an identifier bound to a loop-unit variable (`Bind::Var`), recorded at lowering
   as the variable id (`Target::IndexField` carries it; any other index expression -> the general fast access);
2. the variable's walker slot is not in `defs(L)`: no assignment, `let`, `for` binding, compound assignment or struct
   rebuild defines a variable with that slot anywhere inside L (this is how the lowering knows the index is
   invariant: an induction variable of an OUTER loop, or a variable only assigned before L; the variable of L's own
   counter or `j += 1` is in `defs(L)` and is therefore never hoisted for L). Slots are compared, not SSA values:
   Cranelift's phi simplification is not final until the loop head is sealed, so SSA identity cannot be tested;
3. at L's preheader `lookup(name)` finds a `Bind::Var` with exactly that slot (a shadowing `let` in the body has another
   slot/var id: its accesses fall back to the general fast access);
4. at the access, the variable id equals the id the entry was made for.
`bad_v` is tested at EVERY access (`bail_if`, the statement's ordinary exit): an out-of-range `v` never errors at the
preheader (the loop may run zero iterations: `while j < n` with `n = 0` must not raise), only at the statement that
indexes it, as today.

### 1.3 Per access in the fast copy (all rows, hoisted index or not)
`oob = !(i <u len)` -> `bail_if(oob)` (the statement's exit; the closure engine re-runs the statement and raises the
walker's own error: the old slow path called a helper that failed, which led to the same exit); then
`addr = *(tbl + 8*i) + off` (or `fields_v + off` for a hoisted index) and the payload load/store. No key compare,
no `ROW_HITS` store per access (the preheader adds the row count to `row_hits`, so `rowview` is still counted).
There is NO slow path in the fast copy: no `row_build`, no helper that stores, so no register is spilled around a call there.

### 1.4 What disqualifies a loop (pass A `safe = false`)
Any `Lo::call` of a helper not in SAFE = {index_get, index_field_get, field_get, len, map_get, int_op, fmod, round}
while the loop is active (the call is emitted anywhere in its body, including nested loops and inlined calls), i.e.
`run_stmt` (a closure-run statement: it clears all views and may share/replace anything), `push`, `index_set`,
`index_field_set`, `field_set`, `map_insert`. The slow closures INSIDE `row_access` are exempt (they do not exist in the
fast copy) -- but the packed-vector `view_store` slow path calls `index_set`, which clears rows, so a loop with a
packed `v[i] = x` store is not safe (conservative). The fast copy asserts the same list at every `call` and, on a
violation, abandons pass B (A's function is kept). `row_build` itself is only emitted in preheaders.
Why SAFE is safe: these helpers read (`with_cell`/`with_slot` peek, `lean_map`) and never take a struct's `Rc`
mutably or clone it into a longer-lived value (F14); `index_get` may fill a packed READ view of a different vector
(a `Vec<f64>` body is not a `Vec<struct>` body) which is not a row view.

## 2. Aliasing of rows i and j (stores in the loop)
The hoisted items are ADDRESSES of field arrays (and the validated table), never values. A fast store writes the payload
in place through the same address a later load reads, in program order: `bodies[j].vx += ..` while the `i` row's
`fields_i` is hoisted is a memory operation on the real storage, so `j == i` (or two aliases of one element) reads
the value just stored, exactly as the walker's in-place store does. Nothing is reloaded or checked because nothing
is cached. Field arrays do not move while the view is valid: a store of a payload does not reallocate
(`s.fields` is a `Vec` whose length never changes in a view-stable loop); a whole-struct replacement
(`bodies[i] = ..`: `index_set`), a `let b = bodies[i]` clone (closure-run statement) and any retag-capable helper
are disqualified by 1.4 (the loop is not versioned; the slow loop runs, validating per access).
nbody: the pair loop has j > i (`j = i + 1`), so rows i and j are different elements, but the argument above does not
need that.

## 3. Entry guard and fallback
Guard: all keys valid after builds. Fallback: the existing lowering of the same statement (its exits, its resume paths
are the same `Level`s, since both copies are lowered at the same `path`/`stmt_exit`). Failure causes: shared element
(`let b = v[0]` earlier), element of another shape, credit exhausted, two rows mapping to one view slot. The guard
reads memory the builds just wrote, after all builds, immediately before the branch; nothing runs between the guard
and the first iteration except register arithmetic.

## 4. Exits and deopt
Every exit of the fast copy has the same `Exit{path, spills}` as the slow copy would have (same scopes at the same
statements); hoisted values are NOT spill entries (they are pointers/lengths, not walker variables) and are
re-derived (views rebuilt) when machine code is entered again. `osr::level` resumes the closure engine at the
statement; the closure engine's code may mutate anything because the machine code that holds the hoisted values has
already returned. A trap in iteration k resumes in iteration k exactly as before (the fast copy commits the same
stores in the same order).

## 5. Interaction
- fnunit / loop OSR units: versioning is inside the unit's lowered code (statement loops only; a unit's OWN
  loop is not versioned: its entry is the unit). Every unit run starts with fresh `Ctx::rows`; hoisted values live in
  registers/stack of one run of machine code, which never crosses a closure-run statement in a fast copy.
- Hot reload / redefinition: units are keyed and dropped as before (unchanged); the plan's `row_shapes` of pass B
  belong to the unit.
- Recursion: a recursive call is a closure-engine call; the callee unit has its own `Ctx`. The caller's fast copy has
  no call-out (`call_unit` is not in SAFE).
- Interleaving through references: a view is keyed by local slot and built through `with_value_mut` following
  references exactly as before; the only new fact is "between the preheader check and any use no helper that clears
  runs", established by 1.4 and by the absence of a `row_build` in the fast copy.

## 6. Stopped / restricted
- Scalar replacement of VALUES (the ladder's L4 `i` row in registers) is NOT done: addresses only.
- A unit's own loop is not versioned.

## 7. Why `index_set` is not in the safe list, and why listing it would change nothing observable (mutant M4)
`index_set` is not safe by construction, so the disqualification of such a loop is conservative, not required. Proof that
treating it as safe is behavior-equivalent (a hoisted loop containing a call reaching it stays correct):
1. Which stores reach the helper natively. A whole-element store (`bs[i] = Body{..}`, any struct/Vec/String element) has no
   native lowering (`store` handles only scalar `Ty`): it is a closure-run statement, i.e. `run_stmt`, which stays
   disqualified (M1). The only natives calling `index_set` are (a) `v[i] = x` for a scalar element `x` that is not a
   packed-view type (`Vec<i32>`, `Vec<u8>` ...: `view_elem` is `None`), and (b) the slow path of a packed `i64`/`f64`/`bool`
   store (view miss). Their target is a local whose class at entry is `Vec(Scalar)`; a row vec has class `Vec(Struct)`, and
   one vector value has one class, so the written vec is never a row vec (its slot, its `Rc`, its storage).
2. What `index_set` does to row state: `clear_rows` zeroes the `key` of each `RowView`; `row_tbls` stay allocated (the doc
   of `clear_rows`), and the tables hold addresses of struct field arrays, which a scalar-vec store does not touch (it may
   `Rc::make_mut` or repack ITS OWN vec only). The fast copy never reads `ctx.rows` after the preheader (hoisted `len`,
   `tbl` are SSA values; no key compare, no `row_build` there), so zeroed keys are unobserved inside the loop; after the
   loop the keys are zero exactly as in the slow copy, and the next versioned loop or row access rebuilds or checks them.
3. A failing store (out of range, wrong element type) raises the helper's fail flag: the statement's `Exit` is the same in
   both copies, and the closure engine re-runs the statement and raises the walker's error (4).
4. `views` (packed read/store views) are cleared or replaced by the helper in both copies identically; they are a different
   table from `rows` and the fast copy's row accesses do not use them.
Hence the mutant changes only the `hoisted`/`hoistfail` counters: confirmed by running it (release build, diff on
pc-results) against engine_parity (incl. the new `jit_hoist_scalar_store.wj`, whose loops contain
`Vec<i32>`, `Vec<u8>` and packed `Vec<f64>` stores beside row accesses and become hoisted under the mutant: hoisted fires under the
mutant and not otherwise), fast_paths, hoist_traps, jit_loops, packed_vec: all pass, output identical to the walker. The earlier sentence "a
whole-element store is never native" in the original hoist report is right for struct elements; the scalar stores above are the
real route into the helper.

## 8. Mutant M3 (skip the preheader entry guard), re-run against the current tests
KILLED BY OUTPUT (release mutant, diff on pc-results: the guard value replaced by constant true):
`engine_parity` jit_fnunit_views.wj (a closure-run `let b = bodies[0]` shares the element and clears the keys; the
rebuild fails; the stale view writes through into the shared element) and the new `jit_hoist_slot_collide2.wj` (two vecs of
different contents on one view slot: the stale `a` view reads and writes `c`'s elements). `jit_hoist_pairs`' own collide
case and `jit_hoist_disqualified` do NOT distinguish the mutant by output (identical walker output, the counter gate
`hoistfail` kills it there). Note: with `.len()` in the loop conditions (pairs' collide) the output stayed equal; the
collide2 loops use a plain count.

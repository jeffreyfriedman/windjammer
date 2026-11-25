---
status: proposal
last_verified: dae10c39a885
---
# Struct loops in machine-code units: `for p in v`, `let p = v[i]`, nested fields

> **Proposal: not implemented.** Machine code for three loop shapes that real game code uses and that units do not
> run today. File and line references are to `wt-rc3-int` dae10c39a. Measured numbers are quoted from pc-results
> measurement files, named at each use: `findings/interp-e1-e6-game-shapes.md` (E3 and the probe table: Ir per
> iteration and per update), `findings/interp-logic-top20.md` (real logic modules) and `findings/interp-f32-probes.md`.
> Numbers marked *assumed* are not measured. Companion designs: [interp-struct-vec-loops.md](interp-struct-vec-loops.md),
> [interp-call-unit.md](interp-call-unit.md), and the f32 and inline-callee proposals (`interp-f32-units.md` on branch
> `design-f32-units`, `interp-inline-callee.md` on branch `design-inline-callee`; neither is on this branch yet).

## 0. Executive summary (one screen)

**Verdict: sound as designed for all three shapes.** One part is argued less firmly than the rest, and is named here.
Every new access reads or writes through the row views that exist today (`jit/rows.rs`). The one new protocol is a
**pin**: a reference-counted snapshot of the vector that a `for p in v` loop iterates. It is the same `Rc` the
closure engine's iterator already holds, so it adds no new staleness rule. Nothing in this design flushes,
writes back lazily, or keeps a side copy of vector data. The flattened side-table option (copy in at entry, write back
at exit) is **rejected** (section 2.4).

**First branch: N1, `let p = v[i]` by scalar replacement** (section 3). The binding loads every field of row `i`
through the row view into one machine variable per field, as the existing struct locals do (`lower_struct.rs`). A
whole-element store `v[i] = p` writes the fields back in place. Row views also learn `bool` fields.
- Expected effect: probe "`while` + `let p = v[i]`, then read fields" from **3,596 Ir per iteration to about 60**
  (the native `v[i].x` probe is 61; both in `findings/interp-e1-e6-game-shapes.md`, probe table).
- Gate: at most 100 Ir per iteration (*assumed* budget), counter `rowlet` fires, parity programs and mutants green.
- **Stop if** it is above 150 Ir per iteration. Profile before N2, because N2 binds its loop variable with this code.
- Why first: it has the smallest surface (one lowering form plus one store form, no OSR change), it is width-agnostic
  (so it runs in parallel with F1), and N2, N3 and N4 all bind their element variable with it.

**The plan after N1:** N2 `for p in v` (snapshot form, pinned read view) -> N3 nested fields (two-level row table)
-> N4 `for mut p in v` (write-back form). N3's real-code gate needs F2 (flat f32 row views). N4 turns the
`b.update(..)` call of the real code from an element-place call (needs inline stage S3) into a struct-local call
(stage S2 is enough). Section 8 has the order, gates and stop criteria. Section 9 lists the E3 rows each stage retires.

**What is not argued firmly (read first):**
1. **Pin soundness rests on an audit, not on a proof** (section 5.2). A pinned body is read through a view **without**
   a uniqueness check. This holds only if every in-place writer of a `VecBody` or `StructVal` reaches it through
   `Rc::get_mut`/`Rc::make_mut` (or through a view that checked uniqueness). F14 of
   [interp-struct-vec-loops.md](interp-struct-vec-loops.md) is the same kind of assumption, and it is still open. N0
   performs this audit (item A2). If the audit finds a writer that bypasses copy-on-write, N2 falls back to option N2':
   uniqueness-checked views over `v` itself, legal only when the loop provably cannot write `v`
   (section 5.3). That costs coverage, not soundness.
2. **Nested uniqueness is checked at build time, element by element.** A vector built from one shared `Vec2` local
   (`Body { pos: origin, vel: origin }`) never gets a nested view until copy-on-write has unshared every element.
   This is a performance hazard, not a soundness hazard: the credit rule in `rows.rs` bounds the rescans. Whether
   real game code hits it is not measured (N0 item M3).
3. **The nested update cost today is not measured.** The probe `n1.wj` (below) shows the verdict: refused, "3 of its
   4 statements run in the closures". The Ir figure needs a callgrind run (N0 item M1). This design was written
   without valgrind, so the box stayed free for the measuring jobs.
4. **`for x in v` over a `let mut` vector sometimes runs as a walker island in the closure engine** (probes `n2b`,
   `n2d`, `n2h`: `[fallback] island` rises by one per execution of the `for` statement). Over an immutable local it
   does not (`n2i`). The cause is not located (N0 item A1). N2's OSR site lives in the closure engine's `for`, so N2
   inherits this gap until it is fixed.

Probe programs used here (verdicts only, `WJ_JIT_DEBUG=1`, binary `~/own/bin/wj-e1`): `~/own/perf/nested-variants/`
(`n1.wj` nested update, `n2*.wj` for-in and `let p = v[i]`, `n3.wj` and `n4.wj` iterator invalidation).

## 1. What the interpreter does today (verified in the code at dae10c39a)

| # | Fact | Where |
|---|---|---|
| T1 | Units take only `while` and **range** `for` as roots and nested loops. `Root::Loop(Statement::For ..)` requires an integer counter (`no counter type` otherwise). A `for` over anything else hits `_ => return None` in `Lo::native`, so the statement runs in the closures. The loop itself is never an OSR site | `jit/lower.rs` (`lower`, `native`, `unit_for`, `for_stmt`) |
| T2 | The closure engine's `for` has an OSR site only on its range path (`range` -> `osr::enter_for`). A vector is iterated by `iter::as_iter` -> `IterState::from_rc`. The iterator **holds an `Rc` clone of the body** and fixes `end = len` at loop start. Each step yields `items.get_value(p)`, an `Rc` clone of the element | `resolved/forloop.rs`, `interpreter/iter.rs` (`as_iter`, `from_rc`, `advance`) |
| T3 | So `for p in v` iterates a **snapshot**: pushes inside the body do not add iterations, and stores to `v[j]` inside the body are not seen by later iterations. The store copies the body on write, because the iterator shares it. Probe `n4.wj` prints `in 1`, `in 2` after `ps[1].x = 50.0` and two pushes in the first iteration. Walker and resolved agree | probe `n4.wj` |
| T4 | **Write-back form.** When the iterable is a place and the body mutates the loop variable (`analysis::loop_var_mutated`), the closure engine declines (`for_stmt` returns `None`) and the walker runs it: `n = len` at start; per iteration the element is **re-read** through its place (`place.proj(Index(i))`); after the body, unless it ended with an error, the variable's current value is **written back** to that element. That includes `break`, `continue` and `return`. Probe `n4.wj`: a store `ps[1].x = 1000.0` in iteration 0 is seen by iteration 1, whose `+ 1.0` gives 1001 | `eval_stmt.rs` (`exec_for`, `run_iteration_wb`, `rebuild_element`) |
| T5 | `let p = v[i]` is not struct-valued for the lowering (`struct_valued`: literals, struct locals and method calls only), so it is a closure-run statement: an `Rc` clone of the element (F3 of the struct-vec design). Every later `p.f` is closure-run too. The probe verdict is "2 of its 3 statements run in the closures" | `jit/lower_struct.rs` (`struct_valued`, `vexpr`), `findings/interp-e1-e6-game-shapes.md` |
| T6 | Row views cover `v[i].f` only, with `v` a local handle (`Target::IndexField`). `v[i].pos.x` has a `FieldAccess` whose object is not an `Index`, so it is not a target. A row view admits only `f64`/`i64` fields (`row_field`). A nested struct field has type `None` in the `Shape` and is not checked by `fill` | `jit/lower_expr.rs` (targets), `jit/rows.rs`, `jit/kinds.rs` (`Shape`, `ty_of`) |
| T7 | A nested struct is `Value::Struct(Rc<StructVal>)` inside the parent's `fields: Vec<(Rc<str>, Value)>`. A nested read from the parent's field array is three dependent loads: the `Rc` pointer, then the `StructVal`'s `fields` pointer, then the payload | `interpreter/value.rs` (`StructVal`) |
| T8 | Physical sharing is not observable. `Rc::strong_count` and `impl Drop` are answered by reachability from variable slots (`heap.rs`), and Rust-side temporaries such as an iterator's `Rc` are invisible to it. So a pin held in `Ctx` is observationally the closure engine's iterator | `interpreter/heap.rs` (module doc) |
| T9 | The compiled build: `wj build --no-cargo` emits `for p in ps.iter().copied()` for a read-only loop over a `Copy` struct vector, `for q in &mut ps` for the write-back form, and `let p = ps[k]` as a copy. A body that pushes to `ps` inside `for p in ps` transpiles without a Windjammer diagnostic (probe `n3.wj`), and rustc's borrow checker rejects it (mutable borrow while `iter()` borrows). So such programs have **no compiled meaning**, and the interpreter's snapshot is the only defined behaviour | probes `n2.wj`, `n3.wj` (`~/own/perf/nested-variants/b2`, `b3`) |

## 2. Nested fields: the two-level row view (N3)

### 2.1 Representation

A view today is `RowView { key, len, tbl }` with `tbl[i]` = the address of element `i`'s field array. N3 widens the
table to a fixed **stride** `S = 1 + P` per element, where `P` is the number of nested struct paths the unit
actually uses for this view (not every nested field of the type):

```text
tbl[i*S + 0]      = element i's field array            (as today)
tbl[i*S + 1 + j]  = field array of element i's nested path j   (e.g. j=0: .pos, j=1: .vel)
```

A nested leaf access `v[i].pos.x` lowers to the address `tbl[i*S + 1 + j] + hint_x * stride + payload_off`: **two
loads, the same as a flat access** (table entry, payload). The plan's row shape (`row_shapes`) records the path list,
so the view key `row_key(slot, idx)` already distinguishes two units that use different paths. Paths may be one or two
levels deep (`e.pos.x`, `e.xf.pos.x`). Deeper paths are refused in v1 (no measured demand).

### 2.2 Shape tables

`Shape.fields: Vec<(Rc<str>, Option<Ty>)>` gains a third case for a struct-valued field:
`FieldTy::{Scalar(Ty), Struct(Box<Shape>), Other}`, built by `shape_of` from the first element (as today) and
compared structurally. A nested shape lists its scalar leaf types like a top-level one, so it inherits F2's `F32`
admission without extra work when F2 lands first.

### 2.3 Build and validity

`fill` additionally runs, for each element and each used path `j`, the walk `fields[hint].1` ->
`Value::Struct(rc)` -> `Rc::get_mut(rc)` -> same name, same field count and names, `ty_is` per leaf, `uid == 0`. It then
pushes `fields.as_mut_ptr()`. Any failure fails the whole build, as today. This is the uniqueness requirement
applied **to every nested `Rc` on a used path**: a store in place through the view is then the walker's copy-on-write
store with nothing to copy, the flat argument of `rows.rs` applied one level down.

Invalidation events: exactly today's (F7 of the struct-vec design: every closure-run statement, `push`, and every
helper store clears all views). There are three new obligations, each enforced at lowering:
- A native store that **replaces** a nested struct (`v[i].pos = Vec2 { .. }`, `v[i].pos = q`) is lowered as
  per-leaf stores in place, never as an `Rc` replacement. The value is scalar-replaced (a literal or struct local of
  the same shape). Otherwise it is a helper store, which clears the views.
- A native read of a whole nested struct (`let q = v[i].pos`) is a **copy** into machine variables (section 3), never
  an `Rc` clone. If it were an `Rc` clone, the nested count would be 2 while the view lives, and a later in-place
  store would leak into `q`.
- F14 extended (audit A3): no helper callable while a view is live may leave an `Rc` clone of an element **or of a
  nested struct** in a slot or anywhere else that outlives the helper.

Cost: table memory `8 * n * S` bytes per view, recycled through `Unit::spare` like today's tables (for a 3-path body,
32 bytes per element). Build time O(n * S). The credit rule already charges `tbl.len()`, so a vector whose views
keep failing scans at most a constant factor of the helper work it does. The rescan hazard is item 2 of section 0.

### 2.4 Alternatives considered

- **Flattened side table at unit entry, written back at exit (rejected).** This would copy every used nested scalar
  to a flat buffer and store it back at exit. While the copy is live the vector is stale, so every helper call that can
  observe `v` (`len`, `index_get`, any call, `run_stmt`, every exit, every deopt) needs a flush first. That is the
  pending-length protocol of [interp-soundness/pushlen.md](interp-soundness/pushlen.md) widened from one length word to
  all fields of all elements. Omitting a flush is invisible on the happy path; this is exactly the class of protocol
  [interp-soundness/vecinline.md](interp-soundness/vecinline.md) stopped once. It also costs O(n) at entry **and** at
  exit even for a loop that touches a few elements, and it double-books memory. Option A has no stale state at all.
- **Pointer chase without a table (rejected for v1).** Three dependent loads per access (T7), and a per-access
  check of the nested `Rc` count would be needed because nothing proves uniqueness. It is slower and no simpler.
- **Packed rows with nested structs flattened by declaration (future).** Step 4 of
  [interp-struct-vec-loops.md](interp-struct-vec-loops.md) (`Packed::Rows`, not built) extended so that a nested
  all-scalar, `Drop`-free struct field is laid out inline in the row stride. Elements and nested structs then have
  no `Rc`, the uniqueness problem disappears, and a build costs O(1). This is the right end state. N3's lowering keeps
  (path -> leaf) as its unit of access, so moving to packed rows changes only the address computation.

## 3. `let p = v[i]`: copy, not borrow (N1)

**Decision: always a copy, by scalar replacement.** `let p = v[i]` (no type annotation, `v` a local of class
`Vec(Elem::Struct(shape))`) lowers to one row-view load per field into the variables of a `VStruct`, the existing
struct-local representation (`bind_struct`). Fields the view cannot serve fall back to `index_field_get`, field by
field. The view is built by `row_build` after the first slow access, as today.

Why copy: Windjammer `let` of a struct is a by-value copy (F1 of the struct-vec design: copy-on-write `Rc` trees).
In the interpreter, `p` must not see later stores to `v[i]`, and `v[i]` must not see stores to `p`. A copy of K
scalars through a live view costs K loads, and a borrow would read the same K payloads on use. The only saving a
borrow offers is the loads of fields the body never reads. Getting it needs a proof that no store to row `i` (native,
helper or closure-run) happens between the `let` and every use. Copying needs no such proof. The cases:

| use of `p` | lowering | notes |
|---|---|---|
| only read (`p.x`) | the copied variables | the K loads happen at the `let`. Cranelift does not drop unused `trusted` loads, so a struct with many unused fields pays for them (*assumed* small: K is under 10 in the measured shapes) |
| written (`p.x = ..`), no write-back | the copied variables change; `v[i]` is untouched | exactly the walker: `p` was an `Rc` clone, and its first store copies on write |
| written then stored back (`v[i] = p`, same `i` or not) | **N1 store form**: per-field stores in place through the row view (`row_store`), helper `index_set` otherwise | equal to the walker's element replacement because the view proved the element unique, same type name, same field types, `uid == 0` |
| escapes (passed to a non-inlined call, pushed, returned, captured, used by a closure-run statement) | the existing struct spill (`struct_spill`) builds a fresh `StructVal` into `p`'s slot at that point | a fresh value is observationally the walker's shared one (T8). Its cost is one allocation per escape (*assumed* acceptable: the walker allocates on every write today) |

Refused in N1 (the statement stays closure-run): a struct with a field that is neither scalar nor (after N3) a
nested viewed struct, such as `String` or `Vec`. Also refused: an index that is not an integer, and a typed `let`
(`let p: Body = v[i]`, as the existing rule for typed lets of calls). An out-of-bounds `i` takes the helper, which
raises the walker's error at the same statement. Parity program `jit_let_row_oob.wj` pins the message and location.

Today's hazard goes away as a side effect. While a closure-run `let b = bodies[i]` held its `Rc` clone, `rows::fill`
failed on that element and rescanned on every miss (F3 of the struct-vec design). N1 never takes the clone.

## 4. `for p in v` and `for mut p in v`

### 4.1 Semantics the unit must reproduce

Two forms, chosen by the **same predicate** the walker uses (`is_place_expr(iterable) && loop_var_mutated(..)`,
called at lowering so that the two can never disagree):

- **Snapshot form** (T2, T3): `{ pin = v's body; n = pin.len(); for k in 0..n { let p = pin[k]; body } }`. Stores to `v`
  in the body copy `v` on write (the pin shares the body), and pushes do not change `n`.
- **Write-back form** (T4): `{ n = v.len(); for k in 0..n { let mut p = v[k]; body; if !error { v[k] = p } } }`.
  The element is re-read every iteration, `n` is fixed, and the write-back happens on `break`/`continue`/`return` too.

Both desugarings are exact for every body, including bodies that mutate `v`, because the interpreter's behaviour **is**
the desugaring (T2-T4, probe `n4.wj`). The language side (T9): the compiled build rejects a body that mutates `v`
during `for p in v`. **Recommendation, separate from this design:** an analyzer diagnostic, "`v` is mutated while
`for .. in v` iterates it", so that the interpreter no longer accepts programs the compiler cannot build. It belongs on
the diagnostics list, not in the JIT.

### 4.2 Snapshot form (N2)

- **Closure engine:** the `Value::Vec` path of `forloop.rs` gets an OSR site, `osr::enter_vec_for(it, b, site,
  &iter)`, armed like the range site. The iterator's `Buf { items, pos, end }` is handed to the unit: `items` becomes
  the unit's pin, `pos` its counter and `end` its bound. The unit returns the next position, and the iterator continues
  from there. Nothing is copied: the pin **is** the iterator's `Rc`.
- **Nested in a unit** (a `for p in v` inside a unit's `while`, or in a function unit): the helper `rt::vec_pin(slot)`
  clones the body `Rc` into `Ctx::pins` and returns its index and length. `rt::vec_unpin` drops it on every loop-exit
  edge. A unit exit inside the loop hands the pin to `osr::resume`. Its `iterations` gains a vector variant that runs
  the remaining `k+1..n` from the pin, the way it runs the remaining range iterations from `buf[counter]` today.
- **Element binding:** when the body never writes `p` (the snapshot form only exists when `loop_var_mutated` is false,
  but a non-place iterable can still be mutated through `p`; that case uses the N1 copy), `p` is bound as a **borrowed
  pinned row**, `Bind::Row { pin, k }`: `p.f` loads straight from the snapshot. That is sound **because the snapshot
  cannot change** while pinned (section 5.2), so borrow and copy are indistinguishable. If `p` escapes, the helper
  `pin_get(pin, k)` returns the element's `Rc` clone: what the closure engine binds.
- **Read view on the pin:** `fill_shared` is `fill` without `Rc::get_mut` (shape, names, types, `uid == 0` checked as
  today). It is keyed in its own small array `Ctx::pin_rows`, so it never evicts or aliases a write view of `v`
  (`ROWS` is 2 today and keyed by `slot % ROWS`). Building is O(n) under the credit rule. An early-`break` search
  loop therefore runs its first iterations through `pin_get` rather than paying for a full scan.
- **Packed scalar vectors** (`for x in xs` over `Vec<f64>`): the same pin with a packed read view (`View` with
  `READ_VIEW`), built without the uniqueness check by the same argument.

### 4.3 Write-back form (N4)

- **Closure engine first:** the walker island is replaced by a closure-engine loop, the T4 desugaring with an OSR site
  counting `k`. It is parity-tested against the walker before any machine code exists.
- **Unit:** `p` is a `VStruct` loaded from `v`'s **write** view at iteration start (N1 copy). At the latch, and on
  every `break`/`continue`/`return` edge, the fields of `p` are stored back in place (N1 store form). On an error exit
  nothing is stored, and `v[k]` keeps its value from before the iteration, as in the walker. A non-error exit inside
  the body (a guard) spills `p` into its slot. `resume` finishes the body in the closure engine and then performs that
  iteration's write-back. This needs a new `Level` kind for the write-back loop, the one protocol addition of N4.
- **v1 restriction:** refuse a body that changes `v`'s **length** (push, pop, clear, insert, remove, truncate, or
  assignment of `v`) natively or in a closure-run statement (a syntactic scan with the `loop_var_mutated` machinery
  on the root name). The desugaring reproduces those programs too, but a removal shifts the element the write-back
  targets, and those programs do not compile in the build anyway (T9). Element stores to `v[j]` stay allowed.
- **Why it matters beyond the loop:** with `p` a struct local, the real `for mut b in self.bodies { b.update(dt, ..) }`
  becomes a mutating call on a struct local. That is the inline-callee design's G2 shape (stage **S2**, exit-free),
  not the element-place G3 shape that needs **S3** (section 6).

### 4.4 The other spellings

- `for i in 0..v.len() { let p = v[i]; .. }` is a range loop today. N1 alone makes it native.
- `while i < n { .. v[i].pos.x .. }` is a `while` today. N3 alone makes it native.
- `while i < n { update(v[i], dt) }` / `v[i].update(..)` are element-place calls (E3 a2, a4, a5): stage S3. When S3
  inlines `self.velocity.x = ..` with `self` bound to `v[i]`, the result is `v[i].velocity.x`, an N3 nested path. So
  **N3 is a prerequisite of S3 on the real code**, not an alternative to it.
- An iterable that is a field (`for b in self.bodies`, the real physics world) needs `vec_pin` and `row_build` over a
  field path of a handle. This is **N2b**, after N2 has shipped on locals and parameters.

## 5. Aliasing and uniqueness

### 5.1 The existing protocols, one by one

- **Hoist (L1, [interp-soundness/hoist.md](interp-soundness/hoist.md)).** The for-in counter is a hidden variable
  defined only by the latch, so it is the induction variable. The views of `v` (or of the pin) are built and checked at
  the preheader, and `len`/`tbl` become SSA values. Per iteration, one row base per used path (`tbl[k*S + j]`) feeds
  every `p.f`. The rule for hoisted nested addresses is the flat one, applied per path. `vec_pin` must **not** be in
  `SAFE_CALLS`, because it clears the write views of its slot (next item). So an outer loop that writes `v` and contains
  a `for p in v` takes the slow copy. That is correct and rare.
- **Pin versus write views.** Pinning makes `v`'s body shared. A write view of `v` built earlier in the run would then
  store into the pinned snapshot in place. So `vec_pin` **clears every row view, packed view and push record of
  its slot before cloning**. Later write-view builds on `v` fail `Rc::get_mut` while the pin lives, and stores take the
  copy-on-write helper, which is what T3 requires.
- **Pending-length push ([interp-soundness/pushlen.md](interp-soundness/pushlen.md)).** `vec_pin` reads `v`'s length,
  so it must run after the flush. `Lo::call` flushes before every helper unless a helper is on the exemption list, and
  `vec_pin`/`pin_get` must never be added to it. A push to `v` inside a snapshot loop copies `v` on write (shared with
  the pin) and arms on the new body. The pin's `end` never consults `v`.
- **Read views ([interp-soundness/vecinline.md](interp-soundness/vecinline.md)).** Read views on `v` itself keep the
  uniqueness rule. Only views built on a pin skip it (5.2).
- **`f(v, v)` and aliased parameters ([interp-soundness/fnunit2.md](interp-soundness/fnunit2.md)).** `params_alias`
  still refuses a function unit with aliased mutable parameters for write views. A pin is read-only and immutable, so
  aliasing cannot reach it.
- **`let b = bodies[i]` (strong count 2).** It is native after N1 (a copy), so it no longer blocks views (section 3).
- **Nested `Rc` counts.** These are checked per used path at build (2.3). After N1 and N3 no machine code creates a
  nested clone, and the closure-run statements that can create one clear the views.

### 5.2 Why a pinned body needs no uniqueness check (the argument N0 audits)

Claim: while `Ctx` holds an `Rc` clone of a body `B`, no code changes `B`'s buffer or any field of any element reached
from it. Argument: an in-place write to a `VecBody` or `StructVal` needs `&mut`. The interpreter obtains `&mut` to a
shared tree only through `Rc::make_mut` (which copies when the count is above one) or `Rc::get_mut` (which fails).
The pin keeps `B`'s count at two or more as long as `v` still holds `B`. If `v` was reassigned, the pin is the only
owner, and no code holds a path to `B` except the pin, whose code never writes. An element of `B` with count 1 can be
reached for writing only through `B`, so through a `make_mut` that first copies `B` and so lifts every element's count
to two. Row views check `get_mut` (`rows::fill`), and packed store views are taken after `Rc::make_mut`
(`rt::packed_set`), both read at dae10c39a. The audit, item A2 of N0,
must confirm that **no other** `unsafe` or raw-pointer write exists in `rt.rs`, `vecbody.rs`, `place.rs`, `kmap.rs`,
`builtins/` and `scalar_pipe.rs`. If it does not confirm it, use option N2' (5.3).

Drop timing: the pin is released at the same program point as the closure engine's iterator (loop end or exit
handoff). Reachability ignores both (T8), so destructor timing cannot differ.

### 5.3 Option N2' (only if the audit fails)

No pin. Lowering proves that the body cannot write `v`: no native target on `v`'s slot, no `push` or other mutating
method on it, no helper that takes the slot mutably, and **no closure-run statement at all** in the body. Then the
snapshot equals `v` for the whole loop, and the ordinary uniqueness-checked view of `v` serves it. Everything else
stays closure-run. That loses the loops that write `v` or contain a closure-run statement.

## 6. Interactions and sequencing

- **f32 (F1/F2, `interp-f32-units.md`).** Every real game field is `f32` (E3 reading (3) in
  `findings/interp-e1-e6-game-shapes.md`). Nothing here has a real-code effect before F1. N1 and N2 are width-agnostic
  (they move payload bits through `row_load`/`row_store`), so they run **in parallel with F1**. F2 and N3 edit the same
  code (`row_field`, `row_store_ok`, `ty_is` in `fill`, `Shape`), so **F2 lands before N3**. N3 then admits `F32` leaves
  through the shared check, and needs no separate f32 nested stage. N1's `bool` row fields also serve F2's real shapes
  (`is_kinematic`, `is_grounded`).
- **Inline-callee S1-S3.** S1 (pure free functions) is independent. S2 (exit-free mutating callee on a struct local)
  covers `b.update(..)` **once N4 makes `b` a struct local**: E3 rows a1 and a3. S3 (element-place arguments) covers a2,
  a4 and a5, and needs N3 for the nested paths the inlined body produces. So the order is **F1 ∥ N1 -> F2 -> N2 -> N3 ->
  N4 -> S1/S2 -> S3**. This ranks N1-N4 with F1/F2 and above S2/S3, as the game-shapes findings recommend
  (roadmap items 3 and 4 there).
- **Function units and call units ([interp-call-unit.md](interp-call-unit.md)).** A for-in inside a function body
  unit lowers the same way, with pins in that unit's `Ctx`. A native call (`rt::call_unit`) must not hand a pin across
  in v1. The callee pins again, which costs an `Rc` clone, not a scan of the vector.
- **vecpass retag (branch `perf-vecpass-share`).** It retags local literal `Vec<f64>` arguments. It does not touch
  struct elements. Struct fields are retagged when constructed: E3 row a6 f64 compiles with `rowview` firing, so the
  push-built `Vec<Body>` holds `F64`, not `Lit`. N0 item A4 confirms the same for nested literals
  (`Body { pos: Vec2 { x: 1.0, .. } }`) and, after F2, for `F32`. A `Lit` leaf would fail `ty_is` and lose the view
  silently. That is a performance bug, not a soundness one.
- **Real logic modules (`findings/interp-logic-top20.md`).** Their loops are `for` loops over vectors of strings and
  structs whose bodies are string operations, `match` and calls. N2 makes such a loop an OSR candidate, but the bodies
  stay closure-run, and the statement-ratio rule (F10 of the struct-vec design) refuses them. N1-N4 do **not** move
  those modules. The place-machinery and allocation items in that file do.

## 7. Parity programs, mutants and counters per stage

All parity programs go under `tests/engine_parity/`. They are run by `tests/interpreter_engine_parity_test.rs` at the
default threshold and eagerly (`WJ_JIT_LOOP_HOT=0`), and compared with the walker. Each stage's counter is asserted in
`tests/interpreter_fast_paths_test.rs`, so parity is not vacuous. Mutants are debug-build `WJ_JIT_MUTANT` values, and
every one must change the **printed output** of some parity program (release-observable), unless marked debug-only.
Each stage lands a soundness note under `docs/design/interp-soundness/` with a row in its README.

| stage | parity programs | mutants (what the program does to catch it) | counters |
|---|---|---|---|
| N1 | `jit_let_row.wj` (read-only, write-no-writeback, `v[i] = p`, `v[j] = p` with j != i, escape by push and by call), `jit_let_row_oob.wj`, `jit_let_row_bool.wj` | `letrowshare`: bind `p` to the row (the program stores `v[i].x` after the `let` and prints `p.x`); `letrowlazy`: load a field at first use (same program); `rowsetpartial`: `v[i] = p` stores only the fields the body wrote (the program assigns `p = q` wholesale first) | `rowlet`, `rowset` |
| N2 | `jit_forin_snapshot.wj` (push in the body, `v[j]` store in the body for j > k, `v` reassigned in the body, early `break`, nested in a `while` unit and in a function unit, exit mid-loop and resume), `jit_forin_packed.wj` | `forinlive`: read `v`'s current body instead of the pin (prints the stored value); `forinlen`: re-read the length each iteration (the iteration count changes); `forinresume`: resume at `k` instead of `k+1` (a counter prints one more); `pinnoclear`: `vec_pin` keeps `v`'s write view (an in-place store leaks into the snapshot) | `forin`, `forpin`, `pinrow` |
| N3 | `jit_nested_row.wj` (`v[i].pos.x += v[i].vel.x * dt`, `v[i].pos = Vec2 {..}`, `let q = v[i].pos` then a store to `v[i].pos.x`, elements built from one shared `Vec2` local, two-level `e.xf.pos.x`) | `nestshare`: skip the nested `get_mut` (shared `Vec2` elements: a store leaks into its siblings); `nestrc`: lower `let q = v[i].pos` as an `Rc` clone (`q` changes after the store); `nestpath`: use path `j-1`'s entry (wrong field printed) | `nestview` |
| N4 | `jit_for_wb.wj` (write-back on latch, `break`, `continue`, `return`; a store to `v[k+1]` seen by the next iteration; a guard exit mid-body), `jit_for_wb_err.wj` | `wbskip`: no write-back on `break` (printed field); `wbstale`: write back the values loaded at iteration start (printed field); `wbresume`: `resume` skips that iteration's write-back; `wberr`: write back on an error (debug-only unless a program can observe `v` after an error) | `forwb` |

## 8. Staged plan with gates

Gates use callgrind marginals (Ir 10x minus Ir 1x, divided by the work difference), the method of
`findings/interp-e1-e6-game-shapes.md`.

**N0: measure and audit (no engine change).**
- M1: callgrind `n1.wj` (nested, hand-inlined, f64) and its f32 twin. These are the missing nested baselines. Add
  `a7` (`for mut b in ents` with the update hand-inlined, f64 and f32) and a `for p in v` read-only loop over a nested
  struct.
- M2: split probe "`for x in v` over `Vec<int>`": its 2,425 Ir per iteration includes the push loop.
- M3: count shared nested `Rc`s in the real game construction paths (does `Body::new` build fresh `Vec2`s?).
- A1: locate why `for x in xs` over a `let mut` vector runs as an island (section 0, item 4).
- A2: the copy-on-write writer audit (5.2).
- A3: the F14 audit extended to nested `Rc`s (2.3).
- A4: literal retagging of nested and `F32` fields (section 6).
- Stop criterion: if A2 fails, N2 is planned as N2'.

**N1: `let p = v[i]` and `v[i] = p` (first branch).**
- Expected: probe `let p = v[i]` from 3,596 to about 60 Ir per iteration.
- Gate: at most 100 (*assumed*).
- Stop if above 150.

**N2: `for p in v`, snapshot form, locals and parameters.**
- Expected: probe "`for p in v` over `Vec<struct>` reading fields" from 3,853 Ir per iteration to the N1 figure plus
  the counter (about 60-80, *assumed*).
- Gate: at most 120 (*assumed*), and `forin` fires on the probe.
- Stop if above 200, or if the pin build or credit shows above 20% of the loop's Ir. In that case the view build
  needs the per-element check variant before N2b.

**N2b: field iterables (`for b in self.bodies`).**
- Gate: the same probe written with `self.items` within 1.2x of N2 (*assumed*).

**N3: nested two-level views (after F2).**
- Expected: nested hand-inlined update (M1's f64 figure, not yet measured) to within 1.5x of the flat a6 f64 figure,
  258 Ir per update (*assumed* ratio, for the extra path loads and the wider table). Its f32 twin should be within 2x
  of the f64 one, F2's own stop rule.
- Stop if the f64 nested update is above 2x a6 f64. Profile table width and build credit first.

**N4: write-back form.**
- Expected: `a7` (hand-inlined, from M1) to within 1.3x of N3's nested figure (*assumed*).
- Then, with S2: E3 a3 (68,038 Ir per update today) and a1 (37,703) towards the a7 figure.
- Stop if a7 is above 2x N3's figure: the per-iteration write-back stores are not being merged.

The gates against the flat f32 hand-inlined row (25,257 -> f64's 258) belong to F2, not here. This plan relies on F2
having passed them.

## 9. The E3 not-native rows each stage retires

From the probe table and E3(a) of `findings/interp-e1-e6-game-shapes.md`:

| row (today) | retired by |
|---|---|
| probe `while` + `let p = v[i]`, then read fields (3,596) | N1 |
| probe `for p in v` over `Vec<struct>` reading fields (3,853, no verdict) | N2 |
| probe `for x in v` over `Vec<int>` summing (loop not split, see M2) | N2 (packed pin) |
| `v[i].pos.x` refused for every width (section 1, item 8 of the f32 proposal) | N3 (f64), N3 after F2 (f32) |
| E3 a6 flat, hand-inlined, f32 (25,257) | F2, not this design |
| E3 a1 `for mut e in ents { update_body(e, dt) }` (37,703) | N4 + S2 (a free function on a struct local: G2) |
| E3 a3 `for mut b in ents { b.update(..) }` (68,038) | N4 + S2 |
| E3 a2, a4, a5 (element-place calls: 45,010, 87,676, 33,215) | S3, with N3 for the nested paths |
| E3(c) logic modules, E3(b) struct registries | not this design (section 6) |

## 10. Risks

1. **The pin audit (A2).** If some builtin writes through a raw pointer without copy-on-write, a pinned read view
   would show a value the closure engine's iterator would not. Mitigation: the audit, then the `pinnoclear` and
   `forinlive` mutants, then option N2'.
2. **Write-back `resume` (N4).** A guard exit mid-body must hand the closure engine both the rest of the body and that
   iteration's write-back. Getting this wrong is visible only on guard exits. `wbresume` and the forced-exit parity
   program exist for exactly this.
3. **Nested sharing in real construction code** (section 0, item 2). It can make N3 look like no gain on real data
   while the micro gate passes. M3 measures it before N3 is built. If it is common, unsharing on first view build
   (a deliberate `make_mut` per element: an allocation per shared element, once) is a cheap, sound follow-up.
4. **Table memory for very large vectors**: `8 * n * S` bytes per view, rebuilt per run. It is bounded by credit, but
   a 1M-entity world with S = 4 is a 32 MB table per build (*assumed* not a game-sized number). Packed rows
   (2.4) remove it.
5. **Scope creep into the logic modules.** For-in support makes their loops candidates but not native (section 6). Do
   not count them as N2's win.

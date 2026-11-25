---
status: rejected
last_verified: 5457283f4156
---
# perf-jit-row-cache: invalidation argument (written before any change). DECISION: STOPPED, no implementation.

> **Rejected design: not implemented.** The invalidation argument for row views kept across OSR entries could not be completed, so no code was written.

Base: cc-file-transfer/wt-rc3-int 712b09471. Goal: keep `rows::fill` row views and `osr::unit_for` results across OSR
entries behind a Vec mutation epoch. I could not make the argument for the row views, so nothing was implemented.

## What a cached row view refers to
`RowView{key,len,tbl}` + `Ctx::row_tbls` hold the raw ADDRESS of every element's `StructVal::fields` buffer
(rows.rs `fill`: `s.fields.as_mut_ptr()`), obtained under `Rc::get_mut` on the vec body AND on every element
(so: vec Rc unique, each element Rc<StructVal> unique, each field Value has exactly the shape's variant/width).
Machine code then stores f64/i64 payloads straight into those Values. Soundness of such a store needs, at the moment of
the store: (1) the same vec value is still in the slot, (2) its body is the same allocation with the same len,
(3) every element Rc is still unique (strong==1, weak==0), (4) every element's fields buffer address is unchanged,
(5) every field Value still has the shape's variant (an f64 store into a field that became an Int corrupts the Value).

## Events that break (1)-(5), and whether an epoch can cover them
* Storage mutations (push/pop/clear/truncate/resize/swap/sort/extend/replace/index store): reached through
  `Rc::make_mut`/`Rc::get_mut` on `VecBody` or `DerefMut for VecBody` (vecbody.rs:112): 45 `make_mut`/`get_mut` sites in
  interpreter/ (place.rs:12-47,116; jit/rt.rs:206,330,361,395,434,728; builtins/seq.rs:23; builtins/shared.rs:75-93;
  ...) plus every `DerefMut` use. Enumerable, and `VecBody` could carry a per-body generation bumped in DerefMut /
  set_value / push_value. BUT:
* SHARING needs no mutable access. `let b = bodies[i];` / `.clone()` / passing the element by value is `Rc::clone` of the
  element `Rc<StructVal>` (breaks (3)) through `&` access only: there are ~800 `.clone()`/`dup()` sites and
  `Value::clone` is derive-level; no `&mut VecBody` is involved, so a body-level epoch is NOT bumped. A later in-place
  store through the cached view would then write into a struct a second owner can see (the walker would have copied).
  An epoch would have to be bumped by every `Rc<StructVal>` clone anywhere in the interpreter: not hookable
  (and a bump per clone is a cost on every program). Today's design is sound only because every closure-run statement
  (`run_stmt`) clears all views (rt.rs:310,355,371,415,653) and a view never outlives one unit run.
* Statements BETWEEN two entries run in the closure engine with no interpreter-level mutation hook: the step loop,
  the code between `advance`'s two loops, `resume` of the exit statement. An epoch bumped "per mutation" is not
  observable there; bumping per statement makes every entry miss (the two entries per step are separated by closure
  statements), so the cache would never hit in the nbody shape this job targets.
* Per-body generation + Rc address as key: ABA (drop, then a new vec at the same address) needs a globally unique id
  (counter = new static mutable state, rejected by the static_state ratchet), or holding an `Rc` clone (makes the vec
  non-unique: every write COW-copies the whole vec and its elements, an asymptotic regression), or a `Weak` (also defeats
  `Rc::get_mut`, so `fill` itself would refuse). Neither is acceptable; (5) (field retyping through `place.rs` struct
  field stores) is again a `&mut` path that only a per-StructVal generation would see.
* A cheaper O(n) revalidation (ptr/strong/len compare, no field scan) removes most of `fill` but cannot prove (5) and
  has element-level ABA (an element replaced and freed twice can reuse an address). Not argued, not built.

## Wraparound / threads / hot reload (for the record)
A u64 epoch bumped at most once per interpreted operation cannot wrap (2^64 operations is centuries). The
interpreter is single-threaded per `Interpreter` (Rc everywhere); recursion is covered by `Unit::spare` already taking
the tables so a nested run allocates its own. None of this rescues the sharing hole above.

## unit_for
`unit_for` (osr.rs:142) is keyed by the classification of the loop's locals (`sig_is` -> `classify_is`, osr.rs:359),
which is checked in place per entry. A cache of "last class per slot" would need the same epoch for any slot
holding a Vec/struct (class_of peeks element kinds), and for plain scalars the check is already the cheap path. Stopped
with the row part (no independent win large enough: the measured per-step cost is the in-place check itself).

## What would make it provable (follow-up design, not done)
Callee-frame/effect analysis: a static proof that the closure statements between two entries (and the loop exit
statement) touch no Vec/struct-typed local aliasing the cached one (no calls, no clones of its elements, no stores
through it), i.e. a compile-time "this region is scalar-only" bit on the OSR site pair. Or: make
`Value::Struct` element sharing impossible while a vec holds a row view (a "pinned" flag in the vec body checked by
the clone path of `Rc<StructVal>`), which changes Value representation. Either is a larger, separate design job.

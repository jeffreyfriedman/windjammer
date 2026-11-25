---
status: current
last_verified: bdf758062bea
---
# perf-jit-push-inline: the pending-length protocol (written before the code)

[vecinline.md](vecinline.md) stopped inlining `push` because a pending length "needs a protocol across every call,
exit and deopt" and it could not be argued there. This note is that argument. The generic fallback is the lean helper
`rt::push` (`lean_push`); the inline path must be observationally identical to it.

## The protocol
`Ctx` gets one push record: `pk` (key: 0 = empty, else `view_key(slot, element type)`), `pptr` (address of element 0),
`plen` (the PENDING length), `pcap` (capacity), `pvec` (address of the `Vec<bool>` / `Vec<i64>` / `Vec<f64>` inside the
packed body) and `pkind`. While the record is armed the real `Vec` header is STALE: its `len` is the length at arm time;
elements `[len, plen)` are initialised in spare capacity.

* Machine code `v.push(x)` (an eligible site, below): `pk == key(slot, type) && plen < pcap` -> store the element at
  `pptr + plen*size`, `plen += 1`, count `pushinl`. Otherwise call the helper (a normal `rt::push`).
* Arming: only `rt::push` arms. After it has pushed through the existing code (so the body is the unique, packed one,
  `Rc::make_mut` done, `cache` dropped), it re-reads the body; if that is `Packed` with EXACTLY the pushed element type
  (bool, `i64`, float: `view_elem`; every other width stays on the old path) it fills the record from the new buffer.
* Flush = `Vec::set_len(plen)` of `pvec` (by `pkind`) and `pk = 0`, counted `pushflush` when it does work. Idempotent.

## 1. Where the pending length lives and when it is flushed
It lives in `Ctx`, in memory, never across a Rust call. The invariant: **armed implies no Rust code has run since arming**
(so nothing has read or touched the `Vec`, its `Rc`, views, rows or the `Value` cache). It is kept by flushing at the
only two kinds of place where Rust code can run:

1. EVERY helper call. `Lo::call` (src/interpreter/resolved/jit/lower.rs) is the only `call_indirect` emitter of a unit
   (the one other `call` in `jit/mod.rs` is the pure-integer function compiler, which has no `Ctx` and no vectors). In a
   unit that has an eligible push site, it emits `if ctx.pk != 0 { rt::flush_push(ctx) }` immediately before the
   call. That covers `len`, `index_get`, `index_set`, `index_field_*`, `map_*`, `run_stmt` (every closure-run statement,
   so also panics raised by them and anything that reads or clones the vector), the slow `rt::push` itself (it needs the
   real length), `int_op`, and calls that enter other units (they run inside `run_stmt` or a helper).
2. EVERY unit exit. A unit has many exits (planned loop exit, `break`/`return` exits, unplanned failed-check exits,
   `bail_if` after a failed helper, arithmetic overflow traps, a stack/step refusal, the end of a function unit) but they
   all return from the compiled function `unit.f` to ONE place, `execute` in osr.rs, which flushes right after the call and
   BEFORE it reads `ctx.raised`, writes spills back or lets the closure engine resume. A trap after pushes therefore leaves
   the `Vec` with exactly the pushed length. Machine code never panics or unwinds; Rust panics inside helpers happen after
   (1)'s flush. `Ctx` has no `Drop`-based flush on purpose: the single explicit site is mutation-testable.

Exit kinds: planned, unplanned, `bail_if` (fail flag), overflow/div traps, `break`, `return`, function-unit end, failed
guard at entry (nothing ran: `pk == 0`). All are returns of `unit.f`.

## 2. Views and hoisted rows
Arming happens inside the slow `rt::push`, which (as today) first clears all views and row views (`clear_rows`). While
armed, no view or row view can come into existence: they are built only by helpers (`index_get`, `index_set`, `row_build`)
and every helper call flushes first (disarming) and then builds them against the real `Vec`. So a view's `len` is never
stale relative to the pending length: when armed, the view table is empty, and `view_load`/`view_store` miss (the key is
0) and go to the helper, which flushes. Bounds checks therefore use the REAL length in every code that can read; the
pending length is consulted only by the inline push itself (against capacity). I chose this alternative, not
"pending-aware bounds checks": reading what the loop pushes simply takes the old helper path, with a flush each time.
Hoisting: `push` is not in `SAFE_CALLS` ([hoist.md](hoist.md) 7), and the inline push still calls `note_call("push")`, so a
loop containing a push is never hoisted; no hoisted view coexists with a pending length.

Cost control (a perf restriction, not a soundness one): the inline push is emitted only where the INNERMOST enclosing
loop's whole body is "push-only": scalar `let`/assignments, `if`, nested `while`/`for` over ranges, `break`/`continue`/
`return`, and `push` statements; no index, call, method call, field access or macro. Anything else in the loop would
flush on every iteration and re-arm, which costs more than the old helper. A loop failing the scan keeps `rt::push`.

## 3. Aliasing and uniqueness
The record names ONE vector. A second name for the same vector (a reference local, a `&mut` param, a `let r = &mut v`) is
a different slot, so its push misses the key, takes the helper (flush first), and re-arms for that slot. The key is
slot-based but the AUTHORITY is `pvec`: the buffer pointers come from the body the slot resolves to through plain
references (`lean_value`), the same resolution `lean_push` uses. Uniqueness: arming happens after `Rc::make_mut`, so a
shared body is copied before the record is built (copy-on-write happens in the helper, on the arming push). While armed
nothing can clone the `Rc` (clone is a closure-run statement: it flushes first). A vector with an outstanding read/store
view: views are cleared by the arming push (as `rt::push` already does), so the view's raw pointer is dead before the
buffer can move.

## 4. Capacity growth
The inline path never allocates: it requires `plen < pcap`. At capacity the helper runs: flush (real len = pending),
then the existing `Packed` push (`Vec::push` reallocates with its normal doubling, so the memory behaviour is the old
one), then arm again from the NEW buffer (`pptr`, `pcap` re-read, nothing from the old buffer is kept). Views were
cleared first, so nothing can point at the old buffer. A first push to an empty `Values` body or any unpacked body goes
through the helper's general path and arms only if the result is packed with the pushed type.

## 5. Element-type guards
Arming requires the body packing's element type equal to the pushed value's type (`Packed::Bool`/`Int(_, I64)`/
`Float` against `Ty::Bool`/`Ty::Int(I64)`/`Ty::Flt`), the same test as `lean_push`, and a key that includes the type. A
float into an int vector, an `i32` into an `i64` vector or an unpacked vector never arms and never hits (key differs),
so they take the old code, which may unpack. A bool is stored as `x != 0` (one byte of 0/1: a valid `bool`).

## 6. Traps and limits
The inline path performs no allocation, no arithmetic that can trap and no call; the only way out of it is the helper.
Allocation failure, length limits and the memory limit are all in `Vec::push` inside the helper, unchanged. A trap
elsewhere in the same unit exits through (2): `set_len(plen)` first. The interpreter's recursion/step limits are checked
by closure code and helpers, all after the funnel flush. Elements in `[len, plen)` are plain scalars with no `Drop`, so a
lost flush could never leak or double free: it could only lose pushed elements, which the mutants below demonstrate.

## 7. Hot reload
The record is per `Ctx`, i.e. per unit RUN; it is flushed when the run returns and holds no pointer into a compiled unit
or the program. Reload takes effect between runs.

## Stopped / excluded
Pending-aware bounds checks for reads of the pushed vector in the same loop (not done: reads flush). Two vectors armed at
once (one record only; alternating pushes to two vectors alternate flushes and re-arms and are correct).

## Tests (tests/engine_parity/jit_push_inline.wj; counters `pushinl`, `pushflush`)
Growth mid-loop, push then read/len in the loop, outstanding view, alias, wrong kind, pop/clear in the loop, trap after
pushes, function unit, two vectors, early break/return, empty first push, many pushes, the sieve.

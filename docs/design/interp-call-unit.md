---
status: proposal
last_verified: 529cda57f8fd
---
# Native calls between machine-code units (`rt::call_unit`)

> **Proposal: not implemented.** Step 2 of [interp-struct-vec-loops.md](interp-struct-vec-loops.md). Step 1 (the
> function unit, branch `perf-jit-fn-unit` 8d6418b86) is merged; this page designs the next step and its gates.
> File and line references are to `wt-rc3-int` 529cda57f.

## 0. Verdict, and what changed from the step-2 sketch

**The protocol below is sound by a complete argument, with one hazard found and closed.** It differs from the
sketch in interp-struct-vec-loops.md section 6 in three places, each of which removes a proof obligation:

1. **No nested exit protocol is needed.** The callee's frame is built *eagerly*, in Rust, before its machine code
   runs, by the same functions the closure engine calls (`fill_frame`, `run_compiled_at`). Every exit of the callee
   unit is finished inside that real frame by the existing `osr::level`, on the native stack *above* the suspended
   caller. The caller never sees a callee exit. It sees one of two outcomes, exactly the contract of `rt::run_stmt`:
   *continue* or *leave after this statement with `raised`*. The obstacle that stopped P3 ([interp-soundness/p3.md](interp-soundness/p3.md): "the exit
   must resume inside the callee, in a callee frame that does not exist yet") does not arise, because the frame
   exists.
2. **The caller's views never survive a call implicitly.** `call_unit` clears every caller view and resolved cell
   when it starts, as `run_stmt` does. The only views that cross the call are *moved* through an explicit `Handoff`
   value that lives on the Rust stack (never in `Interpreter`), and each moved view is re-validated by O(1) checks
   when the callee adopts it. "Reload after call" is therefore not a lowering-time proof. It is the run-time rule
   "a call clears", which already holds for `run_stmt` and is tested by the `fnviews` and `vecstale` mutants.
3. **One code path for the call.** The native call runs the closure engine's own call-site object
   (`expr.rs` `Site`, shared through an `Rc`) with its own argument codes. Native code does not evaluate arguments,
   does not resolve the callee by name, and does not build `Value`s. Evaluation order, shadowing, `place::arg`
   semantics, deopt and the walker fallback are the closure engine's by construction.

**The hazard (section 3.3):** `f(v, v)` where the first parameter is an inferred `&mut` and the second is by value.
The by-value argument is an `Rc` clone of the vector body that lives in the callee frame. A view handed in to the
`&mut` parameter would then write in place into storage the by-value copy shares, which breaks value semantics.
The existing `params_alias` check (jit/osr.rs:121-148) does not see it, because it only follows `Value::Ref`
parameters. The adoption check `Rc::strong_count(body) == 1 && Rc::weak_count(body) == 0` closes it and is
mandatory. Mutant `handoffunique` must make a parity program fail.

**What step 2 does not buy.** The coordinator's projection of about 50% came from the attribution report, measured
*before* step 1 (17,002 Ir/step, two entries per step). Step 1 already took 35% (11,085). After step 2 the callee
frame build (`fill_frame`, `enter_compiled`, `bind_params`, the boxed-parameter prologue, `leave_compiled`) and the
callee unit entry (`execute`, `Ctx::new`, `unit_for`) are still paid once per call. Only inlining (step 5) removes
them. The projection for step 2 is **about 9,300 to 10,000 Ir/step** (section 9). It is a projection from the
attribution report's components, not a measurement.

**Recommended restriction for v1:** native calls only in statement position (`f(args)` with the result unused), to
callees that can be function units (`f.ret == None`, a loop in the body, not `main`, not generic). The view handoff
additionally requires every argument to be an identifier or a literal. No trap-freedom restriction is needed.

## 1. The protocol

### 1.1 Who decides what is called (compile time, closure compiler)

`expr::call` (src/interpreter/resolved/expr.rs:417-515) already decides, per call expression, that `name` is a
module function and not a callable local (`cx.local(name)` first, :426), builds argument codes
(`place::arg` for a mutated parameter, `expr_arg` otherwise, :450-456) and a `Site` (:462). Step 2 changes:

- `Site` becomes `Rc<Site>`, and the closure body (:477-486) moves into a method
  `Site::call(&self, it, b, loc, handoff: Option<&mut Handoff>) -> R`. The closure calls `site.call(it, b, loc, None)`.
- When the expression is recorded (`cx.loop_depth > 0 || cx.record_all`, the gate `record_stmt` uses), the compiler
  registers `reg.record_call(expr, CallReg { site: Rc<Site>, loc, args: Vec<ArgInfo> })`, where `ArgInfo` is
  `Place(slot)` for a bare identifier passed to a mutated parameter, `Value(slot)` for a bare identifier passed by
  value, `Lit` for a literal, and `Other` for anything else. `Reg` (src/interpreter/resolved/osr.rs:55) gains one map
  keyed by expression address, like `stmts`.

The JIT never looks a function up by name. A local closure called `advance` shadows the module function in the
closure compiler, so no `CallReg` is recorded, and the lowering keeps today's behaviour (a closure-run statement).
This also covers a gap in the lowering: `osr::stmt_names`/`expr_names` (jit/osr.rs:454-493) do not descend into
`Expression::Call`, so the lowering's own scopes cannot tell a shadowed name from a function.

### 1.2 What machine code emits (lowering)

In `Lo::stmt_in` (jit/lower.rs:676-699), before the fall-back to `run_stmt`: a `Statement::Expression` whose
expression is `Expression::Call` with a registered `CallReg`, whose callee passes `compile::fn_unit_ok`
(compile.rs:185-194, made `pub(super)`), outside any inlined frame (`self.frames.is_empty()`, as `run_stmt` requires
at :402), lowers through `Lo::call_stmt`:

1. **Spill** every variable in scope to the spill buffer, exactly as `run_stmt` does (:405-433), with its two
   refusals (a boxed struct `let` outside its own block, a boxed fresh `let`). The shared part becomes one
   function `spill_around(s, helper)`, used by both.
2. Push `CallRun { stmt, reg: CallReg, spills, structs }` onto a new `Plan::calls` (jit/lower.rs:97-111).
3. Emit `status = call_unit(ctx, buf, k)`.
4. Emit the after-exit (planned, `after: true`, path = this statement's index + 1) taken when `status != 0`, then the
   reload of the spills: the same code as `run_stmt` (:441-455).
5. Count the statement in `natives`, not in `stmts`, so the refusal rule (:288-292) passes for the step loop
   (`advance(bodies, 0.01)` native, `step += 1` native: 0 closure-run statements of 2).

The caller emits no argument code and no parameter code. Scalars reach the callee through their slots (the spill
wrote them), handles through their cells, both read by the `Site`'s own argument codes.

### 1.3 What `rt::call_unit` does (Rust, `extern "C" fn(*mut Ctx, i64 buf, i64 k) -> i64`)

```text
call_unit(ctx, buf, k):
  run = &plan.calls[k]
  offers = take_offers(ctx, run)        // 2b only: caller views and parked views of Place(slot) args (section 2.3)
  ctx.cells.clear(); ctx.cell_hit = empty; ctx.views = empty; ctx.clear_rows(); ctx.parked.clear()
  write run.spills / run.structs to their slots           // as run_stmt (rt.rs:679-691)
  it.mark(loc(run.stmt))
  r = run.reg.site.call(it, b, run.reg.loc, Some(&mut Handoff { offers, back: [] }))
        // = it.mark(callee loc); site.target(it) -> fill_frame(args, left to right) -> run_compiled_at_with(.., handoff)
        //   or site.walk(it, b) when the callee is not compiled (exactly expr.rs:477-486)
  match r { Err(e) => { ctx.raised = Some(e); return 1 } , Ok(_) => {} }
  reload run.spills / run.structs from their slots (type check, else return 1)  // as run_stmt (rt.rs:701-712)
  park(ctx, handoff.back)                // 2b only
  it.fast.callunit += 1
  return 0
```

The result value of the call is dropped; v1 callees return `Unit` (`finish_value`, mod.rs:261).

### 1.4 How the callee runs (Rust, inside `Site::call`)

`run_compiled_at_with(f, c, base, bound, handoff)` is today's `run_compiled_at` (mod.rs:106-133) with a handoff
parameter passed to `run_body`. For a function-unit body, `run_body` calls a new
`fn_unit_run(it, base, fu, handoff)`. `fu: Rc<FnUnit>` holds what the body closure captures today
(compile.rs:301-321): the boxed-parameter prologue, the `OsrSite`, the `rest` code and the poisoning slots. The
`Body::V` closure becomes `move |it, b| fn_unit_run(it, b, &fu, None)`, so the closure engine and `call_unit` run the
same function, and the order *prologue, then hook, then rest* stays one piece of code (the M5 survivor review in
[interp-soundness/fnunit2.md](interp-soundness/fnunit2.md) depends on it). `fn_unit_run` passes the handoff to `osr::enter_body` and on to `execute`,
which adopts the offers right before calling the unit and fills `back` at the end exit (section 2.3). On every other
path (not armed, `params_alias` refused, `unit_for` refused, a live-in of another type, deopt in `bind_params`, an
error in `enter_compiled`) the handoff is ignored and `call_unit` drops it.

The callee lookup is therefore the closure engine's: `Site::target` (expr.rs:530-539, a `OnceCell` filled on the first
call) gives the `CompiledFn`. The unit for the arguments' classification is chosen by `unit_for`
(jit/osr.rs:196-246), with its `sig_is` guard on every call, `MAX_SIGS` and `dead`. If the callee is not hot yet,
its hook ticks and `rest` runs in the closures, as today. An optional per-`CallRun` cache of the last unit (skipping
the `site.units` borrow but keeping `sig_is`) is step 2c, only if measurement asks for it.

### 1.5 The native stack during a call

```text
caller unit machine code                (caller Ctx: views cleared, offers moved into the Handoff)
 └ rt::call_unit                         (Handoff on this Rust frame)
    └ Site::call -> fill_frame -> run_compiled_at_with
       └ enter_compiled (depth_check, backtrace, scope) -> bind_params -> run_body -> fn_unit_run
          └ prologue -> osr::enter_body -> unit_for -> execute
             └ callee unit machine code   (callee Ctx; adopted views)
          └ osr::resume / level           (only after a non-end exit: closure code in the callee's real frame)
       └ finish_value -> leave_compiled
```

## 2. View lifetime across calls

### 2.1 The rule

A view lives for one contiguous machine-code run, extended by **Rust code that is proved to run no user code and
create no `Rc` clone of a vector body or element** (F14 extended). A native call does not extend a view by
itself. The caller's views end when `call_unit` starts: they are moved into the `Handoff` or cleared. Inside the
callee, the existing rule applies unchanged: `run_stmt`, `push`, `index_set`, `index_field_set` and `field_set` clear
(jit/rt.rs:312, 349, 357, 373, 416-417, 673-674). After the call the caller has no views except the *parked* ones
(2.3), which its machine code cannot read.

### 2.2 How the code generator enforces "reload after call"

It does not need a compile-time invalidation pass. Machine code reaches a view only through `Ctx::rows` and
`Ctx::views` with a key compare (lower_rows.rs:54-58, lower_expr.rs:598-600, 635), and on a miss calls
`row_build`. `call_unit` zeroes every key before any other code runs. The next access after the call misses and
rebuilds, which is the reload. The lowering keeps nothing about views in registers across a helper call (the key and
table are loaded from `Ctx` at each access). **Step 3 (hoisting the key and length checks out of an inner loop) must
treat `call_unit` like `push`: a loop body that contains a native call cannot hoist.** Mutant `hoistcall` gates that.

### 2.3 The handoff (step 2b)

Without it, the callee rebuilds its views on every call (`rows::fill`, about 930 Ir per call on nbody). With it,
`fill` runs once per run.

- **Offer** (`take_offers`, in `call_unit`, before the clear). Only when every argument is `Place`, `Value` or `Lit`
  (no argument code can run user code). For each `Place(cs)` argument: the caller's row view and packed view keyed by
  slot `cs` and the parked entry for `cs`, each with the caller's innermost cell (`resolve_cell`, rt.rs:804-819,
  kept as an `Rc` clone of the cell, so the comparison is ABA-free), the body pointer and length at offer time, the
  shape (a row view) and the element type and `READ_VIEW` bit (a packed view). A row view moves its element table
  `Vec` with it.
- **Adopt** (in `execute`, jit/osr.rs:273-334, after the live-ins are read and `stack_room` passed, right before
  `(unit.f)(..)` at :299). For each offer, find the callee parameter slot `ps` whose `resolve_cell` is `Rc::ptr_eq`
  to the offer's cell. Adopt only when all of these hold:
  - exactly one parameter matches;
  - the cell holds `Value::Vec(body)` with `Rc::as_ptr(body)` equal to the offer's pointer and `len` equal;
  - `Rc::strong_count(body) == 1 && Rc::weak_count(body) == 0` (section 3.3);
  - for a row view, the callee plan has a shape `==` to the offer's (`Shape: PartialEq`, kinds.rs:58-64), giving the
    callee index `idx`. For a packed view, the element type is unchanged and the `READ_VIEW` bit is kept;
  - the callee `Ctx` position (`ps % ROWS`, `ps % VIEWS`) is still empty.

  The adopted key is `row_key(ps, idx)` or `view_key(ps, t) | (offer & READ_VIEW)`. `row_build` (rows.rs:88) then
  finds the key and skips `fill`. Count `it.fast.rowhandoff`.
- **Hand back** (in `execute`, after the machine code returns and before the tables go back to `Unit::spare` at
  :301-305). Only when the exit is the function's normal end (`planned`, path `[Level { body, idx: body.len() }]`),
  move every row and packed view whose slot is a parameter slot into `handoff.back`, with that parameter's cell. At
  any other exit nothing is handed back: closure code runs after it (`level`), and the caller would see a view
  older than that code.
- **Park** (in `call_unit`, after the reload). Map each returned view's cell to the caller slot of the `Place`
  argument that has that cell, and store it in a new `Ctx::parked` (at most `ROWS + VIEWS` entries). Machine code
  never reads `parked`. Every site that clears any view also clears `parked`: one `Ctx::clear_views()` replaces the
  eight clear sites listed in 2.1. The next `call_unit` offers parked entries like live views.

### 2.4 Why a moved view is still valid

Between the caller's last machine-code instruction and the callee's first, and between the callee's end exit and the
caller's next instruction, only this Rust code runs. Each item has been read for user code and `Rc` clones:

| Code | User code? | Body or element `Rc` clone that outlives it? |
|---|---|---|
| `call_unit` spill writes (`osr::write`) | no | no (scalars, struct locals rebuilt as fresh `Rc`s) |
| `Site::call` argument codes, restricted to identifiers and literals | no | a by-value identifier clones the **body** (closed by the uniqueness check); `place::arg` clones the cell's value transiently (interpreter/place.rs:257-261, dropped before it returns) |
| `fill_frame` (expr.rs:552-567) | no | no |
| `enter_compiled` (mod.rs:206-225) | no | no |
| `bind_params` / `coerce` (mod.rs:288-306, coerce.rs:29) | no: `coerce` takes `&self`; a `Ref` falls through to `Ok(v)` | no |
| prologue (`island::boxed`, compile.rs:283-289) | no | no (wraps the `Ref` in a new cell) |
| `armed`, `fast_off`, `params_alias`, `unit_for` (with a possible Cranelift compile), live-in reads | no | `load_struct` clones a struct parameter transiently (jit/osr.rs:340-353) |
| `level` on the end path, `finish_value`, `leave_compiled`, poisoning check | no (no statement runs at `idx == len`; the body block is unregistered) | drops only; a drop cannot move or share storage. Programs with `impl Drop` types never compile functions (`compiled_for`, mod.rs:82) |

So no element of the vector became shared and no storage moved. Whole-body sharing is the one effect the table
admits, and the adoption check rejects it. The body pointer and length checks are a belt over the same argument.
In debug builds and `WJ_JIT_DEBUG` runs, adoption also rebuilds the table with `fill` into a scratch `Vec` and
asserts that it equals the adopted table, so the whole test corpus checks the argument on every handoff, the way
`unit_for` asserts `sig_is` against `classify` (jit/osr.rs:208-210).

## 3. Aliasing

1. **The same vector passed twice to two `&mut` parameters.** `params_alias` refuses the callee unit
   (jit/osr.rs:106-108); the callee runs in the closures and the handoff is dropped. The adoption rule "exactly one
   parameter matches" refuses it independently.
2. **A parameter aliasing a caller local the caller keeps using** (`let r = &mut v` in the caller, then `f(v)`, then
   `r[0].x` natively). The caller's views of `r` and `v` are both cleared at the call. After it, `v`'s view may be
   parked. `r`'s next access rebuilds. Two views of one cell are the same memory and are cleared together
   ([interp-soundness/fnunit2.md](interp-soundness/fnunit2.md), M2), so a parked `v` and a rebuilt `r` are consistent.
3. **A by-value copy of the passed vector alive during the call:** `f(v, v)` with one `&mut` and one by-value
   parameter, or a caller's `let c = v` (closure-run, it clears everything, and `fill` then fails on the shared body).
   Closed by `strong_count == 1` at adoption. Without the check, the adopted view writes in place into the body the
   by-value parameter shares, and the callee's by-value copy changes. This is a real bug, and mutant `handoffunique`
   must be killed by a parity program that prints the by-value copy after the callee writes through the other
   parameter.
4. **`&mut` scalars.** A caller scalar passed to a mutated parameter: the spill writes it to its boxed slot,
   `place::arg` hands the callee its cell, the callee writes through it, and the reload reads it back. This is the
   same sequence as today's `run_stmt` around the same statement. Mutant `callnoreload` must be killed.
5. **Closures passed in that captured caller cells.** They read and write the caller's slots, which hold the
   spilled values during the call, and the reload picks up their writes. Nothing here relies on "the callee cannot
   see the caller's locals". The spill and reload make that question irrelevant, at a cost of one store and one load
   per scalar in scope (one variable, `step`, in nbody's step loop).

## 4. Recursion

- **Depth semantics are unchanged.** Every native call goes through `enter_compiled`, so `depth_check`
  (call.rs:698-710) runs at the same point as in the closure engine, with the same message ("stack overflow:
  recursion too deep for the interpreter stack") and the same backtrace. The `depth` counter (used when there is no
  `stack_guard`) counts the same frames.
- **Native stack per level.** With a byte budget (`stack_guard`: 4 GiB or 512 MiB main stack minus 256 MiB headroom,
  runner.rs:49-53; 256 MiB task stack minus 32 MiB, sched.rs:41-42) the depth at which the error fires depends on
  bytes per level. A recursive call inside a function unit today is a closure-run statement:
  `run_stmt` -> statement closure -> call closure -> `run_compiled_at` -> body -> hook -> `enter_body` -> `execute`
  -> machine code. With `call_unit` it is `call_unit` -> `Site::call` -> `run_compiled_at_with` -> ... -> machine
  code, which is one closure frame fewer. Per-level stack does not grow, and the headroom (32 MiB or more) dwarfs one
  level. No extra depth limit is added.
- **Re-entrancy.** Unit machine code declares no data objects (no `declare_data` in jit/), so it is re-entrant. Each
  run has its own `Ctx`. `Unit::spare` is taken and put back with `try_borrow_mut` (jit/osr.rs:276-280, 52-56), so a
  nested run of the same unit allocates its own buffers. The handoff moves an outer frame's views into the inner run
  and back. That is sound by section 2 at every level, because each level's offers are re-validated.
- **Mutual recursion and `param_mutation`.** On a recursive cycle, `param_mutation` reports the inner call's
  parameter as not mutated (call.rs:1164-1171, finding 5 of interp-struct-vec-step0a.md). That decision is the
  closure compiler's and is carried in `ArgInfo` (`Value` instead of `Place`), so no view is offered.

## 5. Traps, errors and unwinding

- **A trap inside the callee unit** (overflow, divide by zero, bounds, a helper failure): the callee takes an
  ordinary exit. `execute` writes the callee spills into the callee's real frame, and `level` re-runs the
  statement in the closures, which raises the walker's error with the callee's location. `run_body` attaches the
  callee's locals (`attach_slot_locals`, failure_locals.rs:187-193: only when none are attached yet).
  `finish_value` annotates the error, and `leave_compiled` pops the frame and the backtrace. `Site::call` returns
  `Err`.
- **The caller then** stores it in `ctx.raised` and returns 1. The machine code takes the after-exit. `execute`
  writes nothing back for an `after` exit (jit/osr.rs:313-318), because the slots already hold the spilled values.
  `level` returns the error at that level (:555-556), and the caller's frame unwinds as in the closure engine. An
  error raised before the callee's `run_body` (the depth check, an argument code) has no locals attached, so
  the caller's `run_body` attaches the **caller's** locals from its slots. Those are current because of the spill.
  This is why the spill before the call is required, not just convenient.
- **A callee that exits early without error** (a closure-run statement, an element of another shape, `return` in
  the glue): the callee is finished in the closures inside its frame, `Site::call` returns `Ok(Unit)`, and the
  caller continues natively at the next instruction. No token travels up the native stack.
- **Rust panics** (engine bugs) unwind into an `extern "C"` frame and abort, as for every helper today.

## 6. Hot reload

Machine code is off under `--hot`: `OsrSite::armed` requires `it.hot.is_none()` (resolved/osr.rs:157-159). `hot` is
set once, before `call_main` (hot.rs:64-65), so no caller unit, and hence no `call_unit`, runs in a hot session.
`call_unit` uses the same `Site` as the closure engine, so it resolves the callee exactly as the compiled call does,
and a swapped body is a new `FuncDef` with a new `CompiledFn` (`compiled` is keyed by the `FuncDef` pointer,
mod.rs:76). If machine code is ever allowed under `--hot`, `call_unit` must call `hot_safe_point` first and compare
`site.f` with the module's current function by `Rc::ptr_eq`, falling back to the statement's closure code on a
mismatch (call.rs:774-776 is the walker's safe point). A debug assertion `it.hot.is_none()` in `call_unit` pins the
current rule.

## 7. Side effects, shadowing, closures, threads

- **Argument side effects and order.** Arguments are evaluated by the `Site`'s codes in `fill_frame`, left to right,
  all of them, as `expr::call` does. A native call adds nothing before them except the spill writes and `mark`. A
  call whose argument calls a function (`f(v, g())`) is still native. Its `ArgInfo` has an `Other`, so only the
  handoff is off.
- **Shadowed names.** Resolved by the closure compiler (1.1). A parity program defines `let advance = |..| ..`
  inside the caller and calls it in a hot loop.
- **Closures calling the function.** A closure body is compiled by the same `Cx` with `closure_body: true`
  (compile.rs:222-223). Its loops can be units, and their call statements get `CallReg`s the same way. A call
  *through* a closure value (`call_value`, expr.rs:426-436) is not a module-function `Site`, so it is never native.
- **Threads and tasks.** One OS thread runs the interpreter at a time under the baton (F15, interp-struct-vec-step0a.md).
  `call_unit` never yields. A switch can happen only inside closure code in the callee (a builtin reached through
  `run_stmt` or through `level` after an exit). By then the caller's views are cleared, and the adopted views sit
  in the callee `Ctx`, where `run_stmt` clears them before the builtin runs. The `Handoff` is a Rust stack
  value on the task's own native stack, reachable from no `Interpreter` field, so another task cannot see it. A
  mid-call stash in `Interpreter` (keyed by frame base and site) was considered and rejected. `rstack` is swapped
  per task, so a base can repeat across tasks, and the walker's deopt path runs user code before any stash could be
  consumed.

## 8. JIT-to-JIT exits and counters

There is no JIT-to-JIT exit. A callee exit is finished in Rust inside the call (5). The caller's statuses are
`run_stmt`'s: 0 means continue, 1 means leave after the call with `raised`, or a spilled variable no longer has its
type. The coordinator's status 1 ("closure code ran, clear views") is not needed, because the call always clears.

| Counter | Meaning | Where |
|---|---|---|
| `callunit` | a native call completed (status 0) | `call_unit` |
| `fnunit` | a function unit ran (existing; on nbody it stays one per `advance` call) | `enter_body` |
| `rowhandoff` | a row or packed view adopted by a callee | `execute` |
| `rowpack` | unchanged (step 4) | |
| `jitrefused` | must drop to 0 on nbody | `unit_for` |

`WJ_JIT_DEBUG` prints `[jit] loop <file>:<line>: compiled (native calls: advance)`. `fast_off("callunit")` makes the
lowering emit `run_stmt` instead, which gives the A/B switch and the `nofast-callunit` debug mutant.

## 9. Implementation plan

Each step is separately mergeable, with byte-identical scaling outputs (1x, 10x, 100x), green engine parity, JIT
loops, JIT differential and fast-path tests, and the step's mutants killed.

### Step 2.0: refactor, no behaviour change

- `Rc<Site>` plus `Site::call`; `CallReg` registration; `CompiledFn::fn_unit` plus `fn_unit_run`;
  `run_compiled_at_with`; a handoff parameter threaded to `execute` and always `None`.
- Gate: all suites unchanged. `fnunit` on nbody unchanged (19,744). Every perf row within ±0.3%. The M5 tests
  (`jit_fnunit_boxed_params.wj`, `jit_fnunit_param_writes.wj`) stay green. A release mutant that moves the hook
  before the prologue inside `fn_unit_run` must still fail them.

### Step 2a: native call, no handoff

- `Lo::call_stmt`, `Plan::calls`, `rt::call_unit`, the natives count, the `callunit` counter, `fast_off("callunit")`.
- Parity programs (`tests/engine_parity/jit_callunit_*.wj`, each run walker vs resolved, plus a fast-paths
  assertion on `callunit > 0`):
  1. `basic`: a mini n-body, 2,000 steps, `advance` called natively;
  2. `panic`: the callee goes out of bounds on call k: message, location, backtrace, failing-frame locals;
  3. `deep`: native self-recursion to the stack limit: same error text and first backtrace lines;
  4. `push`: the callee pushes to the passed vector between its loops, and the caller reads `len` after each call;
  5. `half`: the callee exits to the closures half way (an element of another shape) and returns;
  6. `alias`: `f(v, v)` (both `&mut`), and `r = &mut v` used by the caller after `f(v)`;
  7. `mutscalar`: an inferred `&mut` scalar argument incremented by the callee;
  8. `cold`: the caller loop gets hot before the callee; the callee's classification changes mid-run;
  9. `recursive`: self and mutual recursion of unit functions with loops;
  10. `shadow`: a local closure named like the module function;
  11. `order`: `f(v, g())` where `g` prints, so the order of effects is visible;
  12. `tasks`: the callee does a channel `send`/`recv` (baton switch) inside a hot native call, from two tasks;
  13. `return`: an early `return` in the callee's glue.
- Debug mutants (`WJ_JIT_MUTANT`, `cfg(debug_assertions)`):
  - `callnospill` (skip the spill: the callee sees a stale scalar argument, killed by 1 or 7);
  - `callnoreload` (killed by 7);
  - `callkeepviews` (skip the clear at the call: killed by 4);
  - `callswallow` (return 0 on `Err`: killed by 2);
  - `callnomark` (skip `mark`: the backtrace line differs, killed by 2).
- Release-observable mutants (patched release builds, as M1-M7 in REPORT-fnunit2.md): the same five, plus
  "count every closure-run call as native". That last one is behaviourally equivalent and is pinned by a
  fast-paths test that a loop calling a function with a declared return stays refused.
- Perf: nbody `jitrefused` 0, `callunit` about 19,999 at 1x, marginal Ir/step measured with
  `interp_instr.wj --marginal`. Projection: 10,500 to 10,900 (the step loop's closure dispatch is removed; the
  spill/reload of `step` is added). Other kernels within ±0.3%.

### Step 2b: view handoff

- `take_offers`, adoption and hand-back in `execute`, `Ctx::parked`, `Ctx::clear_views()`, the debug `fill`
  re-check.
- Parity programs:
  - 14. `parked`: in the caller loop, a closure-run `let b = v[0]` between two calls, then `b.x` printed;
  - 15. `unique`: `f(v, v)` with one `&mut` and one by-value parameter, the by-value copy printed;
  - 16. `readview`: a sieve-like read-only packed view handed to a callee that writes;
  - 17. `earlyexit`: the callee builds a view, exits to the closures, clones an element into another vector,
    and returns.
- Mutants (debug and release):
  - `parkstale` (`clear_views` keeps `parked`: killed by 14);
  - `handoffunique` (no `strong_count` check: killed by 15);
  - `handoffkey` (drop `READ_VIEW`: killed by 16);
  - `handbackearly` (hand back at every exit: killed by 17);
  - `handoffguard` (no `ptr_eq`: expected to be killed by 6; if it survives, prove equivalence or pin it with a
    refusal counter, as M2 was in [interp-soundness/fnunit2.md](interp-soundness/fnunit2.md)).
- Perf: `rowhandoff` about `callunit - 1` on nbody, and the `rows::fill` call count per run (callgrind calls) drops
  from about 20,000 to about 2 at 1x. Projection: a further 900 to 1,300 Ir/step, so **about 9,300 to 10,000 Ir/step**
  after step 2.

### Step 2c (only if measurement asks): callee unit cache

Keep the last `(sig, Rc<Unit>)` on the `CallRun`, check it with `sig_is`, and skip the `site.units` borrow and the
`Weak` upgrade. Projection: 200 to 400 Ir/step. Gate: `unit_for`'s debug assertion also runs on cache hits.

## 10. Assumptions not verified by reading

| # | Assumption | How to verify |
|---|---|---|
| A1 | No test pins the exact depth at which "recursion too deep" fires (conformance `177_deep_recursion.wj`, `bug_mutual_recursion_free_fns_codegen_must_not_stack_overflow_test.rs`, `self_field_mutation_recursion_test.rs`) | read the three tests; run parity 3 under both engines |
| A2 | (verified) `coerce` of a `Value::Ref` for a `&mut T` parameter returns the `Ref` unchanged (coerce.rs:127-133); for an inferred `&mut` (declared `Vec<T>`) the `Ref` reaches `_ => Ok(v)` (:134) | the adoption `strong_count` check would catch a violation anyway |
| A3 | (verified) `tail_hint` is false when `f.ret == None` (compile.rs:432-434), so `expect_tail_type` never runs for a function-unit callee | none |
| A4 | The lowering never rebinds a slot that holds a `Ref` (`Lo::assign` refuses a `Bind::Handle` identifier target, lower_expr.rs:678-681: verified for `assign`; to check for `let` shadowing of a parameter slot) | read `Lo::native`'s `let` arm with `reg.let_slot` |
| A5 | `it.cur` after a native call equals the closure engine's (`Site::call` marks the callee, as the closure does) | parity 2 compares backtraces |
| A6 | The eight view-clear sites are all the sites (`grep "clear_rows\|views = Default"` in jit/, as of 529cda57f) | `Ctx::clear_views()` makes it one site; a source-level test fails on any other assignment to `views` or `rows[..].key` |
| A7 | `map` lean cells ([interp-soundness/maplean.md](interp-soundness/maplean.md)) need no handoff and are per `Ctx` | read lower_map.rs; v1 never offers them |
| A8 | The projections in section 9 | `interp_instr.wj --marginal` on the PC after 2a and after 2b |
| A9 | `Shape == Shape` is enough for a row view built under one plan to serve another (same name, field names, `Option<Ty>` per field, `copy`) | rows.rs `fill` validates exactly these; parity 1 under the debug `fill` re-check |
| A10 | Cranelift compiling a callee unit during a caller run (inside `unit_for`) does not move the caller's code (new functions get new memory) | already exercised today through `run_stmt`; keep parity 8 |

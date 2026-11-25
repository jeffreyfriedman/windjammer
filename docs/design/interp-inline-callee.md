---
status: proposal
last_verified: d16ed1a59c36
---
# Inlining small callees into machine-code units with a lazy frame (L3)

> **Proposal: not implemented.** L3 of [interp-nbody-ladder.md](interp-nbody-ladder.md) and step 5 of
> [interp-struct-vec-loops.md](interp-struct-vec-loops.md). It re-argues the inline-with-deopt design that
> [interp-soundness/p3.md](interp-soundness/p3.md) rejected, using per-exit deopt maps. File and line references are
> to `wt-rc3-int` d16ed1a59. Measured numbers are quoted from pc-results measurement files, which are named at each
> use. Numbers marked *assumed* are not measured.

## 0. Verdict

**Sound with restrictions.** The P3 hazard is that a failed check at iteration k of an inlined loop has already
committed k-1 iterations, so the exit must resume *inside* the callee. A per-exit deopt map closes it. At an exit
inside the inlined body, Rust rebuilds the callee frame from the spill buffer, using the closure engine's own entry
and exit functions. The closure engine then finishes the callee in that frame, and then finishes the caller. Section 3
shows that the rebuilt frame holds exactly what a function unit of the same callee would hold at the same exit. That
state is already argued and tested in [interp-soundness/fnunit2.md](interp-soundness/fnunit2.md).

The restrictions are what make the argument complete. v1 inlines only callees with all of these properties:

1. **Leaf and fully native.** Every statement lowers to machine code, including nested pure method inlines. There is
   no closure-run statement, so the body has no `panic!`, no print and no call that stays a call. So no user code
   runs inside the inlined body, there is no recursion, and no task switch can happen.
2. **No early exit.** No `return`, `?`, or `break`/`continue` that leaves the callee. Every exit is a failed check or
   a helper bail (section 2.4).
3. **No closures or islands.** There is no closure literal in the body, and every statement has a registered code and
   a location (the `unit_body` rule, jit/lower.rs:679-684).
4. **Unit return first.** The return type is `None` and the call is in statement position (`f(args)`). Typed
   returns come in stage S4, where the continuation is one binding.
5. **Simple arguments.** Every argument is an identifier or a literal. Section 4 gives the aliasing rule for
   aggregates passed by value.

**Not argued, and what to do about it:**

- **A call inside an arbitrary expression whose remaining work is not a plain binding** (`x += f(y) * 2`,
  `v[g(i)] = f(x)`). The caller statement cannot be resumed in the middle. S4 handles
  `let x = f(..)`, `x = f(..)` and the pure-callee case (S1). For everything else the call stays a call. The
  options are a Rust continuation per statement shape, or a mid-statement machine-code re-entry point. Neither is
  designed here.
- **A closure-run statement inside the callee.** v1 refuses to inline such a callee. An exit at that statement
  boundary would be sound (it is the same protocol as a failed check). But it would run on every call, so it is a
  performance cliff. It is allowed later, as stage S7, only for statements in cold branches.
- **Game-logic payoff is not measured.** Among the bench kernels, only nbody has a mutating user call in a hot loop.
  For the other shapes this page gives a cost model with assumed inputs, and section 9 lists the experiments that
  measure those inputs.

**Fallback priced (section 7).** The fallback is to keep row views alive across calls through the call-unit handoff.
The dynamic cache is unsound ([interp-soundness/rowcache.md](interp-soundness/rowcache.md)). On nbody the handoff
removes about 1,050-1,480 Ir/step, taking 7,531 to about 6,050-6,480. L3 removes about 5,430-5,510 Ir/step. The
handoff needs steps 2a and 2b of [interp-call-unit.md](interp-call-unit.md), so it is not cheaper to build than
S0-S3.

## 1. What a call costs today, and what L3 removes

The source is pc-results `findings/interp-nbody-e0-post-l1.md`: head e13594de5, 10x-1x marginal, per step. One step
is one `advance` call.

| Component | Ir/step | Removed by L3 |
|---|---|---|
| Closure-engine call and dispatch (`run_compiled_at`, `bind_params`, `expr::call`, `coerce`, ...) | 2,331 | all: no call |
| OSR/unit entry and exit (`row_build`/`rows::fill` 1,051, `execute`/`unit_for`/`enter_body`/`shape_is` 944) | 1,752 | all: one entry per run, `fill` once per run under L1 |
| Allocation (4.0 mallocs/step) | 830 | all (v1 measures 0 mallocs/step) |
| Boxing (`Value` clone/drop) | 739 | all |
| Machine code | 1,844 | none (1,961 when inlined by hand) |
| **Total** | **7,531** | v1 (`advance` inlined by hand) measures **2,017** |

The per-call remainder is 5,514 Ir/step, which is 73% of the step. The target for 2x node is about 2,100 Ir/step
(interp-nbody-ladder.md section 2). After L3, the projection is 2,020-2,100: v1's 2,017 plus about 10-80 for the
one-off unit entry and a per-call preheader.

## 2. The mechanism

### 2.1 Eligibility (decided at lowering)

The decision is made in `Lo::stmt_in` (jit/lower.rs:823), before the fall-back to `run_stmt`. The statement must be
`Statement::Expression { expr: Expression::Call }`, and it must carry a `CallReg`. That is the step-2.0 refactor of
[interp-call-unit.md](interp-call-unit.md) section 1.1: `Rc<Site>` is recorded per call expression by the closure
compiler. The JIT never resolves a function by name, so shadowing by a local closure is the closure compiler's
decision. The callee `f = site.f` must satisfy all of these:

- `compiled_for(f)` exists. The callee is not generic, `async`, `extern`, `main`, decorated, or `self`-first (S5
  lifts this last one). It has no `tail_hint` (compile.rs:435).
- `f` is not on the inline frame stack, and the depth is at most 1 in v1 (`MAX_INLINE`, lower_struct.rs:46, still
  bounds pure inlines).
- **Trial lowering succeeds.** The callee body is lowered into a throw-away Cranelift function. It uses the same `Lo`
  rules, and its parameters are bound as in 2.2. The trial must finish with `stmts.is_empty()` (no closure-run
  statement), no `exit_now` (no early exit), no `Closure`, and a trial exit count that the real lowering will
  reproduce. The trial exists because Cranelift emission cannot be rolled back halfway through a body. The verdict is
  cached per (callee pointer, argument classes) for one unit compile.
- Every argument is an identifier bound in the unit, or a literal. Its static `Ty` equals the parameter's declared
  type, so `bind_params` is the identity (mod.rs:288-306). Otherwise the statement stays `run_stmt`, as today.
- The aliasing rule of section 4.

The callee must also be compiled with `record_all`, so that every statement has a registered code and `level` can
resume in it. Today `record_all = fn_unit_ok(f)` (compile.rs:184-194), which also installs the function-unit hook.
S3 splits the two flags: `record_all` for every inline candidate, and the hook only for `fn_unit_ok`.

### 2.2 Lowering an inlined call

This extends what `inline_call` already does for pure methods (lower_struct.rs:185-227):

1. **Arguments** are lowered left to right in the caller's scope, by the existing expression lowering. A failed check
   here is an ordinary exit of the **caller** statement. Nothing has happened yet, and the closure engine re-runs the
   whole call.
2. **A frame is pushed.** `frames.push(FrameRec { base: scope_count, kind: Lazy, site, call_path: self.path.clone(),
   loop_barrier: self.loops.len() })`, then `path` is reset to the callee body. In Analyze mode, scopes are pushed as
   today.
3. **Parameters are bound:**

   | Parameter | Binding inside the callee | How the deopt map rebuilds it |
   |---|---|---|
   | by value, scalar | fresh `Var` with the callee slot, `frame: 1` | from its spill (immutable in the callee: a mutated parameter is an inferred `&mut`) |
   | by value, struct of scalars (caller holds `Bind::Struct`) | copied `Bind::Struct` (as `inline_call`) | `make_struct` from its field spills |
   | mutated (inferred `&mut`), scalar or struct local | **alias** of the caller's `Bind::Var` / `Bind::Struct` | re-run the `Site`'s own `place::arg` code (place.rs:90-98) against the caller frame |
   | mutated, `Vec`/map handle | the caller's `Bind::Handle(caller slot, class)` | same as above |
   | by value, `Vec`/map handle | the caller's `Bind::Handle` (no clone) | the `Site`'s `expr_arg` code, allowed only under section 4's rule |

4. **The body is lowered statement by statement**, with `path` levels over the callee lists. Every callee `Var`,
   `Spill` and `StructSpill` carries `frame: 1`, and its `slot` is relative to the callee base. **Slot numbers of two
   frames are namespaced by `frame`** everywhere they are compared: `defs(L)` of the hoist analysis, `in_scope`, and
   the `fresh` and boxed rules of `run_stmt`.
5. **Loop barrier.** `jump_out` (jit/lower.rs:755-774) resolves an unlabeled `break`/`continue` to the innermost
   loop in `self.loops`. That can be a *caller* loop when the callee has no loop around the statement. Inside a frame,
   `jump_out` fails (and so does the trial) when the target index is below `loop_barrier`.
6. **The end of the body** falls through to a continuation block. Callee scopes are popped, `frames.pop()`, `path` is
   restored to `call_path`, and the call statement counts as native.

No helper call marks the inlined call itself. A per-`Ctx` counter `inline_calls += 1` (one add) feeds the
`inlinecall` counter at unit exit, as `row_hits` does.

### 2.3 The deopt map

`Exit` (jit/lower.rs:53-64) gains one optional record. A caller exit has `inline: None` and is unchanged.

```text
Exit {
  path, spills, planned, after, structs,         // unchanged; for an inlined exit: the CALLEE's path and the CALLER's spills
  inline: Option<InlineExit>,
}
InlineExit {
  site: Rc<Site>,                 // callee FuncDef, Rc<CompiledFn>, argument codes, call location
  reg: Rc<Reg>,                   // the callee's registry (slot kinds, statement codes, blocks, for_info)
  callee_spills: Vec<Spill>,      // frame 1: slots relative to the callee base
  callee_structs: Vec<StructSpill>,
  params: Vec<ParamSrc>,          // per parameter: Spill(buf, ty) | Struct(StructSpill) | ArgCode(k)
  caller_after: Path,             // the caller path, the call statement's idx + 1
  cont: Cont,                     // S3: None. S4: Let { slot, kind } | Assign { slot, kind }
}
```

`Lo::in_scope` splits the bindings at `frames[0].base`. Bindings below it are the caller's spills, as today. Bindings
above it are the callee's spills. It no longer drops the callee half (today it cuts at `frames.first()`,
jit/lower.rs:489, which is right only for pure inlines). An exit inside a **pure** inline nested in an inlined callee
is an exit of the enclosing callee statement, which re-runs from scratch. That is the existing pure-inline rule, one
frame down.

### 2.4 The exits an inlined body can take, and how each is rebuilt

All of them return from the machine code to `osr::execute` (jit/osr.rs:279-349). `execute` flushes the pending push
length first (protocol 2 of [interp-soundness/pushlen.md](interp-soundness/pushlen.md)), then writes back the
caller spills.

| Exit inside the inlined body | Where it comes from | Rebuilt as |
|---|---|---|
| Integer overflow, divide by zero, `MIN / -1` | `int_arith` checks (`bail_if`) | callee frame, resume at the failing callee statement; its closure code raises the walker's panic |
| Index out of bounds, `bad_v` of a hoisted index | `bail_if(oob)` (hoist.md 1.2-1.3) | same |
| Guard failure: element of another width or shape, a field of another variant, a row/packed view that cannot serve | `bail_if` after a kind check or a fallible helper | same; the closure code continues without error (e.g. `jit_loop_speculation.wj`'s mixed widths) |
| Helper bail (`ctx.fail`) from `index_get`, `index_field_*`, `field_*`, `map_*`, `int_op`, `push` | `Lo::call(.., fallible: true)` | same |
| Closure-run statement, `return`, `break` out of the callee, user `panic!` | not possible in v1 (refused by the trial) | none |
| Budget or step limit | not an exit: limits tick only in the walker (`code_mode.rs:84` turns the resolved engine off), and machine code has no tick | none |
| Stack: `stack_room(INLINE_STACK)` | checked at unit entry (jit/osr.rs:295-297), before anything runs | the unit does not run |
| Live-in no longer of its class | at entry, before anything runs | the unit does not run |
| Rust panic in a helper | engine bug; aborts through `extern "C"` | none (as today) |

Every exit in the table is taken **before its statement has any effect**. That is the existing lowering invariant:
checks precede stores, and a helper that fails has stored nothing. So the callee statement re-runs from its start.

### 2.5 The materialiser (`osr::finish_inlined`, Rust, after `execute` returns)

These steps run in this order. Each one is the closure engine's own code, or a write the fnunit exit path already
does.

```text
1  write the CALLER spills/structs to the caller slots        (as any exit: execute, jit/osr.rs:327-332)
2  it.mark(site.loc)                                          (the call location: expr.rs:420, the callee name)
3  base = rstack.len()
   for each parameter k:  push  value(buf[i], ty) | make_struct(..) | site.args[k].eval(it, b_caller)
   rstack.resize(base + c.nslots, Unit)                       (= fill_frame, expr.rs:552-567, with spilled by-value args)
4  own = enter_compiled(f, base)?                             (depth_check, backtrace.push((qual, cur)), walker scope, depth += 1)
   debug_assert!(bind_params(f, c, base) == Ok(true))         (identity by 2.1)
5  boxed-parameter prologue: island::boxed on c's boxed params (compile.rs:281-289; BEFORE any spill: fnunit2 M5)
6  write callee_spills/callee_structs at base + slot with reg.kind (osr::write, `fresh` as recorded)
7  r = level_body(it, base, callee_reg, exit.path, buf)        (osr::level, jit/osr.rs:543; S4: returns the tail value)
8  if is_panic(&r): poison_slots(it, base, c.poison_top)     (the `poisoning` wrapper, compile.rs:583-594)
9  if let Err(Ctl::Err(e)) = &mut r: attach_slot_locals(e, &c.names, base)   (run_body, mod.rs:139-143)
   r = finish_value(f, c, r)                                  (annotate: span + backtrace snapshot WITH the callee entry)
   leave_compiled(base, own, c.scalar)                        (truncate, depth -= 1, scope pop, backtrace.pop)
10 match r { Ok(v) => apply cont (S4), raised = None ; Err(e) => raised = Some(e) }
11 level(it, b_caller, caller_reg, exit.inline.caller_after, buf, 0, raised)   (the caller's rest in the closures)
```

Items 4 and 9 are the closure engine's `run_compiled_at` with its body replaced by "resume at statement k". Item 3
is `fill_frame` with the by-value arguments taken from the spill buffer, because the caller variable they came from
may since have been written through a `&mut` argument of the same call (`f(x, x)`). The place arguments are
re-resolved by the `Site`'s own code. Their result depends only on the identity of the caller's cells, which nothing
in a leaf callee can change: a callee cannot name a caller local, and rebinding a parameter is callee-local.

### 2.6 How the caller's exit composes

The machine code has already returned, so the caller unit has exited. Its state was written in step 1, exactly as
for any exit. Its resume point is `caller_after`: the call statement's path with `idx + 1`, which is the `after`
convention of `run_stmt` (jit/lower.rs:561-563). The difference is that the caller spills are written by this exit,
not by a preceding helper. `level` already gives `raised` its meaning at the innermost level: "the statement before
`idx` raised: the block ends with it" (jit/osr.rs:569-570). So a callee error unwinds the caller's blocks, and its
`for` loops through `close_block`, as the closure engine does after a failing call statement. With `raised = None`,
the caller continues in the closures from the statement after the call. An enclosing loop's next iteration may
re-enter the unit through its OSR hook. `MAX_UNPLANNED` (jit/osr.rs:29) counts the inlined exit like any failed
check.

### 2.7 Why the rebuilt state is the closure engine's

Compare the inlined run with an **eager** run of the same callee as a function unit, entered by the closure engine
with the same arguments (fnunit2's path):

1. Between entry and an exit, a function unit writes **nothing** into its own frame slots. It holds every local in
   registers, and only exits and closure-run statements write slots. A leaf callee has no closure-run statement. So
   at its exit the eager frame is: the `fill_frame` arguments, the prologue cells, and `Unit` in every other slot.
   The materialised frame is built the same way (steps 3-5).
2. The exit writes the same spills with the same `Reg` kinds and the same `fresh` flags (step 6 = `execute`'s
   write-back with the callee `reg`).
3. Heap effects up to the exit are the same stores through the same views into the same storage. A callee handle
   parameter is the caller's handle in both runs (`Value::Ref` to the same cell, `place::arg`). In the inlined run,
   the views are the caller's `Ctx` views keyed by the caller slot. In the eager run, they are the callee `Ctx`'s
   views keyed by the parameter slot. Both are built by `rows::fill` under the same uniqueness check, and section 4
   covers the one case where the eager run creates a clone that the inlined run does not.
4. Interpreter state outside the frame: `depth`, `backtrace`, `scope` and `frames` are set by `enter_compiled` in
   both runs before any callee statement's closure code runs. `cur` is set by `mark` (step 2, then `statements`).
   [interp-soundness/fnunit2.md](interp-soundness/fnunit2.md) already does not claim the value of `cur` after a unit
   run, and `level` re-marks before every statement.
5. From the resume point on, both runs execute the same closure code (`level` over the callee `Reg`), then the same
   `finish_value`/`leave_compiled`, so errors, locals (ADR-0036) and backtraces are identical. The locals are printed
   by `attach_slot_locals` over the callee's slots, so the format is the resolved engine's. That includes the known
   struct-vec `..` form, which is canonical (the gates compare resolved against resolved, section 8).

What an inlined call that ends **normally** leaves out (frame push and pop, depth, backtrace, the walker scope, the
prologue cells, the callee's `calls` and OSR tick counters) is invisible unless something reads it between entry and
end. In a leaf, nothing does: there is no user code, no `depth_check`, and no error. The `stack_room` check at unit
entry is what makes eliding the callee's `depth_check` sound. With a stack budget, the inlined path uses less native
stack than the closure-engine call, and 1 MiB of headroom is required. With the depth counter, `depth + 64 <=
max_depth` and the inline depth is at most `MAX_INLINE`. So the closure engine's check could not have fired.

## 3. P3's obligations, one by one

[interp-soundness/p3.md](interp-soundness/p3.md) lists six things a deopt inside the callee must do. Each one maps to
a step above that reuses existing code:

| P3 obligation | Here |
|---|---|
| (1) push a callee frame sized `nslots`, params bound as `bind_params` does | step 3 + 4 (identity bind by eligibility; debug assert) |
| (2) callee spills to callee slots with callee kinds | step 6, `frame: 1` spills, callee `reg` (mutant `inlspill`) |
| (3) register the frame as `enter_compiled` does | step 4 is `enter_compiled` itself (mutants `inlbt`, `inlmark`) |
| (4) resume `level` over the callee path | step 7 with the callee `Reg` (mutant `inlpath`) |
| (5) `finish_value`/`leave_compiled`, locals | step 9, in the `run_body` order |
| (6) continue the caller after the call | step 11, `caller_after` with `raised` (mutant `inlrestart` = the P3 hazard itself) |
| "exit records need a frame index per spill and per path level" | `frame` on spills; the path is callee-only, the caller's is `caller_after` (one record, not interleaved levels) |
| "`in_scope` must span both frames" | 2.3 |
| "`Plan::inlines` must cover depth accounting" | 2.7, last paragraph |

The step the P3 note could not afford was proving this equal to the closure engine on every exit. Section 2.7
reduces that to the function unit's exit, which fnunit2 has already argued and pinned with mutants.

## 4. Views across the inlined call, and aliasing

[interp-call-unit.md](interp-call-unit.md) section 2.3 moves views across a native call through a `Handoff` and
re-validates them on adoption. When the callee is inlined there is no call, and the handoff becomes the identity.
The callee's handle parameters *are* the caller's bindings, so its accesses use the caller's views in the one `Ctx`.
Each adoption condition becomes a lowering-time fact or an existing run-time rule:

| `call_unit` adoption condition | Inlined equivalent |
|---|---|
| exactly one parameter matches the offered cell | a caller slot passed twice with at least one mutated parameter refuses the inline (static); two distinct caller slots that alias one cell at run time are two views of one memory, cleared together ([interp-soundness/fnunit2.md](interp-soundness/fnunit2.md) M2) |
| same body pointer and length | trivially: no Rust code runs between caller and callee |
| `Rc::strong_count(body) == 1` (the `f(v, v)` hazard, call-unit 3.3) | **by-value aggregate rule, below** |
| shape equality | same `Shape` objects: one plan |

**The `f(v, v)` hazard in inlined form.** In the closure engine, a by-value `Vec` parameter is an `Rc` clone of the
body. A write through another `&mut` parameter to the same storage then copies on write, and the by-value copy keeps
the old contents. Inlining binds the by-value parameter to the caller's handle without a clone, so it would see the
write. Distinct caller slots can alias at run time (`let r = &mut v`), so the static slot check is not enough.
**Rule (v1): a by-value aggregate (`Vec`, map, or an opaque struct) parameter is inlined only if the trial lowering
of the callee emits no store, `push` or map insert through any handle.** Then nothing writes while the by-value alias
is alive, and reading shared storage is reading a copy. A by-value struct of scalars is always copied field by field
(`Bind::Struct`), so it needs no rule. Mutant `inlbyval` (drop the rule) must be killed by
`engine_parity/jit_inline_byval_alias.wj` (`f(v, r)` with `r = &mut v`, the callee writes `r` and prints `v`'s copy).

**`let b = bodies[i]`.** This is a closure-run statement, so it is never inside an inlined callee (rule 1). In the
caller it clears every view and shares the element. While `b` lives, `fill` fails on that element, so no view is
built and the helpers copy on write. Inlining creates no `Rc` clone of a body or an element (F14): by-value structs
are field copies, and by-value aggregates are borrowed under the rule above. So it cannot make a view where the
closure engine's argument clone would have mattered.

## 5. Interactions

- **Function unit (fnunit).** An inlined callee that has loops no longer enters its own function unit on this path.
  Its other call sites keep it. The materialiser does not enter the callee's unit: `level` runs closure code, and an
  inner loop of the callee may OSR into its own loop unit, as after any fnunit exit.
- **Hoist (L1).** The analysis pass lowers inlined bodies inline, so `safe` already covers them (hoist.md 1.4:
  "including nested loops and inlined calls"). `defs(L)` must compare `(frame, slot)`, because a callee slot number
  can equal a caller index variable's slot. Without the frame tag the error is conservative (fewer hoists), but
  mutant `inlslotns` checks it both ways. With L1 and L3 together, the step loop and the inlined `advance` loops form
  one view-stable nest, so `rows::fill` runs once per run. That is the v1 measurement.
- **pushlen.** No helper boundary is added, and every materialisation runs after `execute`'s flush. `pushscan`
  classifies per loop on the AST, and an inlined call statement in a loop body is not a push statement, so the loop
  is conservatively not push-only. A push-only loop *inside* the callee is keyed by the caller slot through the
  aliased handle. The trial lowering must produce the same push eligibility as the real one, which a debug assert
  compares.
- **vec-inline views.** These are unchanged ([interp-soundness/vecinline.md](interp-soundness/vecinline.md)): a
  view exists only on unique packed storage, and every mutation clears it.
- **Hot reload.** Machine code is off under `--hot` (`OsrSite::armed`). Inlined code bakes in `site.f`'s body. If
  units are ever allowed under `--hot`, the unit entry must check `Rc::ptr_eq(site.f, module's current f)` for every
  inlined site. A debug assert `it.hot.is_none()` in `finish_inlined` pins today's rule.
- **Tasks and the baton.** A switch happens only inside builtins, which run through closure code. A leaf inlined
  body has none. The materialiser's `level` may switch, but by then the frame is real and the `Ctx` is gone, as
  after any exit.
- **Recursion limits.** The callee's `depth_check` is elided on the normal path (2.7). On the deopt path it runs for
  real in `enter_compiled`, with the same message ("stack overflow: recursion too deep for the interpreter stack",
  call.rs:707). It cannot fire there by the same bound, and a debug assert checks that.
- **Closure-run statements inside the callee.** v1 refuses the inline (rule 1). S7 may allow them in cold branches
  as an unconditional exit at that statement boundary. That needs the same deopt map and a planned-exit budget.
- **Typed returns.** In S4 the materialiser needs the tail value. `level` discards statement values
  (`statements`, jit/osr.rs:577-583), so `level_body` must return the last expression statement's value when the
  outermost level runs it, as `seq` does (compile.rs:491-502). `Body::I` callees also need their `Step::Guard`s
  registered: guards are not `record_stmt`ed (compile.rs:455-456), so `level` cannot resume at one. They also need
  `finish_int` instead of `finish_value`. S4 therefore starts with `Body::V` callees (float, struct and unit
  returns), and `Body::I` is S4b.

## 6. Beyond nbody: what each kernel and shape would gain

Kernel numbers come from pc-results `findings/interp-nbody-e0-post-l1.md` (marginal Ir per unit) and
`findings/interp-scaling-2026-11-24-rerun/scaling-rc3head.txt` (steady resolved/node).

| Kernel | Marginal Ir/unit now | User calls on the hot path | L3 change | Why |
|---|---|---|---|---|
| nbody | 7,531 per step | `advance` from the step loop | **to about 2,020-2,100** | S3; v1 measures the end state (2,017) |
| fib30 | about 36 per call, an upper bound* | recursive `fib`, typed `int` return with a guard | **0** | already native: the integer function compiler (jit/mod.rs) runs the whole call tree in machine code after `HOT` = 64 calls; steady resolved/node is 0.61-0.64x |
| closures | 30,176 per element (300 rounds) | closures called by the iterator drain in Rust | **0** | not called from a unit; already 0.31x node; the typed scalar pipeline ([interp-soundness/pipeline.md](interp-soundness/pipeline.md)) is the mechanism there |
| strings | 951.2 per iteration | none (loop refused: `String` methods) | 0 | |
| hashmap | 373.6 per iteration | none | 0 | |
| sieve, loop5m | 107.1, 36.86 | none | 0 | |
| structs | not in the e0 table | pure methods `add`, `dot` | 0 | already inlined by `inline_call` |

\* (warm fib30 101,790,350 Ir - warm startup 4,309,252) / 2,692,537 calls, from pc-results
`data/instr-baseline-rerecord2.patch`. It includes parsing and compile, so it is an upper bound. The coordinator's
"3.19M Ir per unit" for fib30 is not in the e0-post-l1 tables, and experiment E1 measures it.

**fib30 and typed or recursive functions.** Typed returns do not block fib30, because the int compiler handles fib.
They do block L3's resume path for `Body::I` callees (guards are unregistered, and the tail value is needed), which
is S4b's work. Self-inlining one level inside the int compiler would cut native call overhead, which is a minority of
about 36 Ir. It is not worth doing for a kernel already ahead of node. E1b prices it. A **method JIT for non-int pure
functions** is a separate mechanism (M-pure). It would extend `jit/mod.rs`'s `Tr` from `i64` to `f64` and `bool`, and
take struct-of-scalars arguments as fields. It is sound by the same "no side effect, bail means re-run the call"
argument, and it applies to calls from *any* closure code, not only from units. A recursive non-pure function (tree
walks) would need the frame-free native call variant, with the same deopt map and a native exit propagation through
the caller's machine code. Hot game loops rarely recurse (*assumed*), so that is deferred.

**Game-logic shapes** (many small functions, struct arguments, `&mut` struct parameters, `Vec<struct>` state,
method calls). Today a user call in a unit loop is a closure-run statement. The loop is refused outright when
closure-run statements are at least as many as native ones (jit/lower.rs:401-402). A `for i in 0..n { update(ents,
i, dt) }` loop is therefore not a unit at all.

Per call today (*assumed* components, which E3 measures):

```text
T_call  = R (run_stmt spill/reload + view clear, 50-200)
        + K (closure-engine call + frame: nbody measures 2,331 for a &mut-Vec callee; a scalar leaf 800-2,300 assumed)
        + B (callee body in the closures, 40-150 per simple statement assumed; a callee with a loop instead pays
             a fnunit entry, 1,752 on nbody, plus allocation and boxing, 1,569 on nbody)
        + V (caller views rebuilt after the clear: about 150 + 30 per element per row used, assumed from nbody's 1,051 for 5x7)
T_after = B_native (3-10 per simple statement) + 0 per call
```

| Shape | Example | Stage | Expected saving per call (*assumed*) |
|---|---|---|---|
| G1 pure math helper in an expression | `let d = dist2(x, y, cx, cy)` (`-> f64`) | **S1** (pure free-function inline, restart-statement exits) | 1,000-2,800, plus the loop may become a unit |
| G2 `&mut` struct local, float body, no checks | `nudge(p, dt)` | S2 (exit-free) | 1,000-2,800 |
| G3 `&mut Vec<struct>` + index, bounds checks | `apply(ents, i, dt)` | S3 | 2,000-5,500 (nbody: 5,514 incl. per-call `fill`) |
| G4 typed result of a mutating or checked callee | `let e = take_damage(ents, i, d)` | S4 | as G3 |
| G5 method on an element | `ents[i].update(dt)` (`&mut self`) | S5 (element-place arguments, `self` methods) | as G3, plus it removes `index_get`/clone traffic |
| G6 helper calling helpers | `update` calls `clamp`, `len2` | S1 inside S3, then S6 (depth 2) | sum of the above |

At 1,000 entities and 3 helper calls each per frame, 1,000-2,800 Ir per call is 3-8.4M Ir per frame. At
interp-nbody-ladder.md's 0.084-0.092 ns/Ir, that is about 0.25-0.8 ms per frame (*assumed*). **The broadest payoff
is S1, then S3.** S1 is pure helpers, the most common call in game math. It needs no new exit protocol, and its exits
restart the caller statement as `inline_call` already does. S3 is the protocol itself, and everything after it
reuses it. This section's own risk: game loops written as `for e in ents` or `let p = ents[i]` are not native today
(non-range `for`, struct-valued index `let`), and inlining does not change that. E3 says how much game code those two
forms block.

## 7. The fallback: row views kept across calls (handoff only)

| | Persisted views via `call_unit` 2a+2b | L3 S0-S3 |
|---|---|---|
| nbody saving | `row_build` 953 + `rows::fill` 98 + `shape_is` 171 = about 1,220; plus `unit_for` 255 only with a 2c cache: **1,050-1,480 Ir/step** (to about 6,050-6,480) | **about 5,430-5,510** (to about 2,020-2,100) |
| What it leaves | the closure-engine call (2,331), the unit entry (`execute`, `enter_body`), allocation and boxing (1,569) | machine code only |
| New soundness | adoption re-validation (call-unit 2.4, argued) | the deopt map (this page) |
| Size of work | 2.0 + 2a + 2b (call-unit section 9) | 2.0 + S0-S3 |
| Game shapes | only callees with loops over row-view parameters | every leaf callee |

The source for the nbody rows is pc-results `findings/interp-nbody-e0-post-l1.md` section 4. The handoff is not
cheaper to build than L3's first three stages, and it reaches about a quarter of L3's saving. Build it only if S3's
deopt gates cannot be closed. 2a's frame half is not needed by this design: the materialiser calls
`enter_compiled`/`leave_compiled` directly. Only the step 2.0 `Rc<Site>`/`CallReg` refactor is shared.

## 8. The staged ladder

Gates common to every stage:

- the `engine_parity` corpus, run twice: walker against resolved, and **resolved against resolved** with
  `WJ_JIT_MUTANT=nofast-inlinecall`, so the resolved form of failure locals is the reference;
- the eager run (`WJ_JIT_LOOP_HOT=0`);
- `interpreter_fast_paths_test` counters;
- each listed mutant killed by at least one named program (release-observable unless marked debug-only);
- callgrind marginal Ir on the kernels.

| Stage | Scope | New programs (`tests/engine_parity/`) | Mutants | Counters | Expected Ir | Stop here if |
|---|---|---|---|---|---|---|
| **S0** | step 2.0 refactor (`Rc<Site>`, `CallReg`, `expr_names` descends into call arguments); trial lowering and eligibility verdicts printed by `WJ_JIT_DEBUG`; `frame` field on `Var`/`Spill`; no behaviour change | none (corpus parity unchanged) | none | `inlinecand` (debug) | 0 | E3 shows fewer than a quarter of game-shaped hot calls are eligible |
| **S1** | pure **free** functions inlined anywhere in an expression (`inline_call`'s rule extended from methods to `Site`s: `let`s plus a tail, no parameter mutation); exits restart the caller statement (existing argument, [interp-soundness/p3.md](interp-soundness/p3.md) "what the existing method inline relies on") | `jit_inline_pure_fn.wj` (float/int/struct returns, a trap in the callee with its backtrace, a shadowing local closure of the same name, recursion refused) | `inlpurebarrier` (no loop barrier), `inlshadow` (resolve by name) | `inlinepure` | G1 per E3; nbody 0 | G1 hand-inlined saves under 500 Ir/call |
| **S2** | mutating callee, statement position, `ret == None`, trial lowering emits **no exit at all** (float-only bodies on `&mut` struct locals and scalars): no deopt map needed | `jit_inline_exitfree.wj` (`&mut` struct local, `&mut` scalar, `f(x, x)`, a callee loop, a `break` inside the callee's own loop) | `inlbreak` (barrier off), `inlalias` (by-value scalar re-read from the caller after the call) | `inlinecall` | G2 per E3; nbody 0 | no G2-shaped call site in the game engine |
| **S3** | the deopt map and `finish_inlined` (2.3-2.6); callees with checks, loops and handle parameters | `jit_inline_trap_loop.wj` (overflow at iteration k of an inlined loop, the failure locals show k-1 committed stores), `jit_inline_resume_mid.wj` (mixed-width element at k: non-error exit, resume in the callee), `jit_inline_oob.wj`, `jit_inline_byval_alias.wj`, `jit_inline_poison.wj` (a lock guard parameter, a panic in the resumed callee), `jit_inline_two_sites.wj` (one callee, two call sites, different argument classes), the step-2a parity programs with inlining forced | `inlspill`, `inlbt`, `inlmark`, `inlpath` (resume in the caller), `inlrestart` (re-run the caller statement: the P3 hazard), `inlbyval`, `inlslotns`, `inlpoison` (skip step 8, debug-only if no program can observe poisoning), `inlprologue` (spills before the prologue: fnunit2 M5) | `inlinecall`, `inlineexit`, `jitrefused` = 0 on nbody | **nbody 7,531 to 2,020-2,100**; G3 per E3 | nbody above 2,600 after S3 (the model is wrong: re-measure before S4) |
| **S4** | `let x = f(..)` and `x = f(..)` with `Body::V` callees; tail value from `level_body`; `Cont` | `jit_inline_let.wj` (float/struct result, a trap inside, a fresh boxed `let` captured by a later closure) | `inlcont` (skip the binding), `inlfresh` (non-fresh write of a `let`) | `inlinelet` | G4 per E3 | |
| **S4b** | `Body::I` callees: register guards as statements, `finish_int` | `jit_inline_typed.wj` | `inlguard` | | | int helpers are already served by M-pure or the int compiler |
| **S5** | element-place arguments `f(v[i])`, `&mut self` methods on struct locals and elements | `jit_inline_elem.wj`, `jit_inline_method_mut.wj` | `inlelem` (re-resolve the place with the post-callee index) | | G5 per E3 | |
| **S6** | inline depth 2 (a stack of `InlineExit`s, materialised outermost first) | `jit_inline_depth2.wj` | `inlorder` | | G6 | |
| **S7** (optional) | closure-run statements in **cold** branches of a callee: an unconditional exit with materialisation | `jit_inline_cold_print.wj` | | `inlineexit` per call under 1% on the corpus | | `inlineexit`/`inlinecall` above 1% on any game kernel |

Separate track, M-pure: extend the int function compiler to `f64`/`bool` pure functions. The gates are its own
parity program (`jit_pure_float_fn.wj`: overflow and NaN edge cases, bail and re-run), a `bail` mutant, and E6.

## 9. Measure first (callgrind only, no engine change)

Use E0's method (pc-results `findings/interp-nbody-e0-post-l1.md`): 1x and 10x, marginal = difference over work,
`split.py` categories, `WJ_JIT_DEBUG` counters.

| Exp | Run | Answers |
|---|---|---|
| E1 | fib30 at 1x/10x (`work` 1/6.85 calls ratio, sizes.toml) | the per-call marginal Ir; the share in the int compiler's machine code against closure-engine entry; confirms L3 = 0 for fib30 |
| E1b | fib30 with the recursion hand-unrolled one level (`fib(n-1)` expanded once) | the int compiler's native call overhead per call; prices self-inlining |
| E2 | closures E0 split (`call_compiled_closure`, `scalar_pipe`, `iter::advance`) | confirms no user call from a unit; what the pipeline still pays per closure call |
| E3 | game-shaped pairs, each `stock` against `hand-inlined`: G1 `dist2` in a `while i < n` over `Vec<P>`; G2 `nudge(p, dt)` on a struct local; G3 `apply(ents, i, dt)` with `ents[i].vx += ..`; G4 `let e = hit(ents, i, d)`; G5 `ents[i].update(dt)`; plus a `for e in ents` form of G3 | marginal Ir per call of each (K + R + B + V of section 6); mallocs per call; whether the stock loop is refused (`WJ_JIT_DEBUG`); how many forms are outside the unit subset whatever L3 does |
| E4 | nbody and E3 with `WJ_JIT_DEBUG`: `jitexit` per call | how often a materialisation would run (expected about 0 per call in steady state) |
| E5 | fixed Ir of `v1-inline` against `v0-base` (already 311.4M against 313.0M on e13594de5) and of each E3 pair | compile-time cost of inlining (trial lowering about doubles a callee's lowering: bound it) |
| E6 | G1 written all-`int` (int compiler applies today) against all-`f64` (it does not), called from closure code | what M-pure would save per call |

The nbody hand variants (`v0`-`v5`, `~/own/perf/*.wj`) already answer the nbody questions. E3's variants should live
beside them with the same 1x/10x pair convention.

## 10. Biggest risks

1. **Game payoff is gated outside L3.** Non-range `for`, `let p = v[i]` and struct fields holding a `Vec` keep many
   game loops out of units. E3 sizes this before S3 is built.
2. **The materialiser is invisible on the happy path.** Mistakes show only in backtraces, locals or committed-state
   counts. That is why the resolved-against-resolved gate and the release-observable mutants are mandatory at S3.
3. **Trial lowering must match real lowering** (exit counts, push eligibility, hoist versions). Debug builds compare
   them on every inline.
4. **Compile time inside the steady window** (E5). Inlining is bounded by callee size: refuse above a statement
   budget, which S0 records.

## 11. Assumptions not verified by reading

- The resolved engine never ticks `--limits`. This is inferred from `code_mode.rs:84` and from `limit_tick`'s only
  caller being `eval_expr.rs:123`. A run that sets limits without code mode is not checked here.
- `site.args[k]` for a mutated identifier argument returns an equal `Value::Ref` when evaluated twice with no
  intervening write to the caller's cell identity.
- `poison_slots` is the only observable effect of `poisoning`. A `poison_top` field must be added to `CompiledFn`.
- The section 6 game-shape components are assumed until E3.

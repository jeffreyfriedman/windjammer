---
status: current
last_verified: 5457283f4156
---
# Function unit (perf-jit-fn-unit): soundness note

Base: perf-nbody-step0 (head 6db2bc607 + counters). Code read: compile.rs, resolved/osr.rs, jit/{osr,lower,rt,rows}.rs, mod.rs.
Replaces the "tail-chained loop unit" of SOUNDNESS-fnunit.md (which needed an `osr_skip` protocol and an `end` bound).

## What is compiled
A function whose declared return is None (`f.ret == None`; so `Body::V`, no `guards`, result always `Unit` through
`finish_value`), that is not `main`, has no type parameters, and whose body holds at least one `while` or `for` anywhere in
its statement tree (if/while/for nesting). Its WHOLE body (all statements, both loops and the glue) is lowered by
`lower::lower(Root::Body)` into one Cranelift function with the same `Lo`, `Plan`, `Exit`, `Ctx`, spill buffer and
`osr::level` resume as a loop unit. Statements outside the native subset run through `rt::run_stmt` (closure code) exactly
as inside a loop unit; a statement that cannot be run either way (destructuring `let`, `defer`, `use`, a non-range `for`,
`loop`) refuses the unit (the function then behaves as today: its loops enter by OSR).
Not compiled: typed-return functions (they use `guards`/`Body::I`, their tail value would be dropped by `level`),
functions with a declared non-unit return (same), generic/extern/async functions (`compile_fn` already returns None).

## Entry
The unit is entered from the function's own body code, as the first thing after the boxed-parameter prologue
(`compile_with` builds `prologue; entry hook; rest`), i.e. after `run_compiled_at` did `enter_compiled` (depth check,
backtrace, walker frame) and `bind_params` (coercion, deopt). The frame is real, parameters are bound, boxed
parameters are in their walker slots (so `classify` sees the same `Value::Ref` cells a loop unit sees). The hook arms
after `jit_loop_hot` calls of the function (`OsrSite::armed`: jit enabled, no `--hot`, not dead) and calls `osr::enter_body`:
1. refuse (return false, cool) when two parameters resolve to the same cell (aliasing, below);
2. `unit_for`: refuse under `type_hint`; classify the parameters only (every other local is a body `let`, hence a
   `fresh` unit variable), find or compile the machine code for that classification (<= MAX_SIGS per site);
3. `execute`: read the live-ins; any parameter that no longer has the classified type -> nothing ran, closures run the body.
`poisoning(..)` (panic poisons guards) wraps the whole hook+body as before; the hook sits inside it.

## Exit / deopt state
Every exit of a function unit is an ordinary unit exit whose outermost `Level` is the function body list:
* a failed check (overflow, bounds, element type, a map/vec helper failure) exits BEFORE its statement did anything, spilling
  every variable in scope (parameters, body `let`s); `osr::level` writes them back (`fresh` boxed lets get a new walker slot),
  then re-runs the rest of the body from that statement in the closures (`statements`), nested loops through the existing
  `Level`/`ForState` machinery. Panics, messages, locations and backtraces come from the closure code of that statement.
* a closure-run statement (`run_stmt`): spills in scope, runs `reg.code(stmt)`, reloads; an error/return/break leaves after it
  with `raised` (the existing `after` exit); the rest of the body continues in the closures.
* `return` / `break` outside loops: planned exits resumed by the closure code of that statement (`Ctl::Return` unwinds through
  `level` to `finish_value`).
* the normal end of the body is a planned exit with path `[Level{body, idx: len}]`: `level` runs nothing more. At this
  exit the `fresh` spills (body lets, struct locals bound by `let`) are NOT written back: the frame dies right after
  (`leave_compiled` truncates it), scalars have no destructor, and nothing can observe the slots (no panic can follow).
  Parameter spills are kept: a `&mut` scalar parameter is a place in the caller (a boxed `Ref`), written through `write`.
The function-body level needs no block clear of its own: `reg.block(body)` stays unregistered (`close_block(.., None, r) = r`),
because `leave_compiled` releases the frame; nested blocks (`if`, loop bodies) are registered and cleared by `level` as today.
Recording: `Cx::record_all` (set for eligible functions only) replaces the `loop_depth > 0` gate of `record_stmt`, `record_let`,
`record_block` and the tail sharing in compile.rs, so the body's own statements, lets, nested blocks and tail have statement codes.
The tail is shared through a forwarding closure exactly as inside loops (its value is dropped by `finish_value`: `f.ret == None`).
`MAX_UNPLANNED` kills a unit that fails checks too often; the loop units take over.

## View lifetime rule
Row/packed/cell views live in `Ctx`, which `osr::execute` builds per run. They are built lazily (first access, `row_build`)
and are valid for ONE contiguous machine-code run: `rt::run_stmt` (any closure-run statement), `push`, `index_set`,
`index_field_set`, `field_set` clear them; `Ctx` is dropped at unit exit (the element tables go back to `Unit::spare`
as take/put-back). The function unit makes the run longer but adds NO new statement kind between two accesses that a loop
iteration did not already allow between two iterations: the only statements that run natively between loops are the subset's
`let`/assignment/`if`/loops; none of them can make an element non-unique (`let b = bodies[i]` is a physical `Rc` clone and is
closure-run, so it clears the views; while `b` lives, `fill` fails on the non-unique element and no view is built). Nothing at
a function entry carries a view over from an earlier call (a fresh Ctx has no views), so the unsound cross-entry cache of
SOUNDNESS-rowcache.md is not reintroduced.

## Aliasing of handle args
`enter_body` chases every parameter slot through its chain of `Value::Ref`s to the innermost cell and refuses the unit
when two parameters share it (`f(&mut v, &mut v)`, a boxed `&mut` param and another name of the same vec, `&mut v[0]` and
`&mut v[1]`: conservative, ignores paths). A by-value Vec argument is a clone (strong count 2): `fill` fails, no view, the
helper path (COW `make_mut`) runs, as in a loop unit. Class checks stay per-parameter.

## Recursion, nesting
A recursive call inside the body is a closure-run statement: it runs `run_compiled_at` for a new frame, whose own hook may
enter its own unit with its own `Ctx`; `Unit::spare` is taken with `mem::take` under `try_borrow_mut` (a nested run finds it empty
and allocates). One OS thread runs at a time under the baton (F15), a `Ctx` stays on its thread. Native stack depth: one
extra Rust frame set per nested entry, same as a loop unit entered in a recursive function; `depth_check` ran in `enter_compiled`.

## Traps, early return, evaluation order
Every trap is an exit before the statement's effect (checks precede stores); the closure engine re-executes the statement
from scratch, so the error, its location and its locals (`attach_slot_locals`, ADR-0036 locals of the failing frame) are the
walker's. Call arguments with side effects: a call is never native; it is a closure-run statement evaluated by the same
closure code in program order between a spill and a reload. A native expression statement has no side effects before its
check. Globals/consts/other functions: never read natively (identifier lookup fails -> closure-run or refusal).
Shadowed names: body lets get their own slots (resolved at compile time); the unit's names are scoped lexically in `Lo::scopes`,
the entry classification only covers parameters.

## Interaction with loop units, closures fallback, hot reload
The loop sites inside the body keep their own `OsrSite`s: an exit resumes in the closures, where an inner loop can enter its
own unit again (as today). Compile cost: a function is lowered as loop units first (>= 256 iterations) and as a function unit
after 256 calls: one extra compile per hot function. Closures fallback: any refusal returns `None`, the body runs in
the closures untouched. Hot reload: bodies swap at function entry and machine code is off under `--hot` (`armed`), so a
swapped function is never entered by a stale unit. Debug-only `WJ_JIT_MUTANT` paths are `cfg(debug_assertions)`.

## What is NOT claimed
`it.cur` (last marked location) after a unit run differs from the closure engine's (native statements mark nothing), the same
as after a loop unit. No `Rc` clone is made of a vec body or element by the unit (F14 holds: `entry`, `level` and the helpers
are the existing ones).

## Survivor review (M1, M2, M5)
Mutation testing left three survivors; each is now either killed by a test or proved.
* M5 (entry hook before the boxed-parameter prologue): KILLED. The mutant enters the unit with raw parameter values; a
  captured parameter is then unbound ("cannot find value `base`") and a `&mut` struct parameter loses its writes.
  Tests: `engine_parity/jit_fnunit_boxed_params.wj`, `jit_fnunit_param_writes.wj` (release build of the mutant fails both),
  with `fnunit` counted in `interpreter_fast_paths_test`. Comment at the hook in `compile.rs`.
* M2 (no aliasing refusal): behaviourally equivalent, now pinned. With the refusal removed, 7 aliased shapes (two names for
  one packed int/f64/u8 vec, a row vec with push + element stores, a map, pop/clear) print the walker's output while the
  function unit does run (`fnunit` 96). Proof: a view is a raw pointer into the vec body or its elements, so two views of
  one vec read and write the same memory; every operation that moves or resizes the body (`push`, `index_set`,
  `field_set`, any closure-run statement) calls `clear_rows`/drops views of the whole `Ctx`, not of one slot; a view
  is built only on a unique `Rc` (`fill`: `Rc::get_mut`), which two aliases of one cell satisfy together. The refusal stays
  and is PINNED by `function_units_refuse_aliased_parameters` (fnunit must be 0 on `jit_fnunit_alias.wj`), which fails on
  the mutant (fnunit 96). Comment at `params_alias`.
* M1 (end exit drops the parameter spills; also the struct variant M1s): equivalent, proof. At the end exit the frame dies
  (`leave_compiled`). A spill of a parameter reaches (a) the callee's own plain slot, dead, or (b) a boxed cell. A cell
  is observable after the end only if something else holds it: a caller place (a `Value::Ref` chain: such a parameter
  classifies `Opaque` and is never register-held, so it has no spill to drop) or a closure that captured it (a closure
  literal anywhere in a unit function makes the unit refuse: `jit_fnunit` check "outside the subset: Closure"; closure
  values passed in cannot name this frame's cells). A struct parameter is the same: `make_struct` would write a NEW `Rc`
  into the callee's slot, which the caller cannot see. Invariant: the unit never holds a caller-visible place in a
  register. Release builds of M1 and M1s print the walker's output on `jit_fnunit_param_writes.wj` (a `&mut` scalar, a
  `&mut` struct's fields, a post-loop scalar write) and on the existing corpus. Comment at `unit_body`.

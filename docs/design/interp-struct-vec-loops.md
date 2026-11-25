---
status: proposal
last_verified: ff6e395552fa
---
# Struct-of-floats loops at machine-code speed

> **Proposal: not implemented.** A design for the interpreter's Cranelift tier. Nothing here is built yet; every
> step below ships separately with its own gate.

The interpreter already runs the hot loops of n-body in machine code. What it still pays for is everything around
those loops: entering machine code twice per step, rebuilding the row views of `Vec<Body>` on every entry, and running
the step loop itself in the closure engine because it calls a user function. This page shows how to remove that
overhead soundly. The approach is to make each machine-code run longer, so that views never need to outlive one. A
dynamic cache that tries to keep views alive across closure-engine code is rejected.

The target shape is the n-body kernel (`bench/interp/nbody.wj`; the scaled copies under
`bench/interp/scaling/gen/`, with expected output under `bench/interp/scaling/expected/`): a `Vec<Body>` whose
struct has only `f64` fields, a step loop that calls `advance(bodies, dt)`, and two loops inside `advance`.

## 1. The decision in five lines

1. **Keep the invariant that makes row views sound today:** a view lives for at most one contiguous machine-code run.
   Any closure-engine code, `push`, or helper store clears it. Nothing in this design weakens that.
2. **Make the run longer instead of caching across runs.** A *function unit* compiles all of `advance` (both loops
   and the glue between them) as one run, entered once per call. A *native call* (`rt::call_unit`) lets the step
   loop's machine code call that unit in a real callee frame, so the step loop compiles and the per-step OSR entry
   disappears.
3. **Hand views across the call, not across time.** When the caller's machine code passes a vector handle to the
   callee unit, the caller's row view moves to the callee and back. No closure code runs between the two runs, and a
   pointer-equality guard proves that both name the same cell.
4. **Represent struct-of-scalar vectors as packed rows**, decided by the struct's declaration. Elements of a packed
   body have no `Rc` of their own, so the sharing hazard that ruled out the row cache cannot exist for them, and
   building a view costs O(1).
5. **Inline the callee last.** Inlining reuses the native call's frame builder as its deopt materialiser, so the
   callee-frame protocol that blocked P3 is built and tested first in a setting with no speculation.

## 2. Evidence

All numbers in this section are measured. They are quoted from the PC reports on the `pc-results` branch of the
transfer remote: `findings/interp-loss-attribution.md` (callgrind, 1x and 10x kernels, resolved engine, binary
built from `wt-rc3-int` 00630c8a1) and `findings/interp-nbody/*.md`. No number on this page is estimated by hand.
Where a saving is projected, the projection is the attribution report's own, and the report is cited.

| Fact | Value | Source |
|---|---|---|
| nbody marginal cost | 17,002 Ir per step (1 step = 10 pair interactions + 5 position updates) | interp-loss-attribution.md, headline table |
| JIT machine code share | 5,305 Ir/step (31.2%) | same, nbody category table |
| OSR entry/exit share | 5,144 Ir/step (30.3%), i.e. 2,572 per entry, two entries per step | same |
| Entry anatomy | `rows::fill` 1,866; `osr::execute` 1,000; `osr::unit_for` 801; `kinds::shape_is` 342; `rt::local_cell` 260; malloc+free 824; `Value` drop/clone 533 (Ir/step) | same, "Entry anatomy per step" |
| Other shares | allocation 1,565 (9.2%); dispatch 1,217 (7.2%); boxing 1,131 (6.7%); rt helpers 776 (4.6%); call overhead/frame setup 726 (4.3%) | same |
| Refusal | the step loop (line 76 of the 10x kernel) is refused: its body has a user-function call and `step += 1`, and the call would run in the closures | same, JIT debug counters |
| Entry count | 400,002 `rows::fill` calls at 10x (= OSR entries; 2 per `advance` call) | same |
| Node ratio | 14.2x at 100x | same, "Why node wins" |
| Older entry cost | about 2,170 Ir per entry, cold at 1x; halving entries then bounded at 6-7% of cold nbody | SOUNDNESS-fnunit.md |

The attribution report ranks three improvements for nbody and gives each a bound:

- compile the call, so the step loop is native: about 50% of marginal (about 9,000 of 17,000);
- make entry cheap (the row cache): about 19%;
- tighter pair-loop code (hoisted checks, registers): 1,500-2,500 Ir/step (9-15%).

It also bounds halving the entries at about 15% at 10x.

The row cache is stopped for a structural reason (SOUNDNESS-rowcache.md, REPORT-rowcache.md):

- A view holds the raw addresses of each element's `StructVal::fields` and relies on each element `Rc` being
  unique.
- Sharing an element (`let b = bodies[i]`, `.clone()`, passing it by value) is an `Rc::clone` through `&` access. It
  touches no `&mut VecBody`, so no body epoch can see it, and the report counts about 800 clone sites.
- The statements between two entries run in the closure engine, which has no mutation hook.
- ABA has no sound key without new static mutable state, which the static_state ratchet in `tests/architecture.rs`
  forbids.

Inlining a *mutating* callee (P3, SOUNDNESS-p3.md) is stopped for a different reason. A failed check in iteration k
has already committed k-1 iterations of stores, so the exit must resume inside the callee, in a callee frame that
does not exist yet.

## 3. What the language guarantees and what the interpreter does

Each fact below is labelled **verified** (read in the code at `ff6e395552fa`) or **assumption** (to be confirmed in
step 0 before any code lands).

| # | Fact | Status | Where |
|---|---|---|---|
| F1 | Values are copy-on-write `Rc` trees: `let b = a` is an independent copy, `.clone()` is free, and mutation through a `&mut` place copies only what is shared | verified | `src/interpreter/value.rs` module doc; `docs/INTERPRETER.md` ("Inferred `&mut`") |
| F2 | So Windjammer has no observable aliasing between two locals. The only sharing is physical (`Rc` strong count above one), and it is invisible to the program except through a store that skips copy-on-write | verified (follows from F1 and the `Place` store path) | `src/interpreter/resolved/jit/rt.rs` (stores follow `Place::with_mut`) |
| F3 | In the interpreter, `let b = bodies[i]` is a physical `Rc` clone of the element (verified in step 0a), closure-run (never native), so it clears all views; while `b` holds the clone `rows::fill` fails (`Rc::get_mut` on that element), no view is built and the helper path is used. A failed fill spends no credit, while `row_build` adds credit per call, so each miss rescans the vector up to the element that failed: a perf hazard, not a soundness one. What the compiled build does with the statement is a compiler decision the program cannot observe | verified (interpreter); compiled build irrelevant | `expr.rs`, `eval_expr.rs`, `vecbody.rs`, `rows.rs:82-125`; ADR-0030 |
| F4 | A parameter the body mutates is passed as `Value::Ref(place)`. `analysis.rs` decides which parameters, syntactically, from the function body | verified | `src/interpreter/analysis.rs` (`analyze`), `src/interpreter/resolved/expr.rs` (module-function call, `place::arg`) |
| F5 | The interpreter does not consume the compiler's IR or ownership solver. Its static facts are its own AST scans | verified | `docs/INTERPRETER.md` ("Relation to the compiler pipeline") |
| F6 | A row view is built under `Rc::get_mut` on the vector body and on every element; it checks name, field names, field types (`ty_is`) and `uid == 0` (no destructor) | verified | `src/interpreter/resolved/jit/rows.rs` (`fill`) |
| F7 | Every closure-run statement (`rt::run_stmt`), `push`, `index_set`, `index_field_set` and `field_set` clears the row views. `run_stmt` and `push` also clear the packed views and the resolved cells | verified | `src/interpreter/resolved/jit/rt.rs` |
| F8 | Views live in `Ctx`, which `osr::execute` builds per run. The element tables are recycled through `Unit::spare` | verified | `src/interpreter/resolved/jit/osr.rs` (`execute`, `Spare`) |
| F9 | A view points into heap storage (a `StructVal::fields` buffer or a packed `Vec`), never into `rstack`, so the frame stack can grow while a view is live | verified for `fill` and `packed_set` | `rows.rs`, `rt.rs` |
| F10 | A loop with as many closure-run statements as native ones is refused (`lower`: "... run in the closures") | verified | `src/interpreter/resolved/jit/lower.rs` (`lower`) |
| F11 | `run_stmt` refuses inside an inlined call: "the whole caller statement is the unit of resumption" | verified | `lower.rs` (`Lo::run_stmt`) |
| F12 | A compiled call already builds a real frame: `fill_frame`, then `enter_compiled`, `bind_params`, `run_body`, `finish_value`, `leave_compiled` | verified | `src/interpreter/resolved/mod.rs` (`run_compiled_at`), `expr.rs` |
| F13 | Loop machine code is off under `--hot` (`OsrSite::armed` requires `it.hot.is_none()`) | verified | `src/interpreter/resolved/osr.rs` |
| F14 | No rt helper keeps an `Rc` clone of a vector body or element after it returns. This is what keeps a view valid between helper calls within a run | assumption: audit every helper in `rt.rs`, `lower_struct.rs`, `lower_map.rs` | step 0 |
| F15 | One `Interpreter` is shared by several OS threads, one thread at a time under a baton; a `Ctx` is built in `osr::execute` on the calling task's thread and stays on it (task switches happen only inside builtins, reached through `run_stmt`, which clears the views). `Unit::spare` is a `RefCell` taken with `mem::take` for a run and put back after, so a second run of the same unit finds it empty and allocates; keep this take/put-back for `call_unit` | verified (step 0a) | `sched.rs:1-30, 270-282, 422-436`; `rt.rs:644-653` |
| F16 | `Class::Vec` is decided by the first element only. The row view then validates every element | verified | `src/interpreter/resolved/jit/kinds.rs` (`class_of`, `class_of_is`) |

The consequence that drives the design: **aliasing is a non-issue in the language (F1-F3) and a physical issue in the
representation (F2, F6)**. Physical sharing is created only by code that clones an `Rc`. Inside machine code no such
clone survives a helper (F14). So a view stays valid for as long as no other code runs. The design keeps it that way
and widens what counts as "no other code".

## 4. Question 1: the static region proof

### What would be needed for the cross-entry cache, and why we do not build it

Keeping a view alive across closure-engine statements would need a proof, per pair of OSR sites, that the
statements between them:

- (a) name no local that holds or reaches the cached vector;
- (b) call no user function, method or closure value (any of these could receive the vector, or capture it, and
  clone an element);
- (c) rebind no reference local (a `Ref` the unit resolved into `Ctx::cells`);
- (d) cover the loop's own exit statement and the caller's frame (for `advance`, the region between two entries
  spans the return to `main`, the step loop's `step += 1`, the condition, and the call back in).

Items (a)-(c) are decidable over the AST with the scans the interpreter already has (`osr.rs` `stmt_names`, F4, F5).
Item (d) is not local: the region leaves the function. For n-body it covers code in two functions. That is exactly
the code that sections 5 and 6 compile, so there is nothing left to prove there. A proof whose only use-case
disappears once the call is native is not worth its soundness surface.

### The proof we do build: view-quiet statements (step 6)

There is one place where a small static proof pays off: a native run that contains a few closure-run statements
that cannot touch any vector, such as a progress `println` in a step loop. Today each one clears every view (F7),
and the next access rebuilds them.

Definition, computed once at lowering time in `Lo::run_stmt` (`src/interpreter/resolved/jit/lower.rs`) and stored
on the `StmtRun` as `quiet: bool`. A statement `s` is **view-quiet** when:

1. every name `stmt_names(s)` reports is bound to `Bind::Var` (a scalar in a register), never to `Bind::Handle` or
   `Bind::Struct`;
2. every call in `s` is an intrinsic of the print/format family (`Intrinsic::Print`, `Println`, `Eprint`,
   `Eprintln`, `format`), and every argument and interpolation operand is a scalar `Bind::Var` or a literal. With
   these operands no user `Display` impl can run;
3. `s` is not a `let` or assignment whose target is a boxed slot (a `Ref` rebinding);
4. `s` contains no closure literal, `spawn`, `async`, `await`, macro or method call.

For a quiet statement, `rt::run_stmt` skips `clear_rows`, the `views` reset and the `cells` reset. It still spills
and reloads the scalars. Soundness: by (1) and (2) the closure code can reach no aggregate local and run no user
code, so it can neither clone an element `Rc` nor move or retype storage. By (3) it cannot rebind a resolved
reference. Globals in Windjammer cannot hold a `Vec` that aliases a local (F1: a global read is a copy). The rule is
deliberately syntactic and small. Anything outside it keeps today's behaviour.

Gate:

- a parity program under `tests/engine_parity/` with a view-holding loop that prints every k iterations;
- a debug mutant `quietall` that marks every statement quiet. A companion program whose loop body runs a user
  function that clones an element and then stores through the vector must produce a different output under that
  mutant.

## 5. Question 2: the function unit

### Shape

A **function unit** is a loop unit whose root is a function body instead of a `while`/`for` statement. It reuses
`lower::Lo`, `Plan`, `Exit` and `osr::level` unchanged where possible.

- **Site.** `CompiledFn` gains `fn_site: Option<Rc<FnSite>>`, built by `compile_fn` (`src/interpreter/resolved/compile.rs`)
  when the body contains at least one `while` or range `for`. `FnSite` holds `hits: Cell<u32>`, `dead: Cell<bool>`,
  `units: RefCell<Units>` (the same `osr::Units` type), and `reg: Weak<Reg>`. It arms at the existing call threshold
  (`jit::HOT`) under the same conditions as `OsrSite::armed` (`jit_enabled`, no `--hot`, no `type_hint`).
- **Classification.** The live-ins are only the parameters (and `self`): every other local of the body is a `let`
  inside the unit, so it is a fresh variable. The signature is `Vec<Class>` over the parameter slots, checked with
  `osr::classify_is` as today. For `advance` that is two classes (`Vec(Struct(Body))` and `Scalar(f64)`) instead of
  each loop's full name set.
- **Lowering.** A new entry point `lower::lower_body(func, fctx, it, body: &'static [S], reg, refs, ptr)` runs
  `Lo::block` over the body with `path = [Level { list: body, idx, for_: None }]`. Both `while` statements lower
  with the existing `unit_loop` machinery as nested loops. The `let`s and the scalar glue between them lower as
  ordinary statements. The normal end is a new planned exit whose path is `[Level { list: body, idx: body.len() }]`.
- **Return value (v1).** Only functions with no declared return type (`f.ret == None`, like `advance`) are eligible,
  because `osr::level` returns `R<()>` and drops a tail value. v2 extends `level` to return the last statement's
  value, and lowers a scalar tail into a spill-buffer slot.
- **Entry.** In `run_compiled_at` (`src/interpreter/resolved/mod.rs`), after `bind_params` succeeds and before
  `run_body`: `if let Some(fs) = &c.fn_site { if fs.armed(self) { if let Some(r) = fnunit::enter(self, f, c, base, fs) { ... } } }`.
  The frame is real by then: depth check, backtrace and scope are done (F12). That is why no callee-frame protocol
  is needed.
- **Coexistence.** The loop sites inside the body keep their own `OsrSite`s. A function that is called rarely but
  loops for a long time still enters its loops through OSR. A function that is called often enters once per call.
  An exit from the function unit resumes in the closures, where the inner loop's own site may enter its loop unit
  again, which is already allowed today.

### Exit state: what must be materialised

Everything below already exists for loop units, so a function-unit exit is an ordinary exit whose outermost
`Level` is the function body:

| State | Materialised by | Notes |
|---|---|---|
| scalar locals in scope (params and body `let`s) | `Exit::spills`, written by `osr::execute` with `fresh` for body `let`s | a boxed fresh `let` gets a new walker slot (`island::boxed`), as today |
| struct locals held as fields | `Exit::structs` / `StructSpill`, rebuilt with `make_struct` | rebuilt values are fresh `Rc`s (no sharing created) |
| `for` counters on the path | `Level::for_` / `ForState` | `osr::iterations` runs the rest |
| the statement to resume | `Exit::path`, outermost `Level` = the function body | `osr::level` runs the rest of the body, then `close_block` |
| an error raised by a closure-run statement | `Ctx::raised`, returned by `level` | then `finish_value` and `attach_slot_locals`, as in `run_body` |
| `return` inside the unit | planned exit, resumed by the closure code of the `return` statement | `Ctl::Return` reaches `finish_value` |

Two points to verify in step 0: whether `reg.block(f.body)` is registered (`close_block` at the body level must not
duplicate what `leave_compiled` does), and whether the statement codes of a top-level body statement are registered
when the statement is not inside a loop. SOUNDNESS-fnunit.md notes that `record_stmt`/`record_let`/`record_block`
run only at `loop_depth > 0`. The function unit needs them at depth 0 as well, for the statements of the body.

### Compared with the tail-chained loop unit

The tail-chained unit (SOUNDNESS-fnunit.md) enters at the first loop's head and continues past its end. It needs a
new `osr_skip` protocol in the block runner and an `end` bound on `Level`. The function unit enters at statement 0
and runs to the end of the list. `level` already runs "the rest of the list", so neither addition is needed. The
function unit is the smaller change and the more general one.

### Saving

It removes one of the two OSR entries per step and one of the two `rows::fill` scans: both loops run in one `Ctx`,
and the views built in the pair loop are still valid in the `k` loop because only native glue runs between them.
The attribution report bounds halving entries at about 15% of marginal at 10x (interp-loss-attribution.md,
improvement 1, risk note).

## 6. Question 2, continued: the native call, and the step loop

### `rt::call_unit`

The step loop is refused because `advance(bodies, 0.01)` would be a closure-run statement (F10). Running the call as
a closure statement would gain nothing anyway: the closure code would enter `advance` through its own unit. The
step loop instead gets a native call:

- **Lowering.** In `lower_expr.rs`, a call statement (the expression statement `f(args)` with the result unused,
  in v1) to a module function `f` is native when all of these hold:
  - `f` has a `CompiledFn` with a live `fn_site`;
  - each argument is either a scalar expression of the subset, or a bare identifier bound to `Bind::Handle(slot, class)`
    whose parameter is an inferred `&mut` (F4) or a read-only handle;
  - no argument is a scalar local passed to a `&mut` parameter (v1 refuses that case: the callee would need a place
    into a register).

  The lowering evaluates the scalar arguments left to right, as `expr::call` does, into a small argument area in
  the spill buffer. It then calls `rt::call_unit(ctx, buf, k)`, where `k` indexes a new `Plan::calls: Vec<CallSite>`.
- **`CallSite`** holds the target `Rc<FuncDef>`, a pointer to its `CompiledFn`, for each argument either a scalar
  (spill index and `Ty`) or a handle (caller slot), and the callee parameter class signature the caller's
  classification implies. Because the caller's entry already checked its own classes, the callee's unit can be
  chosen at the caller's compile time. The callee's `sig_is` runs only in debug builds and `WJ_JIT_DEBUG` runs,
  where it asserts agreement, as `unit_for` does today.
- **`rt::call_unit`** does what the compiled call path does (F12), with machine code in the middle:
  - `fill_frame`: scalar arguments as `Value`s; handle arguments through `place::arg`, exactly as `expr::call`
    builds them;
  - `enter_compiled`, `bind_params`;
  - run the callee's function unit with a fresh callee `Ctx` (view handoff below);
  - if the callee exits early, finish it in the closure engine *inside its real frame*: `level`, then
    `finish_value`, then `leave_compiled`.

  It returns a status:
  - 0: the callee ran entirely in machine code;
  - 1: closure code ran in the callee, so the caller must clear its views and cells, as after `run_stmt`;
  - 2: the callee raised; `Ctx::raised` holds the error.
- **The caller's state across the call.** The callee cannot read or write the caller's locals. Windjammer scoping
  is lexical, and the only way in is through the arguments: scalars by value, handles through their cell. So the
  caller's register variables stay valid across the call, and no spill is needed on the fast path. On status 2
  the caller takes a new exit flavour: `Exit { path: <the call statement, idx + 1>, after: false, spills: in_scope }`,
  with `raised` set. `osr::level` already returns a raised error when the path ends at that level, and the spills
  give `attach_slot_locals` the caller's bindings.
- **Refusal rule.** `Lo::natives` counts a native call as native, so the step loop (`advance(...)`, `step += 1`)
  compiles.

### View handoff

On entry, `call_unit` moves each caller row view and packed view whose handle is passed as an argument into the
callee `Ctx`. It recomputes the key with the callee's parameter slot and swaps the element table `Vec` between the
two `row_tbls` arrays. On a status-0 return it moves them back. Guard: before handing off, `call_unit` resolves the
callee parameter's cell (`local_cell` on the callee frame) and the caller handle's cell. It hands off only when they
are `Rc::ptr_eq`, which costs one pointer compare per handle per call. On any other status, the caller's views are
cleared.

Soundness:

- the view describes the vector in that cell;
- the guard proves both frames reach the same cell;
- between the caller's last use and the callee's first use, only `call_unit`'s own Rust code ran. That code clones
  no element `Rc` (it builds a `Ref` place to the cell, not a copy of the vector) and moves no storage;
- the same holds on the way back for a callee that never left machine code.

Two read-only views need care:

- **`READ_VIEW` keys** (the sieve's read-only packed views): the bit must survive the key recomputation, or the
  callee could store through a view that was only proved readable. A debug mutant `handoffkey` drops the bit and
  must make a parity program fail.
- **Map cells** (map-lean): not handed off in v1. The callee resolves its own.

### Saving

With the step loop native and `advance` called natively, nbody has one machine-code entry per run instead of two per
step. `rows::fill` runs once per run, and the closure-engine dispatch of the step loop disappears. This is the
attribution report's improvement 1: about 50% of marginal, about 9,000 of 17,002 Ir/step
(interp-loss-attribution.md). The frame build stays (call overhead/frame setup 726 Ir/step in the same report);
step 5 removes it.

## 7. Question 3: representation, packed rows

### The idea

`VecBody` already packs vectors of one scalar type (`Packed::Bool/Int/Float`, `src/interpreter/vecbody.rs`):

- readers that want `&[Value]` get an unpacked cache;
- writers that want `&mut Vec<Value>` unpack for good;
- the hot paths use the packing directly.

Add one variant:

```text
Packed::Rows(RowPack)
RowPack { shape: Rc<RowShape>, data: Vec<u64> }     // element i, field f at data[i * shape.width + f]
RowShape { name: Rc<str>, copy: bool, fields: Vec<(Rc<str>, Ty)> }   // Ty: f64, i64 (bool later)
```

The layout is array-of-structs, packed: one contiguous buffer, a fixed stride, the payload bits only, no `Rc<str>`
name and no 32-byte `Value` per field. That matches V8's hidden class with in-object unboxed doubles, which is the
reason node keeps the whole n-body nest in registers and cache lines. Struct-of-arrays was considered and not
chosen. It helps vectorisation, which is not the bottleneck. It makes `push` and element materialisation touch one
buffer per field, and the machine code would need one base pointer per field instead of one per view.

### Decided by declaration, not by observation

`RowShape` is built once, when the struct declaration is registered, and only when every one of these holds:

- every field's declared type is `f64` or `i64`;
- the struct is not generic;
- it has no `impl Drop` (`uid == 0` by construction).

The engine never guesses from values. A body is created packed when its first pushed element is a struct of a
declaration that has a `RowShape` (`Packed::for_value`). `Packed::put` accepts only a struct of that declaration
whose fields already have the declared types (they always do after `retag`). Anything else unpacks for good, the
same rule as the scalar packings.

### Why value semantics make this sound

An element of a packed-rows body has no identity: no `Rc`, no destructor, no interior mutability (every field is a
scalar). Reading `bodies[i]` as a value materialises a fresh `StructVal` (F1: a copy is what the language means
anyway). So the hazard that stopped the row cache, an element `Rc` cloned through `&` access, **cannot exist** for
this representation. The remaining sharing is the body `Rc` itself (`let c = bodies`). `Rc::get_mut` on the body
checks that once per view build, and copy-on-write on the body copies the buffer. Retyping a field (F6's fifth
condition) cannot happen either: a store of another type unpacks the body, and unpacking moves storage, which clears
views (F7).

### What it buys

- `rows::fill` becomes O(1): one `Rc::get_mut` on the body, then `(data.as_mut_ptr(), len)`. No per-element
  uniqueness check, no `ty_is` scan, no element table. The view is a `View { key, ptr, len }` like the packed scalar
  views, and the machine code address is `ptr + (i * width + f) * 8`. That is one load fewer per field access than
  `lower_rows.rs` emits today (the table entry load).
- In the closure engine, `bodies[i].x` and `bodies[i].x += e` get `VecBody` fast paths (`row_field_get`,
  `row_field_set`) that work on the buffer without materialising a `StructVal`. `src/interpreter/resolved/place.rs`
  learns an `Index` then `Field` projection on a packed-rows body.

### Costs at the boundary, and how they are bounded

| Operation | Today | Packed rows |
|---|---|---|
| `let b = bodies[i]` (`energy`) | `Rc` clone of the element | a new `StructVal`: allocation of the struct and its field vector; names are `Rc<str>` clones from the shape |
| iteration, printing, `&[Value]` readers | direct | unpack into the cache once, kept until the next write (the scalar packings' rule) |
| a write through a path the packing does not serve | in place | unpack for good, then as today |
| `push` | `Value::Struct` appended | fields copied into the buffer |

The first row is the real price. A loop that reads whole elements by value pays an allocation per read where it now
pays a reference-count increment. Gate: the `interp_instr.wj` ratchet rows for `structs` and `agent_loop`
(`bench/interp/`), plus a new parity program that alternates whole-element reads and field writes, must not
regress past the ratchet's tolerance. If they do, the packing is limited to bodies that machine code has touched
(packed on the first `row_build`), which is still a declaration-decided shape, applied lazily.

## 8. Question 4: the order of work

Each step is independently shippable. Each comes with byte-identical outputs at all three scales (the scaling
expected files and `bench/interp/expected/nbody.out`), green engine parity
(`tests/interpreter_engine_parity_test.rs`, `tests/interpreter_jit_loops_test.rs`,
`tests/interpreter_jit_differential_test.rs`), a fast-path counter asserted in
`tests/interpreter_fast_paths_test.rs`, and at least one debug mutant (`WJ_JIT_MUTANT`) that a test catches.

The cost gate is the marginal Ir per step: the total instruction count at the 1x and 10x kernels, as
`scripts/wj/interp_instr.wj` already counts them (a `--marginal` mode reads the two generated sizes and divides by
the work difference). That is the same formula as the attribution report, computed in Windjammer.

### Step 0: verify (no engine change)

- Confirm the assumptions: F3, F14, F15, the `reg.block(f.body)` question and the depth-0 statement registration.
- Add counters `fnunit`, `callunit`, `rowhandoff`, `rowpack` to `FastHits`, reported by `WJ_JIT_DEBUG`. The counters
  only count; no path changes.
- Record the baseline marginal for nbody with the new `--marginal` mode on the PC, and check it against the
  report's 17,002 before any step claims a saving.

### Step 1: function unit for unit-returning functions

- Files:
  - `src/interpreter/resolved/compile.rs` (`FnSite`, depth-0 recording);
  - `src/interpreter/resolved/jit/lower.rs` (`lower_body`, end-of-body exit);
  - `src/interpreter/resolved/jit/osr.rs` (`enter` generalised over a site trait, so loop sites and function sites
    share `unit_for`, `execute` and `resume`);
  - `src/interpreter/resolved/mod.rs` (entry hook).
- Expected: one entry per `advance` call instead of two. The attribution report bounds this at about 15% of marginal.
- Gate:
  - counter `fnunit` fires on nbody;
  - parity programs under `tests/engine_parity/`:
    - a trap (overflow, index out of bounds, divide by zero) in the first loop, in the glue and in the second loop;
    - an early `return` in the glue;
    - a recursive unit-returning function with a loop;
    - a function whose `&mut` Vec parameter is pushed to between the loops;
    - shadowed `let` names across the two loops.
- Mutants:
  - `fnspill`: drop the glue `let`s from an exit's spills;
  - `fnpath`: resume one statement late;
  - `fnviews`: skip `clear_rows` after a closure-run statement inside the unit.

### Step 2: native call and view handoff

- Files:
  - `src/interpreter/resolved/jit/lower_expr.rs` (call statement);
  - `src/interpreter/resolved/jit/lower.rs` (`Plan::calls`, the raised-call exit, the natives count);
  - `src/interpreter/resolved/jit/rt.rs` (`call_unit`, handoff);
  - `src/interpreter/resolved/expr.rs` (factor `fill_frame` so `call_unit` shares it).
- Expected: the step loop compiles. The attribution report puts this at about 50% of marginal (improvement 1),
  minus the call overhead that stays.
- Gate:
  - counters `callunit` and `rowhandoff` fire on nbody, and `jitrefused` is 0;
  - parity programs:
    - a callee that panics (backtrace and failing-frame locals identical to the closure engine's);
    - a callee that pushes to the handed-off vector;
    - a callee that exits to the closures half-way and then returns;
    - the same handle passed to two parameters;
    - a callee that is not yet hot on its first native call.
- Mutants:
  - `handoffguard`: skip the `ptr_eq` guard;
  - `handoffkey`: drop `READ_VIEW`;
  - `callnoclear`: ignore status 1.

### Step 3: tighter pair-loop code

- What: hoist the view key and length checks out of an inner loop whose body cannot clear views (statically: no
  helper store, `push` or closure-run statement in the body, which the lowering knows). Keep `dx/dy/dz` and the
  loaded `mi/mj` in registers across the six updates; Cranelift does this once the loads are not behind a branch
  per access.
- Files: `src/interpreter/resolved/jit/lower_rows.rs`, `src/interpreter/resolved/jit/rows.rs`.
- Expected: 1,500-2,500 Ir/step, 9-15% (attribution report, improvement 3).
- Gate:
  - `rowview` hits unchanged;
  - mutant `hoistpush`: hoist even when the body has a `push`;
  - a parity program that pushes inside the inner loop must then fail.

### Step 4: packed rows

- Files:
  - `src/interpreter/vecbody.rs` (`Packed::Rows`);
  - the struct registration (`RowShape`);
  - `src/interpreter/resolved/place.rs` and `src/interpreter/resolved/jit/rows.rs` (O(1) view);
  - `src/interpreter/resolved/jit/lower_rows.rs` (direct address).
- Expected: no hand estimate. Measure the drop in machine-code Ir per pair (the table load) and the disappearance
  of the per-element `fill` scan where it is still paid (outside nbody: any loop entered often).
- Gate:
  - counter `rowpack`;
  - the `structs` and `agent_loop` ratchet rows;
  - parity programs for `let b = v[i]` then `v[i].x = ..` then reading `b`, a struct with a field of another type
    (stays unpacked), `@derive(Copy)` rows, and a body shared by `let c = v` then written.
- Mutants:
  - `rowpackshare`: skip the body `Rc::get_mut` in the O(1) view;
  - `rowpackput`: accept a field of another type.

### Step 5: inline the callee (P3), with lazy frame materialisation

- What: inline a native-call callee's body into the caller unit. An exit inside the inlined body calls
  `rt::materialise_callee(ctx, buf, exit)`. That helper runs the frame half of `call_unit` (`fill_frame`,
  `enter_compiled`, `bind_params`), writes the callee spills into callee slots with the callee `Reg`'s slot kinds,
  resumes `level` over the callee path, runs `finish_value` and `leave_compiled`, and then continues the caller at
  the statement after the call. Every part except writing the callee spills already runs, and is already tested,
  in step 2's path.
- `Exit` gains `frame: u8` (0 = caller, 1 = inlined callee) on its spills and path levels. `Lo::in_scope` splits its
  spills by frame. `Lo::run_stmt` still refuses inside the inlined callee (F11), so the inlined body must be fully
  native.
- Expected: removes the call overhead/frame setup (726 Ir/step in the attribution report, an upper bound) and lets
  loop invariants such as `dt` and `n` stay in registers across calls.
- Gate: the step 2 parity programs, run again with inlining forced. Mutants:
  - `inlspill`: write callee spills to caller slots;
  - `inlbt`: skip `enter_compiled`'s backtrace push in materialisation, so a callee panic's backtrace must differ.

### Step 6: view-quiet statements

- What: the section 4 proof.
- Expected for nbody: nothing (it has no such statements). For loops with progress prints: views survive the
  prints instead of being rebuilt after each.
- Gate and mutant: as in section 4.

### Ladder

Each rung is a measurable step on the way to V8's shape.

| Rung | What it means | Reached by |
|---|---|---|
| 1 | one entry per call | step 1 |
| 2 | one entry per run | step 2 |
| 3 | the inner loop is loads, flops and stores only | step 3 |
| 4 | the data is laid out like a hidden class | step 4 |
| 5 | the call is gone | step 5 |

Rung 5 is the shape that the attribution report credits for node's 14.2x lead at 100x.

## 9. Question 5: risks, interactions, and what not to do

### What not to do

- **No mutation-epoch or generation cache across closure-engine code.** SOUNDNESS-rowcache.md shows why: element
  sharing is invisible to the body, and the region between entries has no hook.
- **No `Rc` or `Weak` hold on a vector to key a cache**: either one defeats `Rc::get_mut`, so `fill` itself
  refuses. **No global counter** either: the static_state ratchet forbids it.
- **Do not relax the closure-statement refusal (F10) in general.** Step 2 counts native calls as native; it does
  not make closure-run statements cheaper.
- **Do not inline before the native call has shipped.** Step 5's deopt is safe only because step 2 already runs
  and tests its frame half on every call.
- **Do not specialise on values** (`dt`, `n`), only on classes. Classes are checked at entry; values would need
  guards that exit inside the nest.
- **Do not make packed rows the representation of every struct vector.** The shape is decided by declaration, and
  only for scalar-field, non-`Drop`, non-generic structs.
- **Do not grow `Value`** (the 32-byte assertion in `src/interpreter/value.rs`). Packed rows live inside
  `VecBody`, not in `Value`.

### Interactions

- **Typed scalar pipeline** (`src/interpreter/scalar_pipe.rs`): independent. It drains `Int` iterator chains in the
  closure engine and never sees row views. Function units refuse under `it.type_hint`, as loop units do.
- **Sieve read-only views (perf-jit-vec-inline)**: handoff must carry the `READ_VIEW` bit (mutant `handoffkey`).
  Packed rows reuse the same `View` struct, and a view built by a read is keyed read-only in the same way.
- **Map-lean**: map cells are per `Ctx` and are not handed off in v1. The callee resolves its own, which is
  correct and costs one resolution per call.
- **Hot reload** (`src/interpreter/hot.rs`): bodies are swapped at function entry, and machine code is off under
  `--hot` (F13). The function site keeps that rule. If loop machine code is ever allowed under `--hot`, `CallSite`
  must compare its `Rc<FuncDef>` with the module's current one at each call, because function entry is the swap
  point.
- **Threads and tasks**: `spawn`, channels and locks are never native, so they are closure-run statements, which
  clear views. A `Ctx` never leaves its `Interpreter` (F15, to verify).
- **Recursion**:
  - `call_unit` runs `depth_check` through `enter_compiled`, and checks `stack_room` before each native call: native
    frames nest on the Rust stack, as inlined method frames do (`INLINE_STACK` in `osr.rs`);
  - a recursive callee gets its own `Ctx` and its own spare buffers (`Unit::spare` is taken with
    `try_borrow_mut`, so a nested run allocates its own);
  - inlining (step 5) refuses a callee already on the inline stack.
- **Destructors and `Copy` structs**: `uid != 0` refuses views and packing (F6). The `copy` flag is part of the
  shape (`kinds::shape_is`) and of `RowShape`.
- **Error locations and backtraces**: every trap is an exit, and the closure code of the same statement raises the
  error in a real frame. Step 2's raised-call exit spills the caller's bindings, so the failing-frame locals of
  ADR-0036 are the closure engine's.

### Residual risks

- **Function-unit compile cost.** `advance` is compiled twice: as loop units at first, then as a function unit.
  The fixed-cost column of the attribution report (Cranelift `define_function` among the fixed Ir) must not grow
  past the ratchet. If it does, a loop site whose function is already armed as a function site skips its own
  compile.
- **`MAX_UNPLANNED` churn**: a function unit that exits often is killed after the existing bound, and the loop
  units take over, as today.
- **Boundary costs of packed rows** (section 7): bounded by the ratchet rows, with the lazy-packing fallback.

## 10. Verified and not verified

Read in the code at `ff6e395552fa`:

- F1, F2, F4-F13, F16;
- the refusal message and the loop structure of the bench kernel;
- the entry and exit machinery of `osr.rs`;
- the view-clearing sites of `rt.rs`;
- the row access emitted by `lower_rows.rs`;
- the call path of `mod.rs` and `expr.rs`;
- the packed bodies of `vecbody.rs`.

Not verified, and owned by step 0:

- F14 (no helper keeps an `Rc` clone past its return);
- whether the function body has a registered block scope and statement codes at depth 0;
- whether `place::arg` on a caller handle yields a `Ref` whose `local_cell` resolution equals the caller's;
- the 1x/10x numbers on the current head (the report measured `wt-rc3-int` 00630c8a1).

No saving on this page has been measured for the proposed code. The expected savings are the attribution report's
bounds, and each step's gate measures the real one.

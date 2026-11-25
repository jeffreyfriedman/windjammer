---
status: proposal
last_verified: f5a1d311b5d9
---
# The n-body ladder: the cheapest path to within 2x of node

> **Proposal: analysis, nothing implemented.** It orders the steps of
> [interp-struct-vec-loops.md](interp-struct-vec-loops.md) and [interp-call-unit.md](interp-call-unit.md) by cost
> per Ir saved on the n-body kernel, adds one step those pages do not have (hoisted row views), and names the
> experiments that confirm each projection before any code lands. The hand-written kernel variants used as evidence
> are in [interp-nbody-ladder/](interp-nbody-ladder/).

## 1. Answer

**Three steps reach the goal: hoisted row views, the native call without handoff (2a), and inlining the callee
(step 5).** The projected end state is 1,200-1,800 Ir/step, against a budget of about 2,100. At 100x that is
0.13-0.23 s steady, 1.0-1.8x node. Confidence that this prefix reaches 2x or better: **about 70%**. Most of the
remaining uncertainty is how few instructions a hoisted row access really costs in Cranelift. Experiment E3 measures
that today, before any code is written.

Five measured facts make the case:

- One hand-written variant keeps the bodies in 35 scalar locals and unrolls the 10 pairs. It runs at **50 ns/step on
  today's engine, faster than node (104 ns/step)**. The Cranelift tier already produces node-class float code, so the
  gap is not code generation.
- With `advance` inlined by hand into the step loop, the kernel runs at **474 ns/step**, against 1,437 for the same
  binary on the stock kernel. The call boundary costs two-thirds of the time.
- What stays at 474 is almost all the row-view access protocol. There are 243 accesses per step, each costing about
  17-28 instructions where 1-2 would do.
- Neither half reaches the goal by itself. Removing the call leaves about 4.7x node, and hoisting alone leaves about
  5-6.6x. Together they reach 1.0-1.8x.
- No new representation is needed for nbody. Packed rows (step 4), struct-of-arrays and the step-2 view handoff (2b)
  are all off the cut line.

## 2. The target in Ir/step

| Quantity | Value | Source |
|---|---|---|
| 100x program | 1,200,000 steps (`while step < 1200000`), 60x the 1x work | `bench/interp/scaling/gen/100x/nbody.wj` |
| resolved steady at 100x | 1.893 s | findings/interp-scaling-2026-11-24-rerun (head ff6e39555, before step 1) |
| node steady at 100x | 0.1253 s, i.e. 104 ns/step | same |
| marginal cost on that head | 17,136 Ir/step | REPORT-fnunit2.md, step 0b |
| marginal cost after step 1 | 11,085 Ir/step | marginal1.json |

**ns per Ir.** 1.893 s / (1.2M steps x 17,136 Ir) = **0.092 ns/Ir**. This charges the whole steady time to the
marginal cost. The fixed part inside the steady window (JIT compiles, the warm-up before OSR, two `energy` calls) is
about 30 ms of 1.89 s. A second reading comes from this page's own single run of the stock kernel on the same binary
(section 4): 1,437 ns/step marginal at 17,136 Ir/step = **0.084 ns/Ir**. This page uses 0.084-0.092.

**Assumption: time is proportional to Ir for this kernel.** This holds for today's interpreter-heavy profile. It is
weaker for the end state, which is float code with `sqrt` and `div` on the critical path: fewer instructions, but
each one waits longer. Cache and branch-miss effects are unmeasured. They are small for a 1.1 KB working set and
well-predicted loops, but unmeasured all the same. E1 below measures ns/Ir on float-bound code directly: it divides
the variant v5's wall time by its callgrind Ir.

**Budget.**

1. 2x node is 0.2506 s at 100x.
2. Reserve about 15 ms for fixed costs inside the steady window. That is v1's measured 1x fixed part. The stock
   kernel's is about 30 ms; v5's is about 7 ms.
3. That leaves (0.2506 - 0.015) / 1.2M = **196 ns/step**.
4. At 0.084-0.092 ns/Ir, that is **2,130-2,340 Ir/step**. This page uses **2,100 Ir/step** as the target.
5. If float-bound code ran at 0.15 ns/Ir, the budget would be about 1,300 Ir/step. The ladder's low end still meets
   that; its high end does not (section 6).

**Distance:** 11,085 / 2,100 = 5.3x to go.

## 3. Where the 11,085 Ir/step go

The attribution report (`findings/interp-loss-attribution.md`, NBODY) was measured before step 1. Its categories
were JIT machine code 5,305 (31%), OSR entry/exit 5,144 (two entries of 2,572), allocation 1,565, dispatch 1,217,
boxing 1,131, rt helpers 776 and call overhead 726, for a total of 17,002. No attribution of the post-step-1 binary
exists yet; E0 provides one. Until then, the split below is an estimate that agrees with two independent readings.

| Part | Ir/step | Basis |
|---|---|---|
| machine code (MC) | 5,150-5,650 | (a) the attribution report: 5,305 by category, 5,892 for the two code addresses; (b) the v1 variant (all calls inlined, so only MC and a native step loop remain): 474 ns/step at 0.084-0.092 ns/Ir |
| per-call and per-entry remainder | 5,400-5,900 | 11,085 - MC: one OSR entry (`execute`, `Ctx`, `unit_for`, `classify_is`/`shape_is`, `rows::fill` of 5 bodies x 7 fields), the closure engine's step loop and `advance(..)` call (`expr::call`, `fill_frame`, `enter_compiled`, `bind_params`, `leave_compiled`), and their malloc/free and `Value` clone/drop |

**What the machine code spends.** This is read from `lower_rows.rs` `row_access` at f5a1d311b. Every `bodies[i].f`
read or write emits:

- two loads (view key and length);
- a compare against a 64-bit key immediate;
- an unsigned bound compare, a `band` and a `brif`;
- the table load, a scale and an add, and the fields-pointer load;
- a load, add and store of the `ROW_HITS` counter;
- the payload load or store;
- a join block with a variable, whose slow path calls `row_build`.

That is about 16-20 x86 instructions per access. Spills are on top: every f64 live across an access diamond can be
spilled, because the slow path's call clobbers every XMM register. The counters give **243 accesses per step**
(`rowview` 48,592,574 at 10x / 200,000 steps; 4,852,574 at 1x in this page's run). Per pair there are 14 reads
(compound assignments read and write separately) and 6 writes. The `k` loop adds 9 accesses per body. So:

- accesses: 243 x 17-28 Ir = 4,100-6,800 (the high end is the ihoist measurement, section 4);
- float arithmetic and register traffic: about 550-600 (the v5 floor);
- loop control: about 25 loop iterations per step with checked integer adds, 150-200.

That is consistent with 5,150-5,650 once the access cost is near 17-19. **The access protocol is 70-80% of the
machine code.** The arithmetic is not the problem.

**Ideal per pair (static count).**

- f64 operations: 3 subtractions, 5 for `d2`, `sqrt`, one multiply and one divide for `mag`, and 6 updates of
  2 multiplies and 1 add each, so 29 plus `sqrt`. Cranelift does not reassociate floats, so `dx * mj * mag` stays two
  multiplies and nothing is shared between the updates.
- Memory: 14 loads (x86 folds many into the arithmetic) and 6 stores. Floats have no overflow checks.
- Loop and indexing: about 5 for `j += 1`, the compare and the branch, the row pointer of `j`, and a bound check per
  index value.

That is about **45-55 Ir per pair**. The `k` loop is about 15-18 per body. Each step is then 10 pairs x 50 + 5 x 17,
plus `i` loop overhead (about 40) and the step loop (about 10): **a floor of about 600-700 Ir/step** with
array-of-structs (AoS) rows and the fields in memory. Struct-of-arrays (SoA) has the same load count (base plus
displacement either way). Only keeping the fields in registers goes lower, and the v5 variant shows that floor:
about 50 ns/step.

## 4. Evidence: hand-written variants on today's engine

Binary: `~/own/bin/wj-rc3head` (0.50.0+8c9f17fb2f, head ff6e39555, **before step 1**: no `fnunit` counter). Each
variant was type-checked with `wj check` and run **once** at 1x and once at 10x with `wj run --interpret`
(`nice -n 19`, box load 0.1). The output is byte-identical to `bench/interp/scaling/expected/{1x,10x}/nbody.out`
for every variant. Marginal ns/step = (steady 10x - steady 1x) / 180,000 steps. These are single runs, so read
them to about ±10%. Callgrind (section 8) replaces them.

| Variant | What it emulates | JIT (WJ_JIT_DEBUG) | steady 1x | steady 10x | marginal ns/step | vs node 104 |
|---|---|---|---|---|---|---|
| `v0-base` | the stock kernel | step loop refused; 2 entries/step | 59.1 ms | 317.8 ms | 1,437 | 13.8x |
| `v1-inline` | steps 2 + 5 (`advance` inlined into the step loop) | all 4 loops compiled, 1 entry/run | 24.9 ms | 110.3 ms | **474** | 4.6x |
| `v2-ihoist` | v1 plus the `i` row's fields in locals across the `j` loop (scalar replacement of one row) | all compiled; `rowview` -50/step | 21.1 ms | 85.1 ms | **355** | 3.4x |
| `v3-soa-inline` | v1 with 7 `Vec<f64>` columns (today's packed views) | all compiled; `vecresolve` 111/step | 51.2 ms | 395.7 ms | 1,914 | 18x |
| `v4-soa-call` | SoA with the call (today's entries plus packed views) | step loop refused | 176 ms | not run | n/a | n/a |
| `v5-scalars` | every field in a scalar local; 10 pairs and 5 updates unrolled | 1 loop compiled | 7.65 ms | 16.7 ms | **50** | **0.48x** |

What the table says:

1. **Removing the call boundary (v0 to v1) cuts the time by two-thirds.** That is everything the remainder row of
   section 3 holds, plus the second entry, which step 1 has since removed. v1 is the end state of steps 2 and 5
   without any machine-code change: about 4.6x node.
2. **Row access dominates what is left.** v1 minus v2 is 119 ns for 50 fewer accesses per step, about 2.4 ns
   (26-28 Ir) per access. That is more than the 16-20 static count, which fits spill traffic around the access
   diamonds. v1 minus v5 is 424 ns for 243 accesses plus loop control, which bounds the access at about 1.7 ns
   (19 Ir) on average. So **A = 17-28 Ir per access today.**
3. **The Cranelift float pipeline is not the limit.** v5 is register-resident and unrolled. Node does not unroll,
   so v5 is a floor, not a forecast, but it beats node by 2x.
4. **SoA on today's engine is a trap.** `VIEWS = 4` (`jit/rt.rs:79`) is direct-mapped by `slot % 4`. Seven column
   vectors collide, so about 111 accesses per step miss and go through `vecresolve`. That is a separate small
   finding for user programs written in SoA style: make the view table 8-way or associative. It is not on nbody's
   path.

## 5. The ladder

Projections use only the components above. "Cumulative" assumes the order shown. Wall time = 1.2M x Ir x
(0.084-0.092) ns + about 15 ms fixed.

| # | Step | Saves (Ir/step) | Cumulative Ir/step | 100x steady (vs node) | Risk | Size |
|---|---|---|---|---|---|---|
| 0 | step 1, function unit (merged) | (measured) | **11,085** | 1.13-1.24 s (9.0-9.9x), estimated | done | done |
| L1 | **hoisted row views** (new; refines design step 3) | 3,600-5,000 | **6,100-7,500** | 0.63-0.84 s (5.0-6.7x) | low-medium | about 400 lines, 3 parity programs, 4 mutants |
| L2 | **step 2a**: native call, no handoff | 185-585 (interp-call-unit.md section 9) | **5,500-7,300** | 0.57-0.82 s (4.6-6.5x) | medium (designed; sound by a complete argument) | about 600-800 lines (2.0 + 2a), 13 parity programs, 6 mutants |
| L3 | **step 5**: inline the callee, lazy frame | 4,300-5,500 (the whole remainder) | **1,200-1,800** | **0.13-0.21 s (1.0-1.7x)** | medium-high | about 700-900 lines, the 13 step-2 programs forced inline + 4, 5 mutants |
| L4 | machine-code polish (only if above 2x) | 200-600 | 700-1,600 | 0.09-0.19 s | low | 4 small changes |
| | **off the cut line for nbody:** 2b handoff, 2c unit cache, step 4 packed rows, SoA, whole-program unit, step 6 | | | | | |

### L1: hoisted row views (loop versioning). Do this first

This step is not in the coordinator's list in this form. It is design step 3 made concrete, and it is the largest
single saving.

- **Rule.** A loop nest is *view-stable* when its body lowers with no operation that clears views (F7 of the
  coordinator design):
  - no closure-run statement;
  - no `push`, `index_set`, `index_field_set`, `field_set` or map insert helper;
  - no `call_unit`.

  The lowering knows this once it has lowered the body, so it can lower it twice.
- **Preheader** at the outermost view-stable loop. For each `(slot, shape)` row the nest reads or writes:
  - if the key misses, call `row_build`;
  - if a view still is not present (the fill failed), branch to the existing per-access version of the nest
    (today's code, unchanged);
  - otherwise load `tbl` and `len` once into variables.

  The `ROW_HITS` counter is added once per preheader (or only when `jit_debug` is on), not per access.
- **Fast nest.**
  - `bodies[i].f` becomes a row pointer `tbl[i]`, loaded once per index value per iteration, then
    `load/store [row + off]`.
  - The bound check stays (`i < len`, a branch to the statement's ordinary exit). It is deduplicated per index
    variable while that variable is not reassigned, so there are 2 checks per pair, not 20. This adds no new trust.
  - No slow path is left inside, so no call clobbers the XMM registers.
- **Saving.** Each access goes from A = 17-28 to 1-3 (the payload access, plus an amortised row-pointer load and
  bound check), and the nest gains 2 preheaders per call. 243 x (16-25) = 3,900-6,100. That is capped by the machine
  code's size: what stays is the floor (550-600), loop control (150-200), 243 x (1-3) and the preheaders. Projected
  saving **3,600-5,000 Ir/step**.
- **Soundness.** A view is valid until something clears it. In a view-stable nest nothing can (F7 enumerates the
  clearing sites, and A6 of the call-unit page makes that one function). F14 says no helper keeps an `Rc` clone.
  Bounds are still checked. Links: SOUNDNESS-rowcache.md (why the view may not outlive a run: it does not here),
  interp-call-unit.md 2.2 (a nest with `call_unit` cannot hoist).
- **Gates.**
  - Parity programs:
    - a `push` inside the inner loop;
    - a bound failure on iteration k of the hoisted nest;
    - a fill that fails because an element is shared (`let b = v[0]` before the loop), so the slow version runs.
  - Mutants: `hoistpush`, `hoistcall`, `hoiststore` (hoist anyway), `hoistnobound` (drop the bound check).
- **Files:** `jit/lower_rows.rs` (fast access), `jit/lower.rs` (view-stable test, versioning), `jit/rows.rs`
  (preheader helper).
- **Dependencies:** none. It benefits every row-view kernel (`structs`, `agent_loop` rows).

### L2: step 2a, the native call without the handoff

As designed in interp-call-unit.md sections 1-8 and 9 (2.0 refactor + 2a). For nbody it is mainly the **frame half
that L3 reuses as its deopt materialiser**. Projected 185-585 Ir/step on its own (10,500-10,900 from 11,085). Skip
2b (the handoff, a further 900-1,300) and 2c on this path. Once the callee is inlined, views never cross a call.
Build them only if L3 slips by more than a quarter.

### L3: step 5, inline the callee with a lazy frame

As in interp-struct-vec-loops.md section 8, step 5:

- inline `advance` into the step loop's unit;
- an exit inside the inlined body calls `rt::materialise_callee`, which runs 2a's `fill_frame`, `enter_compiled`
  and `bind_params`, writes the callee spills, resumes `level` in the callee frame, and finishes;
- a `Ref` parameter binds to the caller's own `Bind::Handle`, a scalar parameter to a fresh variable.

With L1 in place, the whole step loop is one view-stable nest, so `rows::fill` runs **once per run**.

- **Saving:** the whole remainder of section 3, minus L2's share: 4,300-5,500. v1 measures the end state directly
  (474 ns/step before L1).
- **End state:** floor 550-600, plus loop control 150-200, plus 243 x 2-4 (486-972), plus about 10 for the step
  loop: **1,200-1,800 Ir/step.**
- **Allocation (9%) and boxing (7%) need no step of their own.** The attribution's sources are all per entry or per
  call: malloc/free 824/step in the entry anatomy (`Ctx` tables, spill and element tables), `Value` drop/clone 533
  (classification, live-in reads, `place::arg`, `fill_frame` argument values) and the closure engine's call
  argument vectors. L3 removes the call, so they go with it. The machine code allocates nothing. E2 checks this:
  malloc calls per step on v1 should be about 0.
- **Risk:** the P3 hazard. A trap at iteration k of an inlined loop has committed k-1 iterations, so the exit must
  resume inside the callee. Lazy materialisation closes it only if 2a's frame half is the code that builds the frame
  (SOUNDNESS-p3.md; interp-call-unit.md section 0.1). Failing-frame locals (ADR-0036) and backtraces must match the
  closure engine's.
- **Gates:**
  - the 13 step-2a parity programs with inlining forced;
  - a trap at iteration k inside the inlined pair loop;
  - an early `return` in the inlined glue;
  - a shadowed parameter (`step2-checks/a4-shadow.wj`);
  - a callee called from two sites with different argument classes.
- **Mutants:** `inlspill`, `inlbt` (from the design), plus `inlpath` (resume in the caller instead of the callee) and
  `inlparam` (bind a `Ref` parameter to the wrong slot).
- **Alternative with the same deopt protocol: frame-free native calls.** Call the callee unit's machine code
  directly and build the frame lazily, without merging the code. That saves all of L3 except a native call and a
  per-call preheader, about 100-300 Ir/step, and it works for recursion. For nbody, inlining is simpler: one `Ctx`,
  so no view crosses a call. Choose frame-free calls instead if inlining's code growth (Cranelift compile time inside
  the steady window) shows in E5.

### L4: machine-code polish (only if E-measurements put L3 above 2x)

1. **Scalar replacement of the `i` row** across the `j` loop. This needs `i != j` (a range fact: `j` starts at
   `i + 1` and only grows) or a hoisted guard that exits when `i == j`. Saves about 100 accesses per step: 100-300
   Ir. v2 measures the same transformation at today's access cost.
2. **Bound elimination by range** (`n = v.len()` with no length change in a view-stable nest): 50-100.
3. **Typed f64 variables**, if a reading of `lower.rs` `cty` shows i64 to f64 bitcasts per access: 50-250.
4. **Fixed costs inside the steady window** (compile time, warm-up before OSR): 7-30 ms at 100x is 3-12% of the
   2x budget.

### Off the cut line, and why

- **Step 4, packed rows.** Five bodies x 7 fields x 32 bytes is 1.1 KB, so everything sits in L1 cache. After L1
  the table load is hoisted per index value, and after L3 `fill` runs once per run. What it buys (an O(1) `fill`, one
  load fewer, a smaller footprint) matters for large N and for kernels that enter often. It is not on nbody's path.
- **SoA / scalar-replacing the whole `Vec<Body>`.** It would require:
  - an all-f64, non-generic, `Drop`-free struct (step 4's `RowShape`);
  - a unique body `Rc` and unique element `Rc`s (or packed rows, where elements have no identity);
  - no by-value copy of the vector alive in the unit (the `strong_count == 1` check of interp-call-unit.md 3.3);
  - unpacking and packing at the unit boundary: 35 fields x 2 x about 10 Ir, about 700 Ir per run, or per call
    without L3.

  Its loads are the same as hoisted AoS (base plus displacement), so the gain over L1 + L3 is about 0. Only keeping
  fields in registers (L4.1) wins, and that needs no layout change. v3 also shows that today's packed views thrash
  on 7 columns (section 4).
- **Whole-program unit** (`main` + `advance` + `energy` as one unit). For nbody nothing is left to gain after L3:
  the step loop is already the unit, and `energy` runs twice. It would need `main` as a function unit and a native
  struct-valued `let b = v[i]` (a field copy as `Bind::Struct`, sound by F1 because a read is a copy). It is worth
  doing for kernels whose hot loop sits in `main` next to closure-run code, not here.
- **A different JIT shape** (tracing, or typed SSA over `Value` with escape analysis). v5 shows the existing
  lowering already emits node-class float code once the values are in variables. Nbody's gap is protocol (views,
  calls), not code generation. Such a tier would pay off for polymorphic fields and temporary structs in loops
  (escape analysis), which nbody does not have. Rough cost: 8-15k lines and several months. Not recommended for this
  goal.

## 6. If the ladder tops out above 2x

The likely cause is ns/Ir. Float-bound code may run at more than 0.092 ns/Ir, and E1 measures that.

- If L3 lands at 1,800 Ir/step and the code runs at 0.15 ns/Ir, 100x takes 0.34 s (2.7x). Then do the L4 items in
  order 1, 3, 2, 4, for 200-600 Ir.
- If that is still above 2x, raise the floor's quality the way v5 shows: keep the `i` row and the loop-invariant
  `dt`/`n` in registers. That is L4.1, generalised to every row whose index is invariant in the inner loop.
- The remaining gap would then be the 2x between v5 (fully register-resident, unrolled) and a rolled loop with
  memory fields. Node itself sits between the two.

Nothing in this plan requires a new JIT shape. If even v5-class code were needed, unrolling by a constant trip count
is out of scope: `n` is a runtime value.

## 7. Recommended cut line

**L1 -> L2 (2.0 + 2a) -> L3.** Projected **1,200-1,800 Ir/step**, 0.13-0.21 s at 100x, **1.0-1.7x node**.
Confidence about 70% for ≤ 2x:

- the arithmetic leaves margin at 0.084-0.092 ns/Ir (the high end, 1,800, is about 1.7x);
- the risks are ns/Ir on float-bound code (E1) and the real per-access cost after hoisting (E3, plus a disassembly
  check of the fast nest).

L1 goes first because it has no dependencies, it is the largest saving per line, it helps other row kernels, and its
result (E3's prediction) calibrates L3's end state. L2 alone is worth little for nbody. It is built because L3's
deopt depends on it.

## 8. Experiments to run first (callgrind, by the measurement owner)

All programs are in [interp-nbody-ladder/](interp-nbody-ladder/) (1x and `-10x` copies). Run each with
`--interpret`, check its output against `bench/interp/scaling/expected/{1x,10x}/nbody.out`, and compute
marginal = (Ir 10x - Ir 1x) / 180,000 per step.

| # | Run | Gives | Predicts |
|---|---|---|---|
| E0 | `findings/interp-loss-attribution/run-callgrind.sh` on the post-step-1 head (stock kernel) | the real split of 11,085 into MC and remainder | MC 5,150-5,650 |
| E1 | `v5-scalars` (callgrind and one timed run) | the floor in Ir/step, and **ns/Ir for float-bound code** (= 50 ns / Ir) | 550-650 Ir/step; ns/Ir 0.08-0.10 |
| E2 | `v1-inline` | the L2 + L3 end state before L1; malloc calls per step | 5,150-5,650 Ir/step; about 0 malloc per step |
| E3 | `v2-ihoist` vs `v1-inline` | A, the Ir per row access today: (v1 - v2) / 50 | 17-28 |
| E4 | `v1-inline` with `--dump-instr=yes`, annotated on the JIT address | instructions per access diamond, and spills around them | 16-20 + spills |
| E5 | timed scaling of v1 at 100x (make it with `sed s/200000/1200000/` on the `-10x` file) | wall at 100x for the L2 + L3 shape, and the fixed part in the steady window | 0.57-0.60 s |
| E6 | `v3-soa-inline` | confirms the `VIEWS = 4` thrash (`vecresolve` 111/step) | not a floor; side finding |

Decision rule: if E1's ns/Ir is above 0.12, or E3 puts A below 15 (less to gain), recompute section 5 before
starting L1. Otherwise start L1.

## 9. Step-2 design checks A1 and A4: done now

Programs in [interp-nbody-ladder/step2-checks/](interp-nbody-ladder/step2-checks/). Each was run once under both
engines (`--engine resolved` and `--engine walker`) with wj-rc3head.

- **A1 (recursion depth).**
  - Verified by reading: no test pins the exact overflow depth or even the message. `grep "too deep" tests/` finds
    only an unrelated glob test. `tests/conformance_programs/177_deep_recursion.wj` pins floors (50,000 frames for
    a value-returning function, which is not a function-unit callee). The two Rust tests are about compile-time
    analysis recursion.
  - `a1-depth.wj` pins the floor for a function-unit callee: 3 rounds of 50,000-deep self-recursion through a
    unit-returning function with a loop, plus a hot caller loop. Both engines print `150003` then `2000`.
  - `a1-overflow.wj` (unbounded recursion; it must fail with "stack overflow: recursion too deep for the interpreter
    stack" and never print the depth) is written and type-checked but not run. It recurses to a stack budget of up
    to 4 GiB.
- **A4 (a `let` shadowing a parameter slot).**
  - Verified by reading: `Cx::declare` (`resolved/compile.rs:93-98`) gives every `let` a fresh slot
    (`nslots += 1`), and the lowering's `let` arm binds `reg.let_slot(s)` (`jit/lower.rs` `native`). So a shadow
    never reuses a `Ref` parameter slot.
  - `a4-shadow.wj` has two hot callees: one mutates the `Vec` parameter and then shadows it with a fresh vector it
    writes; one reads the parameter, shadows it and writes the shadow. Both engines print `3000 0`.
- **Found while writing A4: three front-end bugs on today's head**, with repros in `step2-checks/`:
  1. `a4-shadow-scalar.wj`: `let n = n * 2` shadowing a scalar parameter, read by a `while` condition, **aborts
     `wj check` and `wj run` with a Rust stack overflow** (both engines; before any engine runs). Renaming the
     shadow to `m` passes.
  2. `shadow-scalar-infers-param-mut.wj`: `let mut n = n * 2; n += 1` makes `wj check` report WJ0003 (`f2(3)` "needs
     `&i64`"). The shadow's mutation is attributed to the parameter.
  3. `shadow-vec-infers-param-mut.wj`: a `push` on a shadowing `let mut ps` makes the caller's `b` "changed but not
     declared mut" (WJ0051).

  None of these involve the interpreter's engines. For step 2, the interpreter's own `analysis.rs` (F4) should be
  checked for the same blindness. If it marks a shadowed parameter as mutated, the caller passes a `Ref`. That is
  harmless for semantics (the shadow has its own slot) and for the handoff (the offer matches the parameter cell,
  which the callee no longer uses for that name). Each needs its own `bug_*_test.rs` (ADR-0013).

## 10. Unmeasured assumptions

| # | Assumption | Cheap check |
|---|---|---|
| U1 | time is proportional to Ir at 0.084-0.092 ns/Ir in the end state | E1 (ns/Ir of v5), E5 |
| U2 | the post-step-1 split MC vs remainder (section 3) | E0 |
| U3 | A = 17-28 Ir per access today; 1-3 after hoisting | E3, E4; after L1, the same callgrind on the stock kernel |
| U4 | allocation and boxing are per call, not per pair | E2 (malloc calls/step on v1) |
| U5 | single-run walls (section 4) are within ±10% | E5 with `--runs 5` |
| U6 | inlining's extra Cranelift compile time stays within the 15 ms fixed reserve | E5 fixed part = steady 1x - 20,000 x marginal |
| U7 | fixed costs inside the steady window (compile, warm-up) are about 15 ms at 100x for the L3 shape | v1 1x shows about 15 ms; E5 |
| U8 | cache and branch-miss effects are negligible (1.1 KB working set) | `perf stat` branch-misses and L1-dcache-misses on v1 at 10x, once |

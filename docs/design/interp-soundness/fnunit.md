---
status: superseded
last_verified: 5457283f4156
---
# Function-level JIT unit: soundness note and decision

> **Superseded design: this note's tail-chained loop unit was NOT implemented.** The function-level unit that landed is argued in [fnunit2.md](fnunit2.md); this page is kept for the hazards list and the test matrix it shares.

Verdict: design argued below (sound on paper), NOT IMPLEMENTED in this run: the change touches the compile pass, the
block runner and the deopt resume protocol at once, and the budget left after jobs 1 and 2 cannot carry the build
cycles (4 min each), the 9-row measurement, the guard-test matrix and a real mutation check per hunk. Stopping
before shipping a deopt change that is not fully tested is the owner's rule.

## What it would be
Today `advance` has two loop sites (the i/j nest, then the k loop): 2 OSR entries per call, 40,000 per run.
The callgrind profile of nbody (see the measurement files on pc-results) puts the entry overhead (live-in reads, spill
buffer, row-view fill, write-back, resume) at a few percent of a cold run per entry pair; halving the entry count is the
target (one entry per call instead of one per loop).

Smallest sound shape (a "tail-chained loop unit", not a new frame protocol): when loop L1 at index k of block B arms, the
unit compiled for L1 continues past the loop's normal end into the statements k+1..m of the SAME block B, where m is
the longest prefix of natively lowerable statements (let, assignment, while/for, if) that stops before B's tail
expression and before anything `run_stmt` would need. The unit therefore is the callee body's suffix; it runs in the
frame that already exists (the callee's own), so no callee frame is ever rebuilt: that is what blocked free-function
inlining (SOUNDNESS-p3.md) and why this avoids it.

## State at entry, exit, deopt
* Entry: unchanged (live-ins classified and read at L1's head; statements 0..k-1 already ran in the closures).
* Normal end: new exit "chain done" (planned) whose spills write back every variable in scope, including the `let`s of
  k+1..m (`fresh`, boxed ones get a new walker slot exactly as `write` does today), then sets `Interpreter` field
  `osr_skip = m-k` and the L1 closure returns Ok(Unit) at once (it must NOT re-test L1's condition: the continuation
  may have changed L1's variables).
* Failed check or trap inside k+1..m: an exit with path [Level{B, j}, ...nested]; spills of everything in scope; the
  closure engine resumes statement j (the existing `resume`/`level`) and then the statements j+1..m of B ONLY (a new
  `end` bound on the top Level; running to B's end would double-run m+1.. and the tail), then sets `osr_skip` as above.
  Every trap is an exit, as today; panics are raised by the closure code for that statement (message, location,
  backtrace identical to the walker's because the same closure runs).
* Block runner: `seq` after a While statement that owns a site reads and clears `osr_skip` and advances over that
  many statement codes (only While/For-led sequences pay the check; other blocks are unchanged).

## Required changes (the reason it was not done here)
1. Compile pass: `record_stmt/record_let/record_block` are loop_depth>0 only; statements after a loop in the same block
   need recording too (resume needs their codes and let slots).
2. `Level`: a bound for the top list; `resume` stops at it.
3. `lower`: continue after `unit_loop` end into B's tail statements with the same `block()` machinery; new exit kind.
4. Block runner skip protocol, and for `for` sites (`enter_for` returns `next`) the same.

## The listed hazards
* Aliasing of handle args (`&mut`/Vec params): handles stay `Bind::Handle(slot, class)`, as today for a loop unit; two
  names for one handle stay a refusal of the existing unit (unchanged). No new aliasing arises: no frame is created.
* Recursion: the unit is bound to the call's frame (`b`); a recursive call inside the continuation is not native
  (`run_stmt` closure call), runs in its own frame and its own unit entry, `spare` buffers are taken while in use
  (already handled by `try_borrow_mut`).
* Early return: `return` in the continuation is an exit (not native); the closure engine runs it and the Ctl::Return
  propagates out of the L1 closure; `osr_skip` must be cleared on every error path (a mutation target).
* Traps as exits: see above; `it.fallback.jitexit` accounting unchanged, MAX_UNPLANNED bound unchanged.
* Argument evaluation order / calls: a call statement is never native; it is `run_stmt`, evaluated by the closures in
  program order between spills and reloads, as now.
* Globals/consts: read through helpers or closures, as in loop units.
* Shadowed names: the continuation's `let` slots are distinct slots (resolved at compile time); `names` of the site
  (locals visible at L1) does not include them, so they are fresh vars in the unit (`new_var(.., fresh=true)`).
* Interaction with existing loop units: the L1 unit is the same Unit with a longer tail; the k loop's own site stays
  armed for the exit path (a resume that reaches it runs it in the closures, which may enter its own unit).
* Closures fallback: if any statement in the chain is outside the subset the chain is cut there (m smaller), never
  refused; with m = k the unit is exactly today's.

## Test matrix to ship with it (not written)
callee with an `&mut`/Vec param aliased twice; recursive callee; early return inside the chain; a trap (overflow,
index out of bounds, division by zero) in the continuation's first, middle and last statement; a callee reading a
const/global; call arguments with side effects; shadowed `let` names across L1 and the chain; byte-identical nbody at 1,
7, 1000, 3333, 20000 steps; mutations: skip count off by one, `end` bound removed, `osr_skip` not cleared on error.

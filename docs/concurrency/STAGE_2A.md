---
status: proposal
---
# Stage 2a leftovers: `wait_timeout`, cancellation, un-waited tasks at exit

> **Proposal: not implemented.** Design and acceptance programs for the rest of PLAN.md stage 2a; `select { }` (also 2a) has shipped.

Each item lists the semantics, the interpreter and Rust lowering, and the acceptance programs (expected output written
by hand from the semantics; they become `tests/conformance/concurrency/*.wj` when the item lands, and run under the six
schedules and through `wj run --verify` like the existing ones).

## 1. `t.wait_timeout(d) -> T?`

Wait at most `d` for the task. `Some(value)` when it finished in time (a panic is re-raised, as with `wait`), `None`
otherwise; the task keeps running and can still be waited on. Never takes the value on `None`.

- Interpreter: `block_on(Wait::Any([Join(id), Sleep(clock + d)]))`, then the task's result or `None` (virtual clock: exact).
- Rust runtime: `Task::wait_timeout(&self, d)`: `Condvar::wait_timeout` on the task's slot; a local (`async`) task is
  run first (it cannot time out halfway: it runs to completion on this thread, as the interpreter's one-thread mode).
- Typed surface (`std/rust_core.wji`): `fn wait_timeout(self, d: Duration) -> Option<T>;` under `Task<T>`.
- std surface row: `{ type = "Task", name = "wait_timeout", sig = "t.wait_timeout(d: Duration) -> T?" }`.

```windjammer
// Acceptance: wait_timeout.wj
// EXPECTED OUTPUT:
// [fast] 42
// [slow] timed out
// [slow later] 7
// [all] PASSED
use std::time::Duration

fn slow() -> int {
    std::thread::sleep(Duration::from_millis(200))
    7
}

fn main() {
    let fast = spawn { 40 + 2 }
    match fast.wait_timeout(Duration::from_secs(5)) {
        Some(v) => println("[fast] ${v}"),
        None => println("[fast] timed out"),
    }
    let s = spawn slow()
    match s.wait_timeout(Duration::from_millis(10)) {
        Some(v) => println("[slow] ${v}"),
        None => println("[slow] timed out"),
    }
    println("[slow later] ${s.wait()}")
    println("[all] PASSED")
}
```

## 2. Cancellation on first failure

When a task started in a `scope` panics, the scope cancels its other tasks: each is told to stop at its next
cancellation point (`wait`, `recv`, `send` on a full channel, `select`, `sleep`, `wait_timeout`), where it unwinds as
if it panicked with "cancelled". The scope then waits for all of them (structured: nothing outlives the scope) and
re-raises the first real panic. A cancelled task's own `defer`s run. Code without cancellation points runs to its end
(cooperative cancellation; no thread is ever killed).

- Interpreter: a `cancelled` flag per task; `block_on` checks it on entry and after every wake-up and returns a
  `Ctl::Err(Panic "cancelled")`; `run_scope` sets the flag on every task of the scope when one finishes with `Err`.
  Deterministic under every schedule seed.
- Rust runtime: an `Arc<AtomicBool>` per scope registry; `Receiver::recv`, `Task::wait`, `select`, and the runtime
  sleep check it (blocking calls wait in slices of at most 10 ms, or on a condition variable the scope notifies).
  A cancelled task unwinds with `resume_unwind(Cancelled)`, which the scope recognises and does not re-raise.
- Spec: a new rule in `docs/spec/concurrency.md` (the next free SPEC-CONC id, with its example under `tests/spec/concurrency/`) when this lands. Diagnostics: none (run-time behaviour).

```windjammer
// Acceptance: cancellation.wj (expected exit status 101; stderr names "worker 2 failed")
// EXPECTED OUTPUT:
// [start]
// [cleanup] waiter
use std::sync

fn main() {
    let (tx, rx) = sync::channel()
    println("[start]")
    scope {
        spawn {
            defer println("[cleanup] waiter")
            let v: int = rx.recv()     // never sent: cancelled here
            println("[unreachable] ${v}")
        }
        spawn {
            panic("worker 2 failed")
        }
    }
    tx.send(0)
    println("[unreachable] after scope")
}
```

Without cancellation this program deadlocks (the interpreter reports it; compiled it would hang): the acceptance
test asserts the output above, exit 101, and that the run ends in under a second compiled.

## 3. Un-waited tasks at program exit

WJ0190 is conservative (a name mentioned after its binding counts as waited on). The run-time complement: when `main`
returns, any task that was never waited on, is not in a scope, and has not finished is reported on stderr
(`warning: 1 task was never waited on (started at main.wj:6:13); its result and panic are lost`) and the program waits
for it before exiting, so its effects are not cut off silently. A task that finished but was never waited on and
panicked makes the program exit 101.

- Interpreter: the scheduler knows every task (`tasks[id].waited`, set by `wait`/`scope`); checked after `call_main`.
- Rust runtime: a process-wide registry of thread tasks not in a scope; `main`'s generated epilogue calls
  `windjammer_runtime::task::finish_main()`.

```windjammer
// Acceptance: unwaited_at_exit.wj (expected stderr: "1 task was never waited on"; exit 0)
// EXPECTED OUTPUT:
// [main] done
// [task] ran
fn note(t: Task<int>) {
    // the handle is mentioned here, so WJ0190 accepts the program, but nobody waits on it
    println("[main] done")
}

fn work() -> int {
    std::thread::sleep(std::time::Duration::from_millis(5))
    println("[task] ran")
    1
}

fn main() {
    let t = spawn work()
    note(t)
}
```

## Order and estimate

`wait_timeout` (0.5 day), cancellation (1.5 days: interpreter first, then the runtime), un-waited report (1 day).
Each lands with its acceptance program interpreted and compiled, its spec rule, and a mutation check (cancellation:
disable the flag check in `recv`, the program must deadlock and the test fail on its timeout assertion).

# Cooperative executor for wasm32 (stage 3, wasm half): implementation plan

Status: plan, not implemented. Owner decision 2026-11-16: async on every backend where feasible, **this stays a BETA item**
(the concurrency conformance programs are in the wasm tier ledger with this plan as the reason; the ledger shrinks as the steps land).
Design source: [JS_WASM.md](JS_WASM.md) section "WASM" and model A ("nested run"); reference semantics: the interpreter's one-thread
mode (`src/interpreter/sched.rs`, "One-thread mode", ADR-0034), held to the thread mode by `tests/interpreter_one_thread_tasks_test.rs`.

## Why the wasm rows fail today (measured)

The 7 programs `tests/conformance/concurrency/{channels,scope_waits,select,spawn_and_wait,task_handles}.wj` and
`tests/spec/concurrency/SPEC-CONC-009/010` fail in two layers, in this order:

1. **Harness**: the wasm row builds the Rust backend's output with plain `rustc --target wasm32-wasip1`; the generated Rust says
   `windjammer_runtime::task::...` / `windjammer_runtime::sync::...` and rustc reports `E0433: cannot find module or crate windjammer_runtime`
   (the rust rows fail the same way for the same reason; the cargo-based drift gates build these programs). Fixing only this exposes layer 2.
2. **Runtime**: `crates/windjammer-runtime/src/task.rs` lowers `spawn` to `std::thread::spawn` with `Mutex`/`Condvar` hand-off, `select`
   polls with `std::thread::sleep`, `Deadline` uses `Instant`; `sync.rs` channels are `std::sync::mpsc` with a blocking `recv`. On
   `wasm32-wasip1` (no threads) `thread::spawn` returns an error, a blocking `recv` on an empty queue never returns, `Instant` is wasi's clock.

## Target semantics

Exactly the interpreter's one-thread mode and JS_WASM.md model A: a task is a queued closure; a blocking point (`wait`, `recv` on an empty
channel, `select`, scope exit, `sleep`) runs queued tasks to completion on top of the current stack until the condition holds; no task is
ever resumed below another (a schedule that needs it is refused at compile time, never run with a different interleaving). `sleep` and
`timeout(d)` use a virtual clock that jumps to the earliest deadline when everything waits; nothing runnable and nothing on the clock is a
deadlock reported with the interpreter's message and exit 101. Output is byte-identical to the interpreter on the conformance corpus.

## Steps

| # | Step | Files | Gate | Days |
|---|---|---|---|---|
| 1 | Executor core behind a cfg: `#[cfg(any(target_arch = "wasm32", feature = "one_thread"))] mod coop` with the queue, `Task<T>` (`Local` variant already exists: `task::local`), `run_until(cond)`, virtual clock, deadlock report | `crates/windjammer-runtime/src/task.rs` (split into `task/thread.rs`, `task/coop.rs`), new `task/clock.rs` | unit tests of the queue and clock; the same corpus run natively with `--features one_thread` against the interpreter | 2 |
| 2 | `spawn` = `local` on the coop executor; `Scope` drop runs every task started inside and re-raises the first panic (panic = `catch_unwind` is unavailable on wasm32 panic=abort: store the first panic as a payload by running the task body behind a `Result`-returning wrapper the backend emits, or build the wasm target with `-C panic=unwind` where the toolchain has it) | `task/coop.rs`, `src/codegen/rust/async_generation.rs` (emit the wrapper only for the wasm target) | SPEC-CONC-003, -009; `scope_waits`, `spawn_and_wait`, `task_handles` | 2 |
| 3 | Channels without `mpsc` blocking: `Sender/Receiver` over `Rc<RefCell<VecDeque>>` in one-thread mode; `recv` = `run_until(queue non-empty or all senders dropped)` | `crates/windjammer-runtime/src/sync.rs` (cfg split), `Selectable` impls | SPEC-CONC-004, -010; `channels`, `select` | 2 |
| 4 | Static admission check (model A): refuse at compile time a task body that blocks on a source the spawner feeds after the spawn (needs suspension); the message names the task and suggests running on a target with threads | a new check module named task_suspension (reads the same task structure as `src/analyzer/task_structure.rs`), diagnostic code allocated from the registry | positive/negative cases in `tests/promises`, oracle cases for admitted and refused programs | 2 |
| 5 | Harness: the wasm and rust rows build programs that reference `windjammer_runtime` through cargo (one cargo build of a scratch workspace, as `scripts/wj/spec_oracle.wj` does for compiled spec cases), then run the wasm module under wasmtime | `scripts/wj/target_tiers.wj` (`run_one` for `rust` and `wasm`), `scripts/wj/MODULE.md` | the ledger rows below disappear; `target_tiers.wj check` ratchets lower | 1.5 |
| 6 | Exit status of a panic on wasm32 (the 13 panic-mode wasm failures of the tier table): `panic = "abort"` traps with a different status than 101; decide per ADR-0007: a per-target exit-status mapping in the runner or a `std::process::exit(101)` panic hook installed by the generated `main` for the wasm target | `src/codegen/rust/` main emission, `ADR-0007` amendment | the 13 wasm panic ledger rows | 1 |

Total about 10.5 agent-days (JS_WASM.md estimated the whole stage at 10 including JS; this is the wasm half plus the harness and the
panic status). Order: 1, 2, 3 in sequence (the executor, then its two users), 5 in parallel with 4, 6 independent.

## Gates and the ledger

* Conformance: the five programs and SPEC-CONC-009/010 byte-identical to the interpreter under wasmtime; the earlier concurrency specs (001..005, 007, 008, 011)
  re-measured, since 001..004 are ledgered for the same harness reason.
* `tests/interpreter_one_thread_tasks_test.rs` stays the oracle of the semantics; add a native one-thread run of the corpus (`--features one_thread`) so the
  executor is tested without wasmtime, on every machine.
* Ledger: the wasm rows for these seven programs (and the rust rows for the harness reason) are removed as the steps land; the ratchets in
  `tests/conformance/backend_ratchet.tsv` are lowered with them and never raised.
* Mutation checks: executor that does not run queued tasks at scope exit (scope_waits fails), channel `recv` that returns without waiting (channels fails),
  deadlock not reported (a planted program hangs under the run timeout), admission check disabled (the refused program builds and prints a different order).

## Risks and cut lines

* `catch_unwind` on wasm32: the first panic of a scope must propagate; without unwinding it needs the result-wrapper of step 2 or a toolchain with
  `panic=unwind` for wasm (not stable): choose the wrapper; if it costs more than 1 day extra, scopes re-raise only panics of the scope body itself and the
  ledger says so.
* `wasm32-wasip1-threads` keeps the OS-thread lowering (JS_WASM.md); the cfg selects the coop executor only where there are no threads.
* Browser wasm (`wasm32-unknown-unknown`, `wasm-bindgen-futures`) yields to the event loop at blocking points: out of scope here, the same executor with a different `run_until`.

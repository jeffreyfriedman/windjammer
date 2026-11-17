---
status: proposal
---
# Tasks on JavaScript and WASM (PLAN.md stage 3)

> **Proposal: not implemented.** The JavaScript backend reports `spawn`, `async`, `scope` and `select` as unsupported today; this is the design for stage 3.

Engine principle: single-threaded by default. The browser main thread is the game loop and must never block, and most
programs gain nothing from workers. Parallelism is opt-in and only where the compiler proves it changes no output.
The reference semantics is the interpreter's scheduler (`src/interpreter/sched.rs`); every model below is judged by
whether the conformance programs (`tests/conformance/concurrency/`) print exactly what the interpreter prints.

## Three execution models

| Model | When | Exact? | Cost |
|---|---|---|---|
| A. Nested run (synchronous) | default when the whole program fits it (decided at compile time, see below) | exact, or refused at compile time: never a different output | zero: plain synchronous JS |
| B. Async transform (microtasks) | default otherwise | exact | functions that can reach a blocking point become `async function`s (decided by the compiler, invisible in source) |
| C. Workers | opt-in `wj build -t javascript --threads`, only for `spawn` tasks whose captures are messages | exact output; real parallelism | worker start-up, structured clone of captures; cross-origin isolation headers in browsers |

### A. Nested run

The interpreter's one-thread mode, compiled: a task is a closure in a queue; a blocking point (`wait`, `recv` on an
empty channel, `select`, scope exit) runs queued tasks to completion, on top of the current stack, until the condition
holds. It is exact for programs whose tasks never need to be suspended halfway (a task that waits for something only
the code that started it will produce later). The compiler decides statically: a task body that contains a blocking
point on a source the spawner feeds after the spawn needs suspension, so the program uses model B. Programs that sleep
use a virtual clock in model A (jump to the next deadline when everything waits); `timeout(d)` arms compare against it.

| Windjammer | JS (model A) |
|---|---|
| `spawn f(x)` / `async f(x)` | `$task(() => f(x))` (queued; captures are copied as the Rust backend clones them) |
| `t.wait()` | `$wait(t)`: run queued tasks until `t` is done, rethrow its panic |
| `scope { b }` | `$scope(() => { b })`: run `b`, then run every task it started; rethrow the first panic |
| `tx.send(v)` / `rx.recv()` | `$send(ch, v)` / `$recv(ch)`: run queued tasks until a value is queued; none can run and none queued: deadlock error (exit 101 with the interpreter's message) |
| `select { .. }` | `$select([sources], hasElse)`: same loop over readiness |

### B. Async transform

Every function that may reach a blocking point (computed over the call graph: a blocking builtin, a task body, or a
call of such a function) is emitted as `async function`, and every call of one is `await`ed. Tasks are promises
started with `queueMicrotask`, so a task runs when its spawner awaits, as in the interpreter. Function colouring
exists only in the generated JS; Windjammer source never shows it.

| Windjammer | JS (model B) |
|---|---|
| `spawn f(x)` / `async f(x)` | `$task(async () => await f(x))`: a promise plus a settled flag, scheduled as a microtask |
| `t.wait()` | `await $wait(t)` (rethrows the task's panic) |
| `scope { b }` | `await $scope(async (s) => { b })`: `Promise.allSettled` over the tasks, rethrow the first rejection; `return`/`?`/`break` inside `b` are lowered like the Rust `ScopeExit` (the block becomes a closure) |
| `tx.send(v)` | `$send(ch, v)` (resolves a waiting receiver's promise) |
| `rx.recv()` | `await $recv(ch)` |
| `select { .. }` | `await $select([..])`: `Promise.race` over readiness promises; the winner is the first ready in source order (all ready sources are checked synchronously before racing) |
| `timeout(d)` | a `setTimeout` promise |

Exact: output order, panics, deadlock detection (a run whose promise graph settles with tasks still pending reports
the interpreter's deadlock message instead of exiting silently).

Approximated: none for output. Performance: `await` on a hot path costs a microtask turn; the transform only colours
functions that can block, so pure code stays synchronous.

### C. Workers

`spawn` of a task whose captures are all plain data (numbers, strings, lists, maps, structs of these, channel ends) may
run on a Web Worker / Node worker thread: captures are structured-cloned (the same copy semantics the Rust backend
has, WJ0192 forbids writes to them), channels between workers use a `SharedArrayBuffer` ring plus `Atomics.wait`
(allowed in workers and under Node; never on the browser main thread, where `wait` stays the async model B).

| Windjammer | JS (model C) |
|---|---|
| `spawn f(x)` | `$spawnWorker("f", [x])`: a pooled worker runs the module's `f` |
| `t.wait()` on the main thread | `await` the worker's completion message (model B on the main thread) |
| `t.wait()` inside a worker | `Atomics.wait` on the task's slot |
| channel ends crossing a worker | a shared ring buffer; `recv` blocks with `Atomics.wait` in workers |

Refused (compile-time Windjammer error, not a silent fallback): `--threads` with a `spawn` that captures a closure or
an `Rc`-like value (WJ0193 already covers the latter), and a blocking `wait` on the browser main thread.

## WASM

Through the Rust backend (`wasm32-unknown-unknown`) with `windjammer_runtime::task` built for one thread:
`std::thread::spawn` does not exist there, so `spawn` runs on the local queue (model A semantics: exact output, no
parallelism), and a receive that can never be satisfied reports a deadlock instead of hanging. `timeout(d)` uses the
virtual clock unless the program is built with the browser event loop (`wasm-bindgen-futures`), where the
generated `main` yields to the loop at blocking points. `wasm32-wasip1-threads` keeps the OS-thread lowering.

## Game engine

The frame loop runs on the main thread and never awaits. Engine jobs use `scope { for chunk in chunks { spawn
update(chunk) } }`: model A on the main thread (sequential, exact, cheap), model C with `--threads` on platforms that
allow cross-origin isolation (parallel, exact). Asset loading uses `async load(path)`, settled between frames by the
`requestAnimationFrame` callback draining the microtask queue.

## Conformance and order of work (10 agent-days)

1. Runtime `wj_core.mjs`: task queue, `$wait`, `$scope`, channels, `$select`, deadlock report (model A) (2 days).
2. Portable lowering: `SpawnCall`/`AsyncCall`/`Scope`/`Select` into the portable IR; captures copied (1 day).
3. Model A decision (static check for tasks that would need suspension) and refusal message (1 day).
4. Model B: blocking-reachability over the call graph, `async`/`await` emission, scope exit lowering (3 days).
5. Model C: worker pool, structured clone, shared ring channels, `--threads` flag with promise cases (3 days).

Gates: the five conformance programs on Node in models A (where the static check admits them) and B, byte-identical
to the interpreter; the same in headless Chrome for model B; model C on Node with `--threads`, plus a test that a
refused capture names the value; `tests/interpreter_one_thread_tasks_test.rs` stays the oracle for model A.

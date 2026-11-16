# Concurrency surface: staged plan to beta (ADR-0004)

Owner decision 2026-10-10 ([ADR-0004](../adr/0004-concurrency-surface.md), "Decision"), directive 2026-11-16: implement
async for beta on every backend where feasible. This plan turns the decision into stages, each of which ships something
usable, and records what is exact, what is approximated and how each claim is tested. Language rules live in
[spec/concurrency.md](../spec/concurrency.md); this file is the work plan.

## The surface (one paragraph)

`spawn f(x)` / `spawn { }` start a task that may run in parallel; `async f(x)` / `async { }` start a task on the
current executor (overlap, no parallelism). Both evaluate to one handle type, `Task<T>`; `t.wait()` returns the value
(`t.await` is the same) and re-raises the task's panic. Concurrency is structured: `scope { }` waits for every task
started inside it (also by those tasks), even when the block ends early, and a task's panic propagates out of the
scope; outside a scope a task must be kept and waited on, and a dropped task is a compile error (WJ0190). Channels are
values with methods (`tx.send(v)`, `rx.recv()`); `thread { }` and the `<-` operators are removed (WJ0191, migrated by
`wj fix`). `select { }` stays and is designed in stage 2. Functions are not coloured: there is no `async` on a
declaration in the target surface (`async fn` stays until stage 2b removes it with a `wj fix`). `Send`/`Sync` never
appear in source: the solver decides them.

## Stages and order

The order delivers value early and keeps every stage testable on the interpreter first (interpreted-first, ADR-0033):
the interpreter's deterministic scheduler is the reference semantics every backend is compared against.

| Stage | Delivers | Backends | Estimate (focused agent-days) | Gate |
|---|---|---|---|---|
| 1 (this branch) | Parser forms, AST nodes, WJ0190/WJ0191 with fixes, `wj fix` migration, interpreter semantics (per-task scopes), `Task`/channel rows in the std surface, conformance corpus, spec text | interpreter (thread and one-thread modes) | 4 (done) | `tests/concurrency_surface_test.rs`, promises, diagnostics suite |
| 2a | `select { }` (parser, interpreter, spec); timeouts (`rx.recv_timeout`, `t.wait_timeout`); scope cancellation on first failure (cooperative: `sleep`, `recv`, `wait` are cancellation points) | interpreter | 4 | conformance `select_*.wj`, seed sweeps |
| 2b | Rust backend: tasks, scopes, channels, select on OS threads; `Send` inference in the solver; remove `async fn` with a `wj fix` (variants on demand, below) | Rust | 12 | the stage-1 conformance programs compiled, byte-identical to the interpreter under `wj run --verify` |
| 3 | JS/WASM backend: `async` on the event loop (promises), `spawn` on the same loop by default (single-threaded engine principle) and Web Workers when the solver proves message-only sharing | JS, WASM | 10 | conformance on node + headless browser |
| 4 | Go backend: goroutines, `sync.WaitGroup` scopes, channels, `select` | Go | 5 | conformance through `go run` |
| 5 | Game engine job system on `scope`/`spawn` (data-parallel `par` helpers over `scope`), agent tooling (deadlock reports with task stacks, `wj test --simulate` coverage of task programs) | all | 6 | engine frame benchmark, planted-bug study |

Total to beta: about 41 agent-days after stage 1, with 2a and 2b in parallel (different files: interpreter vs
codegen). Stages 3 and 4 depend only on the conformance corpus and the IR nodes, not on each other.

Kill/rescope criteria: if 2b's `Send` inference cannot be decided by the solver without user annotations on more than
5% of the dogfood corpus, fall back to "spawn requires owned captures" (move semantics everywhere, clones inserted by
the solver) and record the cost; if the JS worker path cannot pass conformance within its estimate, ship JS with
`spawn` = `async` (single-threaded, exact output, no parallel speedup) and keep workers for after beta.

## Stage 1 (implemented here)

| Piece | Where | Status |
|---|---|---|
| `spawn expr`, `spawn { }`, `async expr`, `async { }`, `scope { }` | `src/parser/task_prefix.rs` | parsed into `Expression::SpawnCall` / `AsyncCall` / `Scope` |
| `thread { }`, `ch <- v`, `<-ch` removed | `task_prefix.rs` (`RemovedForm`), `check/tasks.rs` | WJ0191 with machine-applicable edits; `Statement::Thread`/`Async` and `Expression::ChannelSend`/`ChannelRecv` deleted from the AST |
| Un-waited task | `src/analyzer/task_structure.rs` | WJ0190 in `wj check` (fix: wrap the rest of the block in `scope { }`), `wj build` and the interpreter loader |
| `wj fix` migration | `src/cli/fix.rs` `migrate_concurrency` | runs before the parse guard, iterates WJ0191 then WJ0190 fixes to a fixed point |
| Interpreter semantics | `src/interpreter/builtins/thread.rs` (`run_scope`, `start_task`), `sched.rs` (per-task scope stacks) | scope evaluated in place (`return`, `?`, `break` behave as in any block), waits for all tasks, first panic re-raised |
| Rust backend | `src/codegen/rust/async_generation.rs` | refuses with `compile_error!` naming this plan (never different semantics) |
| JS/Go backends | `src/codegen/portable/lower/expr.rs` | explicit unsupported error, as before |
| Std surface | `docs/std_surface.toml` rows (types `Task`, `Sender`, `Receiver`; the std surface redesign) | rows added; the gate lives on the std-redesign branch |
| Conformance corpus | `tests/conformance/concurrency/*.wj` | expected output in each header; run under six schedules |
| Spec | `docs/spec/concurrency.md` SPEC-CONC-001..008 | examples executable |

Mutation check (2026-11-16, one mutant build with three planted bugs, then restored): (1) WJ0190 never reported
(`scope_depth == usize::MAX`): `an_unwaited_task_is_refused_by_check_run_and_build` fails (check exits 0);
(2) `scope { }` joins nothing: `a_task_panic_propagates_out_of_its_scope_after_the_other_tasks_finish` (prints the
line after the scope) and `conformance_programs_print_their_expected_output_under_every_schedule` (`scope_waits.wj`:
`[nested_tasks] received 1`) fail; (3) the `thread { }` fix carries no edit: `removed_forms_are_refused_with_their_fix`
and `wj_fix_migrates_thread_blocks_and_channel_operators` fail. `scopes_are_per_task_under_interleaving` survives
mutant 2 (its `recv()` calls synchronise on their own); it guards the per-task scope stacks, which no mutant touched.

What is approximated in stage 1: the interpreter runs `async` and `spawn` on the same cooperative scheduler (only one
task runs at a time; a schedule seed explores interleavings at synchronisation points). The observable contract is the
same: output that depends only on `wait()`, `recv()` and scope exit is identical. WJ0190 counts any later mention of a
task's name as "waited on" (it may be passed to a function that waits); the runtime does not yet report a task that
is dropped through such a call (stage 2a adds a run-time check at program exit in the interpreter).

## `select { }` (stage 2a design)

```windjammer
select {
    msg = inbox.recv() => handle(msg),
    n = worker.wait() => println("worker gave ${n}"),
    _ = timeout(Duration::from_millis(50)) => println("nothing yet"),
    else => println("nothing ready"),
}
```

Arms are `pattern = source => body`; a source is `rx.recv()`, `t.wait()` or `timeout(d)` (any other expression is a
compile error naming the three forms). Sources are evaluated once, in order, before waiting. The first ready arm wins
(source order; with a schedule seed the interpreter picks among ready arms, so tests can sweep the choice); `else`
runs when nothing is ready (never waits). A closed, empty channel is never ready; when no arm can ever become ready
the `select` panics ("every channel of this select is closed") instead of hanging. Interpreter: one `Wait::Any` over
the arms' waits, so the virtual clock and deadlock detection cover `select`. A `select` is an expression (the chosen
arm's value). Losing arms are not cancelled in 2a (a `wait()` arm keeps its task; a `recv()` arm takes nothing).

## Rust backend (stage 2b)

**Executor choice: OS threads for `spawn`, a per-thread local executor for `async`, no tokio in the default build.**
Reasons: (1) `scope { }` maps exactly onto `std::thread::scope`, which lets spawned work borrow from the enclosing
frame without `'static` bounds, so the ownership solver's borrow decisions carry over unchanged; tokio's `spawn`
requires `'static + Send` and would force clones the language promises to avoid. (2) Compile time and binary size are
product features (light builds on weak machines): tokio adds about 30 s of cold build time and several hundred KB;
`std` threads cost nothing. (3) Structured concurrency needs no work stealing for beta workloads (agents, CLIs,
servers with modest fan-out); a pool can be swapped in behind the same runtime API later. (4) `async` without a
runtime keeps `async` functions callable from synchronous code: `t.wait()` drives the local executor (a small
`block_on` with a thread-local run queue, about 150 lines, no dependency). Programs that use `std::http` server or
`std::db` keep tokio through the existing `async` feature of `windjammer-runtime`: the runtime crate bridges by running
those futures on its own runtime thread, as `block_on` does today.

Lowering table:

| Windjammer | Generated Rust | Exact? |
|---|---|---|
| `scope { body }` | `windjammer_runtime::task::scope(\|__s\| { body })` over `std::thread::scope`; the closure is FnOnce, so `return`/`?`/`break` inside `body` are lowered through a `ScopeExit` enum the generator matches after the call | exact (the generator owns the control-flow translation) |
| `spawn f(x)` inside a scope | `__s.spawn(move \|\| f(x))` returning `Task<T>` (wraps `ScopedJoinHandle`) | exact |
| `spawn f(x)` outside a scope (returned handle) | `windjammer_runtime::task::spawn(move \|\| f(x))` (`'static`; captures moved or cloned by the solver) | exact |
| `async f(x)` | `windjammer_runtime::task::local(move \|\| f(x))`: queued on the current thread's local executor; runs when the owner waits, a channel blocks, or the scope ends | exact output; overlap only at blocking points (as the interpreter) |
| `t.wait()` / `t.await` | `t.wait()` (join; a panic payload is resumed with `std::panic::resume_unwind`) | exact |
| `ch.send(v)` / `ch.recv()` | `std::sync::mpsc` (unbounded), `sync_channel(n)` (bounded); `recv()` panics on a closed channel as in the interpreter | exact |
| `select { }` | `windjammer_runtime::task::select` over channel readiness (a small `Condvar` multiplexer, no busy wait) | exact for channels; timers via `recv_timeout` |
| panic in a task | the scope joins every handle, then resumes the first panic | exact |

Send/Sync: the IR solver already decides ownership; it gains a `Sendable` fact per captured value (a type is sendable
when it contains no `Rc`, `RefCell` or non-`Send` extern type, computed structurally over the type registry). A
`spawn` whose captures are not sendable is a Windjammer error naming the value and the type ("`cache` cannot move to
another thread: it holds an `Rc`"), with the fix "use `async`" (same thread) or "wrap it in `Shared`"; never a rustc
`Send` error. The user never writes `Send`.

Variants on demand (decision 5): a function called with `async` needs no special variant in this design (it runs on
the local executor as an ordinary closure), so function colouring disappears without generating async twins. A call
the backend cannot satisfy (an `extern` that blocks inside `async` on JS) is the error decision 5 describes.

Conformance: the stage-1 programs compiled with `wj run --compiled` must print the expected output; `wj run --verify`
compares interpreter and compiled output (ADR-0033); seed sweeps do not apply to OS threads, so each program also runs
100 times compiled under `stress` load in the PC verification flow.

## JavaScript and WASM (stage 3)

Single-threaded by default (engine principle: the browser main thread is the game loop). `async` maps to promises on
the event loop; `spawn` maps to the same loop unless the solver proves the task shares nothing but channel messages,
in which case it may be placed on a Web Worker (opt-in per build, `--threads`, because workers need cross-origin
isolation headers for shared memory).

| Windjammer | JS | Exact? |
|---|---|---|
| `async f(x)` / `spawn f(x)` (default) | `__wj.task(async () => f(x))`: a promise scheduled with `queueMicrotask`; generated functions that block become `async function`s transitively (decided by the compiler, invisible in source) | exact output; no parallelism |
| `t.wait()` | `await t` inside generated async code | exact |
| `scope { }` | `await __wj.scope(async (s) => { ... })` collecting promises, `Promise.allSettled` then rethrow the first rejection | exact |
| channels | an async queue (`recv` returns a promise) | exact |
| `select { }` | `Promise.race` over the arms' readiness promises, losers re-queued | exact for channels |
| blocking call in `wait()` on the main thread with a Worker task | not allowed: WJ error "blocking wait on the browser main thread" (decision 5) | refused, never approximated |

WASM (through the Rust generator): stage 2b's lowering with the `one_thread` runtime the interpreter playground uses
(tasks as nested calls); `wasm32-unknown-unknown` has no threads, so `spawn` runs on the local executor, exact output,
no speedup. `wasm32-wasip1-threads` can use the OS-thread lowering later.

## Go (stage 4)

| Windjammer | Go | Exact? |
|---|---|---|
| `spawn f(x)` / `async f(x)` | `t := wj.Go(func() T { return f(x) })` (goroutine + result slot + `sync.Once`) | exact |
| `t.wait()` | `t.Wait()` (re-panics with the task's value) | exact |
| `scope { }` | `wj.Scope(func(s *wj.S) { ... })` with a `sync.WaitGroup`; `return` inside lowered like the Rust `ScopeExit` | exact |
| channels | Go channels (unbounded: a goroutine-backed queue; bounded: `make(chan T, n)`) | exact |
| `select { }` | Go `select` | exact |

## Effects, capabilities and security

- `spawn` and `async` are not effects (SPEC-EFF-007): a task's effects are the spawning function's, so capability
  checks see through tasks, and a capability denial inside a task fails the task like any panic (already in the
  escape suite). No new capability is introduced for concurrency; a `[policy] max_tasks` limit (resource, not
  authority) is proposed for 2a, enforced by the scheduler and the runtime's spawn.
- Taint (ADR-0009): channels carry taint with their values (the effect facts already flow through calls; stage 2a adds
  `send`/`recv` as flow edges, so `rx.recv()` is tainted when any `send` into the channel is). Test: a tainted value
  sent through a channel and used as a path is reported (`tests/effects_escape_test.rs` style).
- Durable functions: a `durable fn` may not start tasks in stage 1 (WJ0047 already refuses channel receives in durable
  code); journaled tasks are after beta.
- Record/replay and simulation: task interleavings are a function of the schedule seed (SPEC-EFF-028), so the new
  forms are replayable with no format change.

## Game engine

Single-threaded default: the frame loop never blocks on a task; `async` is the tool for overlapping I/O (asset loads)
on the main thread. Parallel work goes through a job system built on `scope`: `scope { for chunk in chunks { spawn
update(chunk) } }` per frame, a pool-backed `scope` in the Rust runtime (stage 5) so frames do not pay thread start
cost, and the same code runs single-threaded on web. Owning every interface (engine principle) means the job system
is a Windjammer module over these primitives, replaceable later by a pure-Windjammer scheduler. Benchmark gate:
a particle update at 100k entities, frame time vs. single-threaded, on the PC.

## Agents (no deadlock surprises)

- Every task is waited on (WJ0190), so results and panics cannot vanish; scope exit is the single join point.
- Deadlocks are reported, never hangs, in the interpreter (`deadlock: every thread is blocked`); stage 2a adds the
  blocked tasks and what each waits on to the message, and `wj test --simulate --seeds` sweeps schedules for task
  programs.
- Errors propagate structurally: a task panic surfaces at `wait()` or at the scope; `Result`-returning tasks give
  their `Result` to `wait()`.
- The playground (one-thread mode) runs the same programs or says that a task would have to resume; it never
  prints something different (`tests/interpreter_one_thread_tasks_test.rs`).

## What is not verified yet

- Compiled execution of the new forms (all backends): stage 2b onward.
- Performance of the interpreter scheduler under thousands of tasks (each task is an OS thread with a private stack
  in thread mode; a fiber-based mode is a candidate for 2a if the agent workloads need it).
- The one-thread mode on the new conformance programs: they wait only on tasks that start later, so they are expected
  to agree; not yet gated.

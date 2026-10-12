---
status: proposed
last_verified: d893d6e8
---
# ADR-0019: Lightweight processes, mailboxes and supervision (Elixir-inspired)

- **Status:** Proposed
- **Date:** 2026-10-11
- **Related:** [ADR-0004](0004-concurrency-surface.md) (accepted: `async`/`spawn`, `Task<T>`, `scope { }`, channel methods, `select`; supervisors deferred), [ADR-0007](0007-target-tiers-conformance.md), [ADR-0018](0018-expressiveness-phase-2.md), `docs/spec/concurrency.md`

Design only. Nothing here is implemented; no syntax below parses today.

## Context
ADR-0004 gives Windjammer structured concurrency: `spawn f()` returns a `Task<T>` that must be waited on inside a
`scope { }`, channels are values with `send`/`recv`, and `select { }` multiplexes. That covers fork/join and pipelines.
It does not cover long-lived, independently failing actors: servers, connection handlers, game entities, agents
that hold state and talk by message. Erlang/Elixir show the model that makes those reliable: cheap isolated
processes, one mailbox each, links, and supervisors that restart failed children by policy ("let it crash").
Agents write these programs often (bots, crawlers, job runners) and get shared-memory locking wrong; a message-only
model is easier to get right first time and easier to test deterministically.

## Decision (proposed)
Layer a process model **on top of** ADR-0004, as a library-first design with minimal syntax:

```text
let pid: Pid<CounterMsg> = spawn_process(counter, 0)     // a process = a function looping on its mailbox
pid.send(Inc(5))                                          // asynchronous, never blocks
let n = pid.call(Get)                                     // request/reply with timeout (sugar over send + reply channel)

fn counter(state: int, inbox: Inbox<CounterMsg>) {
    for msg in inbox {                                    // receive loop; ends when every sender is gone
        match msg {
            Inc(k) => state += k,
            Get(reply) => reply.send(state),
        }
    }
}

supervise(strategy: OneForOne, max_restarts: 3, within: 5.seconds) {
    child("counter") { spawn_process(counter, 0) }
    child("logger") { spawn_process(logger, ()) }
}
```

Semantics:
- A process owns its state; messages are **moved** (ownership transfer, so no shared mutable state crosses a
  mailbox; the IR solver enforces it like any move). `Pid<M>` is typed by its message enum.
- A process that panics terminates alone; linked processes and its supervisor receive an `Exit(pid, reason)`.
- Supervisors restart children by strategy (`OneForOne`, `OneForAll`, `RestForOne`) within a restart budget;
  exceeding it fails the supervisor itself (escalation).
- Mailboxes are unbounded by default with an optional bound (`spawn_process(f, s, mailbox: 1000)`) that applies
  back-pressure on `send` (named argument, ADR-0018 feature 3). Selective receive is `inbox.recv_matching(pattern)`.
- Everything is inside a `scope { }` (ADR-0004 structured concurrency): leaving the scope stops its processes.

## Ambition ladder
Each rung ships alone, with conformance programs on every backend that claims it.
1. **Typed mailboxes over ADR-0004 tasks** (library only): `spawn_process`, `Pid<M>.send`, `Inbox<M>` iteration,
   `call` with timeout. Prerequisite: ADR-0004 `spawn`/`scope` and channels on the backend.
2. **Failure isolation and links**: panics are caught per process and turned into `Exit` messages.
3. **Supervisors** with the three strategies and restart budgets; trailing-block `child(..) { }` (ADR-0018 feature 4).
4. **Deterministic testing**: the interpreter's deterministic scheduler (being built in `wt/interp-gaps2`) runs a
   process system reproducibly from a seed; `wj test` can replay an interleaving that failed.
5. **Registry and named processes**, `GenServer`-style behaviours as traits (`handle_call`, `handle_cast`).
6. **Distribution** (processes on other machines) - only with a separate ADR; not part of this one.

## Mapping per backend
| Concept | Rust | Go | JS/TS | Interpreter |
|---|---|---|---|---|
| process | `tokio::task::spawn` (multi-thread runtime) or `std::thread` for blocking work | goroutine | async function on the event loop; `worker_threads` / Web Workers only for CPU-bound children | task in the deterministic scheduler |
| mailbox | `tokio::sync::mpsc` (bounded/unbounded) | buffered `chan` | an async queue (array + resolver) | in-memory queue, scheduled deterministically |
| `call` | mpsc + `oneshot` reply, `tokio::time::timeout` | `chan` + `select` with `time.After` | `Promise.race` with a timer | scheduler-level timeout in virtual time |
| panic isolation | `JoinHandle` returns `Err(JoinError)` on panic (`panic = "unwind"` required) | `recover()` in the goroutine wrapper | `try/catch` around the process body | error value per task |
| supervisor | a process owning `JoinSet` + restart policy | goroutine + `sync.WaitGroup` | async loop over child promises | same logic, virtual time |

Ownership: messages move, so Rust needs `M: Send + 'static` (the solver reports a non-sendable message type at
the `send` site with a Windjammer diagnostic, not a rustc trait error). Go/JS copy on send for value types
(the portable backends already clone on move where needed).

## Prerequisites, first experiment, benchmark, kill criteria
- Prerequisites: ADR-0004 rung 1 (`spawn`/`scope`, channels) on Rust and the interpreter; the deterministic
  scheduler in the interpreter.
- First experiment: implement rung 1 as a pure Windjammer library over channels (`std/process.wj`), port two
  agent-dx concurrency tasks (t043 channel pipeline, t046 producer/consumer) to it, compare first-draft pass rate
  and tokens against the channel versions.
- Benchmark: 100k processes ping-ponging on Rust (memory per process, messages/s) against a hand-written tokio
  version; target within 20%.
- Kill criteria: if rung 1 does not improve first-draft pass rate on the concurrency tier, or costs more than 2x
  memory per process versus raw tokio tasks, stop at channels + `scope` (ADR-0004) and document actors as a
  library pattern instead of a language feature.

## Decision needed from: project owner
Accept the layering (library first, on ADR-0004) and the ladder; choose whether rung 3 syntax (`supervise { }`)
may use trailing blocks or must stay plain function calls.

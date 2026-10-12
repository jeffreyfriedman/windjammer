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

## Durable processes (optional)
Input (2026-10-11): Cloudflare's acquisition of Deno was reported to centre on celld, an open-source Rust
implementation of the Durable Objects model that depends only on object storage for coordination and persistence,
so the Workers programming model can be self-hosted. The model fits this ADR: a process is already a single-writer
owner of its state; making that state durable and the identity stable gives the same program three deployments.

**Model.** `durable process` = a process with
- a **stable identity** (`DurableId<M>` derived from a name or key, e.g. `room("lobby")`), at most one live instance
  per identity at a time (single writer), activated on first message and passivated when idle;
- **transactional persisted state**: the state is loaded on activation; each handled message runs as a transaction
  whose writes commit atomically after the handler returns (or are discarded on panic, so a supervisor restart sees
  the last committed state); output messages sent during the handler are delivered only after commit (no
  externally visible effect of an aborted transaction);
- **location transparency**: `id.send(msg)` / `id.call(msg)` work the same whether the instance is in this process,
  in another binary on this machine, or on another machine.

**Storage port.** Durability goes through one trait, never a hard-wired backend:

```text
trait DurableStore {
    fn load(id: DurableId) -> Result<Bytes?, StoreError>
    fn commit(id: DurableId, expected_version: int, state: Bytes) -> Result<int, StoreError>  // compare-and-set
    fn lease(id: DurableId, owner: NodeId, ttl: Duration) -> Result<Lease, StoreError>         // single-writer
}
```

Adapters: in-memory fake (tests, interpreter, deterministic scheduler), local file / SQLite (single self-hosted
binary), object storage with conditional writes (distributed: leases and versions are the only coordination, as in
celld). The compare-and-set version makes split-brain writes fail instead of corrupting state.

**Same program, three deployments.** Interpreter (in-memory store, deterministic, replayable), one self-hosted binary
(SQLite/file store, all actors local), distributed (object-storage store, actors placed by lease on any node; a thin
router forwards messages to the lease holder).

**Mapping onto a database's actor/reducer layer.** A database built as single-writer actors with reducers
(state' = reduce(state, command)) maps directly: each table partition or document is a durable process, a reducer
is the message handler, the storage port is the database's own log/page store, and commit = append to its log.
The language contributes typed messages, ownership-checked state (no shared mutable state escapes a handler) and the
deterministic interpreter for testing reducers; the database keeps its own storage engine behind `DurableStore`.

**Capabilities (Deno's lesson).** Deno's permission flags were reported to be its most-loved feature; Node still has
no per-host network allowlist. Processes are the natural unit for capabilities: a process (durable or not) is spawned
with an explicit capability set, tied to the effects/capability work, at fine grain - e.g.
`spawn_process(fetcher, s, caps: [net("api.example.com:443"), fs_read("./data")])`. A child can only receive a
subset of its parent's capabilities; the storage port is itself a capability. The interpreter enforces them at the
call sites of std effects; compiled backends check at the std boundary (Rust runtime, Go/JS wrappers). Violations
are errors naming the missing capability, not silent failures.

**Ambition ladder (durable).**
- D1: `DurableStore` trait + in-memory and SQLite adapters; durable processes on one node; tests in the interpreter
  with deterministic crashes (kill between handler and commit, assert state = last commit).
- D2: capability sets on `spawn_process`, enforced in the interpreter and the Rust runtime (per-host net, fs paths).
- D3: object-storage adapter with leases + compare-and-set; multi-node routing; chaos test (two nodes race for a lease).
- D4: a database's actor/reducer layer plugged in through `DurableStore` as an external adapter (separate repo).

**Kill criteria (durable).** Stop at D1 if the persisted-state transaction adds more than 2x latency per message
over an in-memory process with the SQLite adapter on a local benchmark, or if exactly-once-after-commit delivery
cannot be guaranteed by the store contract (then durable processes become a library pattern, not a language
feature). Drop D3 if leases on the object-storage adapter cannot be made safe without a separate coordination
service (that would contradict the "only object storage" premise).

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

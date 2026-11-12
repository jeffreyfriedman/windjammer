---
status: proposed
last_verified: a9bcd09a3
---
# ADR-0035: Time-travel debugging from the effect journal

- **Status:** Proposed (first slice implemented: `wj replay` with a command script; see "Slice status")
- **Date:** 2026-11-11
- **Related:** [ADR-0021](0021-effects-and-capabilities.md) (section 10: the effect port, `--record`, `--replay`,
  `--simulate`), [ADR-0032](0032-raw-sockets.md) (sockets through the port, the virtual network),
  [ADR-0020](0020-interpreter-execution-engine.md) (the tree walker is the reference engine), `docs/INTERPRETER.md`,
  `docs/testing/INTERPRETER_CONTRACT.md`, `src/interpreter/effect_port/`, `src/interpreter/debugger/`,
  `tests/time_travel_test.rs`

## Context

Windjammer already makes a run reproducible. `wj run --record LOG` writes every effect whose answer can differ between
two runs (file, environment, process and network results, standard input, real-clock readings) to a journal;
`wj run --replay LOG` re-runs the program and answers each effect from the journal, performing none, and it is
bit-exact on the replay corpus (ADR-0021 E3 gate, `tests/effect_replay_test.rs`). `wj run --simulate --seed N` runs
in a seeded world with a virtual clock, seeded random numbers, a seeded scheduler and in-memory files and sockets.

What is missing is a way to *look inside* such a run. Today an agent that has a failing seed or a recorded failure can
reproduce the failure, but to learn the value of a binding at the failure it edits the program to print it, which
changes the program digest, which makes the recorded log diverge (`WJ0049`). A debugger that reads state without
changing the program closes that loop.

Agents are the primary users. An agent cannot drive an interactive prompt well: it sends one command, reads one
answer, and pays a round trip for each. It needs a debugger that takes a whole script of commands and answers in a
machine-readable form.

## Decision

### 1. Deterministic replay is the time machine

A replayed or simulated run is a function of the program, the journal (or the seed) and nothing else. Re-executing it
from the start reaches the same state at the same step every time. So "go back to step K" is "run again from the
start and stop at step K". No snapshots, no reverse execution, no undo log of the heap: the effect journal plus
determinism give time travel for free.

A **step** is one statement started by the tree walker (the reference engine, ADR-0020), counted from 1 across the
whole run and every task. The debugger always runs the walker, with machine code off, so a step means the same thing
in every run.

### 2. Command surface

```
wj replay LOG PROGRAM [--script FILE|-] [--json] [--allow EFFECT[=RES]]...
wj replay --simulate [--seed N] [--faults R] PROGRAM [--script FILE|-] [--json]
```

The first form replays a journal written by `wj run --record LOG PROGRAM`; seeds, arguments and the time mode come
from the journal's header, as for `wj run --replay`. The second form debugs a simulated run. Commands come from
`--script FILE`, or from standard input when no script is given (`--script -` says so explicitly): one command per
line, `#` starts a comment. The replayed program's own standard input is answered from the journal, so standard input
is free for commands.

Commands of the first slice:

| Command | Meaning |
|---|---|
| `break LINE` / `break FILE:LINE` | stop when a statement on that line starts (once per entry to the line) |
| `continue` | run to the next breakpoint, the failure, or the end |
| `step [N]` | run N statements (default 1) |
| `back [N]` | go back N statements (default 1): re-execute from the start to step K-N |
| `print NAME` | the value of a binding in the current frame (`{:?}` form) |
| `locals` | every binding of the current frame |
| `where` | the call stack, innermost first, each frame with its file, line and column |

The session starts stopped before step 1. When the program fails (a panic, or a runtime error), the debugger stops
**at the failure, before the stack unwinds**, so `print` and `where` see the failing frame. `continue` from there lets
the failure finish; the session then reports how the program exited. `back` works after the end too.

Planned, not in the first slice: `next` (step over a call), `finish` (run to the caller), `goto STEP`,
`--at-effect K` (stop just before journal event K is answered; the stop events already carry the journal position),
`print EXPR` (a side-effect-free expression evaluated in the stopped frame), watchpoints (`watch NAME`: stop when the
value changes, found by re-execution with a value digest per step), and an interactive prompt with history.

### 3. Output for agents

`--json` prints one JSON object per line on standard output, nothing else. Without it the same events are printed as
short text lines for people. The events:

| Event | Fields |
|---|---|
| `stopped` | `reason` (`start`, `breakpoint`, `step`, `back`, `failure`), `step`, `task`, `function`, `file`, `line`, `column`, `effect` (journal events answered so far; replay only), `message` (failure only) |
| `value` | `name`, `value` |
| `locals` | `function`, `bindings`: `[{name, value}]` |
| `stack` | `frames`: `[{function, file, line, column}]`, innermost first |
| `output` | `stream` (`stdout`, `stderr`), `text`: what the program wrote since the last event. Each byte of program output is reported once per session, even when `back` re-executes the code that wrote it |
| `exited` | `code` (the exit status `wj run` would have had), `steps`, `message` (the rendered error, when there is one) |
| `error` | `command`, `message`: a command the debugger could not carry out; the session goes on |

`file` is relative to the program's directory when it lies inside it. The process exit status of `wj replay` is 0
when every command was understood, 1 when the script itself is bad (an unknown command, an unreadable file); the
program's own exit status is in the `exited` event.

### 4. What is deterministic, and what is not

Deterministic, and therefore reproduced exactly at every step after any number of `back`s:

- every effect answered by the port (ADR-0021 section 10): file, environment, process, network, `mcp::call`, stdin,
  real-clock readings, and the host observations (`process::pid`, `env::temp_dir`, ...);
- random numbers (seeded from the header), the scheduler's interleavings (seeded), `HashMap` iteration (hash seed),
  sleeps (virtual clock);
- the simulated world as a whole (a function of the seed and the fault rate).

Not covered, stated plainly:

- **Programs `--record` refuses.** Live handles (`http::serve*`) are refused by the port, and recorded concurrent
  socket programs are a known gap (ADR-0032); debug those with `--simulate`.
- **Calls into Rust crates and `extern fn`s** are journaled as one opaque event each; the debugger cannot step inside.
- **Compiled code.** The debugger is the interpreter. A bug that only the compiled program shows (the interpreter
  contract says it should not exist) cannot be stepped. Compiled record/replay is ADR-0021 E7.
- **A user `impl Debug` that performs effects.** `print` formats with `{:?}`; output the formatting writes is
  discarded, but an effect it performs would consume a journal event. Derived and built-in `Debug` perform none.
- **Edited programs.** The journal records the program's digest. A replay of an edited program diverges at the first
  changed request with `WJ0049`, as `wj run --replay` does.
- **Durable journals** (`--journal DIR`) are on-disk state outside the effect log; debugging a durable run starts from
  the journal's current contents.

### 5. Capabilities

Replay never opens a real file or socket: every effect is answered from the journal, the simulated world is in
memory, and the gate (ADR-0021) still runs, so a call the recording was denied is denied again. `wj replay` takes the
same `--allow` grants as the recording (a call that the recording was granted, but the replay is not, is denied and
then diverges, which `exited` reports). Planned: write the grant into the journal's header so a replay needs no flags.

### 6. Cost model

Without snapshots, `back N` from step K costs one re-execution of K-N steps in the walker; `continue` and `step` cost
only the steps they run. A session of C commands that go back costs at most C full replays. Measured on the development
machine (2026-11-11, a loop of 1,000,004 statements under `wj replay --simulate`, `continue`): 0.46 s of user time,
1.6 s wall on a loaded machine, against 0.18 s for `wj run --engine walker` with no debugger. A replay performs no I/O,
so for the runs an agent debugs (a test, a failing seed, a short tool run: well under a million steps) a `back` costs
about a second or less. Periodic snapshots (a copy of the interpreter state every S steps, so `back` re-executes at most S steps) are
the optimization to reach for **only** when a measured session shows `back` above 5 seconds on a program people
actually debug; the interpreter's state includes threads and native handles, so snapshots are a large piece of work
that this design does not need yet.

The hook costs one `Option` test per statement in a run without the debugger.

### 7. Acceptance metric

1. **Two commands.** Given a failing simulated run (`wj run --simulate --seed N` exits with a panic), an agent reads the
   value of a binding at the failure with exactly two debugger commands, `continue` then `print NAME`, in one
   `wj replay` call, and the value equals what the failing run computed (`tests/time_travel_test.rs`).
2. **Replay parity.** The program output reported by a `wj replay` session that only says `continue` is byte-identical
   to the output of the recording run, and its exit status is the same.
3. **Back then forward.** For every step K of the test programs, going back from K and stepping forward again reproduces
   the stopped and locals events of the original pass exactly.
4. **Nothing is touched.** A replay that reads a recorded file runs in a directory where that file does not exist.

Kill or rethink: if a test program shows a `back`-then-forward difference that is not a port-routing bug (the
`every_host_read_in_the_interpreter_is_routed_or_justified` audit finds non-determinism, which is fixed by routing it),
the "re-execution is the time machine" claim is wrong for that class of program and snapshots stop being optional.

## Consequences

- One more subcommand, `wj replay`, with the flags above in the promise registry.
- `exec_stmt` in the walker gains a debugger check; the resolved engine and machine code are untouched (the debugger
  does not use them).
- The debugger lives in `src/interpreter/debugger/` (script parsing, the stop hook, the session driver, rendering);
  the CLI adapter in `src/cli/replay.rs`.

## Slice status

Implemented (first slice): both command forms, `--script`, `--json`, `--allow`; the commands `break`, `continue`,
`step`, `back`, `print`, `locals`, `where`; the stop at a failure before unwinding; the events above. Gates in
`tests/time_travel_test.rs`: golden transcripts for a recorded file read and a seeded simulated failure, replay parity
with the recording, and back-then-forward equality at every step. Book page:
`docs/book/src/tutorial/time-travel.md` ("Debug a failing run deterministically").

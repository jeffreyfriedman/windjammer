# ADR-0021: `wj run --watch` and hot reload of function bodies

- **Status:** Proposed (watch implemented; hot reload designed, not implemented)
- **Date:** 2026-10-11
- **Related:** [ADR-0019](0019-interpret-by-default.md), `src/cli/watch.rs`, `src/interpreter/project.rs`

## Context
With interpretation as the default (ADR-0019) a program starts in milliseconds, so "re-run on save" is the natural
loop. Long-running programs (game loops, servers) lose their state on every restart; for those, swapping changed
function bodies into the running process keeps the state and the window.

## Decision 1: `--watch` (implemented)
`wj run --watch` / `wj test --watch` re-invoke `wj` without `--watch` as a child process and restart it when a
`.wj`, `.rs`, `wj.toml` or `wj.lock` file under the project root changes. Detection polls file metadata every 20 ms
(portable; no platform notification APIs, works on network and VM file systems). Restart latency = poll interval +
process start + interpreter load; measured in `docs/INTERPRETER.md`. Target: under 100 ms for a mid-size project.

## Decision 2: hot reload (design; next step)
`wj run --hot` keeps one interpreter process alive and applies edits at a **safe point**:

1. A watcher thread sets an atomic "reload pending" flag with the changed files.
2. Safe point: the interpreter checks the flag on function entry when the call stack depth equals the depth of the
   outermost user loop (the frame running the game/server loop), i.e. between iterations, never mid-call.
3. Reload: re-parse the changed files, run the shared resolver on the new program, and diff definitions by `DefId`
   path:
   - function/method bodies whose signature is unchanged are swapped in `modules[m].funcs` / `methods`
     (closures already created keep their old body, like Erlang's old/new code);
   - a changed signature, struct/enum layout, trait, `const` or module structure forces a **restart** (the
     `--watch` behaviour) with a one-line notice naming the change: state whose type changed cannot be kept.
4. Globals and heap values are untouched (same value model, same types), so state survives.

Spike plan: a `game_loop` fixture whose `update()` prints a counter; edit `update` to print `counter * 10`; assert
the counter continues (state kept) and the new body runs within one loop iteration after the edit. Implementation
touches `call.rs` (safe-point check, one branch) and adds `src/interpreter/hot.rs` (diff + swap).

## Kill criteria
Hot reload is dropped (watch remains) if a swapped body can observe a value whose layout changed (must restart
instead), or if the safe-point check costs more than 1% on the interpreter benchmarks.

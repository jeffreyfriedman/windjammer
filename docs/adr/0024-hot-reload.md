---
status: proposed
last_verified: 5809fb56e863
---
# ADR-0024: `wj run --watch` and hot reload of function bodies

- **Status:** Proposed (watch implemented; hot reload spike implemented as `wj run --hot`)
- **Date:** 2026-10-11
- **Related:** [ADR-0022](0022-interpret-by-default.md), `src/cli/watch.rs`, `src/interpreter/project.rs`

## Context
With interpretation as the default (ADR-0022) a program starts in milliseconds, so "re-run on save" is the natural
loop. Long-running programs (game loops, servers) lose their state on every restart; for those, swapping changed
function bodies into the running process keeps the state and the window.

## Decision 1: `--watch` (implemented)
`wj run --watch` / `wj test --watch` re-invoke `wj` without `--watch` as a child process and restart it when a
`.wj`, `.rs`, `wj.toml` or `wj.lock` file under the project root changes. Detection polls file metadata every 20 ms
(portable; no platform notification APIs, works on network and VM file systems). Restart latency = poll interval +
process start + interpreter load; measured in `docs/INTERPRETER.md`. Target: under 100 ms for a mid-size project.

## Decision 2: hot reload (spike: `src/interpreter/hot.rs`, test `hot_reload_swaps_a_body_and_keeps_state`)
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

Spike (landed): a loop calls `step::step(state)`; the test edits `step` from `n + 1` to `n + 1000` while it runs and
asserts the counter keeps growing (no restart, state kept) and the new body runs. The spike checks at every function entry
(one branch when `--hot` is off); the "outermost loop" safe point of step 2 is the next refinement. Signature/type changes
print a restart notice and keep the old code (swapping only bodies whose signatures match).

## Kill criteria
Hot reload is dropped (watch remains) if a swapped body can observe a value whose layout changed (must restart
instead), or if the safe-point check costs more than 1% on the interpreter benchmarks.

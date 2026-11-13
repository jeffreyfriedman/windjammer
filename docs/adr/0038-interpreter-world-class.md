---
status: proposed
last_verified: 1ffe2962e
---
# ADR-0038: A world-class interpreter, as a ladder of measured rungs

- **Status:** Proposed
- **Date:** 2026-11-12
- **Related:** [ADR-0020](0020-interpreter-execution-engine.md) (engines), [ADR-0022](0022-interpret-by-default.md)
  (interpret by default), [ADR-0033](0033-interpreted-first-cost.md) (interpreted-first cost),
  [ADR-0034](0034-book-playground.md) (playground), [ADR-0035](0035-time-travel-debugging.md) (time travel),
  [ADR-0036](0036-runtime-errors.md) (runtime errors), [ADR-0037](0037-agent-cost-levers.md) (agent cost),
  `scripts/wj/interp_instr.wj`, `tests/interpreter_fallbacks_test.rs`, `scripts/wj/drift_gate.wj`

## Context

The interpreter is where a Windjammer program first runs: `wj run`, `wj test --interpret`, the REPL, the book
playground, `wj replay`, and every interpreted script and gate of this repository. It starts instantly, is
deterministic under `--simulate`, replays recorded effects, and runs in a browser tab. Those are properties a
compiled-only language cannot offer, and they compound: every agent loop and every gate that runs interpreted is
faster, cheaper and reproducible. Making the interpreter world class is therefore a product goal, and preferring it
ourselves (interpreted-first scripts, tests, gates and agent loops) is how we find what it still lacks.

"World class" needs numbers. This ADR defines five ladders (performance, correctness, developer experience,
coverage, cost). Each rung names a metric, today's value, a target and the gate that holds it. A gate is a ratchet:
a number may only move toward its target, and a move is recorded in the repository, never typed into prose.

## How performance is measured

- **Instructions, not seconds.** The build machine runs at load 10 to 60; CPU time on it moves by 2x with no code
  change. `scripts/wj/interp_instr.wj` counts instructions: callgrind where valgrind exists (Linux, the PC), the
  hardware counter of `/usr/bin/time -l` ("instructions retired") on macOS. Each count is the smallest of N runs
  (the counter's noise only adds; spread is under 1% for kernels, about 3% for startup). Counts of different
  counters are never compared: `bench/interp/instr_baseline.json` keeps one table per counter and machine kind.
- **Output first.** Every counted run must print the kernel's recorded output (`bench/interp/expected/`); a faster
  interpreter that prints something else fails before any number is compared.
- **References.** The same kernels exist as twins (`bench/interp/twins/`) for QuickJS (`qjs`, a bytecode
  interpreter with no JIT), Lua 5.5 (the classic fast interpreter), CPython and Node (V8, a JIT). `--refs` counts
  them on the same counter. Ratios are **net of startup**: each side's `hello` program is subtracted, so a ratio
  compares the kernel's own work.
- **Fallbacks are counted.** Every slower path of the resolved engine (a walker island, a call run in the walker, a
  deopt, a loop machine code refuses or gives up on, a machine-code exit at a failed check) is counted and printed
  by `WJ_JIT_DEBUG` (`[fallback] ...`). `tests/interpreter_fallbacks_test.rs` holds the kernels' counts to
  `bench/interp/fallbacks.tsv` exactly, so a fast path that silently stops being taken shows up as a count that grew.

## Ladder 1: performance

Measured on this head, Apple M-series, `retired:Darwin-arm64`, millions of instructions, net ratio = (wj kernel -
wj startup) / (reference kernel - reference startup). Below 1.0 means Windjammer does less work.

| Kernel | wj (M instr) | net vs QuickJS | net vs Lua 5.5 | net vs CPython | net vs Node (JIT) |
|---|---|---|---|---|---|
| closures (map/filter/sum chains) | 893 | 1.21 | 2.16 | 2.22 | 4.84 |
| fib30 (calls) | 213 | 0.09 | 0.16 | 0.07 | 0.59 |
| hashmap (insert/get) | 526 | 1.47 | 6.89 | 0.64 | 6.38 |
| loop5m (integer loop) | 367 | 0.12 | 0.25 | 0.02 | 1.28 |
| nbody (f64 over Vec of structs) | 905 | 0.80 | 1.31 | 0.55 | 5.67 |
| sieve (Vec<bool>) | 616 | 0.52 | 1.33 | 0.09 | 2.68 |
| strings (interpolation, push_str) | 438 | 0.58 | 1.02 | 0.37 | 5.79 |
| structs (methods on a Copy struct) | 184 | 0.04 | 0.04 | 0.02 | 0.73 |
| geometric mean | | 0.34 | 0.67 | 0.17 | 2.49 |

Startup (`fn main() { println("hello") }`, the default `wj run` path, check included): wj 119, Lua 11, QuickJS 14,
CPython 136, Node 214.

The first rungs of this ADR's own work moved nbody: loop entry checks the cached classification in place, and a
call with an inferred `&mut` parameter is compiled instead of running as a walker island. nbody 905M -> 728M
instructions, net 0.63 vs QuickJS and 1.03 vs Lua; kernel islands 21,304 -> 1,304 (geometric means 0.33 vs
QuickJS, 0.66 vs Lua).

The integer kernels (fib30, loop5m, structs) already run as machine code and beat every interpreter; the gaps are
the kernels whose hot loops still leave machine code: closures (a walker island per iterator chain), hashmap (both
loops refused), nbody (an island per call of `advance`, plus loop-entry cost), strings (islands, loops refused).

| Rung | Metric | Now | Target | Gate |
|---|---|---|---|---|
| P0 | every kernel and startup counted, outputs checked, fallbacks ledgered | done | stays done | `interp_instr.wj --check` (+3%), `cargo test --release --test fallbacks` |
| P1 | worst kernel, net vs QuickJS | 1.47 (hashmap) | <= 1.0 | `interp_instr.wj --refs`, baseline shrink-only |
| P2 | startup instructions | 119M | <= 60M, then <= 25M | `instr_baseline.json` `startup` |
| P3 | worst kernel, net vs Lua 5.5 | 6.89 (hashmap) | <= 1.0 | `--refs` |
| P4 | islands and refused loops in kernels | 1,304 islands (was 21,304), 6 refused loops | 0 in any hot loop | `bench/interp/fallbacks.tsv` (exact) |
| P5 | geometric mean, net vs Node (V8) | 2.49 | <= 1.0 (JIT class) | `--refs` |
| P6 | a kernel suite of real programs (JSON, regex, a game tick, a parser) | not yet | added, counted, P1-P5 apply | new kernels in `bench/interp/` |

## Ladder 2: correctness (the compiled program is the oracle)

| Rung | Metric | Now | Target | Gate |
|---|---|---|---|---|
| C0 | drift fast gate: fuzz, corpus (walker, resolved, recorded compiled), spec oracle | fuzz 154 cases 0 divergent; corpus 3047 cases, all divergences ledgered; spec 229 cases, all ledgered | clean on every merge | `scripts/wj/drift_gate.wj --fast` |
| C1 | known divergences in the drift ledger | 90 (ceiling 90) | 0 | `tests/drift_corpus/known_divergences.txt` shrink-only, `ledger_ceiling.txt` |
| C2 | spec-oracle known divergences | 14 | 0 | `tests/spec_oracle/known_divergences.txt` shrink-only |
| C3 | fuzz divergences | 0 | 0 | `tests/drift_corpus/fuzz_divergences.txt` |
| C4 | fuzz throughput (generated cases checked, walker vs resolved) | 154 cases in 14 s wall at load 50 (about 11 per second) | 10x, so the long seed range (`--fuzz-long`) runs on every merge | `drift_gate.wj` step `fuzz` |
| C5 | readiness R2 (no silent divergence) and R5 (diagnostics agree) | 98.5% and 95.1% | 100% | `docs/generated/interp_readiness.json` |
| C6b | integer widths: an unsuffixed literal gets the width the numeric solver gives it in the compiled program (ADR-0028 `literal_widths`), so a `usize` counter underflows interpreted exactly where it panics compiled | the interpreter keeps a literal's width open (`IntTy::Lit`); a countdown `while i >= 0 && ..` that indexes a `Vec` ends at -1 interpreted and panics compiled | the interpreter reads the solver's per-function `literal_widths` (needs `crate::ir` among its allowed imports and the solver in `wj run`'s check, paid in startup instructions, P2) | a drift corpus case per width class; `interp_instr.wj` for the startup cost |
| C6 | engines agree with each other | walker == resolved == machine code on every corpus program | stays | `engine_parity`, `fast_paths`, `fallbacks`, jit differential tests |

## Ladder 3: developer experience

| Rung | Metric | Now | Target | Gate |
|---|---|---|---|---|
| D1 | `wj run` interprets by default; a fallback to the compiler is rare and loud | readiness R3 passes | stays | readiness R3 |
| D2 | runtime errors: source caret, frames, locals, JSON | landed (ADR-0036) | an agent fixes a runtime failure in one turn on the friction corpus | `tests/agent_cost_ratchet_test.rs` |
| D3 | time travel: `wj replay` with scripts, `print`, `next`/`finish`, watches | landed on the walker (ADR-0035) | replay on the resolved engine; `back` in O(steps since a checkpoint); a DAP adapter for editors | `tests/time_travel_test.rs` |
| D4 | REPL: continuation, history, recall | landed | values and types inspectable (`:type`), the resolved engine, under 50 ms per line | REPL tests |
| D5 | hot reload between calls | landed (ADR-0024) | reload a running game loop without losing state | hot-reload tests |
| D6 | tokens an agent reads per error (`wj check`, agent format) | 126 | <= 80 | `cost.diag.check_tokens_per_error_agent` |

## Ladder 4: coverage

| Rung | Metric | Now | Target | Gate |
|---|---|---|---|---|
| V1 | std items with a builtin or a named refusal | all (readiness R6) | stays | readiness R6, `interpreter_features` |
| V2 | threads and async | virtual-clock tasks, one thread in wasm | real threads behind the same determinism switch | `interpreter_concurrency_test` |
| V3 | FFI | `extern fn` bridge (ADR-0023), Rust crates through a host process (ADR-0031) | crates in the browser through prebuilt wasm components | `interpreter_rust_crates_test` |
| V4 | playground (wasm) parity with the native interpreter | every book sample but 3 known gaps (one path dependency, two Rust crates) | 0 gaps | `scripts/book/playground_known_gaps.txt` shrink-only |

## Ladder 5: cost (agents working interpreted-first)

| Rung | Metric | Now | Target | Gate |
|---|---|---|---|---|
| K1 | interpreted-first cost per trial vs compiled-first (pilot 5) | 0.026 vs 0.043 USD | keep the 0.6 ratio | pilots, `bench/agent-dx/` |
| K2 | interpreted-first pass@1 on the compiled oracle (pilot 5) | 0.667 (0.967 with one final build) | >= 0.97 with no final build | ADR-0033 predictions, drift ledger |
| K3 | open friction events (offline replay) | 9 | 0 | `cost.friction.events_open` |
| K4 | cost per task vs plain Rust (pilot 6) | 0.040 vs 0.033 USD, 4.8 vs 3.0 turns | below Rust | `bench/agent-dx/friction/ratchet.json` |

## Decision

1. Adopt the five ladders. A rung is "reached" only when its gate enforces the target.
2. Performance claims are instruction counts on a named counter, net of startup against the twins, with checked
   outputs; wall-clock numbers taken on a loaded machine are not evidence.
3. Every fast path keeps its engagement gate (`fast_paths`) and every slower path is counted (`fallbacks`); a
   change that moves a count updates the ledger in the same commit.
4. Order of work, by leverage: the fallbacks of P4 (they are the kernels' gaps to QuickJS and Lua), then startup
   (P2, paid by every script and gate), then fuzz throughput (C4, so the long fuzz range runs on every merge),
   then the V4 path-dependency gap.

## Consequences

- `interp_instr.wj --check` and the `fallbacks` test join the interpreter's merge checks; the PC (Linux, callgrind)
  records its own table in `instr_baseline.json`.
- The reference tools (`qjs`, `lua`) are optional: `--refs` skips a tool that is not installed.
- The ladders are re-measured, not edited: a table here is updated together with the commit that moved it.

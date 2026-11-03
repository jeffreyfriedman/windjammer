# The drift gate

The interpreter (two engines: the reference tree walker and the resolved engine) and the compiled backend
must say the same thing about every program. `scripts/wj/drift_gate.wj` is the one command that checks it:

```
wj run scripts/wj/drift_gate.wj                      # on the integrated head, from the repository root
wj run scripts/wj/drift_gate.wj --fast               # selftest + differential corpus only (no cargo)
wj run scripts/wj/drift_gate.wj --compiled live      # re-run the compiled side now instead of the recorded baseline
```

`scripts/merge_gate.sh` runs it as its last fast step (compiled side from the recorded baseline) and, with `--full`, again with `--compiled live`. Exit status 1 on any divergence. `WJ` (or `--wj`) names the
binary (default `target/release/wj`); `WJ_CARGO_WRAPPER` prefixes cargo and compiled runs (a build-slot gate on a
shared machine).

| Step | What it compares | Where |
|---|---|---|
| selftest | the drift budget against planted divergences (new, changed, vanished, grown, vacuous) | `scripts/wj/drift_selftest.wj` |
| parity | walker == resolved (stdout, stderr, exit status) on every corpus program, also with every loop in machine code | `tests/interpreter_engine_parity_test.rs` |
| fast-paths | each fast path fires on its programs (a silent fallback keeps output right and hides the regression) | `tests/interpreter_fast_paths_test.rs` |
| jit-loops | generated loop programs: walker == resolved == no-jit | `tests/interpreter_jit_loops_test.rs` |
| packed-vec | packed vectors == boxed vectors | `tests/interpreter_packed_vec_test.rs` |
| corpus | walker, resolved and compiled on every test of the ecosystem snapshot plus the value-position programs | `scripts/wj/drift_corpus.wj`, `tests/drift_corpus/` |

A cargo step that runs zero tests fails (a gate that compared nothing proves nothing); the corpus step fails below a
minimum case count.

## The differential corpus

`tests/drift_corpus/eco/` is a pinned snapshot of every package and app test of the windjammer-ecosystem
(`tests/drift_corpus/MODULE.md` records the commit and how to refresh it). Each test is a case. Three tokens per
case: the walker's, the resolved engine's, and the compiled backend's (`pass`, `fail`, `absent`; programs compare
`exit<N>:<stdout bytes>:<checksum>`). A `<case>#loc` row compares the `file.wj:line` of a panic between the engines.
`tests/engine_parity/vp_*.wj` are value-position programs (below) run the same way, with a compiled run.

The compiled side is expensive (each package is a cargo build), so by default the gate compares the live
interpreted runs with `tests/drift_corpus/compiled_baseline.tsv`, recorded with
`wj run scripts/wj/drift_corpus.wj BIN OUT --compiled live --record` (and re-checked in CI with `--compiled live`).
A test missing from the baseline is `absent` and therefore a divergence: record the baseline when the corpus changes.

## The generated programs

`scripts/wj/drift_gen.wj` writes seeded programs (default seeds 1..40) whose cells put a value of one kind
(`Option<int>`, `Option<int>` from a map lookup, `Result`, an enum with unit/tuple/struct variants, `Option<string>`)
in one value position (match arm result, return, return from a loop, closure, struct field, nested match, shadowing,
`Vec` element, if-expression, bool match), on a hit and on a miss. Every integer literal goes through `num(..)`, so
literal typing is not what the grid probes. Oracles: the walker for the resolved engine (`gen/seed-N`: stdout and exit
status), and rustc for the compiled backend (`gen/seed-N#rustc`: the generated Rust must type-check; `--run-compiled`
also runs it). The rows join the corpus rows, so the same ledger applies. `DRIFT_GEN_BARE=1` switches on bare
unit-variant patterns (a known divergence, `repros/bare_unit_pattern_in_let_match.wj`). Seeds beyond the default:
`wj run scripts/wj/drift_gen.wj BIN DIR OUT.tsv --seeds 41..400`.

## The drift budget

`tests/drift_corpus/known_divergences.txt` lists the divergences we know about: `case | walker=A resolved=B
compiled=C | reason`. The ledger is shrink-only:

- a divergence that is not listed fails the gate (`new`);
- a listed divergence whose shape changed fails (`changed`);
- a listed case that no longer diverges fails until its line is deleted (`stale`), so a fix is always celebrated in a diff;
- a line the base ref (`DRIFT_BASE_REF`, default `next/v0.50.0`) does not have fails (`grew`), unless
  `DRIFT_ALLOW_GROWTH=1` records a new finding together with its entry in `docs/drift/findings.md`.

## Value-position programs

Differential generators mostly produce arithmetic. A bare `None` arm result slipped through because no corpus program
used `None` as a *value*. `docs/drift/generator-audit.md` lists what the generators and corpora cover per value
position (Option/Result/enum values as arm results, returns, captures, nested matches, lookup misses, shadowing,
early exits from loops) and which hand-written `vp_*` program covers each gap.

## Mutation checks

Each new gate was shown to fail when the thing it guards is broken (`docs/drift/generator-audit.md`, section
Mutation checks): the ledger by planted divergences (`drift_selftest.wj`), the value-position programs by removing
the fix for the bare-`None` bug and observing the gate go red.

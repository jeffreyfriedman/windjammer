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
| crate-host | calls into Rust crates (ADR-0031): interpreted == compiled on the interop projects (strsim, shlex, crc32fast, a path crate with every surface shape), panics, replay, grants, simulate | `tests/interpreter_rust_crates_test.rs` |
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

`scripts/wj/drift_gen.wj` writes seeded programs (default seeds 1..60, 8 cells each) whose cells put a value of one kind
(`Option<int>`, `Option<int>` from a map lookup, `Result`, an enum with unit/tuple/struct variants, a nested enum, a
tuple, `Option<string>`, `Option<char>`, `string`) in one of 16 value positions (match arm result, return, return from
a loop, closure, struct field, nested match, shadowing, `Vec` element, if-expression, bool match, by-value method
receiver, `break` with a value, nested loops with a labelled `continue`, early `return` from a `for`, a closure built
from an argument, a closure with an early `return`), on a hit and on a miss. Every integer literal goes through `num(..)`, so
literal typing is not what the grid probes. Oracles: the walker for the resolved engine (`gen/seed-N`: stdout and exit
status), and rustc for the compiled backend (`gen/seed-N#rustc`: the generated Rust must type-check; `--run-compiled`
also runs it). The rows join the corpus rows, so the same ledger applies. `DRIFT_GEN_BARE=1` switches on bare
unit-variant patterns (a known divergence, `repros/bare_unit_pattern_in_let_match.wj`). Seeds beyond the default:
`wj run scripts/wj/drift_gen.wj BIN DIR OUT.tsv --seeds 41..400`.

After a re-record (`drift_corpus.wj ... --compiled live --record`), `wj run scripts/wj/drift_gate.wj --fast --prune`
deletes the ledger lines of fixed cases and lists them; new and changed divergences still fail, so a prune can only
shrink the ledger.

## Move shapes

`tests/drift_corpus/moves/` holds use-after-move programs for ADR-0030. Windjammer clones automatically in many
shapes, so the directory records what rustc really rejects. `must_refuse_*.wj` programs are ones rustc rejects (E0382):
a `<program>#refused` row compares whether each side stopped on an ownership error (the interpreter's WJ0007, rustc's
E0382 behind the compiled build); an interpreter that runs such a program diverges, and the ledger names it until the
ADR-0030 slice that refuses it lands (then the line goes stale and is deleted). The other programs (owned argument
reused, string moved into a struct, move in a `for`/`while` body, partial move, closure capture, push then use, reassign
re-arms, Copy-like struct) are controls: rustc accepts them, so all three modes must run them and agree. A new shape
that rustc rejects is added as `must_refuse_<shape>.wj`.

## Coverage

The differential corpus runs, besides the ecosystem tests, every repository program that has a `main`, deterministic
output and no expected failure: `tests/conformance_programs`, `tests/conformance`, `tests/interpreter_corpus`,
`tests/engine_parity`, `examples/verified` (358 programs), plus the value-position (10), move-shape (13) and repro (28)
programs: 409 of the 1544 `.wj` files with a `main` outside the ecosystem snapshot (26 %). The 261 spec programs are run
with their `// expect:` oracle by `scripts/wj/spec_oracle.wj` (about 43 % together). What is left is deliberate: the
negative cases (`tests/check_negative`, diagnostics and effects fixtures, `// expect-error:` specs), programs that read
stdin, the clock, randomness or the network, and seeded corpus templates (run by the cargo parity test). Each corpus
program gets a `#accepted` row (the interpreter did not refuse it: it is compared with the compiled build accepting
it) and, when it carries `// expect:` lines, an `#expect` row (stdout equals the lines).

## Verdicts by semantic class

`scripts/wj/drift_probes.wj` writes one program per (class, seed), four seeds per class, and compares walker, resolved
engine and compiled backend (stdout, exit status; for panics the `file.wj:line` between the two engines). Verdicts at
the time of writing; the ledger is the source of truth.

| Class (probe family) | Walker vs resolved | Interpreter vs compiled | Verdict |
|---|---|---|---|
| wrapping / saturating / checked ops, i8..u64 (`intwrap`) | agree | agree | agree |
| overflow panics (`panic_overflow`) | agree, same location | agree (exit 101) | agree |
| `as` casts: int widths, float to int (saturating, NaN), char, bool (`casts`) | agree | agree | agree |
| division, modulo, `rem_euclid`, `div_euclid` of negatives (`divmod`) | agree | agree | agree |
| shifts, bit operations, `count_ones` and friends (`shifts`) | agree | agree | agree |
| float formatting: `{}`, `{:.2}`, `{:>12.3}`, `{:e}`, `{:?}`, NaN, inf, -0.0, f32 (`floatfmt`) | agree | agree | agree |
| strings and chars: UTF-8 length, case mapping of non-ASCII, reverse, find (`strings`) | agree | agree | agree |
| slicing a string inside a character (`panic_slice`) | agree, same location | agree (exit 101) | agree |
| `${x:?}`, `{:#?}`, width/fill/sign/radix specs (`format`) | agree | agree | agree |
| panics in nested calls: index, division, `expect`, `assert_eq`, explicit (`panic_nested`) | agree, same location | agree | agree |
| evaluation order: `&&`/`||`, arguments, tuples, `v[i()] += j()`, struct literal fields (`order`) | agree | **was a bug**: struct literal fields ran in declaration order, rustc runs them as written | fixed in both engines (`interpreter: struct literal fields run in written order`) |
| closures: FnMut counters, captures, returned closures (`closures`) | agree | agree | agree |
| drop order: scopes, moved values, vectors (`drop`) | agree | agree | agree |
| sort stability, `dedup`, string order (`sort`) | agree | agree | agree |
| `HashMap` iteration (`hashorder`) | agree | agree when the program sorts the keys | documented divergence (D6): raw iteration order is unspecified, the interpreter uses key order |
| process exit codes (`exit_code`) | agree | agree | agree |
| stdin at EOF and `args()` (`stdin`) | agree | compiled build rejects `read_line(..) ?? 0` (usize vs int); probe uses the value only | agree after the probe fix; the ill-typed form is a finding |
| threads: spawn/join, channel order (`threads`) | agree | `h.join() ?? 0` does not compile (zero-argument closure), see `repros/thread_join_coalesce.wj` | agree for the supported forms; one compiled-backend bug |
| std return shapes: interpreter builtin vs runtime signature (`drift_stdshape.wj`) | agree | `env::current_dir` returned a bare string (fixed); `env::current_exe` and `json::number_f64` are not implemented in the interpreter | one bug fixed, two gaps ledgered |

Mutation checks: the evaluation-order class is a real bug (the gate diverged from the compiled baseline until fixed).
The value-position classes have three engine mutants (`docs/drift/generator-audit.md`). The shared-builtin classes
(integer, float, string, format) have no mutant of their own: the walker and resolved engine share those builtins, so
only the compiled baseline can see a change in them, and the baseline is recorded from the compiled backend.

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

## The spec as an oracle

`scripts/wj/spec_oracle.wj` runs every `tests/spec` case that carries an expected output (`mode: run` and `mode: panic`)
under the tree walker, the resolved engine and the compiled backend, and judges each run against the case's own
`// expect:` lines (stdout and exit status; `expect-error:` for panics). The gate's `spec` step feeds the rows to
`drift_ledger.wj` with its own shrink-only ledger, `tests/spec_oracle/known_divergences.txt`; compiled results come from
`tests/spec_oracle/compiled_baseline.tsv` (`--compiled live --record` rebuilds them: one `cargo build --keep-going` of a
scratch workspace with one binary per case, dev profile). `spec_oracle.wj --selftest` plants wrong outputs the judge must
reject. Expected outputs of new cases come from the compiled program, never from the interpreter; when the compiled
backend cannot build a program yet, the case says so and its output is that of the equivalent Rust program. Findings:
`docs/drift/spec-findings.md`.

## Wall time of the full gate

Measured on a busy shared machine with a release `wj` and every cargo call going through the build-slot gate. The
seconds column of the gate table includes waiting for a slot and for the release test binary to build; the `ran` figure
(from cargo's own report) and the script steps are the work itself:

| Step | Work | Notes |
|---|---|---|
| selftest | 2 s | planted divergences against the ledger comparer |
| parity (engine_parity) | 84 s run | walker == resolved on every corpus program; the first cargo step also pays the release build of `tests/all` |
| fast-paths | 2 s run | |
| jit-loops | 54 s run | |
| packed-vec | 32 s run | |
| crate-host | 56 s run | |
| corpus (compiled: recorded) | 90 s | 1570 cases, ecosystem tests plus value-position and generated programs |
| spec oracle (compiled: recorded) | 36 s | 228 cases x walker and resolved; compiled results come from the recorded baseline |

About 6 minutes of work on a quiet machine; the rest of a gate run is the release build of `tests/all` (once per source
change, 15 to 25 minutes cold, nothing when warm) and queueing. Re-recording the compiled results is a separate, rarer
job: `--compiled live --record` of the spec oracle is one `cargo build --keep-going` plus 228 runs.

---
status: current
last_verified: ceeebb2da7ef
---
# When can the interpreter be the primary development loop?

`wj run` already interprets by default and falls back to the compiled path when a program uses something the
interpreter cannot run faithfully (ADR-0022). This page defines the bar for going further: day-to-day development,
tests and agent loops run interpreted first, and compilation is reserved for release builds and CI.

The bar is a scorecard measured by `scripts/interp_readiness.py`, not an opinion. The tool reports each criterion
as a number and a pass/fail; "ready" means every criterion passes on the same commit.

## Criteria

| # | Criterion | Target | Measured on |
|---|-----------|--------|-------------|
| R1 | Test suites pass interpreted | >= 99% of test functions pass under `wj test --interpret` wherever they pass compiled | ecosystem packages and apps, verified examples, spec examples, plus each private dogfood project |
| R2 | No silent divergence | 0 programs whose interpreted output differs from compiled output (stdout, stderr, exit code) | verified examples, conformance corpus, spec examples, generated differential programs at >= 300 seeds, hash-seed 0..3 |
| R3 | Fallback is rare and loud | <= 2% of `wj run` / `wj test` invocations over the dogfood corpora fall back to compiled, and every fallback prints its reason | dogfood corpora |
| R4 | Fast enough | interpreted test-suite wall time <= compiled build+test wall time on a cold cache for every dogfood project, and a warm edit-run loop <= 1 s for a 50-file project | dogfood projects, `bench/interp` kernels vs Python (>= parity) |
| R5 | Diagnostics agree | `wj check` and the interpreter reject exactly the same programs (interpreter_check_parity gate) and the compiled path accepts everything the interpreter accepts (check-vs-build gate at 0 misses) | negative corpus, mutation programs |
| R6 | Stdlib and project coverage | every `std` item has an interpreter implementation or an explicit capability refusal; Rust-crate dependencies run through the host bridge for the dogfood projects | std_interpreter ratchet at 0, dogfood projects |
| R7 | Stability | R1-R6 hold on the last 20 consecutive commits to `main` | CI history |

## What the interpreter is not for
Benchmarks, production serving, and anything whose point is throughput. `wj build` stays the release path.

## How to run

```
scripts/interp_readiness.py --quick             # small subsets, minutes; exit 1 unless every criterion passes
scripts/interp_readiness.py --full --write      # every program, >= 300 generated seeds, hash seeds 0..3
scripts/interp_readiness.py --report            # print the scorecard, always exit 0
```

Build `wj` first (`cargo build --release --bin wj`); the script uses `--wj`, `$WJ_BIN`, `target/release/wj` or
`target/debug/wj`, in that order, and records which profile it measured (R4 timings only mean something for a
release build). Exit codes: 0 ready, 1 not ready, 3 suspect (a cross-check caught a rubber-stamp binary).

What each criterion measures, and where the evidence comes from:

| # | Measured by |
|---|-------------|
| R1 | `wj test --interpret --json` and compiled `wj test --json` on every project; the two per-test pass/fail *sets* are compared (not counts). Percent = tests passing both / tests passing compiled |
| R2 | each `// expect:` program of `examples/verified`, `tests/conformance_programs` and `tests/spec` (run/panic modes) against its own oracle, against `wj run --compiled` (stdout, exit code, panic message), and against itself under `--hash-seed 0..3`; every `tests/interpreter_corpus` template against its independent python reference model for >= 300 generated seeds |
| R2 (agent pilot) | the programs agents wrote interpreted-first (`bench/agent-dx/results/pilot`): every interpreter-green program that fails the compiled grade is a silent divergence, reported as a rate (pilot: 13 of 30) and per program; those that `wj check` accepts are also counted under R5 as check misses, with the compiler error class (E0308 int widths, E0277 collect element types, ...) |
| R3 | default `wj run` over the same programs: the one-line `wj: running compiled (cargo): <reason>` stderr notice counts as a fallback (reasons grouped, a notice without a reason fails), plus interpreted tests that the interpreter declined as unsupported |
| R4 | interpreted vs compiled wall time per project (compiled cache as found, which favours compiled), the warm edit-run loop on a scratch copy of the largest project, and every `bench/interp` kernel vs its hand-written python twin (`bench/interp/twins/`; a generated equivalent is the fallback), plus startup: a hello-world run, and the share of the largest project's interpreted run spent in the pre-run `wj check` |
| R5 | `wj check` vs `wj run --interpret` refusal over the verified examples and `tests/check_negative/mutants`, and the compiled build (shared with R2) of everything the interpreter accepts, documented `// rust: gap` examples included; `--cargo-gates` also runs the real Rust gates |
| R6 | std functions without an interpreter builtin (`tests/data/std_interpreter_missing.txt`), the interpreter's unsupported-feature list, and projects with Rust-crate dependencies that do not pass interpreted |
| R7 | `.wj-readiness-history.json` (appended on every run) over the last 20 commits: each needs a `--full` entry with R1-R6 all passing. Nightly CI restores and re-uploads this file; without it R7 reads 0% |

Projects. The ecosystem packages and apps (default `../windjammer-ecosystem`, next to the main checkout; override
with `--ecosystem`) are built in. Private dogfood projects are read from an untracked config, never from the
repository: `~/.config/wj-readiness.toml` (or `$WJ_READINESS_CONFIG`)

```
[[project]]
name = "my-app"
path = "/abs/path/to/my-app"
private = true
```

or `WJ_READINESS_PROJECTS="name=/path;name2=/path2"`. Private projects appear as `private-1`, `private-2`, ...
with hashed test ids; the JSON writer refuses to write a file that contains a configured name or path.

Bounds. Runs are serial; `--quick` samples two packages, two apps and about eight programs per corpus;
every command has a timeout (timeouts report a program as unmeasured, never as a pass). Results are cached in
`.wj-readiness-cache/` keyed by the compiler's `src/` + `std/` tree hash (`--cache-id` overrides), the `wj` binary hash
and the program hash, so an interrupted `--full` run resumes where it stopped and a commit that only touches docs reuses
everything. `--budget-min N` stops starting compiled builds after N minutes (they report as unmeasured, never as passes),
builds are skipped when free disk falls below 3 GiB, and compiled runs use a private cargo target dir
(`--cold` starts each project from an empty one; `--full` defaults to cold, `--warm` opts out). Loose programs are
compiled from a private one-file directory.

Trust. Before a verdict is printed the script runs canaries whose right answers do not depend on the binary: a
test that must fail, a seeded program whose output is computed independently, and an ill-typed program that
must be rejected. A `wj` that passes everything is reported as SUSPECT (exit 3) instead of ready. The scorecard
logic is tested against a fake `wj` with scripted pass/fail/divergence patterns
(`scripts/readiness/tests/`, run by `tests/interp_readiness_script_test.rs`).

The committed snapshot is `docs/generated/interp_readiness.json`.

## Review cadence
The scorecard runs in CI nightly (`.github/workflows/readiness.yml`, `--full`, JSON uploaded as an artifact) and on demand (`scripts/interp_readiness.py --report`). The owner is notified when
all of R1-R7 pass; until then the report lists the failing criteria with the programs that fail them.

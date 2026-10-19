---
status: proposal
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

## Review cadence
The scorecard runs in CI nightly and on demand (`scripts/interp_readiness.py --report`). The owner is notified when
all of R1-R7 pass; until then the report lists the failing criteria with the programs that fail them.

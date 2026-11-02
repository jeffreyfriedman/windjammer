---
status: proposal
---
# Making Windjammer the language agents prefer

Goal: given the same task, a coding agent finishes sooner, with fewer tokens and fewer retries, in Windjammer than in
Rust, Go, Python or TypeScript. This page defines how that is measured, how the measurement drives fixes, and how the
program stays honest. The harness is `bench/agent-dx` (see its README); the first live experiment is the loop-mode pilot.

## Hypothesis under test

H1. An interpreted-first loop (`wj check --json`, `wj run --interpret`, `wj test --interpret`, compile only for the final
grade) needs fewer turns, tokens and seconds than a compiled-first loop, at equal final pass rate.
H2. The context pack (`docs/agent/WINDJAMMER_FOR_AGENTS.md`) raises first-draft pass rate and lowers tokens.
H1 can fail in a specific way: interpreted success that is not compiled success (silent divergence). The pilot therefore
reports the final compiled grade and the divergence rate next to the speed numbers; a faster loop that ships red programs
loses. Arms: A interpreted-first, B compiled-first, C = A without the pack.

## Agent preference score (APS)

For a language L, a task set S and a baseline language B (run separately against each of Python, TypeScript, Go, Rust):

```
P = pass@1(L) / pass@1(B)        single-session pass rate on the hidden oracle
I = verify_calls(B) / verify_calls(L)   verify calls (build/run/test/check) per passing trial; fewer is better
T = tokens(B) / tokens(L)        input + output tokens per passing trial
W = wall(B) / wall(L)            wall seconds per passing trial (API time plus tool wait)
APS = 100 * P^0.4 * I^0.2 * T^0.2 * W^0.2        each ratio clipped to [0.25, 4]
```

100 is parity with B; above 100 agents are better served by L. Rules that keep the number meaningful:

- Cost per passing trial, not per trial: failing fast does not earn credit. Ratios use trials that passed, and P carries the pass rate.
- If P < 0.9 the score is reported as `P-limited` and the other terms are shown but not combined.
- Every score ships with a bootstrap CI (tasks are the resampling unit) and N. Differences are claims only when the interval excludes zero.
- Components are always shown; the single number is a dashboard, the components are the diagnosis.
- Costs are per-model; the model id and date are part of the score. Rerun on each new model.

Today only Windjammer arms exist in the live pilot (`agentdx pilot`). Baseline-language trials use the same prompt shape
and sandbox with that language's toolchain; adding them to the pilot runner (language in the plan tuple, a per-language shim)
is the next harness step and is required before an APS can be quoted.

## The improvement loop

```
run (pilot or full) -> pilot-report -> friction list -> fix at the named location -> targeted gate -> re-run
```

1. Run: `agentdx pilot` (budget-gated). Records are schema 2 JSON, replayable, one per trial.
2. Friction list: `agentdx pilot-report`. Each item has a fix kind and location (diagnostic text, pack section, lint rule,
   stdlib API, error-code explanation, interpreter/compiled parity gate) and a priority from estimated tokens wasted.
3. Fix: smallest change at that location, with a regression test that fails without it (diagnostic snapshot, lint test,
   docs-truth block, differential program). A fix without a gate is not done.
4. Re-run only the affected tasks first (cheap), then the full pilot on a fresh task rotation (below).
5. Record the delta in `docs/known-issues/` (resolved) or `bench/agent-dx/results/` (numbers), with the commit.

Ranking rule: priority = estimated tokens wasted = (verify calls spent on that signature) x (arm mean tokens per turn).
It is an estimate that orders work; the confirmation is the re-run delta.

## Guardrails against overfitting the benchmark

- **Held-out tasks.** The 60 tasks are split once, by seed, into 40 development and 20 held-out tasks. Friction analysis and fixes
  use development tasks only. Held-out tasks run at milestones; their friction lists are not read for fixes, only their scores.
- **Rotation.** Pilot task samples rotate (seeded) each cycle so no fix can target a fixed ten. New tasks are added in the same
  tiers each quarter; retired development tasks move to the held-out pool.
- **No task-specific hints.** Pack content must be language-level (idioms, API shapes, error classes). A gate scans the pack for task
  ids, task titles and spec phrases and fails on a match. Examples in the pack come from `examples/verified`, not from `tasks/`.
- **Oracle isolation.** The oracle never enters the sandbox; a canary test plants a marker in the oracle and scans every
  agent-visible file, prompt and argument.
- **Fixes must generalise.** A friction fix is accepted when it also holds on a differential program or verified example outside
  the benchmark. Compiler fixes follow the usual TDD with an independent oracle.
- **Pack budget.** The pack stays inside its token budget (`tests/agent_docs_test.rs`); a section is added only by removing or
  tightening another, and each addition names the friction item it answers.
- **Honest N.** The pilot (10 tasks x 2 arms x 3 trials) detects large effects only. Smaller effects need the full suite and
  more trials; reports say so in the first paragraph.

## How docs-truth keeps the pack accurate

The pack is code, not prose. `tests/agent_docs_test.rs` extracts every fenced Windjammer block under `docs/agent/` and runs
it (`windjammer` with `// expect:`, `windjammer,test`, `windjammer,bad` must produce a diagnostic, `windjammer,parse` must
parse), checks the token budget, checks that `wj docs` answers real queries, and checks CLI command and flag mentions against
the CLI. The agent-dx harness adds a second anchor: statements about benchmark behaviour (for example which stdin spelling
works in both modes) are tied to graded reference solutions, so a compiler change that breaks the advice fails a gate.
Rule of thumb: every claim an agent will act on has an executable witness.

## Ambition ladder

| Rung | Evidence | First experiment | Kill / re-plan if |
|------|----------|------------------|--------------------|
| 1. Loop-mode answer | A vs B on 10 tasks with CIs | `agentdx pilot` | cost of the pilot exceeds budget twice |
| 2. Pack value | A vs C | same pilot plus arm C | pack adds no pass-rate gain on held-out tasks after two edits |
| 3. Parity with Python/TS first-draft pass rate | APS components P >= 0.9 vs each baseline | add baseline languages to the pilot | P stays under 0.7 after two friction cycles (re-plan around compiler bugs) |
| 4. Fewer iterations than Rust/Go | I >= 1.2 | measure verify calls per passing trial | interpreter divergence > 2% (R2) blocks interpreted-first |
| 5. APS > 100 against every baseline | all four components | full 60 tasks, 5 trials, two models | any component regresses on held-out tasks |

Prerequisites already tracked: interpreter-primary readiness (`docs/design/interpreter-primary-readiness.md`, R1-R7) decides how
much of the loop can be interpreted; compiled-path codegen fixes from `docs/known-issues/agent-dx-*.md` decide the final grade.

## First pilot (Oct 2026, wj 0.50.0 tree, model `sonnet`, 10 tasks x arms A/B x 3 trials = 60 trials)

Raw records and report: `bench/agent-dx/results/pilot/`. Total spend about $2 (the pre-run estimate was $24, expected, from
conservative per-turn assumptions; the calibrated estimate is $14). Intervals are 95% task-clustered bootstrap CIs; N is small.

| | A interpreted-first | B compiled-first | A - B (paired, CI) |
|---|---|---|---|
| pass@1 on the compiled hidden oracle | 0.57 [0.30, 0.83] | 0.90 [0.70, 1.00] | -0.33 [-0.63, -0.10] |
| agent turns | 3.5 | 5.3 | -1.8 [-3.5, -0.5] |
| tokens in (incl. cache) | 31.8k | 53.6k | -21.9k [-42.5k, -6.1k] |
| cost per trial | $0.028 | $0.041 | -$0.013 [-0.024, -0.005] |
| wall seconds (3 parallel workers, loaded machine) | 20 | 97 | -77 [-138, -34] |
| silent divergence (interpreter green, compiled red) | 0.43 | 0 | 0.43 [0.17, 0.70] |

Reading: H1 holds for speed and tokens (about 40% fewer input tokens, a quarter of the wall time) and fails for correctness
as the compiler and interpreter stand today: 13 of 30 interpreted-first programs ran correctly in the interpreter and did not
build, mostly from integer-width literal typing (`docs/known-issues/agent-dx-compiled-untyped-int-accumulator.md`) and a method
the interpreter accepts but rustc lacks (t018 fails in both arms). The path to H1 is therefore R2/R5 of
`interpreter-primary-readiness.md` (check-vs-build parity), plus a third arm A' (interpreted-first with one `wj build` before
finishing) that this pilot did not run. Agents iterate very little (3-5 turns): pass@1 and the first-draft failure classes dominate.
Harness notes: concurrent compiled grading briefly shared one binary name and produced 4 false failures; fixed by serialising
build+copy and re-grading every failed trial from its recorded source (`agentdx pilot-regrade`).

## Second pilot (Oct 2026): seven arms, 210 trials, compiler build 8ff93b405

Raw records and reports: `bench/agent-dx/results/pilot2/` (`report.md`, `trials/`; `rerun/` is a second sample of arms A, B, D).
Arms: A interpreted-first, B compiled-first, C no pack, D interpreted-first plus one `wj build`, and Python, Go and Rust baselines
(same tasks, no pack, native toolchain). Total spend about $11. Machine load averaged 30-80 from other work, so wall seconds are
inflated and noisy; turns, tokens and pass rates are not load-sensitive.

| arm | pass@1 | silent divergence | turns | tokens in | cost | verify calls |
|---|---|---|---|---|---|---|
| A interpreted-first | 0.40 [0.17, 0.63] | 0.57 | 3.3 | 34k | $0.028 | 1.8 |
| B compiled-first | 0.87 [0.70, 1.00] | 0.10 | 5.0 | 52k | $0.038 | 2.1 |
| C no pack | 0.20 [0.07, 0.37] | 0.67 | 21.9 | 229k | $0.118 | 6.9 |
| D interpreted + final build | 0.83 [0.63, 1.00] | 0.13 | 3.6 | 54k | $0.055 | 3.7 |
| Python / Go / Rust | 1.00 / 1.00 / 1.00 | n/a | 3.6 / 2.6 / 3.0 | 24k / 18k / 17k | $0.019 / 0.021 / 0.022 | |

Pooling the rerun sample (about 50 trials per arm for A, B, D): A 0.44 pass, 0.52 silent divergence; B 0.90 and 0.06; D 0.86 and 0.12.

- **Loop mode.** Interpreted-first is the cheapest loop (A: 3.3 turns, $0.028) and the least correct (silent divergence 0.52-0.57, unchanged within
  noise from 0.43 in pilot 1). Compiled-first is the most correct Windjammer loop. D buys back nearly all of B's correctness (0.83-0.86 vs 0.87-0.90)
  at about the turns of A, but pays for it in `wj build` time and cost: the one final build is the dominant cost of D.
- **Pack value (A vs C).** Without the pack agents need 6.6x the turns and 6.7x the tokens and pass half as often (paired pass difference A-C
  +0.20 [-0.07, 0.47], turns -18.6 [-21.6, -15.0]). The pack is worth keeping and growing.
- **Versus other languages.** Every baseline passes 30/30. No Windjammer arm reaches P >= 0.9 of baseline pass@1, so the agent preference score is
  reported as P-limited for every arm (B: P 0.87 [0.77, 0.97], I 0.53-0.74, T 0.35-0.47 against Go/Python/Rust). Parity needs two things: first-draft
  pass rate above 0.9 for interpreted arms (check-vs-build parity) and a pack that cuts the 1.5-3x token overhead.
- **What changed since pilot 1.** The int-accumulator literal bug is fixed (its repro now builds). The remaining silent divergence is now mostly
  `usize` versus `int` (`let idx: int = v.len()` passes the interpreter and fails rustc: `docs/known-issues/agent-dx-compiled-len-to-int-binding.md`,
  9 of 12 re-built failures), plus string/char return typing and the `Vec<String>` collect error. No capability denials (WJ004x) occurred: the default
  local profile covers every task.
- **Timing.** Per call: `wj run --interpret` median 0.1 s, `wj run --compiled` 20 s (p90 48 s), `wj build` 167 s (p90 423 s) under load. The final
  `wj build` of arm D averages 375 s of the 454 s wall. `wj build` does not share the cache that `wj run --compiled` uses; this is the top performance
  fix for compiled-last loops.

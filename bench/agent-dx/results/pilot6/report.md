# Pilot 6: Rust vs Windjammer compiled-first vs Windjammer interpreted-first with predictions

90 trials over 10 tasks. Intervals are 95% bootstrap CIs that resample tasks, then trials within a task. With this N they show direction, not significance, unless an interval excludes zero.

## Arms

| arm | name | trials | context pack |
|---|---|---|---|
| R | rust | 30 | no |
| U | compiled-first | 30 | sheet_noloop.md |
| V2 | interpreted-first+verify | 30 | sheet_v2.md |

## Arm means

| metric | R | U | V2 |
|---|---|---|---|
| pass@1 (compiled hidden oracle) | 1.000 [1.000, 1.000] | 1.000 [1.000, 1.000] | 1.000 [1.000, 1.000] |
| verify calls (wj check/run/test/build) | 1.0 [1.0, 1.0] | 1.7 [1.2, 2.2] | 1.9 [1.3, 2.7] |
| verify calls to green (passing trials) | 1.0 [1.0, 1.0] | 1.1 [1.0, 1.4] | 1.6 [1.0, 2.6] |
| agent turns | 3.0 [3.0, 3.0] | 3.8 [3.0, 4.8] | 4.8 [3.3, 7.2] |
| tokens in (incl. cache) | 22774.7 [22310.7, 23448.8] | 32249.0 [25207.7, 43667.1] | 41135.4 [29005.6, 62533.6] |
| tokens out | 1710.2 [1290.6, 2323.8] | 1797.6 [1453.2, 2212.4] | 1869.4 [1441.7, 2530.9] |
| cost USD | 0.033 [0.026, 0.042] | 0.039 [0.033, 0.046] | 0.040 [0.031, 0.053] |
| wall seconds | 22.1 [19.4, 25.3] | 34.1 [28.4, 41.2] | 39.4 [29.0, 52.8] |
| agent-thinking seconds (API time) | 13.8 [11.3, 17.2] | 17.8 [13.7, 23.5] | 17.1 [13.1, 22.0] |
| tool-wait seconds (wj commands) | 2.1 [1.6, 2.6] | 11.3 [7.8, 16.1] | 17.4 [8.7, 30.1] |
| harness permission denials (should be 0) | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.1] | 0.0 [0.0, 0.1] |
| capability denials seen (WJ004x) | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] |
| machine load average during the trial | 36.7 [31.7, 42.7] | 46.7 [39.7, 53.7] | 47.8 [39.8, 56.4] |
| silent divergence (interp pass, compiled fail) | 0.000 [0.000, 0.000] | 0.000 [0.000, 0.000] | 0.000 [0.000, 0.000] |

## Paired differences (task-paired, first minus second)

| metric | R-U | R-V2 | U-V2 |
|---|---|---|---|
| pass@1 (compiled hidden oracle) | 0.000 [0.000, 0.000] | 0.000 [0.000, 0.000] | 0.000 [0.000, 0.000] |
| verify calls (wj check/run/test/build) | -0.7 [-1.2, -0.2] | -0.9 [-1.7, -0.3] | -0.2 [-0.9, 0.2] |
| verify calls to green (passing trials) | -0.1 [-0.4, 0.0] | -0.6 [-1.7, 0.0] | -0.3 [-1.2, 0.0] |
| agent turns | -0.8 [-1.9, 0.0] | -1.8 [-4.2, -0.4] | -1.0 [-2.8, -0.1] |
| tokens in (incl. cache) | -9474.3 [-20513.8, -2640.1] | -18360.7 [-40077.5, -6880.7] | -8886.4 [-24259.1, 209.9] |
| tokens out | -87.4 [-698.8, 666.1] | -159.2 [-1001.0, 695.9] | -71.8 [-487.6, 178.3] |
| cost USD | -0.005 [-0.016, 0.005] | -0.007 [-0.023, 0.006] | -0.001 [-0.011, 0.005] |
| wall seconds | -12.0 [-20.5, -5.1] | -17.3 [-31.6, -5.9] | -5.3 [-18.0, 5.8] |
| agent-thinking seconds (API time) | -4.0 [-10.1, 1.5] | -3.3 [-9.0, 1.8] | 0.7 [-3.1, 4.8] |
| tool-wait seconds (wj commands) | -9.2 [-14.3, -5.5] | -15.3 [-27.7, -6.5] | -6.1 [-18.7, 3.9] |
| harness permission denials (should be 0) | -0.0 [-0.1, 0.0] | -0.0 [-0.1, 0.0] | 0.0 [-0.1, 0.1] |
| capability denials seen (WJ004x) | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] |
| machine load average during the trial | -10.0 [-19.2, -0.4] | -11.2 [-20.9, -2.1] | -1.1 [-11.2, 8.6] |
| silent divergence (interp pass, compiled fail) | 0.000 [0.000, 0.000] | 0.000 [0.000, 0.000] | 0.000 [0.000, 0.000] |

## Per-task pass rate (passes/trials)

| task | R | U | V2 |
|---|---|---|---|
| t011_lru_cache | 3/3 | 3/3 | 3/3 |
| t013_bst | 3/3 | 3/3 | 3/3 |
| t101_json_orders | 3/3 | 3/3 | 3/3 |
| t103_trait_pricing | 3/3 | 3/3 | 3/3 |
| t104_activity_log | 3/3 | 3/3 | 3/3 |
| t105_date_diff | 3/3 | 3/3 | 3/3 |
| t106_priority_queue | 3/3 | 3/3 | 3/3 |
| t107_text_wrap | 3/3 | 3/3 | 3/3 |
| t108_to_rpn | 3/3 | 3/3 | 3/3 |
| t109_spiral | 3/3 | 3/3 | 3/3 |

## Top failure categories (diagnostics hit, all verify calls)

| category | R | U | V2 |
|---|---|---|---|
| other | 0 | 11 | 10 |
| syntax | 0 | 3 | 9 |

## Interpreter vs compiled divergence

0/90 trials had an interpreted hidden-oracle result different from the compiled one (0 silent: interpreter green, compiled red).

## Agent preference score (Windjammer arm vs baseline language)

APS = 100 * P^0.4 * I^0.2 * T^0.2 * W^0.2 (ratios baseline/Windjammer, clipped to [0.25, 4]; I = agent turns). 100 = parity; `P-limited` when pass@1 is below 0.9 of the baseline's. Components are the diagnosis; CIs are task-clustered bootstrap.

| arm | vs | APS | P | I (turns) | T (tokens) | W (wall) |
|---|---|---|---|---|---|---|
| U compiled-first | R rust | 82 [72, 92] | 1.00 [1.00, 1.00] | 0.79 [0.62, 1.00] | 0.72 [0.56, 0.89] | 0.65 [0.52, 0.80] |
| V2 interpreted-first+verify | R rust | 73 [61, 86] | 1.00 [1.00, 1.00] | 0.63 [0.45, 0.89] | 0.57 [0.41, 0.77] | 0.56 [0.43, 0.78] |

## Context attribution (stream-json arms)

First-turn input = system prompt + tool definitions + pack + task prompt. Pack tokens are estimated as bytes/3.5. Schema tokens are the first-turn difference to the matching no-MCP arm (T), minus the pack difference.

| arm | first-turn input | pack tokens (est, bytes/3.5) | other fixed (system, tools, task) | tool-result tokens (est, all turns) | docs shell calls | MCP calls | total tokens in | turns |
|---|---|---|---|---|---|---|---|---|
| R rust | 10554 | 0 | 10554 | 71 | 0.0 | 0.0 | 22775 | 3.0 |
| U compiled-first | 11364 | 455 | 10908 | 536 | 0.2 | 0.0 | 32249 | 3.8 |
| V2 interpreted-first+verify | 11544 | 527 | 11018 | 435 | 0.1 | 0.0 | 41135 | 4.8 |

## Paired differences versus control U (task-clustered 95% CI)

| metric | R - U | V2 - U |
|---|---|---|
| pass@1 (compiled hidden oracle) | 0.000 [0.000, 0.000] | 0.000 [0.000, 0.000] |
| verify calls (wj check/run/test/build) | -0.7 [-1.2, -0.2] | 0.2 [-0.2, 0.9] |
| verify calls to green (passing trials) | -0.1 [-0.4, 0.0] | 0.3 [0.0, 1.2] |
| agent turns | -0.8 [-1.9, 0.0] | 1.0 [0.1, 2.7] |
| tokens in (incl. cache) | -9474.3 [-20513.8, -2640.1] | 8886.4 [-522.7, 23903.9] |
| tokens out | -87.4 [-698.8, 666.1] | 71.8 [-184.8, 487.9] |
| cost USD | -0.005 [-0.016, 0.005] | 0.001 [-0.005, 0.011] |
| wall seconds | -12.0 [-20.5, -5.1] | 5.3 [-5.5, 18.8] |
| agent-thinking seconds (API time) | -4.0 [-10.1, 1.5] | -0.7 [-4.7, 3.1] |
| tool-wait seconds (wj commands) | -9.2 [-14.3, -5.5] | 6.1 [-3.8, 19.5] |
| harness permission denials (should be 0) | -0.0 [-0.1, 0.0] | 0.0 [-0.1, 0.1] |
| capability denials seen (WJ004x) | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] |
| machine load average during the trial | -10.0 [-19.2, -0.4] | 1.1 [-8.8, 11.9] |
| silent divergence (interp pass, compiled fail) | 0.000 [0.000, 0.000] | 0.000 [0.000, 0.000] |

## Top friction by arm (ranked by estimated tokens wasted; Windjammer arms)

**R rust**

(none)

**U compiled-first**

1. unclassified failure: error (`other:error`): 3 trials, 3 calls, fix: inspect example, then file under a rule
2. unclassified failure: error: bad width (`other:error: bad width`): 3 trials, 3 calls, fix: inspect example, then file under a rule
3. type mismatch (often int/float, string/&str) (`type-mismatch`): 3 trials, 3 calls, fix: show both types and the conversion (`as`, `.to_string()`); error-code explain entry
4. slow `wj run` (compiled) >= 30s (`slow-command:run-compiled`): 1 trials, 1 calls, fix: build cache / runtime pre-build; see wall-time table

**V2 interpreted-first+verify**

1. type mismatch (often int/float, string/&str) (`type-mismatch`): 3 trials, 9 calls, fix: show both types and the conversion (`as`, `.to_string()`); error-code explain entry
2. slow `wj run` (default) >= 30s (`slow-command:run-default`): 3 trials, 5 calls, fix: build cache / runtime pre-build; see wall-time table
3. unclassified failure: error (`other:error`): 3 trials, 3 calls, fix: inspect example, then file under a rule
4. unclassified failure: error: bad width (`other:error: bad width`): 2 trials, 2 calls, fix: inspect example, then file under a rule


Mode violations (agent used the other arm's loop commands): 0.

## Friction list (ranked by estimated tokens wasted)

| # | pri | friction | fix kind | fix location | trials | tasks | wasted calls | est. tokens |
|---|---|---|---|---|---|---|---|---|
| 1 | P0 | type mismatch (often int/float, string/&str) (`type-mismatch`) | diagnostic text | show both types and the conversion (`as`, `.to_string()`); error-code explain entry | 6 | 1 | 12 | 108431 |
| 2 | P0 | unclassified failure: error (`other:error`) | diagnostic text (triage) | inspect example, then file under a rule | 6 | 1 | 6 | 53931 |
| 3 | P0 | slow `wj run` (default) >= 30s (`slow-command:run-default`) | performance | build cache / runtime pre-build; see wall-time table | 3 | 3 | 5 | 45416 |
| 4 | P1 | unclassified failure: error: bad width (`other:error: bad width`) | diagnostic text (triage) | inspect example, then file under a rule | 5 | 1 | 5 | 44848 |
| 5 | P1 | slow `wj run` (compiled) >= 30s (`slow-command:run-compiled`) | performance | build cache / runtime pre-build; see wall-time table | 1 | 1 | 1 | 8894 |

## Cost and pass comparison: R, U, V2

Cost per correct program = mean cost per trial / pass@1 (a failed trial is assumed redone in full; an upper-bound convention, as in ADR-0033).

| arm | trials | pass@1 | cost per trial USD | cost per correct program USD | turns | tokens out |
|---|---|---|---|---|---|---|
| R rust | 30 | 1.000 [1.000, 1.000] | 0.033 [0.026, 0.042] | 0.033 | 3.0 [3.0, 3.0] | 1710 [1291, 2324] |
| U compiled-first | 30 | 1.000 [1.000, 1.000] | 0.039 [0.033, 0.046] | 0.039 | 3.8 [3.0, 4.8] | 1798 [1453, 2212] |
| V2 interpreted-first+verify | 30 | 1.000 [1.000, 1.000] | 0.040 [0.031, 0.053] | 0.040 | 4.8 [3.3, 7.2] | 1869 [1442, 2531] |

## Cost and turns by task (mean over 3 trials; USD / turns)

| task | R | U | V2 |
|---|---|---|---|
| t011_lru_cache | 0.024 / 3.0 | 0.030 / 3.0 | 0.029 / 3.0 |
| t013_bst | 0.038 / 3.0 | 0.049 / 3.0 | 0.045 / 3.0 |
| t101_json_orders | 0.069 / 3.0 | 0.034 / 3.7 | 0.032 / 4.3 |
| t103_trait_pricing | 0.023 / 3.0 | 0.043 / 6.3 | 0.044 / 7.0 |
| t104_activity_log | 0.028 / 3.0 | 0.062 / 7.0 | 0.084 / 12.3 |
| t105_date_diff | 0.030 / 3.0 | 0.037 / 3.0 | 0.043 / 4.7 |
| t106_priority_queue | 0.028 / 3.0 | 0.032 / 3.0 | 0.030 / 3.0 |
| t107_text_wrap | 0.024 / 3.0 | 0.027 / 3.0 | 0.025 / 3.3 |
| t108_to_rpn | 0.041 / 3.0 | 0.037 / 3.0 | 0.036 / 3.0 |
| t109_spiral | 0.029 / 3.0 | 0.037 / 3.0 | 0.034 / 4.0 |

Mean cost per trial without the hardest task (t104):

- R: 0.0340 USD over 27 trials
- U: 0.0363 USD over 27 trials
- V2: 0.0353 USD over 27 trials

## Machine load and noisy trials

A trial is noisy when its mean load average exceeded 16. Cost, tokens, turns and pass are not affected by load; wall and tool-wait are.

| arm | trials | noisy | wall s (all) | wall s (quiet only) | tool-wait s (all) | tool-wait s (quiet only) | mean load |
|---|---|---|---|---|---|---|---|
| R | 30 | 30 | 22.1 | n/a | 2.1 | n/a | 36.7 |
| U | 30 | 28 | 34.1 | 24.8 | 11.3 | 5.5 | 46.7 |
| V2 | 30 | 28 | 39.4 | 37.7 | 17.4 | 10.9 | 47.8 |

## Arm R: rustc errors the agent hit

0/30 trials saw at least one rustc error. Mean errors per trial 0.00; mean failing rustc calls 0.00; mean rustc calls spent from the first failure to the next success 0.00 (a proxy for turns spent fixing).


## ADR-0033 acceptance table (arm V2)

| criterion | bar | measured | verdict | note |
|---|---|---|---|---|
| pass@1 (compiled oracle) | >= 0.95 | 1.000 [1.000, 1.000] | PASS | the ADR also asks V2 minus W to include zero; there is no W arm in this pilot (see the V2 minus U row in the paired table) |
| cost per trial, USD | <= 0.030 | 0.040 [0.031, 0.053] | FAIL | point estimate decides; interval shown |
| tool-wait, seconds | <= 1.0 | 17.43 all trials [8.68, 30.11]; 10.95 on 2 quiet trials (load <= 16) | FAIL | includes the `--verify` build; load inflates it |
| silent divergence | <= 0.05 | 0.000 [0.000, 0.000] | PASS |  |
| no prediction for a program the build accepts | 0 false positives | 0 of 0 predictions shown to V2 agents | PASS | no V2 trial was shown a prediction, so this criterion is vacuous |
| machine load within 10% of the comparison arm | within 10% | V2 47.8 vs U/R 41.7 | FAIL | the V arm of pilot 5 is not re-run; U and R of this pilot are the comparison |

V2 trials that ran `wj run --verify`: 30/30. V2 trials shown a prediction: 0/30; U trials shown one: 0.

## Predictions in the field (offline check with the same compiler)

Precision: `wj check` on the final program of every U and V2 trial (60 programs, 60 accepted by the compiled build) printed the width/String prediction for 0 of them (false positives: 0).
Recall: of 12 compiler-rejected programs logged in U and V2 trials (one per failing `wj` call), `wj check` predicted 0. `wj check` exited 0 on 12 of them.

## Appendix: the first run (two trials at a time), discarded for U and V2

U and V2 were first run with two trials in parallel. All agent workspaces are called `ws`, which makes the compiled binary shared between trials (see `docs/drift/pilot6-repros/same_dir_name_runs_other_binary.md`): some `wj run --compiled` / `--verify` calls printed another trial's output and agents chased differences that were not in their programs. Those 60 trials were re-run one at a time (the main tables). Pass and silent divergence are unchanged (the hidden oracle builds in its own directory); cost, turns and tool-wait of the first run are inflated. Kept for the record:

| arm | trials | pass@1 | cost USD | turns | tool-wait s |
|---|---|---|---|---|---|
| U | 30 | 1.000 | 0.0440 | 5.4 | 32.9 |
| V2 | 30 | 1.000 | 0.0417 | 4.7 | 15.5 |

## Setup (recorded)

- Compiler: `wj 0.50.0 (commit 82d5ac0579cd 2026-11-11, clean, release)`, a fresh build of `next/v0.50.0` plus the harness commit, used explicitly through `WJ_BIN` (the installed `wj` was stale and was never used). Each trial record carries the version string.
- Model `sonnet`, `--max-turns 25`, per-trial cap USD 0.40, stream-json logging, tools limited as in pilot 5 (`Read/Edit/Write/Glob/Grep`, `Bash(wj ...)`, `printf/echo/head/tail`; R: `Bash(rustc:*)`, `./main`).
- Tasks (10): t101, t104, t107 (failed in pilot 5), t106 priority queue, t011 LRU cache, t013 BST (data structures), t103, t105, t108, t109. t110 (needs a Rust crate) is excluded because arm R is std-only.
- Arms. U: compiled-first, sheet `packs/pilot5/sheet_noloop.md` (1593 bytes), as in pilot 5. V2: interpreted-first with `wj check` predictions (families A-C), `wj run --interpret` warnings and `wj run --verify` as the last step; sheet `packs/pilot6/sheet_v2.md` (1844 bytes, ratchet 2048): the pilot-5 sheet plus one bullet naming the loop, the prediction messages and `--verify`; the loop paragraph of the prompt names the same steps. R: plain Rust, `rustc` 1.98.0 stable (`AGENTDX_RUSTC`), edition 2021, standard library only, no sheet. The canonical `docs/agent/WINDJAMMER_FOR_AGENTS.md` is unchanged (its tests require the old loop bullet).
- Grading: the hidden oracle (33 to 46 cases per task) grades the native program in every arm: `wj build` + cargo for U and V2, `rustc` for R. The interpreter run is graded as well, for divergence.
- Load: every trial records the mean of the load average before and after; trials above 16 are marked noisy. Cost, tokens, turns and pass are not affected by load; wall and tool-wait are.
- Isolation: the compiled-build cache root was a private directory for the pilot (`WJ_CARGO_TARGET_ROOT`), and the 60 Windjammer trials ran one at a time (see the appendix for why).
- Spend: first harness smoke (path bug, discarded): USD 0.21 recorded; first full run (U, V2 in parallel, discarded for those arms; R kept): 3.57; sequential U and V2 re-run: 2.37. About USD 6.15 recorded in total, plus at most a few cents from two in-flight trials that were stopped; the cap was USD 8.00. The cost check after the first 6 trials projected USD 2.6 for the 90.

## Reading the numbers

Headline.

- Pass: 90/90 in every arm. Nothing separates the arms on pass rate, and the pass criterion of ADR-0033 is met trivially. This task mix is at the ceiling for this model (pilot 5's three failures, t101, t104 and t107, pass 9/9 in every arm now).
- Cost per trial: R 0.033, U 0.039, V2 0.040 USD. Windjammer is not cheaper than Rust in this pilot. The paired differences (U minus R +0.005 [-0.005, 0.016]; V2 minus R +0.007 [-0.006, 0.023]) include zero, so the data do not show a difference either way, but the point estimates favour Rust by 15 to 20 percent. The ranking is driven by task: Windjammer costs more on t104 (0.062 and 0.084 against 0.028) and t103, about the same on 7 tasks and less on t101 (0.034 and 0.032 against 0.069: std-only Rust has no JSON parser, so the Rust agents wrote one). Without t101 the means are R 0.030, U 0.039, V2 0.041; without t104, R 0.034, U 0.036, V2 0.035.
- Turns: R always 3.0 (read, write, one `rustc` run, stop; no trial saw a rustc error); U 3.8; V2 4.8.
- V2 against ADR-0033: pass and silent divergence meet their bars; cost (0.040 against 0.030), tool-wait (17.4 s against 1.0 s) and the load criterion do not; the precision criterion is vacuous in the trials (no prediction was ever shown).

Why V2 behaved like U with an extra interpreter run.

- The V2 agents did not iterate in the interpreter. Of the 57 `wj` calls that ran or checked a program (`check`, `run`), 56 were `wj run --verify`, none was `wj run --interpret` and one was `wj check`. The first run was already the verify run: write the program, verify, stop. `--verify` interprets, then builds and runs compiled, so each call costs a build (median 5.4 s, mean 9.3 s under load 40 to 50) and tool-wait ends up higher than the compiled-first arm (17.4 s against 11.3 s; the paired difference includes zero). The sheet bullet and the prompt both said "iterate with check and `--interpret`, `--verify` last"; the agents optimised for fewest turns and went straight to the last step.
- So the width and String predictions were not exercised by agents. The offline check below measures them on the programs the trials produced.
- `--verify` itself did its job: V2 had 0 silent divergences, and every compiler rejection in V2 trials (t104) was reported by `--verify` with the rustc error and fixed by the agent. That is the safety net of ADR-0033 option (b) working; what it did not deliver is the cost of the interpreted loop.

Predictions in the field. 0 false positives on the 60 final U and V2 programs (all accepted by the compiled build), and 0 of 12 compiler-rejected programs (all from t104) predicted by `wj check`. The 12 programs fall into four classes that are none of families A to C (see `docs/drift/pilot6-findings.md`), so the recall number says the families do not cover the shapes agents hit on this task mix; it is not evidence about precision.

## What this does not show

- Whether Windjammer is cheaper or more reliable than Rust for agents in general. On these 10 tasks, with this model and prompts, the cost difference is within the noise and the point estimate favours Rust; pass rates are all 1.000 and cannot discriminate. A ceiling effect means a harder task set (longer programs, several files, library use, refactoring) is needed before pass rate can tell the arms apart.
- Anything about the interpreted-first loop's value. V2 agents did not use it; the ADR-0033 cost and tool-wait targets were measured for a design where the first run is a build. A rerun needs a loop the agents actually follow (for example `--verify` renamed or gated behind a green interpreted run, or the interpreter run made the only way to see the output until `check` passes), or a measurement of agents told to iterate interpreted explicitly.
- A fair Rust cost. Arm R is std-only `rustc`, one file, no crates and no cargo. Real Rust work would use `serde_json` on t101 and would pay cargo's build time; both move the Rust numbers, in opposite directions. t110 was excluded because it needs a crate.
- Statistical significance. 30 trials per arm over 10 tasks, three trials per cell; the intervals resample tasks and trials and are wide (cost +/- 0.01). Several differences above include zero.
- Wall and tool-wait. 86 of 90 trials ran at load above 16 (means 37 to 48); tool-wait and wall are dominated by contention on the shared machine and by a cold private build cache at the start. The `wj` shim is a Python wrapper and adds start-up time to every call. Quiet-machine numbers are not available.
- Prediction recall or precision at scale. The field check covers 60 accepted and 12 rejected programs of one task family (t104 produces all 12 rejections).
- Differences in failure categories. The harness counts any non-zero exit of `wj run` as a failing verify call, and several tasks demand exit status 1 for invalid input (t101, t107, t109), so the "other" category and the friction list contain the programs' own error output (`error: bad width`, `invalid input`). Only the `WJ0003` rows are compiler diagnostics.
- Anything about readability, maintainability or human preference; the oracle grades behaviour only. The model has seen far more Rust than Windjammer, and the Windjammer arms carry a 0.5k-token sheet.
- A clean first run for U and V2. The first run (two trials at a time) was replaced after a tool defect made some trials see another trial's binary (appendix and `docs/drift/pilot6-repros/same_dir_name_runs_other_binary.md`). The replacement decision was made after looking at the outliers; both runs are in the repository (`results/pilot6-parallel/`), pass rates are identical, and the cost ranking (R lowest) is the same in both.

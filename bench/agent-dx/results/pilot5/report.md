# Agent preference pilot

90 trials over 10 tasks. Intervals are 95% bootstrap CIs that resample tasks, then trials within a task. With this N they show direction, not significance, unless an interval excludes zero.

## Arms

| arm | name | trials | context pack |
|---|---|---|---|
| U | compiled-first | 30 | sheet_noloop.md |
| V | interpreted-first | 30 | sheet_noloop.md |
| W | interpreted+final-build | 30 | sheet_noloop.md |

## Arm means

| metric | U | V | W |
|---|---|---|---|
| pass@1 (compiled hidden oracle) | 1.000 [1.000, 1.000] | 0.667 [0.367, 0.900] | 0.967 [0.867, 1.000] |
| verify calls (wj check/run/test/build) | 3.0 [1.8, 4.3] | 1.8 [1.3, 2.3] | 3.5 [2.2, 4.9] |
| verify calls to green (passing trials) | 2.8 [1.5, 4.5] | 1.5 [1.0, 2.2] | 2.8 [1.7, 4.1] |
| agent turns | 6.7 [4.4, 9.7] | 4.3 [3.4, 5.6] | 6.5 [4.8, 8.4] |
| tokens in (incl. cache) | 47165.1 [27819.9, 76177.7] | 27472.8 [20912.7, 36795.9] | 46407.8 [33290.7, 61675.5] |
| tokens out | 2145.7 [1517.8, 2856.7] | 1423.4 [1217.3, 1627.4] | 1829.5 [1367.7, 2367.9] |
| cost USD | 0.043 [0.031, 0.057] | 0.026 [0.023, 0.029] | 0.038 [0.029, 0.049] |
| wall seconds | 27.1 [19.3, 37.7] | 15.2 [13.9, 16.6] | 22.3 [17.5, 28.2] |
| agent-thinking seconds (API time) | 16.7 [11.7, 23.0] | 10.8 [9.7, 11.9] | 14.4 [11.4, 17.9] |
| tool-wait seconds (wj commands) | 6.5 [3.4, 11.7] | 0.2 [0.1, 0.3] | 3.9 [2.1, 6.5] |
| harness permission denials (should be 0) | 0.3 [0.1, 0.5] | 0.2 [0.0, 0.5] | 0.4 [0.1, 0.7] |
| capability denials seen (WJ004x) | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] |
| machine load average during the trial | 29.1 [26.4, 31.8] | 34.7 [30.0, 39.9] | 32.3 [28.8, 36.2] |
| silent divergence (interp pass, compiled fail) | 0.000 [0.000, 0.000] | 0.300 [0.100, 0.600] | 0.033 [0.000, 0.133] |

## Paired differences (task-paired, first minus second)

| metric | U-V | U-W | V-W |
|---|---|---|---|
| pass@1 (compiled hidden oracle) | 0.333 [0.100, 0.600] | 0.033 [0.000, 0.133] | -0.300 [-0.567, -0.067] |
| verify calls (wj check/run/test/build) | 1.2 [-0.1, 2.5] | -0.5 [-1.7, 0.8] | -1.7 [-3.0, -0.4] |
| verify calls to green (passing trials) | 1.0 [-0.3, 3.0] | 0.5 [-0.3, 1.6] | -0.3 [-1.0, 0.3] |
| agent turns | 2.4 [0.3, 5.0] | 0.3 [-1.7, 3.0] | -2.2 [-4.0, -0.5] |
| tokens in (incl. cache) | 19692.4 [2997.6, 43053.5] | 757.3 [-16405.3, 24738.1] | -18935.0 [-31189.9, -7825.1] |
| tokens out | 722.3 [170.4, 1344.5] | 316.2 [-96.8, 912.3] | -406.1 [-912.7, 8.3] |
| cost USD | 0.017 [0.005, 0.029] | 0.005 [-0.003, 0.018] | -0.011 [-0.022, -0.003] |
| wall seconds | 11.8 [4.1, 21.5] | 4.8 [-2.6, 14.6] | -7.1 [-13.5, -2.2] |
| agent-thinking seconds (API time) | 5.9 [1.1, 11.7] | 2.4 [-1.6, 8.3] | -3.6 [-7.1, -0.6] |
| tool-wait seconds (wj commands) | 6.3 [3.2, 11.3] | 2.6 [-1.2, 7.8] | -3.7 [-6.5, -1.9] |
| harness permission denials (should be 0) | 0.1 [-0.1, 0.3] | -0.1 [-0.3, 0.1] | -0.2 [-0.4, 0.0] |
| capability denials seen (WJ004x) | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] |
| machine load average during the trial | -5.6 [-10.8, -0.5] | -3.1 [-7.8, 1.3] | 2.5 [-3.7, 9.1] |
| silent divergence (interp pass, compiled fail) | -0.300 [-0.600, 0.000] | -0.033 [-0.133, 0.000] | 0.267 [0.000, 0.567] |

## Per-task pass rate (passes/trials)

| task | U | V | W |
|---|---|---|---|
| t101_json_orders | 3/3 | 0/3 | 3/3 |
| t102_file_tally | 3/3 | 2/3 | 3/3 |
| t103_trait_pricing | 3/3 | 3/3 | 3/3 |
| t104_activity_log | 3/3 | 0/3 | 3/3 |
| t105_date_diff | 3/3 | 3/3 | 3/3 |
| t106_priority_queue | 3/3 | 3/3 | 3/3 |
| t107_text_wrap | 3/3 | 0/3 | 2/3 |
| t108_to_rpn | 3/3 | 3/3 | 3/3 |
| t109_spiral | 3/3 | 3/3 | 3/3 |
| t110_crc_lines | 3/3 | 3/3 | 3/3 |

## Top failure categories (diagnostics hit, all verify calls)

| category | U | V | W |
|---|---|---|---|
| other | 28 | 9 | 32 |
| syntax | 20 | 0 | 0 |
| type-mismatch | 0 | 0 | 9 |
| unsupported | 0 | 3 | 0 |
| trait-bound | 1 | 0 | 1 |
| name-resolution | 0 | 0 | 1 |
| lint-style | 1 | 0 | 0 |

## Interpreter vs compiled divergence

19/90 trials had an interpreted hidden-oracle result different from the compiled one (10 silent: interpreter green, compiled red).
- t101_json_orders arm V trial 0: interp=True compiled=False
- t101_json_orders arm V trial 1: interp=True compiled=False
- t101_json_orders arm V trial 2: interp=True compiled=False
- t104_activity_log arm V trial 0: interp=True compiled=False
- t104_activity_log arm V trial 1: interp=True compiled=False
- t104_activity_log arm V trial 2: interp=True compiled=False
- t107_text_wrap arm V trial 0: interp=True compiled=False
- t107_text_wrap arm V trial 1: interp=True compiled=False
- t107_text_wrap arm V trial 2: interp=True compiled=False
- t107_text_wrap arm W trial 0: interp=True compiled=False

## Context attribution (stream-json arms)

First-turn input = system prompt + tool definitions + pack + task prompt. Pack tokens are estimated as bytes/3.5. Schema tokens are the first-turn difference to the matching no-MCP arm (T), minus the pack difference.

| arm | first-turn input | pack tokens (est, bytes/3.5) | other fixed (system, tools, task) | tool-result tokens (est, all turns) | docs shell calls | MCP calls | total tokens in | turns |
|---|---|---|---|---|---|---|---|---|
| U compiled-first | 8295 | 455 | 7840 | 919 | 0.7 | 0.0 | 47165 | 6.7 |
| V interpreted-first | 8345 | 455 | 7890 | 257 | 0.7 | 0.0 | 27473 | 4.3 |
| W interpreted+final-build | 8364 | 455 | 7909 | 788 | 0.4 | 0.0 | 46408 | 6.5 |

## Paired differences versus control U (task-clustered 95% CI)

| metric | V - U | W - U |
|---|---|---|
| pass@1 (compiled hidden oracle) | -0.333 [-0.600, -0.100] | -0.033 [-0.133, 0.000] |
| verify calls (wj check/run/test/build) | -1.2 [-2.5, 0.0] | 0.5 [-0.8, 1.7] |
| verify calls to green (passing trials) | -1.0 [-3.0, 0.3] | -0.5 [-1.6, 0.3] |
| agent turns | -2.4 [-4.9, -0.3] | -0.3 [-3.1, 1.7] |
| tokens in (incl. cache) | -19692.4 [-41500.3, -3252.0] | -757.3 [-24479.3, 16103.0] |
| tokens out | -722.3 [-1357.4, -166.8] | -316.2 [-908.6, 91.5] |
| cost USD | -0.017 [-0.029, -0.006] | -0.005 [-0.018, 0.003] |
| wall seconds | -11.8 [-21.2, -4.1] | -4.8 [-14.4, 2.6] |
| agent-thinking seconds (API time) | -5.9 [-11.6, -1.1] | -2.4 [-8.2, 1.5] |
| tool-wait seconds (wj commands) | -6.3 [-10.8, -3.2] | -2.6 [-7.6, 1.3] |
| harness permission denials (should be 0) | -0.1 [-0.3, 0.1] | 0.1 [-0.1, 0.3] |
| capability denials seen (WJ004x) | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] |
| machine load average during the trial | 5.6 [0.7, 10.9] | 3.1 [-1.2, 7.9] |
| silent divergence (interp pass, compiled fail) | 0.300 [0.000, 0.600] | 0.033 [0.000, 0.133] |

## Top friction by arm (ranked by estimated tokens wasted; Windjammer arms)

**U compiled-first**

1. type mismatch (often int/float, string/&str) (`type-mismatch`): 8 trials, 20 calls, fix: show both types and the conversion (`as`, `.to_string()`); error-code explain entry
2. unclassified failure: error[WJ0031]: `X` is not a reference, so it cannot be dereferenced (`other:error[WJ0031]: `X` is not a reference, so it cannot be dereferenced`): 3 trials, 7 calls, fix: inspect example, then file under a rule
3. unclassified failure: error: bad width (`other:error: bad width`): 4 trials, 5 calls, fix: inspect example, then file under a rule
4. unclassified failure: error (`other:error`): 3 trials, 3 calls, fix: inspect example, then file under a rule
5. unclassified failure: x: cannot read No such file or directory (os error N) (`other:x: cannot read No such file or directory (os error N)`): 1 trials, 2 calls, fix: inspect example, then file under a rule

**V interpreted-first**

1. raw rustc/cargo error surfaced from generated code (`rustc-leak`): 6 trials, 11 calls, fix: map rustc codes back to .wj spans; fix codegen so check-vs-build parity holds (R5)
2. interpreted run passed but compiled grade failed (silent divergence) (`silent-divergence`): 9 trials, 9 calls, fix: R2/R5 gates in docs/design/interpreter-primary-readiness.md; add the program to the differential corpus
3. type mismatch (often int/float, string/&str) (`type-mismatch`): 4 trials, 7 calls, fix: show both types and the conversion (`as`, `.to_string()`); error-code explain entry
4. interpreter refused a feature the compiled path accepts (`interp-unsupported`): 3 trials, 3 calls, fix: src/interpreter + docs/design/interpreter-primary-readiness.md R6
5. unclassified failure: error (`other:error`): 3 trials, 3 calls, fix: inspect example, then file under a rule

**W interpreted+final-build**

1. type mismatch (often int/float, string/&str) (`type-mismatch`): 8 trials, 11 calls, fix: show both types and the conversion (`as`, `.to_string()`); error-code explain entry
2. raw rustc/cargo error surfaced from generated code (`rustc-leak`): 4 trials, 9 calls, fix: map rustc codes back to .wj spans; fix codegen so check-vs-build parity holds (R5)
3. unclassified failure: error (`other:error`): 3 trials, 3 calls, fix: inspect example, then file under a rule
4. unclassified failure: error: bad width (`other:error: bad width`): 3 trials, 3 calls, fix: inspect example, then file under a rule
5. unresolved name (`name-unresolved`): 1 trials, 1 calls, fix: diagnostic should list candidates and the import to add


Mode violations (agent used the other arm's loop commands): 0.

## Friction list (ranked by estimated tokens wasted)

| # | pri | friction | fix kind | fix location | trials | tasks | wasted calls | est. tokens |
|---|---|---|---|---|---|---|---|---|
| 1 | P0 | type mismatch (often int/float, string/&str) (`type-mismatch`) | diagnostic text | show both types and the conversion (`as`, `.to_string()`); error-code explain entry | 20 | 5 | 38 | 268102 |
| 2 | P0 | raw rustc/cargo error surfaced from generated code (`rustc-leak`) | codegen / diagnostic mapping | map rustc codes back to .wj spans; fix codegen so check-vs-build parity holds (R5) | 11 | 2 | 21 | 147625 |
| 3 | P0 | unclassified failure: error: bad width (`other:error: bad width`) | diagnostic text (triage) | inspect example, then file under a rule | 10 | 2 | 11 | 77242 |
| 4 | P1 | interpreted run passed but compiled grade failed (silent divergence) (`silent-divergence`) | interpreter / check parity | R2/R5 gates in docs/design/interpreter-primary-readiness.md; add the program to the differential corpus | 10 | 3 | 10 | 67584 |
| 5 | P1 | unclassified failure: error (`other:error`) | diagnostic text (triage) | inspect example, then file under a rule | 9 | 1 | 9 | 63319 |
| 6 | P1 | unclassified failure: error[WJ0031]: `X` is not a reference, so it cannot be dereferenced (`other:error[WJ0031]: `X` is not a reference, so it cannot be dereferenced`) | diagnostic text (triage) | inspect example, then file under a rule | 3 | 1 | 7 | 48730 |
| 7 | P1 | interpreter refused a feature the compiled path accepts (`interp-unsupported`) | interpreter coverage | src/interpreter + docs/design/interpreter-primary-readiness.md R6 | 3 | 1 | 3 | 20040 |
| 8 | P1 | unresolved name (`name-unresolved`) | diagnostic text | diagnostic should list candidates and the import to add | 2 | 1 | 2 | 14145 |
| 9 | P2 | unclassified failure: x: cannot read No such file or directory (os error N) (`other:x: cannot read No such file or directory (os error N)`) | diagnostic text (triage) | inspect example, then file under a rule | 1 | 1 | 2 | 13923 |
| 10 | P2 | Rust-leakage lint rejected idiomatic-looking code (`lint-leakage`) | lint rule | src/lint (rule text + `wj fix` autofix); docs/agent pack section 'Core idioms' | 1 | 1 | 1 | 6961 |
| 11 | P2 | unclassified failure: error[WJ0017]: trait bound not satisfied: the type `X` cannot be indexed by `X` (`other:error[WJ0017]: trait bound not satisfied: the type `X` cannot be indexed by `X``) | diagnostic text (triage) | inspect example, then file under a rule | 1 | 1 | 1 | 6961 |

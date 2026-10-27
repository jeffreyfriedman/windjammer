# Agent preference pilot

60 trials over 10 tasks. Intervals are 95% bootstrap CIs that resample tasks, then trials within a task. With this N they show direction, not significance, unless an interval excludes zero.

## Arms

| arm | name | trials | context pack |
|---|---|---|---|
| A | interpreted-first | 20 | yes |
| B | compiled-first | 21 | yes |
| D | interpreted+final-build | 19 | yes |

## Arm means

| metric | A | B | D |
|---|---|---|---|
| pass@1 (compiled hidden oracle) | 0.500 [0.200, 0.800] | 0.952 [0.810, 1.000] | 0.895 [0.706, 1.000] |
| verify calls (wj check/run/test/build) | 1.7 [1.2, 2.2] | 1.8 [1.3, 2.4] | 4.1 [2.9, 5.3] |
| verify calls to green (passing trials) | 1.3 [1.0, 1.9] | 1.9 [1.4, 2.5] | 3.4 [2.1, 4.7] |
| agent turns | 3.4 [3.0, 3.9] | 5.0 [3.9, 6.5] | 3.7 [2.3, 5.3] |
| tokens in (incl. cache) | 35220.2 [30684.8, 41124.2] | 47586.9 [37525.4, 60342.1] | 60217.1 [39080.5, 83588.5] |
| tokens out | 1344.0 [1062.3, 1662.0] | 1670.3 [1267.8, 2275.2] | 598.3 [324.6, 906.5] |
| cost USD | 0.031 [0.026, 0.036] | 0.038 [0.029, 0.052] | 0.069 [0.058, 0.082] |
| wall seconds | 23.1 [20.9, 25.3] | 70.7 [49.2, 93.8] | 454.2 [368.0, 544.0] |
| agent-thinking seconds (API time) | 11.2 [9.3, 13.5] | 15.2 [10.4, 23.9] | 37.7 [29.7, 46.4] |
| tool-wait seconds (wj commands) | 0.3 [0.1, 0.5] | 41.2 [26.2, 57.2] | 374.8 [283.9, 466.4] |
| harness permission denials (should be 0) | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.2] | 0.0 [0.0, 0.0] |
| capability denials seen (WJ004x) | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] |
| silent divergence (interp pass, compiled fail) | 0.450 [0.150, 0.750] | 0.000 [0.000, 0.000] | 0.105 [0.000, 0.294] |

## Paired differences (task-paired, first minus second)

| metric | A-B | A-D | B-D |
|---|---|---|---|
| pass@1 (compiled hidden oracle) | -0.452 [-0.750, -0.159] | -0.395 [-0.700, -0.039] | 0.058 [-0.143, 0.283] |
| verify calls (wj check/run/test/build) | -0.1 [-0.8, 0.6] | -2.4 [-3.9, -1.1] | -2.3 [-3.7, -0.9] |
| verify calls to green (passing trials) | -0.7 [-1.2, -0.3] | -1.1 [-2.3, 0.0] | -1.6 [-2.9, -0.3] |
| agent turns | -1.6 [-2.9, -0.5] | -0.3 [-2.2, 1.3] | 1.3 [-0.7, 3.5] |
| tokens in (incl. cache) | -12366.7 [-23202.3, -3764.4] | -24996.9 [-49505.6, -2980.3] | -12630.2 [-38514.1, 14145.1] |
| tokens out | -326.3 [-670.1, -88.1] | 745.7 [270.4, 1150.8] | 1072.0 [517.8, 1742.9] |
| cost USD | -0.007 [-0.019, 0.003] | -0.038 [-0.051, -0.026] | -0.031 [-0.046, -0.015] |
| wall seconds | -47.7 [-70.5, -26.4] | -431.1 [-529.2, -345.6] | -383.4 [-481.2, -292.5] |
| agent-thinking seconds (API time) | -3.9 [-10.7, 0.3] | -26.4 [-34.6, -18.8] | -22.5 [-32.2, -12.6] |
| tool-wait seconds (wj commands) | -40.9 [-57.1, -26.1] | -374.5 [-466.6, -284.5] | -333.7 [-422.4, -241.0] |
| harness permission denials (should be 0) | -0.0 [-0.2, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.2] |
| capability denials seen (WJ004x) | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] |
| silent divergence (interp pass, compiled fail) | 0.450 [0.200, 0.750] | 0.345 [0.000, 0.697] | -0.105 [-0.312, 0.000] |

## Per-task pass rate (passes/trials)

| task | A | B | D |
|---|---|---|---|
| t003_prime_counts | 2/2 | 3/3 | 2/2 |
| t006_base_convert | 0/2 | 2/2 | 2/2 |
| t009_rpn_eval | 1/2 | 2/2 | 2/2 |
| t011_lru_cache | 1/2 | 2/2 | 0/1 |
| t013_bst | 1/2 | 2/2 | 2/2 |
| t018_word_freq | 0/2 | 2/2 | 2/2 |
| t022_csv_parse | 2/2 | 2/2 | 1/2 |
| t030_bank_ledger | 0/2 | 2/2 | 2/2 |
| t034_shapes_trait | 2/2 | 2/2 | 2/2 |
| t050_cat_number | 1/2 | 1/2 | 2/2 |

## Top failure categories (diagnostics hit, all verify calls)

| category | A | B | D |
|---|---|---|---|
| other | 1 | 16 | 25 |
| type-mismatch | 0 | 13 | 20 |
| trait-bound | 0 | 7 | 4 |
| syntax | 2 | 2 | 2 |
| runtime-panic | 1 | 0 | 1 |

## Interpreter vs compiled divergence

15 of 60 trials had an interpreted hidden-oracle result different from the compiled one (11 silent: interpreter green, compiled red).
- t006_base_convert arm A trial 0: interp=True compiled=False
- t006_base_convert arm A trial 1: interp=True compiled=False
- t009_rpn_eval arm A trial 1: interp=True compiled=False
- t011_lru_cache arm A trial 1: interp=True compiled=False
- t011_lru_cache arm D trial 0: interp=True compiled=False
- t013_bst arm A trial 0: interp=True compiled=False
- t018_word_freq arm A trial 0: interp=True compiled=False
- t018_word_freq arm A trial 1: interp=True compiled=False
- t022_csv_parse arm D trial 0: interp=True compiled=False
- t030_bank_ledger arm A trial 0: interp=True compiled=False

Mode violations (agent used the other arm's loop commands): 0.

## Friction list (ranked by estimated tokens wasted)

| # | pri | friction | fix kind | fix location | trials | tasks | wasted calls | est. tokens |
|---|---|---|---|---|---|---|---|---|
| 1 | P0 | type mismatch (often int/float, string/&str) (`type-mismatch`) | diagnostic text | show both types and the conversion (`as`, `.to_string()`); error-code explain entry | 29 | 8 | 50 | 701016 |
| 2 | P0 | slow `wj build` (compiled) >= 30s (`slow-command:build-compiled`) | performance | build cache / runtime pre-build; see wall-time table | 19 | 10 | 20 | 353139 |
| 3 | P0 | interpreted run passed but compiled grade failed (silent divergence) (`silent-divergence`) | interpreter / check parity | R2/R5 gates in docs/design/interpreter-primary-readiness.md; add the program to the differential corpus | 11 | 7 | 11 | 131679 |
| 4 | P1 | slow `wj run` (compiled) >= 30s (`slow-command:run-compiled`) | performance | build cache / runtime pre-build; see wall-time table | 10 | 7 | 12 | 121554 |
| 5 | P1 | collect/iterator element type mismatch in generated Rust (`collect-type`) | codegen | iterator lowering (borrowed vs owned element types); known-issues agent-dx-compiled-str-literal-and-option-default-codegen.md | 7 | 3 | 9 | 115481 |
| 6 | P1 | runtime panic during the loop (`runtime-panic`) | docs | pack section on Option/Result handling and index bounds | 2 | 1 | 2 | 28364 |
| 7 | P1 | raw rustc/cargo error surfaced from generated code (`rustc-leak`) | codegen / diagnostic mapping | map rustc codes back to .wj spans; fix codegen so check-vs-build parity holds (R5) | 1 | 1 | 1 | 10129 |

# Agent preference pilot

120 trials over 10 tasks. Intervals are 95% bootstrap CIs that resample tasks, then trials within a task. With this N they show direction, not significance, unless an interval excludes zero.

## Arms

| arm | name | trials | context pack |
|---|---|---|---|
| K | compiled-first+sheet | 30 | sheet_noloop.md |
| L | interpreted-first+sheet | 30 | sheet_noloop.md |
| M | interpreted+final-build+sheet | 30 | sheet_noloop.md |
| N | interpreted+final-build+v2 | 30 | sheet_v2.md |

## Arm means

| metric | K | L | M | N |
|---|---|---|---|---|
| pass@1 (compiled hidden oracle) | 1.000 [1.000, 1.000] | 0.600 [0.333, 0.867] | 0.833 [0.633, 0.967] | 0.967 [0.867, 1.000] |
| verify calls (wj check/run/test/build) | 2.5 [1.3, 4.3] | 1.5 [1.1, 1.9] | 3.8 [2.8, 5.0] | 3.4 [2.5, 4.4] |
| verify calls to green (passing trials) | 1.9 [1.2, 3.0] | 1.1 [1.0, 1.2] | 2.6 [1.6, 4.0] | 2.3 [1.4, 3.4] |
| agent turns | 6.3 [3.7, 9.8] | 3.4 [3.1, 3.8] | 6.5 [5.0, 8.4] | 7.3 [5.7, 9.1] |
| tokens in (incl. cache) | 47739.0 [23689.9, 82360.0] | 21269.9 [18538.0, 24309.9] | 46877.2 [35390.4, 60023.2] | 48838.3 [34820.4, 67444.0] |
| tokens out | 1987.5 [1395.6, 2699.7] | 1333.0 [1051.2, 1670.5] | 1675.8 [1335.6, 2053.7] | 1948.2 [1399.0, 2608.8] |
| cost USD | 0.041 [0.028, 0.057] | 0.024 [0.020, 0.028] | 0.034 [0.027, 0.042] | 0.042 [0.030, 0.057] |
| wall seconds | 44.4 [28.3, 65.4] | 16.5 [14.1, 19.2] | 31.6 [26.9, 36.7] | 34.7 [27.5, 43.6] |
| agent-thinking seconds (API time) | 15.7 [10.4, 23.0] | 10.0 [8.2, 11.9] | 14.1 [11.3, 16.9] | 15.1 [11.3, 19.7] |
| tool-wait seconds (wj commands) | 19.9 [9.4, 34.6] | 0.2 [0.1, 0.3] | 10.5 [7.2, 14.6] | 12.0 [8.6, 16.2] |
| harness permission denials (should be 0) | 0.5 [0.1, 0.9] | 0.0 [0.0, 0.1] | 0.2 [0.0, 0.4] | 0.2 [0.0, 0.4] |
| capability denials seen (WJ004x) | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] |
| machine load average during the trial | 30.3 [24.7, 35.6] | 29.6 [24.2, 36.0] | 29.7 [24.7, 35.4] | 24.5 [19.8, 29.7] |
| silent divergence (interp pass, compiled fail) | 0.000 [0.000, 0.000] | 0.400 [0.133, 0.667] | 0.167 [0.033, 0.367] | 0.000 [0.000, 0.000] |

## Paired differences (task-paired, first minus second)

| metric | K-L | K-M | K-N | L-M | L-N | M-N |
|---|---|---|---|---|---|---|
| pass@1 (compiled hidden oracle) | 0.400 [0.133, 0.667] | 0.167 [0.000, 0.367] | 0.033 [0.000, 0.133] | -0.233 [-0.533, 0.033] | -0.367 [-0.667, -0.067] | -0.133 [-0.367, 0.067] |
| verify calls (wj check/run/test/build) | 1.1 [-0.2, 2.8] | -1.3 [-2.7, 0.4] | -0.8 [-2.2, 0.8] | -2.3 [-3.5, -1.4] | -1.9 [-2.9, -1.0] | 0.4 [-1.0, 1.9] |
| verify calls to green (passing trials) | 0.4 [-0.1, 1.0] | -0.7 [-1.9, 0.3] | -0.4 [-1.7, 0.7] | -1.0 [-2.5, -0.2] | -1.0 [-2.3, -0.1] | 0.3 [-1.2, 2.0] |
| agent turns | 2.9 [0.3, 6.6] | -0.2 [-2.8, 2.8] | -1.0 [-4.2, 2.9] | -3.1 [-5.1, -1.5] | -3.9 [-5.6, -2.2] | -0.8 [-3.2, 1.7] |
| tokens in (incl. cache) | 26469.2 [2352.3, 61760.9] | 861.8 [-24941.9, 32518.6] | -1099.2 [-30687.5, 36684.8] | -25607.3 [-40241.5, -13338.8] | -27568.4 [-45427.7, -13370.3] | -1961.1 [-24371.1, 17331.4] |
| tokens out | 654.4 [98.6, 1476.6] | 311.7 [-222.7, 976.3] | 39.3 [-669.6, 875.6] | -342.8 [-698.4, -66.1] | -615.2 [-1035.9, -270.6] | -272.4 [-792.3, 197.4] |
| cost USD | 0.017 [0.004, 0.034] | 0.007 [-0.006, 0.021] | -0.001 [-0.018, 0.018] | -0.010 [-0.019, -0.004] | -0.018 [-0.030, -0.009] | -0.008 [-0.022, 0.005] |
| wall seconds | 27.9 [11.5, 49.5] | 12.7 [-3.5, 33.4] | 9.6 [-5.6, 30.8] | -15.1 [-20.6, -9.9] | -18.3 [-26.2, -11.6] | -3.1 [-12.6, 5.4] |
| agent-thinking seconds (API time) | 5.8 [0.2, 13.7] | 1.7 [-3.8, 8.5] | 0.6 [-5.8, 8.9] | -4.1 [-7.2, -1.3] | -5.2 [-8.9, -2.1] | -1.1 [-5.5, 2.9] |
| tool-wait seconds (wj commands) | 19.7 [9.0, 34.9] | 9.4 [-1.5, 23.5] | 7.9 [-1.9, 22.6] | -10.3 [-14.2, -7.1] | -11.9 [-16.1, -8.4] | -1.5 [-6.4, 3.8] |
| harness permission denials (should be 0) | 0.4 [0.1, 1.0] | 0.3 [-0.1, 0.8] | 0.3 [-0.1, 0.8] | -0.1 [-0.4, 0.0] | -0.1 [-0.4, 0.0] | 0.0 [-0.3, 0.2] |
| capability denials seen (WJ004x) | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] |
| machine load average during the trial | 0.7 [-8.5, 9.2] | 0.6 [-8.4, 8.9] | 5.7 [-1.6, 13.0] | -0.1 [-7.5, 7.1] | 5.0 [-3.1, 12.6] | 5.1 [-2.8, 12.7] |
| silent divergence (interp pass, compiled fail) | -0.400 [-0.667, -0.133] | -0.167 [-0.367, 0.000] | 0.000 [0.000, 0.000] | 0.233 [-0.033, 0.533] | 0.400 [0.167, 0.667] | 0.167 [0.000, 0.367] |

## Per-task pass rate (passes/trials)

| task | K | L | M | N |
|---|---|---|---|---|
| t003_prime_counts | 3/3 | 2/3 | 2/3 | 3/3 |
| t006_base_convert | 3/3 | 0/3 | 3/3 | 3/3 |
| t009_rpn_eval | 3/3 | 3/3 | 3/3 | 3/3 |
| t011_lru_cache | 3/3 | 1/3 | 3/3 | 3/3 |
| t013_bst | 3/3 | 1/3 | 2/3 | 3/3 |
| t018_word_freq | 3/3 | 0/3 | 2/3 | 3/3 |
| t022_csv_parse | 3/3 | 3/3 | 3/3 | 3/3 |
| t030_bank_ledger | 3/3 | 3/3 | 3/3 | 3/3 |
| t034_shapes_trait | 3/3 | 2/3 | 1/3 | 3/3 |
| t050_cat_number | 3/3 | 3/3 | 3/3 | 2/3 |

## Top failure categories (diagnostics hit, all verify calls)

| category | K | L | M | N |
|---|---|---|---|---|
| other | 22 | 3 | 34 | 31 |
| type-mismatch | 5 | 0 | 22 | 19 |
| lint-style | 2 | 1 | 0 | 13 |
| trait-bound | 2 | 0 | 11 | 0 |
| syntax | 13 | 0 | 0 | 0 |

## Interpreter vs compiled divergence

28/120 trials had an interpreted hidden-oracle result different from the compiled one (17 silent: interpreter green, compiled red).
- t003_prime_counts arm L trial 1: interp=True compiled=False
- t003_prime_counts arm M trial 1: interp=True compiled=False
- t006_base_convert arm L trial 0: interp=True compiled=False
- t006_base_convert arm L trial 1: interp=True compiled=False
- t006_base_convert arm L trial 2: interp=True compiled=False
- t011_lru_cache arm L trial 0: interp=True compiled=False
- t011_lru_cache arm L trial 2: interp=True compiled=False
- t013_bst arm L trial 1: interp=True compiled=False
- t013_bst arm L trial 2: interp=True compiled=False
- t013_bst arm M trial 0: interp=True compiled=False

## Context attribution (stream-json arms)

First-turn input = system prompt + tool definitions + pack + task prompt. Pack tokens are estimated as bytes/3.5. Schema tokens are the first-turn difference to the matching no-MCP arm (T), minus the pack difference.

| arm | first-turn input | pack tokens (est, bytes/3.5) | other fixed (system, tools, task) | tool-result tokens (est, all turns) | docs shell calls | MCP calls | total tokens in | turns |
|---|---|---|---|---|---|---|---|---|
| K compiled-first+sheet | 8169 | 410 | 7759 | 850 | 0.2 | 0.0 | 47739 | 6.3 |
| L interpreted-first+sheet | 8219 | 410 | 7809 | 111 | 0.1 | 0.0 | 21270 | 3.4 |
| M interpreted+final-build+sheet | 8238 | 410 | 7828 | 547 | 0.0 | 0.0 | 46877 | 6.5 |
| N interpreted+final-build+v2 | 7889 | 190 | 7699 | 1143 | 0.0 | 0.0 | 48838 | 7.3 |

## Paired differences versus control K (task-clustered 95% CI)

| metric | L - K | M - K | N - K |
|---|---|---|---|
| pass@1 (compiled hidden oracle) | -0.400 [-0.667, -0.167] | -0.167 [-0.367, 0.000] | -0.033 [-0.133, 0.000] |
| verify calls (wj check/run/test/build) | -1.1 [-2.7, 0.2] | 1.3 [-0.5, 2.7] | 0.8 [-0.7, 2.2] |
| verify calls to green (passing trials) | -0.4 [-1.0, 0.1] | 0.7 [-0.3, 2.0] | 0.4 [-0.6, 1.7] |
| agent turns | -2.9 [-6.4, -0.3] | 0.2 [-2.9, 2.7] | 1.0 [-2.8, 4.2] |
| tokens in (incl. cache) | -26469.2 [-59341.8, -2553.7] | -861.8 [-32019.0, 24379.3] | 1099.2 [-34165.6, 30909.9] |
| tokens out | -654.4 [-1442.1, -98.8] | -311.7 [-1004.1, 221.6] | -39.3 [-860.2, 636.7] |
| cost USD | -0.017 [-0.034, -0.005] | -0.007 [-0.022, 0.006] | 0.001 [-0.018, 0.018] |
| wall seconds | -27.9 [-49.1, -11.7] | -12.7 [-32.7, 3.5] | -9.6 [-31.3, 5.5] |
| agent-thinking seconds (API time) | -5.8 [-13.4, -0.2] | -1.7 [-8.6, 3.8] | -0.6 [-8.7, 5.8] |
| tool-wait seconds (wj commands) | -19.7 [-34.9, -9.1] | -9.4 [-24.2, 1.5] | -7.9 [-22.7, 1.9] |
| harness permission denials (should be 0) | -0.4 [-0.9, -0.1] | -0.3 [-0.8, 0.1] | -0.3 [-0.8, 0.1] |
| capability denials seen (WJ004x) | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] |
| machine load average during the trial | -0.7 [-9.4, 8.2] | -0.6 [-9.0, 8.0] | -5.7 [-13.0, 1.6] |
| silent divergence (interp pass, compiled fail) | 0.400 [0.167, 0.667] | 0.167 [0.000, 0.367] | 0.000 [0.000, 0.000] |

## Top friction by arm (ranked by estimated tokens wasted; Windjammer arms)

**K compiled-first+sheet**

1. type mismatch (often int/float, string/&str) (`type-mismatch`): 7 trials, 18 calls, fix: show both types and the conversion (`as`, `.to_string()`); error-code explain entry
2. collect/iterator element type mismatch in generated Rust (`collect-type`): 4 trials, 4 calls, fix: iterator lowering (borrowed vs owned element types); known-issues agent-dx-compiled-str-literal-and-option-default-codegen.md
3. Rust-leakage lint rejected idiomatic-looking code (`lint-leakage`): 2 trials, 2 calls, fix: src/lint (rule text + `wj fix` autofix); docs/agent pack section 'Core idioms'
4. unclassified failure: error[WJ0031]: `X` is not a reference, so it cannot be dereferenced (`other:error[WJ0031]: `X` is not a reference, so it cannot be dereferenced`): 2 trials, 2 calls, fix: inspect example, then file under a rule
5. unclassified failure: error[WJ0007]: value `X` is used after it was moved (`other:error[WJ0007]: value `X` is used after it was moved`): 1 trials, 1 calls, fix: inspect example, then file under a rule

**L interpreted-first+sheet**

1. interpreted run passed but compiled grade failed (silent divergence) (`silent-divergence`): 12 trials, 12 calls, fix: R2/R5 gates in docs/design/interpreter-primary-readiness.md; add the program to the differential corpus
2. collect/iterator element type mismatch in generated Rust (`collect-type`): 7 trials, 7 calls, fix: iterator lowering (borrowed vs owned element types); known-issues agent-dx-compiled-str-literal-and-option-default-codegen.md
3. raw rustc/cargo error surfaced from generated code (`rustc-leak`): 3 trials, 6 calls, fix: map rustc codes back to .wj spans; fix codegen so check-vs-build parity holds (R5)
4. untyped integer literal emitted as i32 and mixed with int (i64) (`int-width`): 5 trials, 5 calls, fix: codegen integer-literal typing; docs/known-issues/agent-dx-compiled-untyped-int-accumulator.md; pack 'Compiled path gaps'
5. type mismatch (often int/float, string/&str) (`type-mismatch`): 2 trials, 2 calls, fix: show both types and the conversion (`as`, `.to_string()`); error-code explain entry

**M interpreted+final-build+sheet**

1. type mismatch (often int/float, string/&str) (`type-mismatch`): 9 trials, 26 calls, fix: show both types and the conversion (`as`, `.to_string()`); error-code explain entry
2. collect/iterator element type mismatch in generated Rust (`collect-type`): 4 trials, 9 calls, fix: iterator lowering (borrowed vs owned element types); known-issues agent-dx-compiled-str-literal-and-option-default-codegen.md
3. interpreted run passed but compiled grade failed (silent divergence) (`silent-divergence`): 5 trials, 8 calls, fix: R2/R5 gates in docs/design/interpreter-primary-readiness.md; add the program to the differential corpus
4. raw rustc/cargo error surfaced from generated code (`rustc-leak`): 2 trials, 3 calls, fix: map rustc codes back to .wj spans; fix codegen so check-vs-build parity holds (R5)
5. method accepted by the interpreter is missing on the compiled path (`method-parity`): 1 trials, 2 calls, fix: `wj check` must reject what rustc will (R5 check-vs-build gate); alias or implement the std method; `wj docs` entry

**N interpreted+final-build+v2**

1. type mismatch (often int/float, string/&str) (`type-mismatch`): 9 trials, 20 calls, fix: show both types and the conversion (`as`, `.to_string()`); error-code explain entry
2. Rust-leakage lint rejected idiomatic-looking code (`lint-leakage`): 12 trials, 12 calls, fix: src/lint (rule text + `wj fix` autofix); docs/agent pack section 'Core idioms'
3. unclassified failure: = note: ADR-N: Windjammer has no macros; the `X` forms become errors in a future release (`other:= note: ADR-N: Windjammer has no macros; the `X` forms become errors in a future release`): 1 trials, 1 calls, fix: inspect example, then file under a rule


Mode violations (agent used the other arm's loop commands): 0.

## Friction list (ranked by estimated tokens wasted)

| # | pri | friction | fix kind | fix location | trials | tasks | wasted calls | est. tokens |
|---|---|---|---|---|---|---|---|---|
| 1 | P0 | type mismatch (often int/float, string/&str) (`type-mismatch`) | diagnostic text | show both types and the conversion (`as`, `.to_string()`); error-code explain entry | 27 | 7 | 66 | 470107 |
| 2 | P0 | collect/iterator element type mismatch in generated Rust (`collect-type`) | codegen | iterator lowering (borrowed vs owned element types); known-issues agent-dx-compiled-str-literal-and-option-default-codegen.md | 15 | 4 | 20 | 141367 |
| 3 | P0 | interpreted run passed but compiled grade failed (silent divergence) (`silent-divergence`) | interpreter / check parity | R2/R5 gates in docs/design/interpreter-primary-readiness.md; add the program to the differential corpus | 17 | 6 | 20 | 138325 |
| 4 | P1 | Rust-leakage lint rejected idiomatic-looking code (`lint-leakage`) | lint rule | src/lint (rule text + `wj fix` autofix); docs/agent pack section 'Core idioms' | 15 | 6 | 15 | 101988 |
| 5 | P1 | raw rustc/cargo error surfaced from generated code (`rustc-leak`) | codegen / diagnostic mapping | map rustc codes back to .wj spans; fix codegen so check-vs-build parity holds (R5) | 5 | 2 | 9 | 61700 |
| 6 | P1 | untyped integer literal emitted as i32 and mixed with int (i64) (`int-width`) | codegen (literal defaulting) + pack tip | codegen integer-literal typing; docs/known-issues/agent-dx-compiled-untyped-int-accumulator.md; pack 'Compiled path gaps' | 6 | 3 | 6 | 40222 |
| 7 | P1 | method accepted by the interpreter is missing on the compiled path (`method-parity`) | stdlib parity / check | `wj check` must reject what rustc will (R5 check-vs-build gate); alias or implement the std method; `wj docs` entry | 1 | 1 | 2 | 14926 |
| 8 | P1 | unclassified failure: error[WJ0031]: `X` is not a reference, so it cannot be dereferenced (`other:error[WJ0031]: `X` is not a reference, so it cannot be dereferenced`) | diagnostic text (triage) | inspect example, then file under a rule | 2 | 1 | 2 | 14169 |
| 9 | P2 | unclassified failure: error[WJ0007]: value `X` is used after it was moved (`other:error[WJ0007]: value `X` is used after it was moved`) | diagnostic text (triage) | inspect example, then file under a rule | 1 | 1 | 1 | 7084 |
| 10 | P2 | unclassified failure: = note: ADR-N: Windjammer has no macros; the `X` forms become errors in a future release (`other:= note: ADR-N: Windjammer has no macros; the `X` forms become errors in a future release`) | diagnostic text (triage) | inspect example, then file under a rule | 1 | 1 | 1 | 6772 |

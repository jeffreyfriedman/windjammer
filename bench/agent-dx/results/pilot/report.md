# Agent preference pilot

60 trials over 10 tasks. Intervals are 95% bootstrap CIs that resample tasks, then trials within a task. With this N they show direction, not significance, unless an interval excludes zero.

## Arms

| arm | name | trials | context pack |
|---|---|---|---|
| A | interpreted-first | 30 | yes |
| B | compiled-first | 30 | yes |

## Arm means

| metric | A | B |
|---|---|---|
| pass@1 (compiled hidden oracle) | 0.567 [0.300, 0.833] | 0.900 [0.700, 1.000] |
| verify calls (wj check/run/test/build) | 2.0 [1.4, 2.8] | 2.3 [1.6, 3.1] |
| verify calls to green (passing trials) | 1.1 [1.0, 1.5] | 2.0 [1.3, 2.9] |
| agent turns | 3.5 [3.0, 4.2] | 5.3 [3.9, 6.9] |
| tokens in (incl. cache) | 31754.8 [27254.5, 37765.1] | 53623.9 [37441.5, 73217.7] |
| tokens out | 1435.3 [1155.3, 1764.4] | 1681.6 [1324.3, 2207.1] |
| cost USD | 0.028 [0.023, 0.032] | 0.041 [0.031, 0.054] |
| wall seconds | 20.3 [17.5, 23.6] | 96.9 [55.0, 154.7] |
| agent-thinking seconds (API time) | 11.3 [9.6, 13.3] | 16.2 [10.9, 23.4] |
| tool-wait seconds (wj commands) | 0.3 [0.1, 0.5] | 66.1 [32.8, 105.4] |
| harness permission denials (should be 0) | 0.1 [0.0, 0.3] | 0.2 [0.0, 0.4] |
| silent divergence (interp pass, compiled fail) | 0.433 [0.167, 0.700] | 0.000 [0.000, 0.000] |

## Paired differences (task-paired, first minus second)

| metric | A-B |
|---|---|
| pass@1 (compiled hidden oracle) | -0.333 [-0.633, -0.100] |
| verify calls (wj check/run/test/build) | -0.3 [-1.2, 0.7] |
| verify calls to green (passing trials) | -0.6 [-1.5, -0.0] |
| agent turns | -1.8 [-3.5, -0.5] |
| tokens in (incl. cache) | -21869.1 [-42514.3, -6136.0] |
| tokens out | -246.3 [-590.3, 32.6] |
| cost USD | -0.013 [-0.024, -0.005] |
| wall seconds | -76.6 [-137.5, -34.1] |
| agent-thinking seconds (API time) | -4.9 [-10.7, -0.5] |
| tool-wait seconds (wj commands) | -65.9 [-109.3, -32.2] |
| harness permission denials (should be 0) | -0.1 [-0.3, 0.2] |
| silent divergence (interp pass, compiled fail) | 0.433 [0.167, 0.700] |

## Per-task pass rate (passes/trials)

| task | A | B |
|---|---|---|
| t003_prime_counts | 3/3 | 3/3 |
| t006_base_convert | 0/3 | 3/3 |
| t009_rpn_eval | 3/3 | 3/3 |
| t011_lru_cache | 0/3 | 3/3 |
| t013_bst | 1/3 | 3/3 |
| t018_word_freq | 0/3 | 0/3 |
| t022_csv_parse | 3/3 | 3/3 |
| t030_bank_ledger | 1/3 | 3/3 |
| t034_shapes_trait | 3/3 | 3/3 |
| t050_cat_number | 3/3 | 3/3 |

## Top failure categories (diagnostics hit, all verify calls)

| category | A | B |
|---|---|---|
| other | 7 | 21 |
| type-mismatch | 0 | 16 |
| api-misuse | 9 | 0 |
| trait-bound | 0 | 6 |
| syntax | 1 | 3 |
| runtime-panic | 1 | 1 |

## Interpreter vs compiled divergence

13 of 60 trials had an interpreted hidden-oracle result different from the compiled one (13 silent: interpreter green, compiled red).
- t006_base_convert arm A trial 0: interp=True compiled=False
- t006_base_convert arm A trial 1: interp=True compiled=False
- t006_base_convert arm A trial 2: interp=True compiled=False
- t011_lru_cache arm A trial 0: interp=True compiled=False
- t011_lru_cache arm A trial 1: interp=True compiled=False
- t011_lru_cache arm A trial 2: interp=True compiled=False
- t013_bst arm A trial 0: interp=True compiled=False
- t013_bst arm A trial 1: interp=True compiled=False
- t018_word_freq arm A trial 0: interp=True compiled=False
- t018_word_freq arm A trial 1: interp=True compiled=False

Mode violations (agent used the other arm's loop commands): 1.

## Friction list (ranked by estimated tokens wasted)

| # | pri | friction | fix kind | fix location | trials | tasks | wasted calls | est. tokens |
|---|---|---|---|---|---|---|---|---|
| 1 | P0 | type mismatch (often int/float, string/&str) (`type-mismatch`) | diagnostic text | show both types and the conversion (`as`, `.to_string()`); error-code explain entry | 16 | 5 | 24 | 231117 |
| 2 | P0 | interpreted run passed but compiled grade failed (silent divergence) (`silent-divergence`) | interpreter / check parity | R2/R5 gates in docs/design/interpreter-primary-readiness.md; add the program to the differential corpus | 13 | 5 | 17 | 160553 |
| 3 | P0 | slow `wj run` (compiled) >= 30s (`slow-command:run-compiled`) | performance | build cache / runtime pre-build; see wall-time table | 10 | 7 | 14 | 144692 |
| 4 | P1 | raw rustc/cargo error surfaced from generated code (`rustc-leak`) | codegen / diagnostic mapping | map rustc codes back to .wj spans; fix codegen so check-vs-build parity holds (R5) | 13 | 7 | 14 | 143801 |
| 5 | P1 | method accepted by the interpreter is missing on the compiled path (`method-parity`) | stdlib parity / check | `wj check` must reject what rustc will (R5 check-vs-build gate); alias or implement the std method; `wj docs` entry | 6 | 1 | 12 | 116004 |
| 6 | P1 | untyped integer literal emitted as i32 and mixed with int (i64) (`int-width`) | codegen (literal defaulting) + pack tip | codegen integer-literal typing; docs/known-issues/agent-dx-compiled-untyped-int-accumulator.md; pack 'Compiled path gaps' | 7 | 2 | 9 | 87671 |
| 7 | P1 | slow `wj build` (compiled) >= 30s (`slow-command:build-compiled`) | performance | build cache / runtime pre-build; see wall-time table | 4 | 4 | 5 | 51676 |
| 8 | P1 | collect/iterator element type mismatch in generated Rust (`collect-type`) | codegen | iterator lowering (borrowed vs owned element types); known-issues agent-dx-compiled-str-literal-and-option-default-codegen.md | 3 | 2 | 3 | 29224 |
| 9 | P2 | Rust-leakage lint rejected idiomatic-looking code (`lint-leakage`) | lint rule | src/lint (rule text + `wj fix` autofix); docs/agent pack section 'Core idioms' | 2 | 1 | 2 | 19779 |
| 10 | P2 | unclassified failure: .N (`other:.N`) | diagnostic text (triage) | inspect example, then file under a rule | 1 | 1 | 1 | 10335 |

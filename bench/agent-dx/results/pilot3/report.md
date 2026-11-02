# Agent preference pilot

120 trials over 10 tasks. Intervals are 95% bootstrap CIs that resample tasks, then trials within a task. With this N they show direction, not significance, unless an interval excludes zero.

## Arms

| arm | name | trials | context pack |
|---|---|---|---|
| B | compiled-first | 30 | WINDJAMMER_FOR_AGENTS.md |
| E | tiny-pack+mcp | 30 | tiny_E.md |
| F | tiny-pack+wj-docs | 30 | tiny_F.md |
| T | tiny-pack-only | 30 | tiny_T.md |

## Arm means

| metric | B | E | F | T |
|---|---|---|---|---|
| pass@1 (compiled hidden oracle) | 0.967 [0.867, 1.000] | 0.967 [0.867, 1.000] | 0.900 [0.767, 1.000] | 0.867 [0.700, 1.000] |
| verify calls (wj check/run/test/build) | 2.3 [1.5, 3.1] | 2.4 [1.7, 3.3] | 1.8 [1.1, 2.6] | 3.0 [1.8, 4.9] |
| verify calls to green (passing trials) | 1.7 [1.1, 2.5] | 1.8 [1.2, 2.5] | 1.6 [1.0, 2.5] | 2.0 [1.2, 2.8] |
| agent turns | 3.8 [2.9, 5.2] | 5.4 [3.7, 7.6] | 4.1 [3.2, 5.5] | 4.2 [2.9, 5.7] |
| tokens in (incl. cache) | 44278.0 [35561.2, 57596.1] | 40284.4 [27903.9, 54761.5] | 29407.8 [20654.8, 40450.9] | 28951.1 [21316.1, 39132.1] |
| tokens out | 1294.1 [922.6, 1698.0] | 1671.3 [1174.7, 2191.9] | 1401.7 [1078.4, 1793.1] | 1462.4 [988.2, 1972.4] |
| cost USD | 0.048 [0.037, 0.060] | 0.043 [0.034, 0.052] | 0.040 [0.026, 0.056] | 0.043 [0.032, 0.058] |
| wall seconds | 156.0 [100.6, 222.7] | 166.0 [99.6, 248.3] | 128.1 [55.1, 221.2] | 173.3 [87.7, 295.6] |
| agent-thinking seconds (API time) | 19.4 [14.2, 25.4] | 19.7 [15.0, 25.1] | 17.5 [10.9, 25.0] | 20.2 [14.2, 28.2] |
| tool-wait seconds (wj commands) | 130.5 [70.0, 205.4] | 138.4 [64.3, 232.8] | 107.3 [26.2, 209.0] | 127.8 [53.7, 216.8] |
| harness permission denials (should be 0) | 0.1 [0.0, 0.3] | 0.1 [0.0, 0.3] | 0.2 [0.0, 0.5] | 0.2 [0.0, 0.4] |
| capability denials seen (WJ004x) | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] |
| silent divergence (interp pass, compiled fail) | 0.000 [0.000, 0.000] | 0.033 [0.000, 0.133] | 0.100 [0.000, 0.233] | 0.067 [0.000, 0.200] |

## Paired differences (task-paired, first minus second)

| metric | B-E | B-F | B-T | E-F | E-T | F-T |
|---|---|---|---|---|---|---|
| pass@1 (compiled hidden oracle) | 0.000 [-0.133, 0.133] | 0.067 [-0.100, 0.233] | 0.100 [-0.033, 0.267] | 0.067 [-0.067, 0.200] | 0.100 [-0.033, 0.267] | 0.033 [-0.167, 0.233] |
| verify calls (wj check/run/test/build) | -0.1 [-1.1, 0.9] | 0.4 [-0.6, 1.4] | -0.8 [-2.6, 0.6] | 0.6 [-0.3, 1.4] | -0.6 [-2.6, 0.8] | -1.2 [-3.1, 0.1] |
| verify calls to green (passing trials) | -0.1 [-0.8, 0.6] | 0.0 [-0.8, 0.8] | -0.3 [-1.2, 0.5] | 0.1 [-0.5, 0.7] | -0.2 [-0.9, 0.5] | -0.3 [-1.1, 0.4] |
| agent turns | -1.6 [-3.9, 0.6] | -0.3 [-1.9, 1.6] | -0.3 [-1.8, 1.3] | 1.3 [-0.2, 3.1] | 1.3 [-0.3, 3.1] | -0.1 [-1.5, 1.3] |
| tokens in (incl. cache) | 3993.6 [-12736.7, 22301.2] | 14870.2 [1251.4, 31502.0] | 15326.9 [3804.0, 29748.1] | 10876.6 [-1818.4, 23367.2] | 11333.4 [-1656.1, 25509.9] | 456.7 [-10243.6, 11756.2] |
| tokens out | -377.2 [-958.8, 157.1] | -107.6 [-556.8, 341.2] | -168.3 [-718.2, 321.7] | 269.6 [-120.3, 747.4] | 208.9 [-238.9, 711.4] | -60.7 [-560.1, 420.8] |
| cost USD | 0.005 [-0.008, 0.020] | 0.008 [-0.012, 0.024] | 0.005 [-0.011, 0.018] | 0.002 [-0.012, 0.013] | -0.001 [-0.017, 0.013] | -0.003 [-0.022, 0.017] |
| wall seconds | -10.0 [-112.7, 83.6] | 27.9 [-83.9, 117.7] | -17.3 [-125.1, 71.2] | 37.9 [-76.0, 135.3] | -7.4 [-141.4, 122.6] | -45.2 [-193.7, 94.5] |
| agent-thinking seconds (API time) | -0.2 [-7.6, 7.8] | 2.0 [-6.0, 9.5] | -0.7 [-9.7, 6.7] | 2.2 [-4.8, 8.7] | -0.5 [-10.3, 7.7] | -2.7 [-13.0, 6.1] |
| tool-wait seconds (wj commands) | -8.0 [-120.3, 96.7] | 23.2 [-104.3, 125.6] | 2.6 [-89.8, 88.6] | 31.2 [-107.7, 139.9] | 10.6 [-111.5, 137.4] | -20.6 [-154.2, 119.3] |
| harness permission denials (should be 0) | 0.0 [-0.1, 0.2] | -0.1 [-0.3, 0.2] | -0.1 [-0.3, 0.1] | -0.1 [-0.4, 0.1] | -0.1 [-0.3, 0.1] | 0.0 [-0.2, 0.2] |
| capability denials seen (WJ004x) | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] |
| silent divergence (interp pass, compiled fail) | -0.033 [-0.133, 0.000] | -0.100 [-0.267, 0.000] | -0.067 [-0.200, 0.000] | -0.067 [-0.200, 0.067] | -0.033 [-0.200, 0.100] | 0.033 [-0.167, 0.233] |

## Per-task pass rate (passes/trials)

| task | B | E | F | T |
|---|---|---|---|---|
| t003_prime_counts | 3/3 | 3/3 | 3/3 | 2/3 |
| t006_base_convert | 3/3 | 3/3 | 3/3 | 2/3 |
| t009_rpn_eval | 3/3 | 3/3 | 3/3 | 3/3 |
| t011_lru_cache | 3/3 | 2/3 | 2/3 | 2/3 |
| t013_bst | 3/3 | 3/3 | 3/3 | 3/3 |
| t018_word_freq | 3/3 | 3/3 | 3/3 | 3/3 |
| t022_csv_parse | 3/3 | 3/3 | 3/3 | 3/3 |
| t030_bank_ledger | 3/3 | 3/3 | 2/3 | 3/3 |
| t034_shapes_trait | 3/3 | 3/3 | 2/3 | 3/3 |
| t050_cat_number | 2/3 | 3/3 | 3/3 | 2/3 |

## Top failure categories (diagnostics hit, all verify calls)

| category | B | E | F | T |
|---|---|---|---|---|
| other | 17 | 21 | 14 | 27 |
| type-mismatch | 12 | 11 | 6 | 10 |
| trait-bound | 3 | 6 | 3 | 6 |
| ownership-borrow | 0 | 1 | 2 | 3 |
| lint-style | 1 | 4 | 0 | 1 |
| runtime-panic | 0 | 2 | 0 | 3 |
| syntax | 1 | 0 | 0 | 2 |
| name-resolution | 0 | 0 | 1 | 0 |

## Interpreter vs compiled divergence

16 of 120 trials had an interpreted hidden-oracle result different from the compiled one (6 silent: interpreter green, compiled red).
- t003_prime_counts arm T trial 1: interp=True compiled=False
- t006_base_convert arm T trial 1: interp=True compiled=False
- t011_lru_cache arm E trial 2: interp=True compiled=False
- t011_lru_cache arm F trial 0: interp=True compiled=False
- t030_bank_ledger arm F trial 1: interp=True compiled=False
- t034_shapes_trait arm F trial 2: interp=True compiled=False
- t050_cat_number arm B trial 1: interp=False compiled=True
- t050_cat_number arm B trial 2: interp=False compiled=True
- t050_cat_number arm E trial 0: interp=False compiled=True
- t050_cat_number arm E trial 1: interp=False compiled=True

## Context attribution (stream-json arms)

First-turn input = system prompt + tool definitions + pack + task prompt. Pack tokens are estimated as bytes/3.5. Schema tokens are the first-turn difference to the matching no-MCP arm (T), minus the pack difference.

| arm | first-turn input | pack tokens (est, bytes/3.5) | other fixed (system, tools, task) | tool-result tokens (est, all turns) | docs shell calls | MCP calls | total tokens in | turns |
|---|---|---|---|---|---|---|---|---|
| B compiled-first | 14504 | 4721 | 9783 | 874 | 0.0 | 0.0 | 44278 | 3.8 |
| E tiny-pack+mcp | 8661 | 441 | 8220 | 973 | 0.0 | 0.4 | 40284 | 5.4 |
| F tiny-pack+wj-docs | 8162 | 424 | 7738 | 1107 | 0.1 | 0.0 | 29408 | 4.1 |
| T tiny-pack-only | 8095 | 371 | 7724 | 851 | 0.0 | 0.0 | 28951 | 4.2 |

MCP tool-schema tokens (3 exposed tools): about 495 per turn-1 context.

## Paired differences versus control B (task-clustered 95% CI)

| metric | E - B | F - B | T - B |
|---|---|---|---|
| pass@1 (compiled hidden oracle) | 0.000 [-0.133, 0.133] | -0.067 [-0.233, 0.100] | -0.100 [-0.267, 0.033] |
| verify calls (wj check/run/test/build) | 0.1 [-0.8, 1.1] | -0.4 [-1.4, 0.6] | 0.8 [-0.6, 2.5] |
| verify calls to green (passing trials) | 0.1 [-0.6, 0.8] | -0.0 [-0.8, 0.8] | 0.3 [-0.5, 1.3] |
| agent turns | 1.6 [-0.5, 3.9] | 0.3 [-1.5, 1.9] | 0.3 [-1.1, 1.8] |
| tokens in (incl. cache) | -3993.6 [-21938.4, 11663.0] | -14870.2 [-30749.1, -1265.1] | -15326.9 [-29329.2, -4175.1] |
| tokens out | 377.2 [-166.2, 963.5] | 107.6 [-318.5, 550.1] | 168.3 [-305.2, 705.8] |
| cost USD | -0.005 [-0.019, 0.009] | -0.008 [-0.025, 0.013] | -0.005 [-0.018, 0.009] |
| wall seconds | 10.0 [-83.5, 116.3] | -27.9 [-123.7, 87.3] | 17.3 [-68.7, 113.2] |
| agent-thinking seconds (API time) | 0.2 [-7.5, 7.6] | -2.0 [-9.4, 6.3] | 0.7 [-6.4, 8.6] |
| tool-wait seconds (wj commands) | 8.0 [-96.1, 122.9] | -23.2 [-133.2, 103.3] | -2.6 [-91.7, 84.7] |
| harness permission denials (should be 0) | -0.0 [-0.2, 0.1] | 0.1 [-0.1, 0.3] | 0.1 [-0.1, 0.3] |
| capability denials seen (WJ004x) | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] |
| silent divergence (interp pass, compiled fail) | 0.033 [0.000, 0.133] | 0.100 [0.000, 0.267] | 0.067 [0.000, 0.200] |

Mode violations (agent used the other arm's loop commands): 2.

## Friction list (ranked by estimated tokens wasted)

| # | pri | friction | fix kind | fix location | trials | tasks | wasted calls | est. tokens |
|---|---|---|---|---|---|---|---|---|
| 1 | P0 | slow `wj run` (compiled) >= 30s (`slow-command:run-compiled`) | performance | build cache / runtime pre-build; see wall-time table | 54 | 10 | 75 | 711487 |
| 2 | P0 | type mismatch (often int/float, string/&str) (`type-mismatch`) | diagnostic text | show both types and the conversion (`as`, `.to_string()`); error-code explain entry | 28 | 8 | 40 | 393620 |
| 3 | P0 | slow `wj build` (compiled) >= 30s (`slow-command:build-compiled`) | performance | build cache / runtime pre-build; see wall-time table | 25 | 9 | 27 | 260584 |
| 4 | P1 | raw rustc/cargo error surfaced from generated code (`rustc-leak`) | codegen / diagnostic mapping | map rustc codes back to .wj spans; fix codegen so check-vs-build parity holds (R5) | 11 | 5 | 20 | 163178 |
| 5 | P1 | collect/iterator element type mismatch in generated Rust (`collect-type`) | codegen | iterator lowering (borrowed vs owned element types); known-issues agent-dx-compiled-str-literal-and-option-default-codegen.md | 11 | 4 | 12 | 108636 |
| 6 | P1 | interpreted run passed but compiled grade failed (silent divergence) (`silent-divergence`) | interpreter / check parity | R2/R5 gates in docs/design/interpreter-primary-readiness.md; add the program to the differential corpus | 6 | 5 | 7 | 56530 |
| 7 | P1 | Rust-leakage lint rejected idiomatic-looking code (`lint-leakage`) | lint rule | src/lint (rule text + `wj fix` autofix); docs/agent pack section 'Core idioms' | 6 | 3 | 6 | 53947 |
| 8 | P1 | runtime panic during the loop (`runtime-panic`) | docs | pack section on Option/Result handling and index bounds | 4 | 3 | 5 | 41076 |
| 9 | P2 | method accepted by the interpreter is missing on the compiled path (`method-parity`) | stdlib parity / check | `wj check` must reject what rustc will (R5 check-vs-build gate); alias or implement the std method; `wj docs` entry | 2 | 2 | 2 | 21283 |
| 10 | P2 | unresolved name (`name-unresolved`) | diagnostic text | diagnostic should list candidates and the import to add | 1 | 1 | 1 | 7832 |

# Agent preference pilot

210 trials over 10 tasks. Intervals are 95% bootstrap CIs that resample tasks, then trials within a task. With this N they show direction, not significance, unless an interval excludes zero.

## Arms

| arm | name | trials | context pack |
|---|---|---|---|
| S | sheet | 30 | sheet_full.md |
| Sa | sheet-minus-strings | 30 | sheet_minus_strings.md |
| Sb | sheet-minus-ownership | 30 | sheet_minus_ownership.md |
| Sc | sheet-minus-input | 30 | sheet_minus_input.md |
| Sd | sheet-minus-len | 30 | sheet_minus_len.md |
| Se | sheet-minus-printing | 30 | sheet_minus_printing.md |
| Sf | sheet-minus-tools | 30 | sheet_minus_tools.md |

## Arm means

| metric | S | Sa | Sb | Sc | Sd | Se | Sf |
|---|---|---|---|---|---|---|---|
| pass@1 (compiled hidden oracle) | 0.867 [0.700, 1.000] | 0.967 [0.867, 1.000] | 1.000 [1.000, 1.000] | 0.833 [0.633, 1.000] | 1.000 [1.000, 1.000] | 0.900 [0.767, 1.000] | 0.967 [0.867, 1.000] |
| verify calls (wj check/run/test/build) | 3.1 [2.0, 4.6] | 2.5 [1.9, 3.3] | 2.8 [2.0, 3.7] | 6.8 [5.5, 8.2] | 3.0 [2.1, 4.0] | 2.4 [1.7, 3.3] | 2.0 [1.4, 2.7] |
| verify calls to green (passing trials) | 2.6 [1.6, 3.6] | 2.0 [1.5, 2.7] | 2.1 [1.6, 2.8] | 5.4 [4.2, 6.8] | 2.5 [1.8, 3.4] | 1.9 [1.3, 2.7] | 1.7 [1.4, 2.1] |
| agent turns | 6.8 [4.6, 9.0] | 6.3 [4.3, 9.5] | 4.9 [3.7, 6.4] | 16.6 [11.1, 21.9] | 6.4 [4.8, 8.2] | 4.6 [3.4, 6.1] | 4.5 [3.3, 6.0] |
| tokens in (incl. cache) | 46718.7 [30906.7, 64956.5] | 46362.7 [28928.2, 73340.7] | 33711.4 [24947.0, 44974.2] | 197856.9 [135297.6, 262605.9] | 48072.7 [32285.8, 65917.0] | 33329.4 [24233.9, 45506.0] | 29676.0 [21828.5, 41022.5] |
| tokens out | 1912.0 [1284.0, 2574.8] | 1824.4 [1253.1, 2495.6] | 1510.6 [1014.3, 2098.3] | 2989.9 [1953.9, 3948.2] | 1871.2 [1317.0, 2563.8] | 1415.0 [1009.3, 1887.5] | 1540.5 [1084.7, 2091.2] |
| cost USD | 0.048 [0.037, 0.059] | 0.046 [0.034, 0.061] | 0.046 [0.033, 0.062] | 0.130 [0.118, 0.140] | 0.048 [0.034, 0.063] | 0.043 [0.034, 0.053] | 0.040 [0.030, 0.051] |
| wall seconds | 105.7 [60.9, 163.3] | 134.4 [57.8, 239.7] | 120.7 [66.4, 191.2] | 332.4 [236.1, 448.5] | 136.3 [59.4, 240.0] | 140.7 [64.3, 231.6] | 102.5 [54.6, 165.1] |
| agent-thinking seconds (API time) | 26.3 [18.9, 34.1] | 24.9 [16.9, 34.8] | 24.5 [17.6, 33.8] | 86.8 [76.7, 97.0] | 24.3 [17.9, 31.3] | 23.1 [17.7, 29.2] | 19.9 [14.5, 26.5] |
| tool-wait seconds (wj commands) | 72.1 [29.9, 127.2] | 110.0 [28.0, 228.7] | 94.9 [38.8, 168.8] | 228.4 [146.1, 310.0] | 112.6 [29.9, 234.7] | 94.5 [34.0, 173.2] | 77.9 [30.1, 139.2] |
| harness permission denials (should be 0) | 0.3 [0.1, 0.6] | 0.2 [0.0, 0.5] | 0.2 [0.0, 0.4] | 0.9 [0.5, 1.4] | 0.2 [0.0, 0.4] | 0.2 [0.0, 0.4] | 0.2 [0.0, 0.4] |
| capability denials seen (WJ004x) | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] |
| silent divergence (interp pass, compiled fail) | 0.100 [0.000, 0.267] | 0.033 [0.000, 0.133] | 0.000 [0.000, 0.000] | 0.000 [0.000, 0.000] | 0.000 [0.000, 0.000] | 0.100 [0.000, 0.233] | 0.000 [0.000, 0.000] |

## Paired differences (task-paired, first minus second)

| metric | S-Sa | S-Sb | S-Sc | S-Sd | S-Se | S-Sf | Sa-Sb | Sa-Sc | Sa-Sd | Sa-Se | Sa-Sf | Sb-Sc | Sb-Sd | Sb-Se | Sb-Sf | Sc-Sd | Sc-Se | Sc-Sf | Sd-Se | Sd-Sf | Se-Sf |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| pass@1 (compiled hidden oracle) | -0.100 [-0.300, 0.067] | -0.133 [-0.300, 0.000] | 0.033 [-0.167, 0.267] | -0.133 [-0.300, 0.000] | -0.033 [-0.233, 0.167] | -0.100 [-0.267, 0.033] | -0.033 [-0.133, 0.000] | 0.133 [-0.033, 0.367] | -0.033 [-0.133, 0.000] | 0.067 [-0.067, 0.200] | 0.000 [-0.133, 0.133] | 0.167 [0.000, 0.400] | 0.000 [0.000, 0.000] | 0.100 [0.000, 0.233] | 0.033 [0.000, 0.133] | -0.167 [-0.400, 0.000] | -0.067 [-0.300, 0.167] | -0.133 [-0.333, 0.000] | 0.100 [0.000, 0.233] | 0.033 [0.000, 0.133] | -0.067 [-0.233, 0.100] |
| verify calls (wj check/run/test/build) | 0.6 [-0.7, 2.0] | 0.3 [-1.0, 1.9] | -3.7 [-5.5, -2.0] | 0.1 [-1.3, 1.7] | 0.7 [-0.4, 1.9] | 1.1 [0.1, 2.3] | -0.2 [-0.9, 0.5] | -4.3 [-5.7, -2.8] | -0.4 [-1.8, 0.8] | 0.1 [-1.0, 1.2] | 0.5 [-0.5, 1.5] | -4.0 [-5.7, -2.4] | -0.2 [-1.7, 1.2] | 0.4 [-0.8, 1.5] | 0.7 [-0.3, 1.8] | 3.8 [2.4, 5.2] | 4.4 [2.8, 6.0] | 4.8 [3.1, 6.5] | 0.6 [-0.4, 1.6] | 0.9 [-0.1, 2.2] | 0.4 [-0.3, 1.2] |
| verify calls to green (passing trials) | 0.6 [-0.4, 1.7] | 0.5 [-0.5, 1.6] | -2.7 [-4.2, -1.4] | 0.1 [-1.1, 1.2] | 0.7 [-0.1, 1.7] | 1.0 [0.2, 1.8] | -0.1 [-0.8, 0.6] | -3.4 [-4.6, -2.1] | -0.5 [-1.6, 0.6] | 0.1 [-0.7, 0.9] | 0.3 [-0.2, 0.9] | -3.4 [-4.9, -2.2] | -0.4 [-1.5, 0.5] | 0.2 [-0.5, 0.9] | 0.3 [-0.2, 0.8] | 3.0 [1.9, 4.3] | 3.5 [2.2, 5.0] | 3.7 [2.6, 5.0] | 0.6 [-0.1, 1.5] | 0.7 [-0.1, 1.6] | 0.2 [-0.4, 0.9] |
| agent turns | 0.4 [-2.2, 2.8] | 1.8 [-0.6, 4.4] | -9.8 [-15.2, -4.5] | 0.4 [-1.8, 2.5] | 2.2 [0.2, 4.0] | 2.3 [0.4, 4.3] | 1.4 [-1.4, 5.0] | -10.2 [-15.5, -5.0] | -0.0 [-3.0, 2.9] | 1.7 [-0.7, 4.7] | 1.8 [-0.8, 5.0] | -11.6 [-16.9, -6.0] | -1.4 [-3.5, 0.5] | 0.3 [-1.8, 2.3] | 0.4 [-1.2, 2.2] | 10.2 [5.4, 15.3] | 12.0 [6.5, 17.8] | 12.1 [6.7, 17.7] | 1.8 [0.1, 3.7] | 1.9 [0.6, 3.3] | 0.1 [-1.4, 1.7] |
| tokens in (incl. cache) | 356.0 [-25244.8, 20391.3] | 13007.3 [-3891.3, 34310.7] | -151138.1 [-213599.4, -89512.1] | -1354.0 [-23299.0, 19242.0] | 13389.3 [-2579.7, 28947.6] | 17042.8 [2595.3, 35680.2] | 12651.3 [-7829.6, 38849.6] | -151494.2 [-212247.8, -92224.4] | -1710.0 [-31312.4, 26969.3] | 13033.3 [-9496.8, 39685.3] | 16686.7 [-5532.9, 42878.6] | -164145.5 [-228065.5, -98940.6] | -14361.3 [-33329.5, 2634.6] | 382.0 [-16283.8, 15507.2] | 4035.4 [-7344.9, 17168.4] | 149784.1 [92343.0, 209315.6] | 164527.5 [101868.8, 231798.0] | 168180.9 [106118.0, 235540.9] | 14743.3 [-2881.3, 33864.8] | 18396.8 [6391.5, 34429.1] | 3653.4 [-7608.9, 16403.9] |
| tokens out | 87.6 [-392.9, 533.3] | 401.4 [-155.2, 1016.5] | -1077.9 [-2120.2, -144.2] | 40.8 [-418.3, 434.9] | 497.0 [17.4, 1035.0] | 371.5 [-56.4, 862.0] | 313.8 [-326.1, 1104.2] | -1165.5 [-2209.3, -192.1] | -46.8 [-643.3, 519.9] | 409.4 [-82.9, 997.6] | 283.9 [-227.1, 949.6] | -1479.3 [-2547.0, -473.7] | -360.6 [-864.1, 91.0] | 95.5 [-455.9, 673.6] | -29.9 [-485.4, 424.8] | 1118.7 [239.8, 2139.5] | 1574.8 [453.9, 2760.6] | 1449.4 [467.0, 2473.5] | 456.1 [-10.4, 1041.1] | 330.7 [-21.1, 717.5] | -125.5 [-614.9, 312.7] |
| cost USD | 0.002 [-0.010, 0.013] | 0.002 [-0.014, 0.016] | -0.082 [-0.097, -0.065] | -0.000 [-0.013, 0.011] | 0.005 [-0.004, 0.013] | 0.008 [-0.000, 0.016] | 0.000 [-0.010, 0.012] | -0.084 [-0.100, -0.067] | -0.002 [-0.020, 0.014] | 0.003 [-0.009, 0.016] | 0.006 [-0.003, 0.015] | -0.084 [-0.101, -0.065] | -0.002 [-0.020, 0.016] | 0.003 [-0.011, 0.019] | 0.006 [-0.004, 0.017] | 0.081 [0.061, 0.097] | 0.087 [0.071, 0.101] | 0.089 [0.073, 0.104] | 0.006 [-0.006, 0.018] | 0.008 [-0.003, 0.022] | 0.003 [-0.006, 0.011] |
| wall seconds | -28.7 [-119.3, 46.7] | -15.0 [-103.8, 60.8] | -226.7 [-375.3, -103.1] | -30.6 [-122.0, 37.7] | -35.0 [-116.3, 31.5] | 3.2 [-69.3, 76.3] | 13.7 [-70.0, 101.6] | -198.0 [-355.0, -61.4] | -1.9 [-111.6, 93.4] | -6.3 [-118.8, 97.4] | 31.9 [-45.3, 127.5] | -211.7 [-358.9, -101.5] | -15.6 [-134.5, 88.2] | -20.0 [-132.4, 83.4] | 18.2 [-46.2, 89.4] | 196.1 [39.1, 355.1] | 191.7 [39.8, 349.0] | 229.9 [104.0, 373.6] | -4.4 [-86.5, 80.0] | 33.8 [-53.6, 143.7] | 38.2 [-47.0, 139.0] |
| agent-thinking seconds (API time) | 1.5 [-8.0, 10.4] | 1.9 [-9.9, 12.6] | -60.5 [-73.3, -47.4] | 2.0 [-4.9, 9.2] | 3.2 [-3.6, 10.0] | 6.4 [-0.0, 13.2] | 0.4 [-7.0, 8.9] | -61.9 [-74.6, -49.8] | 0.5 [-9.9, 11.3] | 1.8 [-7.2, 11.4] | 4.9 [-1.6, 11.8] | -62.4 [-75.7, -48.3] | 0.1 [-10.4, 11.5] | 1.4 [-8.0, 11.8] | 4.5 [-2.4, 12.3] | 62.5 [49.4, 74.2] | 63.7 [51.5, 75.3] | 66.9 [53.7, 79.0] | 1.3 [-5.9, 8.5] | 4.4 [-2.5, 11.6] | 3.2 [-3.3, 9.7] |
| tool-wait seconds (wj commands) | -37.8 [-137.5, 44.7] | -22.8 [-110.3, 50.9] | -156.2 [-255.9, -57.2] | -40.5 [-145.4, 34.3] | -22.3 [-95.1, 41.6] | -5.8 [-79.0, 66.9] | 15.0 [-70.7, 110.6] | -118.4 [-217.4, 5.7] | -2.7 [-129.0, 105.2] | 15.5 [-87.5, 117.4] | 32.0 [-53.9, 142.4] | -133.5 [-223.5, -46.9] | -17.7 [-151.8, 92.4] | 0.4 [-93.5, 88.8] | 17.0 [-50.3, 91.6] | 115.8 [-30.9, 236.6] | 133.9 [4.2, 245.4] | 150.4 [41.8, 250.6] | 18.1 [-73.3, 128.7] | 34.7 [-56.8, 156.8] | 16.5 [-53.2, 96.5] |
| harness permission denials (should be 0) | 0.1 [-0.1, 0.2] | 0.1 [-0.2, 0.4] | -0.6 [-1.2, -0.2] | 0.1 [-0.1, 0.3] | 0.1 [-0.1, 0.3] | 0.1 [-0.2, 0.4] | 0.0 [-0.3, 0.4] | -0.7 [-1.1, -0.3] | 0.0 [-0.2, 0.3] | 0.0 [-0.2, 0.3] | 0.0 [-0.2, 0.3] | -0.7 [-1.2, -0.3] | 0.0 [-0.3, 0.3] | 0.0 [-0.3, 0.3] | 0.0 [-0.3, 0.3] | 0.7 [0.3, 1.2] | 0.7 [0.3, 1.2] | 0.7 [0.4, 1.1] | 0.0 [-0.2, 0.2] | 0.0 [-0.2, 0.2] | 0.0 [-0.3, 0.2] |
| capability denials seen (WJ004x) | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] |
| silent divergence (interp pass, compiled fail) | 0.067 [-0.100, 0.233] | 0.100 [0.000, 0.267] | 0.100 [0.000, 0.267] | 0.100 [0.000, 0.267] | 0.000 [-0.200, 0.200] | 0.100 [0.000, 0.267] | 0.033 [0.000, 0.133] | 0.033 [0.000, 0.133] | 0.033 [0.000, 0.133] | -0.067 [-0.200, 0.067] | 0.033 [0.000, 0.133] | 0.000 [0.000, 0.000] | 0.000 [0.000, 0.000] | -0.100 [-0.233, 0.000] | 0.000 [0.000, 0.000] | 0.000 [0.000, 0.000] | -0.100 [-0.233, 0.000] | 0.000 [0.000, 0.000] | -0.100 [-0.233, 0.000] | 0.000 [0.000, 0.000] | 0.100 [0.000, 0.233] |

## Per-task pass rate (passes/trials)

| task | S | Sa | Sb | Sc | Sd | Se | Sf |
|---|---|---|---|---|---|---|---|
| t003_prime_counts | 3/3 | 3/3 | 3/3 | 3/3 | 3/3 | 3/3 | 3/3 |
| t006_base_convert | 3/3 | 3/3 | 3/3 | 3/3 | 3/3 | 2/3 | 3/3 |
| t009_rpn_eval | 3/3 | 3/3 | 3/3 | 3/3 | 3/3 | 3/3 | 3/3 |
| t011_lru_cache | 2/3 | 3/3 | 3/3 | 3/3 | 3/3 | 2/3 | 3/3 |
| t013_bst | 3/3 | 3/3 | 3/3 | 3/3 | 3/3 | 3/3 | 3/3 |
| t018_word_freq | 3/3 | 2/3 | 3/3 | 2/3 | 3/3 | 2/3 | 3/3 |
| t022_csv_parse | 3/3 | 3/3 | 3/3 | 3/3 | 3/3 | 3/3 | 3/3 |
| t030_bank_ledger | 2/3 | 3/3 | 3/3 | 1/3 | 3/3 | 3/3 | 3/3 |
| t034_shapes_trait | 2/3 | 3/3 | 3/3 | 3/3 | 3/3 | 3/3 | 3/3 |
| t050_cat_number | 2/3 | 3/3 | 3/3 | 1/3 | 3/3 | 3/3 | 2/3 |

## Top failure categories (diagnostics hit, all verify calls)

| category | S | Sa | Sb | Sc | Sd | Se | Sf |
|---|---|---|---|---|---|---|---|
| other | 38 | 26 | 30 | 110 | 32 | 23 | 19 |
| type-mismatch | 20 | 9 | 15 | 18 | 21 | 14 | 7 |
| trait-bound | 11 | 4 | 3 | 54 | 13 | 5 | 4 |
| lint-style | 5 | 1 | 18 | 15 | 1 | 2 | 6 |
| runtime-panic | 1 | 1 | 0 | 1 | 6 | 1 | 1 |
| name-resolution | 0 | 0 | 0 | 11 | 0 | 0 | 0 |
| ownership-borrow | 0 | 1 | 0 | 2 | 0 | 0 | 1 |
| syntax | 0 | 2 | 0 | 0 | 0 | 0 | 0 |

## Interpreter vs compiled divergence

24/210 trials had an interpreted hidden-oracle result different from the compiled one (7 silent: interpreter green, compiled red).
- t006_base_convert arm Se trial 2: interp=True compiled=False
- t011_lru_cache arm S trial 2: interp=True compiled=False
- t011_lru_cache arm Se trial 2: interp=True compiled=False
- t018_word_freq arm Sa trial 2: interp=True compiled=False
- t018_word_freq arm Se trial 1: interp=True compiled=False
- t030_bank_ledger arm S trial 2: interp=True compiled=False
- t034_shapes_trait arm S trial 1: interp=True compiled=False
- t050_cat_number arm S trial 0: interp=False compiled=True
- t050_cat_number arm S trial 2: interp=False compiled=True
- t050_cat_number arm Sa trial 0: interp=False compiled=True

## Context attribution (stream-json arms)

First-turn input = system prompt + tool definitions + pack + task prompt. Pack tokens are estimated as bytes/3.5. Schema tokens are the first-turn difference to the matching no-MCP arm (T), minus the pack difference.

| arm | first-turn input | pack tokens (est, bytes/3.5) | other fixed (system, tools, task) | tool-result tokens (est, all turns) | docs shell calls | MCP calls | total tokens in | turns |
|---|---|---|---|---|---|---|---|---|
| S sheet | 8189 | 428 | 7761 | 1220 | 0.1 | 0.0 | 46719 | 6.8 |
| Sa sheet-minus-strings | 8138 | 391 | 7747 | 1044 | 0.5 | 0.0 | 46363 | 6.3 |
| Sb sheet-minus-ownership | 8103 | 379 | 7724 | 1143 | 0.1 | 0.0 | 33711 | 4.9 |
| Sc sheet-minus-input | 8122 | 385 | 7736 | 4758 | 26.5 | 0.0 | 197857 | 16.6 |
| Sd sheet-minus-len | 8146 | 400 | 7746 | 1429 | 0.3 | 0.0 | 48073 | 6.4 |
| Se sheet-minus-printing | 8145 | 403 | 7742 | 893 | 0.4 | 0.0 | 33329 | 4.6 |
| Sf sheet-minus-tools | 8140 | 390 | 7750 | 964 | 0.0 | 0.0 | 29676 | 4.5 |

## Paired differences versus control S (task-clustered 95% CI)

| metric | Sa - S | Sb - S | Sc - S | Sd - S | Se - S | Sf - S |
|---|---|---|---|---|---|---|
| pass@1 (compiled hidden oracle) | 0.100 [-0.067, 0.300] | 0.133 [0.000, 0.300] | -0.033 [-0.233, 0.167] | 0.133 [0.000, 0.300] | 0.033 [-0.167, 0.233] | 0.100 [-0.033, 0.267] |
| verify calls (wj check/run/test/build) | -0.6 [-2.0, 0.6] | -0.3 [-1.8, 1.0] | 3.7 [2.0, 5.5] | -0.1 [-1.8, 1.3] | -0.7 [-2.0, 0.4] | -1.1 [-2.3, -0.1] |
| verify calls to green (passing trials) | -0.6 [-1.6, 0.4] | -0.5 [-1.6, 0.6] | 2.7 [1.3, 4.3] | -0.1 [-1.2, 1.2] | -0.7 [-1.7, 0.1] | -1.0 [-1.9, -0.2] |
| agent turns | -0.4 [-2.8, 2.0] | -1.8 [-4.6, 0.6] | 9.8 [4.6, 15.5] | -0.4 [-2.6, 1.8] | -2.2 [-4.1, -0.3] | -2.3 [-4.4, -0.4] |
| tokens in (incl. cache) | -356.0 [-20361.9, 23233.9] | -13007.3 [-33572.9, 3820.8] | 151138.1 [92535.0, 213201.9] | 1354.0 [-19266.4, 22371.9] | -13389.3 [-29670.9, 3353.8] | -17042.8 [-35559.6, -2069.5] |
| tokens out | -87.6 [-532.3, 367.0] | -401.4 [-1031.4, 129.2] | 1077.9 [146.4, 2128.5] | -40.8 [-455.1, 394.1] | -497.0 [-1022.5, -35.7] | -371.5 [-855.8, 62.3] |
| cost USD | -0.002 [-0.012, 0.010] | -0.002 [-0.016, 0.013] | 0.082 [0.065, 0.097] | 0.000 [-0.011, 0.013] | -0.005 [-0.013, 0.004] | -0.008 [-0.016, -0.000] |
| wall seconds | 28.7 [-45.5, 123.5] | 15.0 [-63.9, 100.8] | 226.7 [105.1, 371.7] | 30.6 [-35.9, 115.3] | 35.0 [-33.2, 115.8] | -3.2 [-75.8, 73.5] |
| agent-thinking seconds (API time) | -1.5 [-10.1, 8.0] | -1.9 [-12.9, 9.8] | 60.5 [46.8, 73.7] | -2.0 [-9.1, 5.1] | -3.2 [-10.0, 4.0] | -6.4 [-13.3, 0.3] |
| tool-wait seconds (wj commands) | 37.8 [-40.0, 144.3] | 22.8 [-54.5, 107.5] | 156.2 [58.4, 256.2] | 40.5 [-29.6, 137.5] | 22.3 [-42.6, 93.1] | 5.8 [-66.3, 81.0] |
| harness permission denials (should be 0) | -0.1 [-0.2, 0.2] | -0.1 [-0.4, 0.2] | 0.6 [0.2, 1.2] | -0.1 [-0.3, 0.1] | -0.1 [-0.3, 0.1] | -0.1 [-0.4, 0.2] |
| capability denials seen (WJ004x) | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] |
| silent divergence (interp pass, compiled fail) | -0.067 [-0.233, 0.100] | -0.100 [-0.233, 0.000] | -0.100 [-0.233, 0.000] | -0.100 [-0.233, 0.000] | 0.000 [-0.167, 0.167] | -0.100 [-0.233, 0.000] |

Mode violations (agent used the other arm's loop commands): 15.

## Friction list (ranked by estimated tokens wasted)

| # | pri | friction | fix kind | fix location | trials | tasks | wasted calls | est. tokens |
|---|---|---|---|---|---|---|---|---|
| 1 | P0 | type mismatch (often int/float, string/&str) (`type-mismatch`) | diagnostic text | show both types and the conversion (`as`, `.to_string()`); error-code explain entry | 63 | 9 | 95 | 829798 |
| 2 | P0 | slow `wj build` (compiled) >= 30s (`slow-command:build-compiled`) | performance | build cache / runtime pre-build; see wall-time table | 56 | 10 | 63 | 625733 |
| 3 | P0 | method accepted by the interpreter is missing on the compiled path (`method-parity`) | stdlib parity / check | `wj check` must reject what rustc will (R5 check-vs-build gate); alias or implement the std method; `wj docs` entry | 22 | 9 | 43 | 616292 |
| 4 | P1 | raw rustc/cargo error surfaced from generated code (`rustc-leak`) | codegen / diagnostic mapping | map rustc codes back to .wj spans; fix codegen so check-vs-build parity holds (R5) | 40 | 10 | 57 | 609265 |
| 5 | P1 | slow `wj run` (compiled) >= 30s (`slow-command:run-compiled`) | performance | build cache / runtime pre-build; see wall-time table | 38 | 10 | 50 | 507971 |
| 6 | P1 | Rust-leakage lint rejected idiomatic-looking code (`lint-leakage`) | lint rule | src/lint (rule text + `wj fix` autofix); docs/agent pack section 'Core idioms' | 39 | 9 | 41 | 425645 |
| 7 | P1 | stdin API differs between interpreter and compiled path (`stdin-api`) | stdlib API | std::io unification; pack section 'Reading input' | 9 | 7 | 20 | 296316 |
| 8 | P1 | collect/iterator element type mismatch in generated Rust (`collect-type`) | codegen | iterator lowering (borrowed vs owned element types); known-issues agent-dx-compiled-str-literal-and-option-default-codegen.md | 27 | 6 | 32 | 264107 |
| 9 | P2 | runtime panic during the loop (`runtime-panic`) | docs | pack section on Option/Result handling and index bounds | 10 | 4 | 11 | 92238 |
| 10 | P2 | interpreted run passed but compiled grade failed (silent divergence) (`silent-divergence`) | interpreter / check parity | R2/R5 gates in docs/design/interpreter-primary-readiness.md; add the program to the differential corpus | 7 | 5 | 9 | 69858 |

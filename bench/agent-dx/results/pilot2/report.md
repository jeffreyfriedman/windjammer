# Agent preference pilot

210 trials over 10 tasks. Intervals are 95% bootstrap CIs that resample tasks, then trials within a task. With this N they show direction, not significance, unless an interval excludes zero.

## Arms

| arm | name | trials | context pack |
|---|---|---|---|
| A | interpreted-first | 30 | yes |
| B | compiled-first | 30 | yes |
| C | no-pack | 30 | no |
| D | interpreted+final-build | 30 | yes |
| G | go | 30 | no |
| P | python | 30 | no |
| R | rust | 30 | no |

## Arm means

| metric | A | B | C | D | G | P | R |
|---|---|---|---|---|---|---|---|
| pass@1 (compiled hidden oracle) | 0.400 [0.167, 0.633] | 0.867 [0.700, 1.000] | 0.200 [0.067, 0.367] | 0.833 [0.633, 1.000] | 1.000 [1.000, 1.000] | 1.000 [1.000, 1.000] | 1.000 [1.000, 1.000] |
| verify calls (wj check/run/test/build) | 1.8 [1.4, 2.3] | 2.1 [1.6, 3.0] | 6.9 [5.5, 8.6] | 3.7 [2.9, 4.7] | 0.3 [0.1, 0.6] | 0.0 [0.0, 0.0] | 1.0 [0.9, 1.0] |
| verify calls to green (passing trials) | 1.4 [1.0, 2.2] | 2.0 [1.4, 3.1] | 4.3 [3.0, 5.8] | 3.0 [2.0, 4.3] | 1.0 [1.0, 1.0] | n/a | 1.0 [1.0, 1.0] |
| agent turns | 3.3 [3.0, 4.0] | 5.0 [4.1, 6.1] | 21.9 [18.4, 24.8] | 3.6 [2.2, 5.1] | 2.6 [2.3, 2.8] | 3.6 [2.8, 4.4] | 3.0 [3.0, 3.0] |
| tokens in (incl. cache) | 34389.2 [29609.1, 43083.3] | 51550.1 [39648.9, 68322.9] | 229448.3 [186375.7, 270015.1] | 54408.2 [33737.4, 77557.6] | 18332.5 [16441.3, 20427.1] | 23548.6 [19234.3, 28245.3] | 17101.5 [15989.8, 18537.6] |
| tokens out | 1388.4 [1063.6, 1772.3] | 1717.8 [1309.8, 2172.5] | 3903.0 [3162.7, 4513.1] | 665.2 [358.9, 1019.5] | 1069.5 [883.6, 1260.8] | 940.8 [789.3, 1103.1] | 1190.7 [940.0, 1484.2] |
| cost USD | 0.028 [0.023, 0.036] | 0.038 [0.030, 0.047] | 0.118 [0.104, 0.131] | 0.055 [0.046, 0.064] | 0.021 [0.018, 0.024] | 0.019 [0.017, 0.022] | 0.022 [0.018, 0.026] |
| wall seconds | 27.8 [22.8, 32.9] | 117.8 [73.0, 190.6] | 147.5 [92.8, 241.6] | 344.9 [295.6, 400.6] | 25.1 [18.7, 32.4] | 86.4 [43.8, 130.2] | 27.6 [22.6, 32.8] |
| agent-thinking seconds (API time) | 12.5 [9.6, 15.5] | 17.6 [13.8, 22.7] | 65.6 [54.5, 76.4] | 32.1 [23.8, 43.5] | 10.5 [9.1, 12.0] | 11.1 [8.5, 14.1] | 10.3 [8.2, 12.3] |
| tool-wait seconds (wj commands) | 0.6 [0.4, 0.8] | 98.4 [41.2, 196.8] | 2.7 [2.0, 3.4] | 326.4 [277.6, 381.7] | 1.6 [0.5, 2.8] | 0.0 [0.0, 0.0] | 3.1 [1.5, 5.0] |
| harness permission denials (should be 0) | 0.1 [0.0, 0.2] | 0.1 [0.0, 0.4] | 1.2 [0.9, 1.4] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.1] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.1] |
| capability denials seen (WJ004x) | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] |
| silent divergence (interp pass, compiled fail) | 0.567 [0.333, 0.800] | 0.100 [0.000, 0.233] | 0.667 [0.433, 0.867] | 0.133 [0.000, 0.333] | 0.000 [0.000, 0.000] | 0.000 [0.000, 0.000] | 0.000 [0.000, 0.000] |

## Paired differences (task-paired, first minus second)

| metric | A-B | A-C | A-D | A-G | A-P | A-R | B-C | B-D | B-G | B-P | B-R | C-D | C-G | C-P | C-R | D-G | D-P | D-R | G-P | G-R | P-R |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| pass@1 (compiled hidden oracle) | -0.467 [-0.733, -0.167] | 0.200 [-0.067, 0.467] | -0.433 [-0.667, -0.200] | -0.600 [-0.800, -0.367] | -0.600 [-0.800, -0.367] | -0.600 [-0.800, -0.367] | 0.667 [0.433, 0.867] | 0.033 [-0.167, 0.267] | -0.133 [-0.300, 0.000] | -0.133 [-0.300, 0.000] | -0.133 [-0.300, 0.000] | -0.633 [-0.800, -0.433] | -0.800 [-0.933, -0.600] | -0.800 [-0.933, -0.600] | -0.800 [-0.933, -0.600] | -0.167 [-0.400, 0.000] | -0.167 [-0.400, 0.000] | -0.167 [-0.400, 0.000] | 0.000 [0.000, 0.000] | 0.000 [0.000, 0.000] | 0.000 [0.000, 0.000] |
| verify calls (wj check/run/test/build) | -0.3 [-1.1, 0.3] | -5.1 [-6.6, -3.7] | -1.9 [-2.8, -1.1] | 1.5 [1.1, 2.0] | 1.8 [1.5, 2.3] | 0.9 [0.5, 1.3] | -4.8 [-6.5, -3.4] | -1.6 [-2.5, -0.8] | 1.8 [1.3, 2.7] | 2.1 [1.6, 3.0] | 1.1 [0.6, 2.1] | 3.2 [1.6, 5.1] | 6.6 [5.0, 8.3] | 6.9 [5.4, 8.7] | 6.0 [4.4, 7.7] | 3.4 [2.5, 4.4] | 3.7 [2.9, 4.7] | 2.8 [1.9, 3.8] | 0.3 [0.1, 0.5] | -0.6 [-0.8, -0.4] | -1.0 [-1.0, -0.9] |
| verify calls to green (passing trials) | -0.5 [-1.5, 0.2] | -3.4 [-5.2, -1.8] | -1.0 [-2.0, -0.1] | 0.0 [0.0, 0.0] | n/a | 0.4 [0.0, 1.3] | -2.8 [-4.7, -1.3] | -1.0 [-2.1, -0.1] | 0.8 [0.4, 1.1] | n/a | 1.0 [0.4, 2.1] | 2.1 [-0.1, 4.3] | 3.2 [1.5, 5.8] | n/a | 3.3 [2.0, 4.8] | 2.4 [1.2, 3.9] | n/a | 2.0 [1.0, 3.3] | n/a | 0.0 [0.0, 0.0] | n/a |
| agent turns | -1.7 [-2.8, -0.7] | -18.6 [-21.6, -15.0] | -0.3 [-2.0, 1.3] | 0.7 [0.2, 1.6] | -0.3 [-1.3, 0.9] | 0.3 [0.0, 1.0] | -16.9 [-20.0, -13.1] | 1.5 [-0.1, 3.0] | 2.5 [1.6, 3.5] | 1.4 [0.1, 2.9] | 2.0 [1.1, 3.2] | 18.4 [14.6, 21.5] | 19.4 [15.8, 22.4] | 18.3 [14.5, 21.5] | 18.9 [15.3, 21.9] | 1.0 [-0.3, 2.4] | -0.0 [-1.5, 1.3] | 0.6 [-0.8, 2.1] | -1.0 [-1.8, -0.3] | -0.4 [-0.7, -0.2] | 0.6 [-0.2, 1.4] |
| tokens in (incl. cache) | -17160.9 [-31341.8, -6915.9] | -195059.1 [-230554.8, -151888.4] | -20019.0 [-45814.6, 2276.7] | 16056.7 [9917.9, 25613.1] | 10840.6 [3151.3, 20977.9] | 17287.7 [12549.4, 25551.6] | -177898.2 [-212261.1, -136821.2] | -2858.1 [-27574.4, 21815.8] | 33217.6 [21452.8, 51618.0] | 28001.5 [14736.6, 46565.2] | 34448.6 [22449.2, 52645.9] | 175040.1 [128434.0, 220253.4] | 211115.8 [167147.6, 250410.8] | 205899.7 [159932.8, 246283.9] | 212346.8 [168038.2, 251486.2] | 36075.7 [16841.7, 57781.6] | 30859.6 [11481.6, 52138.8] | 37306.7 [17067.9, 60629.8] | -5216.1 [-9836.5, -455.7] | 1231.0 [-1500.8, 3838.5] | 6447.1 [1327.9, 11786.6] |
| tokens out | -329.4 [-535.5, -141.6] | -2514.6 [-3085.3, -1879.3] | 723.2 [234.4, 1199.3] | 318.9 [126.6, 562.0] | 447.6 [183.5, 723.9] | 197.7 [58.3, 339.6] | -2185.2 [-2789.6, -1501.6] | 1052.6 [532.8, 1600.4] | 648.3 [369.6, 1019.7] | 777.0 [429.2, 1186.5] | 527.1 [307.6, 798.6] | 3237.8 [2478.5, 3929.1] | 2833.5 [2195.7, 3435.9] | 2962.2 [2285.1, 3573.8] | 2712.3 [2069.6, 3306.0] | -404.3 [-744.0, -9.4] | -275.6 [-583.3, 81.3] | -525.5 [-915.1, -79.7] | 128.8 [-42.3, 293.2] | -121.2 [-267.1, -16.6] | -250.0 [-447.6, -55.1] |
| cost USD | -0.009 [-0.015, -0.005] | -0.089 [-0.102, -0.078] | -0.026 [-0.034, -0.019] | 0.008 [0.003, 0.014] | 0.009 [0.004, 0.016] | 0.007 [0.003, 0.011] | -0.080 [-0.091, -0.069] | -0.017 [-0.025, -0.009] | 0.017 [0.011, 0.025] | 0.019 [0.012, 0.027] | 0.016 [0.011, 0.023] | 0.063 [0.049, 0.078] | 0.097 [0.085, 0.109] | 0.099 [0.085, 0.112] | 0.096 [0.084, 0.108] | 0.034 [0.027, 0.043] | 0.036 [0.028, 0.044] | 0.033 [0.026, 0.041] | 0.002 [-0.001, 0.005] | -0.001 [-0.003, 0.001] | -0.003 [-0.006, 0.001] |
| wall seconds | -90.0 [-169.0, -45.4] | -119.8 [-215.0, -62.4] | -317.1 [-375.5, -264.2] | 2.6 [-7.6, 12.2] | -58.6 [-104.4, -12.9] | 0.1 [-6.5, 7.4] | -29.8 [-145.8, 75.4] | -227.1 [-303.8, -136.2] | 92.6 [45.9, 176.3] | 31.4 [-41.8, 133.7] | 90.1 [43.6, 173.5] | -197.4 [-282.4, -94.1] | 122.4 [67.6, 214.4] | 61.2 [-13.8, 154.1] | 119.9 [62.8, 213.6] | 319.8 [268.8, 374.9] | 258.5 [203.1, 320.0] | 317.3 [267.0, 371.6] | -61.2 [-104.6, -18.6] | -2.5 [-11.4, 7.1] | 58.7 [15.0, 103.6] |
| agent-thinking seconds (API time) | -5.1 [-9.3, -1.3] | -53.1 [-63.6, -41.9] | -19.6 [-31.9, -11.0] | 2.0 [-1.2, 5.3] | 1.4 [-2.7, 5.4] | 2.2 [-0.3, 5.0] | -48.0 [-58.6, -37.3] | -14.5 [-26.3, -6.2] | 7.1 [3.1, 12.6] | 6.5 [2.5, 11.5] | 7.3 [3.9, 11.9] | 33.5 [19.5, 46.8] | 55.1 [43.3, 66.5] | 54.5 [42.9, 66.0] | 55.3 [43.1, 67.2] | 21.6 [13.8, 33.4] | 21.0 [13.8, 31.3] | 21.8 [13.8, 34.4] | -0.6 [-3.7, 2.3] | 0.2 [-2.3, 2.6] | 0.8 [-2.5, 4.6] |
| tool-wait seconds (wj commands) | -97.8 [-204.1, -39.4] | -2.1 [-2.9, -1.4] | -325.8 [-381.0, -277.1] | -1.1 [-2.3, 0.0] | 0.6 [0.4, 0.8] | -2.5 [-4.4, -0.8] | 95.7 [37.9, 207.7] | -228.0 [-314.7, -106.4] | 96.8 [38.8, 209.2] | 98.4 [40.4, 210.6] | 95.3 [37.0, 208.2] | -323.7 [-378.9, -274.9] | 1.0 [-0.3, 2.4] | 2.7 [2.0, 3.4] | -0.4 [-2.7, 1.6] | 324.7 [274.0, 374.7] | 326.4 [275.6, 376.1] | 323.3 [272.7, 372.6] | 1.6 [0.6, 2.9] | -1.5 [-3.8, 0.8] | -3.1 [-4.9, -1.4] |
| harness permission denials (should be 0) | -0.1 [-0.2, 0.0] | -1.1 [-1.4, -0.8] | 0.1 [0.0, 0.2] | 0.0 [-0.0, 0.2] | 0.1 [0.0, 0.2] | 0.0 [-0.0, 0.2] | -1.0 [-1.3, -0.7] | 0.1 [0.0, 0.4] | 0.1 [0.0, 0.3] | 0.1 [0.0, 0.4] | 0.1 [0.0, 0.3] | 1.2 [0.9, 1.5] | 1.1 [0.8, 1.4] | 1.2 [0.9, 1.5] | 1.1 [0.8, 1.4] | -0.0 [-0.1, 0.0] | 0.0 [0.0, 0.0] | -0.0 [-0.1, 0.0] | 0.0 [0.0, 0.1] | 0.0 [-0.1, 0.1] | -0.0 [-0.1, 0.0] |
| capability denials seen (WJ004x) | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] | 0.0 [0.0, 0.0] |
| silent divergence (interp pass, compiled fail) | 0.467 [0.200, 0.733] | -0.100 [-0.367, 0.167] | 0.433 [0.233, 0.667] | 0.567 [0.333, 0.800] | 0.567 [0.333, 0.800] | 0.567 [0.333, 0.800] | -0.567 [-0.800, -0.300] | -0.033 [-0.267, 0.167] | 0.100 [0.000, 0.233] | 0.100 [0.000, 0.233] | 0.100 [0.000, 0.233] | 0.533 [0.300, 0.733] | 0.667 [0.433, 0.867] | 0.667 [0.433, 0.867] | 0.667 [0.433, 0.867] | 0.133 [0.000, 0.333] | 0.133 [0.000, 0.333] | 0.133 [0.000, 0.333] | 0.000 [0.000, 0.000] | 0.000 [0.000, 0.000] | 0.000 [0.000, 0.000] |

## Per-task pass rate (passes/trials)

| task | A | B | C | D | G | P | R |
|---|---|---|---|---|---|---|---|
| t003_prime_counts | 2/3 | 3/3 | 1/3 | 3/3 | 3/3 | 3/3 | 3/3 |
| t006_base_convert | 0/3 | 3/3 | 0/3 | 1/3 | 3/3 | 3/3 | 3/3 |
| t009_rpn_eval | 1/3 | 2/3 | 1/3 | 3/3 | 3/3 | 3/3 | 3/3 |
| t011_lru_cache | 2/3 | 3/3 | 0/3 | 3/3 | 3/3 | 3/3 | 3/3 |
| t013_bst | 0/3 | 3/3 | 1/3 | 3/3 | 3/3 | 3/3 | 3/3 |
| t018_word_freq | 0/3 | 2/3 | 0/3 | 1/3 | 3/3 | 3/3 | 3/3 |
| t022_csv_parse | 2/3 | 3/3 | 1/3 | 3/3 | 3/3 | 3/3 | 3/3 |
| t030_bank_ledger | 1/3 | 3/3 | 1/3 | 3/3 | 3/3 | 3/3 | 3/3 |
| t034_shapes_trait | 2/3 | 2/3 | 1/3 | 3/3 | 3/3 | 3/3 | 3/3 |
| t050_cat_number | 2/3 | 2/3 | 0/3 | 2/3 | 3/3 | 3/3 | 3/3 |

## Top failure categories (diagnostics hit, all verify calls)

| category | A | B | C | D | G | P | R |
|---|---|---|---|---|---|---|---|
| other | 2 | 22 | 58 | 35 | 0 | 0 | 0 |
| type-mismatch | 0 | 17 | 0 | 28 | 0 | 0 | 0 |
| lint-style | 0 | 0 | 45 | 0 | 0 | 0 | 0 |
| unsupported | 0 | 0 | 33 | 0 | 0 | 0 | 0 |
| api-misuse | 0 | 0 | 12 | 0 | 0 | 0 | 0 |
| syntax | 2 | 1 | 4 | 3 | 0 | 0 | 0 |
| runtime-panic | 0 | 0 | 5 | 0 | 0 | 0 | 0 |
| trait-bound | 0 | 1 | 0 | 4 | 0 | 0 | 0 |
| name-resolution | 0 | 0 | 4 | 0 | 0 | 0 | 0 |

## Interpreter vs compiled divergence

50 of 210 trials had an interpreted hidden-oracle result different from the compiled one (44 silent: interpreter green, compiled red).
- t003_prime_counts arm A trial 1: interp=True compiled=False
- t003_prime_counts arm C trial 0: interp=True compiled=False
- t003_prime_counts arm C trial 2: interp=True compiled=False
- t006_base_convert arm A trial 0: interp=True compiled=False
- t006_base_convert arm A trial 1: interp=True compiled=False
- t006_base_convert arm A trial 2: interp=True compiled=False
- t006_base_convert arm C trial 0: interp=True compiled=False
- t006_base_convert arm C trial 1: interp=True compiled=False
- t006_base_convert arm C trial 2: interp=True compiled=False
- t006_base_convert arm D trial 0: interp=True compiled=False

## Agent preference score (Windjammer arm vs baseline language)

APS = 100 * P^0.4 * I^0.2 * T^0.2 * W^0.2 (ratios baseline/Windjammer, clipped to [0.25, 4]; I = agent turns). 100 = parity; `P-limited` when pass@1 is below 0.9 of the baseline's. Components are the diagnosis; CIs are task-clustered bootstrap.

| arm | vs | APS | P | I (turns) | T (tokens) | W (wall) |
|---|---|---|---|---|---|---|
| A interpreted-first | G go | P-limited | 0.40 [0.25, 0.57] | 0.75 [0.59, 0.92] | 0.50 [0.35, 0.68] | 0.91 [0.63, 1.33] |
| A interpreted-first | P python | P-limited | 0.40 [0.25, 0.57] | 1.05 [0.75, 1.44] | 0.63 [0.44, 0.91] | 3.13 [1.50, 4.00] |
| A interpreted-first | R rust | P-limited | 0.40 [0.25, 0.57] | 0.88 [0.71, 1.00] | 0.47 [0.36, 0.59] | 1.00 [0.74, 1.35] |
| B compiled-first | G go | P-limited | 0.87 [0.77, 0.97] | 0.53 [0.44, 0.63] | 0.37 [0.28, 0.48] | 0.25 [0.25, 0.35] |
| B compiled-first | P python | P-limited | 0.87 [0.77, 0.97] | 0.74 [0.56, 1.00] | 0.47 [0.34, 0.66] | 0.69 [0.30, 1.48] |
| B compiled-first | R rust | P-limited | 0.87 [0.77, 0.97] | 0.62 [0.51, 0.76] | 0.35 [0.27, 0.46] | 0.25 [0.25, 0.37] |
| C no-pack | G go | P-limited | 0.25 [0.25, 0.30] | 0.25 [0.25, 0.25] | 0.25 [0.25, 0.25] | 0.28 [0.25, 0.35] |
| C no-pack | P python | P-limited | 0.25 [0.25, 0.30] | 0.25 [0.25, 0.25] | 0.25 [0.25, 0.25] | 0.97 [0.60, 1.37] |
| C no-pack | R rust | P-limited | 0.25 [0.25, 0.30] | 0.25 [0.25, 0.25] | 0.25 [0.25, 0.25] | 0.31 [0.25, 0.38] |
| D interpreted+final-build | G go | P-limited | 0.83 [0.63, 0.97] | 0.63 [0.49, 0.84] | 0.31 [0.25, 0.42] | 0.25 [0.25, 0.25] |
| D interpreted+final-build | P python | P-limited | 0.83 [0.63, 0.97] | 0.88 [0.71, 1.24] | 0.39 [0.31, 0.55] | 0.25 [0.25, 0.34] |
| D interpreted+final-build | R rust | P-limited | 0.83 [0.63, 0.97] | 0.74 [0.54, 1.07] | 0.29 [0.25, 0.42] | 0.25 [0.25, 0.25] |

## Deltas versus the previous pilot (same tasks and arms; this run minus previous)

| arm | metric | previous | this run | delta |
|---|---|---|---|---|
| A | pass@1 (compiled hidden oracle) | 0.567 | 0.400 | -0.167 |
| A | verify calls (wj check/run/test/build) | 2.0 | 1.8 | -0.2 |
| A | verify calls to green (passing trials) | 1.1 | 1.4 | +0.3 |
| A | agent turns | 3.5 | 3.3 | -0.2 |
| A | tokens in (incl. cache) | 31754.8 | 34389.2 | +2634.4 |
| A | tokens out | 1435.3 | 1388.4 | -46.9 |
| A | cost USD | 0.028 | 0.028 | +0.001 |
| A | wall seconds | 20.3 | 27.8 | +7.5 |
| A | agent-thinking seconds (API time) | 11.3 | 12.5 | +1.2 |
| A | tool-wait seconds (wj commands) | 0.3 | 0.6 | +0.3 |
| A | silent divergence (interp pass, compiled fail) | 0.433 | 0.567 | +0.133 |
| B | pass@1 (compiled hidden oracle) | 0.900 | 0.867 | -0.033 |
| B | verify calls (wj check/run/test/build) | 2.3 | 2.1 | -0.2 |
| B | verify calls to green (passing trials) | 2.0 | 2.0 | -0.0 |
| B | agent turns | 5.3 | 5.0 | -0.3 |
| B | tokens in (incl. cache) | 53623.9 | 51550.1 | -2073.8 |
| B | tokens out | 1681.6 | 1717.8 | +36.2 |
| B | cost USD | 0.041 | 0.038 | -0.003 |
| B | wall seconds | 96.9 | 117.8 | +20.9 |
| B | agent-thinking seconds (API time) | 16.2 | 17.6 | +1.4 |
| B | tool-wait seconds (wj commands) | 66.1 | 98.4 | +32.2 |
| B | silent divergence (interp pass, compiled fail) | 0.000 | 0.100 | +0.100 |

Mode violations (agent used the other arm's loop commands): 0.

## Friction list (ranked by estimated tokens wasted)

| # | pri | friction | fix kind | fix location | trials | tasks | wasted calls | est. tokens |
|---|---|---|---|---|---|---|---|---|
| 1 | P0 | type mismatch (often int/float, string/&str) (`type-mismatch`) | diagnostic text | show both types and the conversion (`as`, `.to_string()`); error-code explain entry | 62 | 10 | 96 | 1242074 |
| 2 | P0 | interpreted run passed but compiled grade failed (silent divergence) (`silent-divergence`) | interpreter / check parity | R2/R5 gates in docs/design/interpreter-primary-readiness.md; add the program to the differential corpus | 44 | 9 | 90 | 1011338 |
| 3 | P0 | slow `wj build` (compiled) >= 30s (`slow-command:build-compiled`) | performance | build cache / runtime pre-build; see wall-time table | 33 | 10 | 41 | 636576 |
| 4 | P1 | Rust-leakage lint rejected idiomatic-looking code (`lint-leakage`) | lint rule | src/lint (rule text + `wj fix` autofix); docs/agent pack section 'Core idioms' | 22 | 10 | 40 | 443377 |
| 5 | P1 | interpreter refused a feature the compiled path accepts (`interp-unsupported`) | interpreter coverage | src/interpreter + docs/design/interpreter-primary-readiness.md R6 | 18 | 9 | 27 | 299280 |
| 6 | P1 | slow `wj run` (compiled) >= 30s (`slow-command:run-compiled`) | performance | build cache / runtime pre-build; see wall-time table | 15 | 10 | 19 | 199529 |
| 7 | P1 | unclassified failure: error: `X` doesn't implement `X` (`other:error: `X` doesn't implement `X``) | diagnostic text (triage) | inspect example, then file under a rule | 15 | 10 | 16 | 177351 |
| 8 | P1 | method accepted by the interpreter is missing on the compiled path (`method-parity`) | stdlib parity / check | `wj check` must reject what rustc will (R5 check-vs-build gate); alias or implement the std method; `wj docs` entry | 13 | 8 | 15 | 165684 |
| 9 | P2 | collect/iterator element type mismatch in generated Rust (`collect-type`) | codegen | iterator lowering (borrowed vs owned element types); known-issues agent-dx-compiled-str-literal-and-option-default-codegen.md | 9 | 5 | 12 | 142696 |
| 10 | P2 | unclassified failure: = note: ADR-N: Windjammer has no macros; the `X` forms become errors in a future release (`other:= note: ADR-N: Windjammer has no macros; the `X` forms become errors in a future release`) | diagnostic text (triage) | inspect example, then file under a rule | 4 | 3 | 7 | 77591 |
| 11 | P2 | stdin API differs between interpreter and compiled path (`stdin-api`) | stdlib API | std::io unification; pack section 'Reading input' | 6 | 5 | 7 | 77591 |
| 12 | P2 | raw rustc/cargo error surfaced from generated code (`rustc-leak`) | codegen / diagnostic mapping | map rustc codes back to .wj spans; fix codegen so check-vs-build parity holds (R5) | 4 | 3 | 5 | 58230 |
| 13 | P2 | runtime panic during the loop (`runtime-panic`) | docs | pack section on Option/Result handling and index bounds | 2 | 2 | 3 | 33253 |
| 14 | P2 | unresolved name (`name-unresolved`) | diagnostic text | diagnostic should list candidates and the import to add | 1 | 1 | 1 | 11084 |
| 15 | P2 | unclassified failure: "replace `X` with `X`" (`other:"replace `X` with `X`"`) | diagnostic text (triage) | inspect example, then file under a rule | 1 | 1 | 1 | 11084 |
| 16 | P2 | unclassified failure: `X` doesn't implement `X` (`other:`X` doesn't implement `X``) | diagnostic text (triage) | inspect example, then file under a rule | 1 | 1 | 1 | 11084 |

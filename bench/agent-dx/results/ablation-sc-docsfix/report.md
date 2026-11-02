# Agent preference pilot

8 trials over 8 tasks. Intervals are 95% bootstrap CIs that resample tasks, then trials within a task. With this N they show direction, not significance, unless an interval excludes zero.

## Arms

| arm | name | trials | context pack |
|---|---|---|---|
| Sc | sheet-minus-input | 8 | sheet_minus_input.md |

## Arm means

| metric | Sc |
|---|---|
| pass@1 (compiled hidden oracle) | 0.875 [0.625, 1.000] |
| verify calls (wj check/run/test/build) | 5.8 [4.6, 6.9] |
| verify calls to green (passing trials) | 4.9 [3.7, 5.9] |
| agent turns | 19.9 [13.4, 25.4] |
| tokens in (incl. cache) | 217858.1 [135613.5, 296483.4] |
| tokens out | 3683.2 [2360.8, 4784.0] |
| cost USD | 0.121 [0.101, 0.141] |
| wall seconds | 211.4 [90.7, 410.0] |
| agent-thinking seconds (API time) | 74.2 [59.4, 89.1] |
| tool-wait seconds (wj commands) | 42.2 [20.0, 68.4] |
| harness permission denials (should be 0) | 0.9 [0.5, 1.2] |
| capability denials seen (WJ004x) | 0.0 [0.0, 0.0] |
| silent divergence (interp pass, compiled fail) | 0.000 [0.000, 0.000] |

## Per-task pass rate (passes/trials)

| task | Sc |
|---|---|
| t003_prime_counts | 1/1 |
| t006_base_convert | 1/1 |
| t009_rpn_eval | 1/1 |
| t011_lru_cache | 1/1 |
| t013_bst | 1/1 |
| t018_word_freq | 1/1 |
| t030_bank_ledger | 0/1 |
| t034_shapes_trait | 1/1 |

## Top failure categories (diagnostics hit, all verify calls)

| category | Sc |
|---|---|
| other | 28 |
| trait-bound | 14 |
| name-resolution | 8 |
| lint-style | 6 |
| type-mismatch | 4 |

## Interpreter vs compiled divergence

0/8 trials had an interpreted hidden-oracle result different from the compiled one (0 silent: interpreter green, compiled red).

## Context attribution (stream-json arms)

First-turn input = system prompt + tool definitions + pack + task prompt. Pack tokens are estimated as bytes/3.5. Schema tokens are the first-turn difference to the matching no-MCP arm (T), minus the pack difference.

| arm | first-turn input | pack tokens (est, bytes/3.5) | other fixed (system, tools, task) | tool-result tokens (est, all turns) | docs shell calls | MCP calls | total tokens in | turns |
|---|---|---|---|---|---|---|---|---|
| Sc sheet-minus-input | 8120 | 385 | 7735 | 4616 | 24.1 | 0.0 | 217858 | 19.9 |

Mode violations (agent used the other arm's loop commands): 0.

## Friction list (ranked by estimated tokens wasted)

| # | pri | friction | fix kind | fix location | trials | tasks | wasted calls | est. tokens |
|---|---|---|---|---|---|---|---|---|
| 1 | P0 | method accepted by the interpreter is missing on the compiled path (`method-parity`) | stdlib parity / check | `wj check` must reject what rustc will (R5 check-vs-build gate); alias or implement the std method; `wj docs` entry | 7 | 7 | 13 | 160226 |
| 2 | P0 | stdin API differs between interpreter and compiled path (`stdin-api`) | stdlib API | std::io unification; pack section 'Reading input' | 4 | 4 | 8 | 98601 |
| 3 | P0 | Rust-leakage lint rejected idiomatic-looking code (`lint-leakage`) | lint rule | src/lint (rule text + `wj fix` autofix); docs/agent pack section 'Core idioms' | 5 | 5 | 6 | 73951 |
| 4 | P1 | type mismatch (often int/float, string/&str) (`type-mismatch`) | diagnostic text | show both types and the conversion (`as`, `.to_string()`); error-code explain entry | 2 | 2 | 3 | 36975 |
| 5 | P1 | slow `wj run` (compiled) >= 30s (`slow-command:run-compiled`) | performance | build cache / runtime pre-build; see wall-time table | 2 | 2 | 2 | 24650 |
| 6 | P1 | raw rustc/cargo error surfaced from generated code (`rustc-leak`) | codegen / diagnostic mapping | map rustc codes back to .wj spans; fix codegen so check-vs-build parity holds (R5) | 1 | 1 | 1 | 12325 |
| 7 | P1 | slow `wj build` (compiled) >= 30s (`slow-command:build-compiled`) | performance | build cache / runtime pre-build; see wall-time table | 1 | 1 | 1 | 12325 |

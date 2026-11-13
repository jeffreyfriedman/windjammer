# Release scorecard

The answer to "is it ready?" is a table of numbers, each against a fixed threshold, produced by
`scripts/wj/release_scorecard.wj`. Nothing on this page is typed by hand: the definition and the latest run are
generated regions (`wj run scripts/wj/release_scorecard.wj -- --write`), and CI runs `-- --check` to prove the page
matches the script and `docs/generated/release_scorecard.json`.

GREEN means the target is met, AMBER means within reach, RED means not yet. Criteria that need a build or a quiet machine
read `results/*.json` files written by the verification flow (see `results/README.md`); an absent file shows MISSING and
a file older than the age limit shows STALE. Neither is green. The weekly routine is in `docs/ROUTINES.md`.

## Definition

<!-- scorecard:begin definition -->
| # | Criterion | Rule | Unit | Source |
|---|---|---|---|---|
| 1 | `known_test_failures`: Known failing tests (ledger lines) | green <= 0, amber <= 40 | tests | file .config/known-test-failures.txt |
| 1 | `drift_ledger`: Interpreter/compiled drift ledger lines | green <= 0, amber <= 20 | lines | file tests/drift_corpus/known_divergences.txt |
| 1 | `rustc_rejects`: Golden programs rustc rejects | green <= 0, amber <= 25 | programs | file tests/golden/rustc_rejects.txt |
| 1 | `friction_events_open`: Open agent-friction events | green <= 0, amber <= 5 | events | file docs/generated/measured.json |
| 1 | `friction_over_ratchet`: Agent-cost measures worse than their ratchet | green <= 0, amber <= 0 | measures | file bench/agent-dx/friction/ratchet.json |
| 1 | `width_owners`: D-WIDTH-OWNERS findings (recorded baseline) | green <= 0, amber <= 20 | findings | file tests/golden/explain_counts.txt |
| 1 | `width_owners_over_ratchet`: D-WIDTH-OWNERS live count above its ratchet | green <= 0, amber <= 0 | findings | results explain-counts.json |
| 1 | `heuristic_surface`: Heuristic surface (name matches, arm names, fallback/legacy) | green <= 500, amber <= 1200 | sites | file docs/heuristic-allowlist.toml |
| 1 | `heuristic_published_gap`: Heuristic total published in the burn-down differs from the allow-list | green <= 0, amber <= 0 | sites | file docs/spec/convergence-burndown.md |
| 1 | `ownership_conflicts`: Ownership design-conflict tests ledgered (open) | green <= 0, amber <= 10 | tests | file docs/spec/convergence-burndown.md |
| 2 | `interp_r2`: R2 no silent divergence | green >= 100, amber >= 90 | percent | file docs/generated/interp_readiness.json |
| 2 | `interp_r4`: R4 fast enough | green >= 100, amber >= 90 | percent | file docs/generated/interp_readiness.json |
| 2 | `interp_r5`: R5 diagnostics agree | green >= 100, amber >= 90 | percent | file docs/generated/interp_readiness.json |
| 2 | `interp_r7`: R7 stability over the last 20 commits | green >= 100, amber >= 90 | percent | file docs/generated/interp_readiness.json |
| 2 | `interp_startup_ms`: Interpreter startup (CPU) | green <= 50, amber <= 200 | ms | file docs/generated/measured.json |
| 2 | `interp_kernels_slower`: Kernels where the interpreter is slower than Python | green <= 0, amber <= 3 | kernels | file bench/interp/compare.json |
| 3 | `build_rss_growth`: Library build peak memory growth, N=200 over N=50 | green <= 4.5, amber <= 8 | x | results build-meter.json |
| 3 | `build_wall_growth`: Library build wall-time growth, N=200 over N=50 | green <= 4.5, amber <= 8 | x | results build-meter.json |
| 3 | `build_rss_n200_mb`: Library build peak memory at N=200 (8 GB laptop budget) | green <= 4096, amber <= 6144 | MB | results build-meter.json |
| 3 | `build_wall_n200_s`: Library build wall time at N=200 (4 cores) | green <= 120, amber <= 600 | s | results build-meter.json |
| 3 | `engine_rss_mb`: Flagship engine library transpile peak memory | green <= 4096, amber <= 6144 | MB | results engine-transpile.json |
| 3 | `engine_wall_s`: Flagship engine library transpile wall time | green <= 300, amber <= 900 | s | results engine-transpile.json |
| 4 | `agent_cost_ratio`: Pilot-7 cost per task, Windjammer over Rust | green <= 1, amber <= 1.5 | x | results pilot7.json |
| 4 | `agent_pass_rate`: Pilot-7 Windjammer pass rate | green >= 0.9, amber >= 0.7 | ratio | results pilot7.json |
| 4 | `agent_token_ratio`: Tokens to write a task, Windjammer over Rust (reference solutions) | green <= 1, amber <= 1.3 | x | file docs/generated/measured.json |
| 4 | `agent_fix_coverage`: Error codes seen with a machine fix | green >= 0.8, amber >= 0.4 | ratio | file docs/generated/measured.json |
| 4 | `agent_ref_failing`: Pilot-7 reference solutions failing `wj run --verify` | green <= 0, amber <= 2 | solutions | results pilot7.json |
| 5 | `engine_native_errors`: Flagship engine library native build errors | green <= 0, amber <= 50 | errors | results targets.json |
| 5 | `web_stage`: Flagship engine web target stage reached (0a=0, 1, 2, ...) | green >= 2, amber >= 1 | stage | results targets.json |
| 5 | `playground_parity_failures`: Playground samples that differ from the compiled run | green <= 0, amber <= 3 | samples | results targets.json |
| 6 | `eco_packages`: Ecosystem packages | green >= 60, amber >= 40 | packages | file ecosystem packages/ (or results ecosystem.json) |
| 6 | `eco_without_tests`: Ecosystem packages without tests | green <= 0, amber <= 5 | packages | file ecosystem packages/ (or results ecosystem.json) |
| 6 | `eco_sweep_failing`: Ecosystem sweep failures | green <= 0, amber <= 3 | tests | results ecosystem.json |
| 6 | `orphan_tests`: Orphan tests (no runner reaches them) | green <= 0, amber <= 3 | tests | results docs.json |
| 7 | `book_pages`: Book pages | green >= 60, amber >= 40 | pages | file docs/generated/metrics.json |
| 7 | `features_uncovered`: Registry features without a book page | green <= 0, amber <= 3 | features | file docs/generated/metrics.json |
| 7 | `docs_stale_numbers`: Stale or hand-typed numbers (docs-truth claims) | green <= 0, amber <= 5 | claims | results docs.json |
| 7 | `roadmap_open_gaps`: Open or partial gaps in the ranked known-gaps table | green <= 0, amber <= 10 | gaps | file docs/known-gaps.md |
| 8 | `fmt_failing_files`: Files failing cargo fmt --check | green <= 0, amber <= 20 | files | results fmt.json |
| 8 | `hygiene_allowlist`: Public hygiene allowlist entries | green <= 0, amber <= 10 | entries | file docs/public-hygiene-allowlist.txt |
| 8 | `trailer_commits`: Commits carrying a session trailer in history | green <= 0, amber <= 10 | commits | git log |
| 8 | `governance_missing`: Governance files missing across the public repos | green <= 0, amber <= 0 | files | file README LICENSE CODE_OF_CONDUCT CONTRIBUTING SECURITY |
| 8 | `large_files`: Tracked files over 1 MB | green <= 0, amber <= 20 | files | git ls-tree |
| 9 | `suite_age_days`: Age of the last full-suite result | green <= 7, amber <= 14 | days | results suite.json |
| 9 | `suite_commits_behind`: Commits on the head since the full-suite result | green <= 20, amber <= 100 | commits | results suite.json |
| 9 | `suite_new_failures`: New failures (not in the known list) in the last full-suite result | green <= 0, amber <= 0 | tests | results suite.json |
<!-- scorecard:end definition -->

## Latest run

<!-- scorecard:begin run -->
Head `c39b447b9aae`, run at epoch 1794513798, results older than 8 days count as stale.

**9 green, 8 amber, 9 red, 0 stale, 20 missing** of 46 criteria.

### 1. Language and compiler correctness

| Status | Criterion | Now | Rule | Evidence |
|---|---|---|---|---|
| RED | `known_test_failures`: Known failing tests (ledger lines) | 66 tests | green <= 0, amber <= 40 | .config/known-test-failures.txt |
| RED | `drift_ledger`: Interpreter/compiled drift ledger lines | 80 lines | green <= 0, amber <= 20 | tests/drift_corpus/known_divergences.txt |
| RED | `rustc_rejects`: Golden programs rustc rejects | 78 programs | green <= 0, amber <= 25 | tests/golden/rustc_rejects.txt |
| RED | `friction_events_open`: Open agent-friction events | 9 events | green <= 0, amber <= 5 | measured.json cost.friction.events_open |
| GREEN | `friction_over_ratchet`: Agent-cost measures worse than their ratchet | 0 measures | green <= 0, amber <= 0 | ratchet.json against measured.json |
| RED | `width_owners`: D-WIDTH-OWNERS findings (recorded baseline) | 35 findings | green <= 0, amber <= 20 | tests/golden/explain_counts.txt |
| MISSING | `width_owners_over_ratchet`: D-WIDTH-OWNERS live count above its ratchet | n/a | green <= 0, amber <= 0 | missing: results/explain-counts.json (written by the PC verification flow) |
| RED | `heuristic_surface`: Heuristic surface (name matches, arm names, fallback/legacy) | 1780 sites | green <= 500, amber <= 1200 | docs/heuristic-allowlist.toml |
| GREEN | `heuristic_published_gap`: Heuristic total published in the burn-down differs from the allow-list | 0 sites | green <= 0, amber <= 0 | published 1780 vs allow-list 1780 |
| AMBER | `ownership_conflicts`: Ownership design-conflict tests ledgered (open) | 7 tests | green <= 0, amber <= 10 | named in the burn-down C5 design-conflict section |

### 2. Interpreter

| Status | Criterion | Now | Rule | Evidence |
|---|---|---|---|---|
| AMBER | `interp_r2`: R2 no silent divergence | 98.5 percent | green >= 100, amber >= 90 | interp_readiness.json, 7.45 days old |
| AMBER | `interp_r4`: R4 fast enough | 94.7 percent | green >= 100, amber >= 90 | interp_readiness.json, 7.45 days old |
| AMBER | `interp_r5`: R5 diagnostics agree | 95.1 percent | green >= 100, amber >= 90 | interp_readiness.json, 7.45 days old |
| RED | `interp_r7`: R7 stability over the last 20 commits | 0 percent | green >= 100, amber >= 90 | interp_readiness.json, 7.45 days old |
| GREEN | `interp_startup_ms`: Interpreter startup (CPU) | 20 ms | green <= 50, amber <= 200 | measured.json book.startup.interpret_cpu_ms |
| AMBER | `interp_kernels_slower`: Kernels where the interpreter is slower than Python | 2 kernels | green <= 0, amber <= 3 | resolved engine vs python3, bench/interp/compare.json |

### 3. Build cost

| Status | Criterion | Now | Rule | Evidence |
|---|---|---|---|---|
| MISSING | `build_rss_growth`: Library build peak memory growth, N=200 over N=50 | n/a | green <= 4.5, amber <= 8 | missing: results/build-meter.json (written by the PC verification flow) |
| MISSING | `build_wall_growth`: Library build wall-time growth, N=200 over N=50 | n/a | green <= 4.5, amber <= 8 | missing: results/build-meter.json (written by the PC verification flow) |
| MISSING | `build_rss_n200_mb`: Library build peak memory at N=200 (8 GB laptop budget) | n/a | green <= 4096, amber <= 6144 | missing: results/build-meter.json (written by the PC verification flow) |
| MISSING | `build_wall_n200_s`: Library build wall time at N=200 (4 cores) | n/a | green <= 120, amber <= 600 | missing: results/build-meter.json (written by the PC verification flow) |
| MISSING | `engine_rss_mb`: Flagship engine library transpile peak memory | n/a | green <= 4096, amber <= 6144 | missing: results/engine-transpile.json (written by the PC verification flow) |
| MISSING | `engine_wall_s`: Flagship engine library transpile wall time | n/a | green <= 300, amber <= 900 | missing: results/engine-transpile.json (written by the PC verification flow) |

### 4. Agent cost

| Status | Criterion | Now | Rule | Evidence |
|---|---|---|---|---|
| MISSING | `agent_cost_ratio`: Pilot-7 cost per task, Windjammer over Rust | n/a | green <= 1, amber <= 1.5 | missing: results/pilot7.json (written by the PC verification flow) |
| MISSING | `agent_pass_rate`: Pilot-7 Windjammer pass rate | n/a | green >= 0.9, amber >= 0.7 | missing: results/pilot7.json (written by the PC verification flow) |
| AMBER | `agent_token_ratio`: Tokens to write a task, Windjammer over Rust (reference solutions) | 1.26 x | green <= 1, amber <= 1.3 | measured.json cost.write.ref_token_ratio_wj_over_rust |
| RED | `agent_fix_coverage`: Error codes seen with a machine fix | 0 ratio | green >= 0.8, amber >= 0.4 | measured.json cost.diag.fix_coverage |
| MISSING | `agent_ref_failing`: Pilot-7 reference solutions failing `wj run --verify` | n/a | green <= 0, amber <= 2 | missing: results/pilot7.json (written by the PC verification flow) |

### 5. Targets and backends

| Status | Criterion | Now | Rule | Evidence |
|---|---|---|---|---|
| MISSING | `engine_native_errors`: Flagship engine library native build errors | n/a | green <= 0, amber <= 50 | missing: results/targets.json (written by the PC verification flow) |
| MISSING | `web_stage`: Flagship engine web target stage reached (0a=0, 1, 2, ...) | n/a | green >= 2, amber >= 1 | missing: results/targets.json (written by the PC verification flow) |
| MISSING | `playground_parity_failures`: Playground samples that differ from the compiled run | n/a | green <= 0, amber <= 3 | missing: results/targets.json (written by the PC verification flow) |

### 6. Ecosystem

| Status | Criterion | Now | Rule | Evidence |
|---|---|---|---|---|
| GREEN | `eco_packages`: Ecosystem packages | 64 packages | green >= 60, amber >= 40 | live: ecosystem packages/ |
| GREEN | `eco_without_tests`: Ecosystem packages without tests | 0 packages | green <= 0, amber <= 5 | live: ecosystem packages/ |
| MISSING | `eco_sweep_failing`: Ecosystem sweep failures | n/a | green <= 0, amber <= 3 | missing: results/ecosystem.json (written by the PC verification flow) |
| MISSING | `orphan_tests`: Orphan tests (no runner reaches them) | n/a | green <= 0, amber <= 3 | missing: results/docs.json (written by the PC verification flow) |

### 7. Docs

| Status | Criterion | Now | Rule | Evidence |
|---|---|---|---|---|
| GREEN | `book_pages`: Book pages | 62 pages | green >= 60, amber >= 40 | metrics.json book.pages |
| GREEN | `features_uncovered`: Registry features without a book page | 0 features | green <= 0, amber <= 3 | metrics.json book.features_uncovered |
| MISSING | `docs_stale_numbers`: Stale or hand-typed numbers (docs-truth claims) | n/a | green <= 0, amber <= 5 | missing: results/docs.json (written by the PC verification flow) |
| RED | `roadmap_open_gaps`: Open or partial gaps in the ranked known-gaps table | 13 gaps | green <= 0, amber <= 10 | docs/known-gaps.md ranked table |

### 8. Public readiness

| Status | Criterion | Now | Rule | Evidence |
|---|---|---|---|---|
| MISSING | `fmt_failing_files`: Files failing cargo fmt --check | n/a | green <= 0, amber <= 20 | missing: results/fmt.json (written by the PC verification flow) |
| GREEN | `hygiene_allowlist`: Public hygiene allowlist entries | 0 entries | green <= 0, amber <= 10 | docs/public-hygiene-allowlist.txt |
| AMBER | `trailer_commits`: Commits carrying a session trailer in history | 3 commits | green <= 0, amber <= 10 | git log, session or attribution trailers |
| GREEN | `governance_missing`: Governance files missing across the public repos | 0 files | green <= 0, amber <= 0 | checked: compiler, windjammer-ecosystem, windjammer-ui |
| AMBER | `large_files`: Tracked files over 1 MB | 1 files | green <= 0, amber <= 20 | git ls-tree at HEAD |

### 9. Verification freshness

| Status | Criterion | Now | Rule | Evidence |
|---|---|---|---|---|
| MISSING | `suite_age_days`: Age of the last full-suite result | n/a | green <= 7, amber <= 14 | missing: results/suite.json (written by the PC verification flow) |
| MISSING | `suite_commits_behind`: Commits on the head since the full-suite result | n/a | green <= 20, amber <= 100 | missing: results/suite.json (written by the PC verification flow) |
| MISSING | `suite_new_failures`: New failures (not in the known list) in the last full-suite result | n/a | green <= 0, amber <= 0 | missing: results/suite.json (written by the PC verification flow) |
<!-- scorecard:end run -->

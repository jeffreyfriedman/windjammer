# Results files

Machine-readable results that the release scorecard (`scripts/wj/release_scorecard.wj`, page `docs/RELEASE_SCORECARD.md`)
reads for criteria that need a build or a quiet machine. The scorecard never builds or times anything itself.

Only this README is tracked; `results/*.json` is local output (ignored by git), written by the **PC verification flow**
(the quiet machine that builds the integration head) after each weekly run and copied beside the checkout the scorecard
runs in. A file that is absent shows **MISSING** on the scorecard; one older than the age limit (8 days by default,
`--max-age`) shows **STALE**. Neither counts as green.

## Format

```json
{
  "schema": 1,
  "producer": "pc-flow build-meter",
  "generated_epoch": 1794500000,
  "commit": "f0ae7eaf1686",
  "values": { "n50_rss_mb": 410, "n200_rss_mb": 1630 }
}
```

`schema` (always 1; a file with another version shows MISSING), `generated_epoch` (Unix seconds, required) and `commit`
(the head measured) are top level; `values` is a flat object of numbers. `--selftest` checks every fixture file against
this shape (`schema_problems` in the script). Write the file atomically (temp file, then rename) and only after the measurement finished: a partial run must
leave the previous file in place, not a half-filled one.

## Who writes which file

All files are written by the PC verification flow. Key names are the contract.

| File | Keys | Measured by |
|---|---|---|
| `suite.json` | `new_failures` (failures not in `.config/known-test-failures.txt`), `known_failures_seen`, `total`; `commit` = the head tested | the full `cargo test --release --test all` run on the integration head (`generated_epoch` = when it finished) |
| `explain-counts.json` | `d_width_owners` (live count of `wj explain-ownership` findings over the convergence corpus) | the convergence gate |
| `build-meter.json` | `n50_rss_mb`, `n100_rss_mb`, `n200_rss_mb`, `n50_wall_s`, `n100_wall_s`, `n200_wall_s` | the synthetic library-build meter at N=50/100/200 |
| `engine-transpile.json` | `peak_rss_mb`, `wall_s` | the flagship engine library transpile |
| `pilot7.json` | `wj_cost_per_task_usd`, `rust_cost_per_task_usd`, `wj_pass_rate`, `reference_total`, `reference_verified`; optional `cost_per_task.<model>.<arm>` | the pilot-7 report and `wj run --verify` over the nine reference solutions |
| `targets.json` | `engine_native_build_errors`, `web_stage` (0a = 0, then 1, 2, ...), `playground_parity_failures`; top-level string `web_plan` = path of the web-target plan document the stage was counted from, relative to the project that owns it | the flagship engine native build, its web-target plan checkboxes, the playground sample parity run |
| `ecosystem.json` | `sweep_total`, `sweep_pass`; `packages`, `packages_without_tests` (used when no ecosystem checkout is visible to the scorecard) | the ecosystem test sweep |
| `docs.json` | `stale_or_hand_typed_numbers` (docs-truth claim violations), `orphan_tests` (`check_orphan_tests.wj`) | `scripts/docs_gen.sh --verify` and the orphan check |
| `fmt.json` | `failing_files` | `cargo fmt --check` |

Names that mention the flagship engine stay generic in this public repository; the flow knows which private project they
map to. The scorecard row for the web stage ends with `plan: <web_plan>`, so a reader finds the plan document without this
repository naming the project.

## Running the scorecard

```
export WJ_ECOSYSTEM=/path/to/windjammer-ecosystem   # packages of scripts/wj/wj.toml, and the live ecosystem counts
wj run --interpret --allow "fs.read=$WJ_ECOSYSTEM/**" scripts/wj/release_scorecard.wj -- --write
wj run --interpret --allow "fs.read=$WJ_ECOSYSTEM/**" scripts/wj/release_scorecard.wj -- --check      # CI: the committed page and JSON match the script's definition and each other (reads no results)
wj run --interpret scripts/wj/release_scorecard.wj -- --selftest   # fixtures: every criterion turns RED, GREEN, STALE, MISSING; results files follow the schema
```

Sibling public repos (`WJ_ECOSYSTEM`, `WJ_UI`, or checkouts/symlinks `windjammer-ecosystem/` and `windjammer-ui/` at the
repository root) are included in the governance-file count when visible.

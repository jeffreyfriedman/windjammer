# Routines

## Weekly

1. **Verification flow on the quiet machine.** Build the integration head, run the full suite, the library-build meter, the
   engine transpile, the pilot-7 reference check, the ecosystem sweep, `cargo fmt --check` and the docs check; write each
   result to `results/<name>.json` (contract and key names: [results/README.md](../results/README.md)).
2. **Release scorecard.** Regenerate and commit the page and its JSON:

   ```
   wj run --interpret scripts/wj/release_scorecard.wj -- --write
   ```

   [docs/RELEASE_SCORECARD.md](RELEASE_SCORECARD.md) reports every criterion of the fixed definition as GREEN, AMBER or RED
   with its current number; missing or stale result files show as such. "Is it feature complete?" is answered by that page.
3. Report the green, amber and red counts and the criteria that moved since last week.

On demand: run the same command at any time; `-- --check` (CI) fails when the committed page or JSON no longer matches the
script's definition, and `-- --selftest` proves every criterion can turn RED.

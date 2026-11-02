# tests/drift_corpus

## Purpose
The differential corpus of the drift gate (`docs/testing/DRIFT.md`): real programs written by someone other
than the interpreter's authors, run under the tree walker, the resolved engine and the compiled backend.

## Inputs / outputs
- In: `eco/packages/*` and `eco/apps/*`, each with `wj.toml`, `src/`, `tests/` (and `migrations/`), a verbatim
  snapshot of the windjammer-ecosystem at the commit in PROVENANCE. Names are the ecosystem's public names.
- In: `compiled_baseline.tsv`, the compiled backend's result per case (`id TAB token TAB panic-location`).
- In/out: `known_divergences.txt`, the shrink-only ledger.
- Out (by `scripts/wj/drift_corpus.wj`): one TSV row per case; `scripts/wj/drift_ledger.wj` judges it.

## Layout
- `eco/` snapshot (edit nothing in it; refresh it, see below)
- `PROVENANCE`: source repository, commit, how it was taken
- `compiled_baseline.tsv`, `known_divergences.txt`
- `repros/`: minimal programs of the ledgered divergences (also run as cases; fixing one turns its ledger line stale)
- `engine_probe/`: a one-test project on which the two engines differ by design; `drift_selftest.wj` requires them to differ, so the corpus can never compare an engine with itself

## Invariants
- The snapshot is byte-identical to `git archive <commit> packages apps` restricted to `wj.toml`, `src/`, `tests/`, `migrations/`, except the comment scrub recorded in PROVENANCE (public hygiene: no internal ticket ids).
- A divergence is never fixed by editing the snapshot; reduce it (`wj reduce`), file it in `docs/drift/findings.md`,
  fix the engine.
- Refresh: `git -C <ecosystem> archive <commit> packages apps | tar -x -C tests/drift_corpus/eco`, delete files outside
  the four paths, update PROVENANCE, re-record `compiled_baseline.tsv`, run the gate.

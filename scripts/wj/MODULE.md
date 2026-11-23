# scripts/wj

## Purpose
Repository tooling written in interpreted Windjammer (`wj run scripts/wj/<script>.wj`). Every script here replaced a Python or shell original, keeps its behaviour, and is pinned to it by a recorded differential gate (`tests/py_ports/`).

## Inputs / outputs
- In: command-line arguments and the repository (scripts run from the repository root); packages from `windjammer-ecosystem/` (checked out or symlinked at the repository root, as in the readiness workflow).
- Out: the same stdout, stderr, exit status and files as the original script.

## Layout
- `wj.toml`: package, path dependencies on ecosystem packages, and the capability grant (one grant for the directory; packages get narrower grants).
- Ports: `check_versions`, `sync_version`, `check_orphan_tests`, `check_test_ratchet`, `check_negative_oracle`, `spec_check`, `diagnostics_leak_report` (shell originals); `coverage_report`, `gen_check_mutants`, `regen_conformance_vectors`, `rust_parity_report`, `rust_parity_run`, `arena_bench`, `interp_bench`, `bench_effects_overhead`, `interpreter_project_bench` (Python originals).
- Drift gate (`docs/testing/DRIFT.md`): `drift_gate.wj` (one command, step table, exit status), `drift_corpus.wj` (runs the differential corpus under walker, resolved and compiled; TSV out), `drift_ledger.wj` (judges a run against the shrink-only ledger), `drift_selftest.wj` (planted divergences the ledger must reject), `spec_oracle.wj` (the language spec as an oracle: every `tests/spec` case with an expected output under walker, resolved and compiled; `--report` prints coverage; ledger `tests/spec_oracle/known_divergences.txt`).
- `target_tiers.wj` (ADR-0007): conformance programs and spec run cases on every backend, `tests/conformance/backend_{results,ledger,ratchet}.tsv`, and the generated `docs/TARGET_TIERS.md`; `check` is the toolchain-free gate that fails when the table is stale or the ledger grows. `effect_spelling_gate.wj` (ADR-0021): an ungranted effect is refused by `wj check`, `wj build` and `wj run` in both the `fs::write` and the `fs.write` spelling, for every enforced effect kind. `build_ceiling_gate.wj`: every program in `tests/perf_ceiling` builds within a wall-clock and a resident-memory ceiling (an unbounded pass shows up here first). `lock_gate.wj` (ADR-0010): two builds from one `wj.lock` are byte-identical and a moved dependency fails with the fix.
- `interp_instr.wj`: instruction-count ratchet of the interpreter (ADR-0038): startup and `bench/interp` kernels on callgrind or the macOS hardware counter, outputs checked against `bench/interp/expected/`, `--refs` counts the QuickJS/Lua/CPython/Node twins, `--check` against `bench/interp/instr_baseline.json`.
- `economics.wj` / `economics_report.wj`: the economic-efficiency suite (`docs/economics/DEFINITION.md`): the Windjammer programs against idiomatic Rust twins (`bench/economics/`), per dimension (verify loop, compile, memory, run, size, source), instructions from `/usr/bin/time -l` or `perf`; JSON and a markdown table per dimension. `--static` (source counts only, no build) with `--check bench/economics/results/static.json` is the deterministic CI gate. New tooling, not a port.
- `std_inventory.wj` (no original; ADR-0039): counts every removed std form of `docs/std_surface.toml` across the repository and the ecosystem (`git grep -P`, hence the `git` grant), `--write` stores `docs/generated/std_removal_inventory.tsv`, `--check` fails when a count rose or a known method name is neither canonical nor removed, `--widths` counts the integer casts behind decision B.
- `width_experiment.wj` and `nonneg_analysis.wj` (no originals; ADR-0039 decision B evidence): delete the integer casts of the reference solutions and examples and type-check each variant (`prepare`, `check` under a build slot, `report`); a lexical prototype of the "provably non-negative index" dataflow with a `len() - k` guard lint over the corpora (`--eco`, `--hits`).
- `port_gates.wj`: runs the cases under `tests/py_ports/` (`--record` re-derives expectations from the original; `--time N` produces the CPU-time table of `docs/design/python-replacement-audit.md`).

- `disk_janitor.wj` (docs/dev/disk-hygiene.md): `report` / `clean --dry-run|--apply` / `gate --floor-gb N` for build output and scratch we created (shared cargo target, worktree targets, `wj-*` temp dirs); never deletes another session's dirs or a target a cargo/rustc process uses; `--selftest` includes a mutation check. `scripts/slot_gate.sh` is its shell front door for builds.
## Invariants
- Run from the repository root; paths are relative to it.
- A script that starts a program names it through `env` or `sh` (a package grant cannot name an arbitrary path), so those two are the only broad process grants.
- Writing generated `.wj` sources needs a grant that spells the suffix (`*.wj`); `**` alone never reaches sources.
- Timing scripts read CPU time from `/usr/bin/time -p` (10 ms resolution) and refuse a verdict on a loaded machine where their original did.

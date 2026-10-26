# scripts/wj

## Purpose
Repository tooling written in interpreted Windjammer (`wj run scripts/wj/<script>.wj`). Every script here replaced a Python or shell original, keeps its behaviour, and is pinned to it by a recorded differential gate (`tests/py_ports/`).

## Inputs / outputs
- In: command-line arguments and the repository (scripts run from the repository root); packages from `windjammer-ecosystem/` (checked out or symlinked at the repository root, as in the readiness workflow).
- Out: the same stdout, stderr, exit status and files as the original script.

## Layout
- `wj.toml`: package, path dependencies on ecosystem packages, and the capability grant (one grant for the directory; packages get narrower grants).
- Ports: `check_versions`, `sync_version`, `check_orphan_tests`, `check_test_ratchet`, `check_negative_oracle`, `spec_check`, `diagnostics_leak_report` (shell originals); `coverage_report`, `gen_check_mutants`, `regen_conformance_vectors`, `rust_parity_report`, `rust_parity_run`, `arena_bench`, `interp_bench`, `bench_effects_overhead`, `interpreter_project_bench` (Python originals).
- `port_gates.wj`: runs the cases under `tests/py_ports/` (`--record` re-derives expectations from the original; `--time N` produces the CPU-time table of `docs/design/python-replacement-audit.md`).

## Invariants
- Run from the repository root; paths are relative to it.
- A script that starts a program names it through `env` or `sh` (a package grant cannot name an arbitrary path), so those two are the only broad process grants.
- Writing generated `.wj` sources needs a grant that spells the suffix (`*.wj`); `**` alone never reaches sources.
- Timing scripts read CPU time from `/usr/bin/time -p` (10 ms resolution) and refuse a verdict on a loaded machine where their original did.

# Benchmarks

Benchmark inputs and runners. This repository does not commit measured numbers in prose: a number is only
quoted in documentation when a CI job recorded it with its commit and command (`docs/generated/measured.json`, see
[docs/CONTRIBUTING-docs.md](../docs/CONTRIBUTING-docs.md)).

- `rust_parity/`: Windjammer programs next to hand-written Rust equivalents with a runner (`run.py`) and report generator (`report.py`).
  `results.json` is the last recorded run of that runner; read it, do not retype it.
- `arenas/`: `std::arena` (ADR-0025) workloads: Rust runner comparing per-object allocation, `windjammer_runtime::arena` and
  `bumpalo`, plus compiled-Windjammer programs (`run_wj.py`). `results.json` is the last recorded run.

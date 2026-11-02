# src/diagnostics/

## Purpose
Diagnostics as data: the single schema (docs/diagnostics-schema.json, v2) shared by `wj check/build/test --json`, the LSP and human rendering, plus the stable code registry behind `wj explain`.

## Inputs/Outputs
In: native diagnostics of each producer (lints, surface-syntax findings, IR solver messages, translated rustc errors). Out: `Diagnostic` / `Report` values, rendered text, applied text edits.

## Invariants
- A code is never renumbered or reused; every code has an entry in `codes_errors.rs` / `codes_lints.rs` with explanation, cause, fix and example (enforced by tests/diagnostics_suite.rs).
- Only `machine_applicable` fixes are ever applied automatically; overlapping fixes are skipped whole.
- Lines/columns are 1-based chars; byte offsets are 0-based half-open.
- A rustc error that has no translation becomes `WJ0099`, never raw rustc text. `wj run --compiled`, `wj build` and `wj build --json` all go through `rustc_text` + `rustc_map`.
- Every translated error carries a one-line help saying what to write; a mechanical fix (`hints::FixHint`) becomes an edit only when the expression is found exactly once on the mapped `.wj` line (`rustc_map::locate_fix`), and is `maybe_incorrect`.
- Corpus gate: tests/diag_hints_corpus.rs replays every failure recorded in bench/agent-dx/results/pilot*; uncovered kinds are ratcheted in tests/diag_hints_ratchet/.

## Owns
The schema, the code registry, rustc-to-WJ code and message translation (`rustc_text.rs`), fix-oriented hints for common type errors (`hints.rs`: String/&str, int/usize, Option<T>/T, float widths, collect element types), wording for interpreter run-time messages (`runtime_text.rs`), fix application.

## Never decides
Whether a program is correct (producers decide); how files are discovered (see `check`).

## Allowed imports
Layer `tooling`. Reads `error_mapper`, `linter`, `intrinsics`, `source_map` types for adapters.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`codes::explain`, `codes::lookup`, `convert::*`, `render::render`, `apply::{collect_edits, apply_edits}`, `rustc_text::translate`, `hints::{mismatch, collect_elements}`, `runtime_text::classify`.

## Tests
Unit tests in each file; tests/diagnostics_suite.rs (registry, goldens, schema, false-positive sweep).
Run: `cargo test --release --test diagnostics_suite --test diag_hints_suite --test diag_hints_corpus`.

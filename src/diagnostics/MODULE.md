# src/diagnostics/

## Purpose
Diagnostics as data: the single schema (docs/diagnostics-schema.json, v2) shared by `wj check/build/test --json`, the LSP and human rendering, plus the stable code registry behind `wj explain`.

## Inputs/Outputs
In: native diagnostics of each producer (lints, surface-syntax findings, IR solver messages, translated rustc errors). Out: `Diagnostic` / `Report` values, rendered text, applied text edits.

## Invariants
- A code is never renumbered or reused; every code has an entry in `codes_errors.rs` / `codes_lints.rs` with explanation, cause, fix and example (enforced by tests/diagnostics_suite.rs).
- Only `machine_applicable` fixes are ever applied automatically; overlapping fixes are skipped whole.
- Lines/columns are 1-based chars; byte offsets are 0-based half-open.
- A rustc error that has no translation becomes `WJ0099`, never raw rustc text.

## Owns
The schema, the code registry, rustc-to-WJ code and message translation (`rustc_text.rs`), fix application.

## Never decides
Whether a program is correct (producers decide); how files are discovered (see `check`).

## Allowed imports
Layer `tooling`. Reads `error_mapper`, `linter`, `intrinsics`, `source_map` types for adapters.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`codes::explain`, `codes::lookup`, `convert::*`, `render::render`, `apply::{collect_edits, apply_edits}`, `rustc_text::translate`.

## Tests
Unit tests in each file; tests/diagnostics_suite.rs (registry, goldens, schema, false-positive sweep).
Run: `cargo test --release --test diagnostics_suite`.

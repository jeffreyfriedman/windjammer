# src/intrinsics/

## Purpose
Single registry of Windjammer built-in functions (`print`, `println`, `eprintln`, `debug`, `assert*`, `panic`, `todo`,
`unreachable`) and the `wj fix` rewriter that migrates macro spellings (ADR-0014 Phase 1).

## Inputs/Outputs
In: call names and interpolation templates. Out: `Intrinsic` values, format-spec parsing, rewritten source text.

## Invariants
- This is the only place that matches built-in names; backends, interpreter and lints ask `Intrinsic::from_call_name`.
- `wj fix` is idempotent and writes a file only if the result still parses.

## Owns
Names, default panic messages, newline behaviour, template parsing, macro-to-call rewriting.

## Never decides
Backend lowering (each backend/interpreter lowers an `Intrinsic`), ownership, widths.

## Allowed imports
Layer `tooling`: `error_catalog`, `lexer`, `parser`. Contract: docs/MODULE_MAP.toml (tests/architecture.rs).

## Entry points
`Intrinsic::from_call_name`, `template::*`, `surface_fix::*` (src/intrinsics/mod.rs).

## Tests
tests/surface_syntax_phase1_test.rs, unit tests in this module, tests/backend_conformance.rs (`*_intr_*` programs).

## option_fix (ADR-0018 feature 5)
`option_fix::fix_options` is the `wj fix` migration to optional sugar: `Option<T>` -> `T?` and `recv.unwrap_or(d)` /
`unwrap_or_else(|| e)` -> `recv ?? d` (only for a whole operand with an effect-free default). `.unwrap()` is never
rewritten (panic vs propagate). Token based, idempotent, kept only if the file still parses.

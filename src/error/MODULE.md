# src/error/

## Purpose
Compiler error types: `SourceLocation`, rich `CompileError`s with context and fix suggestions.

## Inputs/Outputs
In: messages/locations. Out: error values and suggestions (`suggest_fix`).

## Invariants
- Leaf-level support: should not depend on pipeline stages.

## Owns
Error data model and suggestions.

## Never decides
Detection of errors.

## Allowed imports
Layer `support`. May import: nothing. No known violations.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`error/mod.rs`, `suggestions.rs`, `context_builder.rs`.

## Tests
tests/error_messages_test.rs.
Run: `cargo test --release --test all <filter>`; architecture rules: `cargo test --release --test all architecture`.

## Span validation (`span_check.rs`)
The one place that decides whether a (line, column, byte range) is well-formed for a source text: `validate_span`, `validate_position`, `validate_report_spans` (library validator for the `wj check --json` schema: call it on the report and fail on any returned string), LSP conversions `byte_to_lsp_position` / `lsp_position_to_byte` (UTF-16), and the assertion layer `assert_location` / `assert_extent` (panics on a zero line/column or reversed range in debug builds and in tests that call `set_strict(true)`; `SourceLocation::new` and the parser's diagnostic emitters call it). Convention: lines and columns 1-based, columns in Unicode scalar values, byte offsets 0-based half-open, LSP positions 0-based lines and UTF-16 units.

# src/doctest/

## Purpose
Fenced Windjammer examples: extraction (Markdown and `///`/`//!` doc comments), annotations, checking (parse /
analyze / lint / interpret / compare `// expect:`), and `wj test` doctests (ADR-0018 feature 6).

## Inputs/Outputs
In: Markdown text or `.wj` sources. Out: fences, check results, doctest reports.

## Invariants
- The only fence extractor and annotation parser: `tools/docs_truth` re-exports `fence` and `check`.
- A doctest's expected output comes from its own fence only, never from the module source.
- Doctests run in the interpreter; `--compiled` also runs them through the Rust backend and both must match.

## Owns
Fence syntax and annotations, doctest program composition, doctest reporting.

## Never decides
Language semantics (it runs the real parser/analyzer/interpreter/compiler).

## Allowed imports
`analyzer`, `interpreter`, `lexer`, `linter`, `parser` (layer `tooling`).

## Entry points
`fence::fences`, `fence::parse_info`, `check::check_wj`, `run::run_doctests`, `run::report`.

## Tests
Unit tests in `run.rs`; `tests/doctest_cli_test.rs`; docs-truth (`cargo test --release --test docs_truth_test`).

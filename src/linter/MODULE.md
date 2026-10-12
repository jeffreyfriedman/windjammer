# src/linter/

## Purpose
Warns about inefficient or non-idiomatic Windjammer (and Rust leakage) without blocking compilation.

## Inputs/Outputs
In: `Program` and analyzed functions. Out: warnings with locations and explanations.

## Invariants
- Never rejects a program; produces warnings only.

## Owns
Lint rules and wording; `codes.rs` is the W-code registry (source of the lint table in docs/reference/errors.md).

## Never decides
Compilation success/failure.

## Allowed imports
Layer `tooling`. May import: every module in a lower layer (support < syntax < resolve < types < ownership < ir < backend). No known violations.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`Linter::new`, `lint_program`, `lint_function`, `run_lint_if_enabled` (src/linter/mod.rs). `lint_text(file, source)` is the single lint entry shared by `wj lint`, `wj build` and `wj run --interpret`. `trailing_return` (W0009, `wj fix`) and `missing_value` (W0011) implement ADR-0015 #4.

## Tests
tests/linter_test.rs, tests/linter_tests.rs.
Run: `cargo test --release --test all <filter>`; architecture rules: `cargo test --release --test all architecture`.

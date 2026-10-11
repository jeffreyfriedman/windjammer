# src/linter/

## Purpose
Warns about inefficient or non-idiomatic Windjammer (and Rust leakage) without blocking compilation.

## Inputs/Outputs
In: `Program` and analyzed functions. Out: warnings with locations and explanations.

## Invariants
- Never rejects a program; produces warnings only.

## Owns
Lint rules and wording.

## Never decides
Compilation success/failure.

## Allowed imports
Layer `tooling`. May import: every module in a lower layer (support < syntax < resolve < types < ownership < ir < backend). No known violations.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`Linter::new`, `lint_program`, `lint_function`, `run_lint_if_enabled` (src/linter/mod.rs).

## Tests
tests/linter_test.rs, tests/linter_tests.rs.
Run: `cargo test --release --test all <filter>`; architecture rules: `cargo test --release --test all architecture`.

# src/test_runner/

## Purpose
Discovers and runs Windjammer tests (`wj test`): compiles test files, builds a test crate, runs cargo, reports results.

## Inputs/Outputs
In: `TestRunOptions`, project root. Out: pass/fail report.

## Invariants
- Uses the shared cargo cache dir.
- Rewrites crate imports for generated test crates (`rewrite_test_crate_imports`).

## Owns
Test discovery, execution, reporting.

## Never decides
Compilation semantics.

## Allowed imports
Layer `driver`. May import: every module in a lower layer (support < syntax < resolve < types < ownership < ir < backend < tooling) and peers `build_utils`, `cargo_cache`, `cargo_toml`. No known violations.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`run_tests`, `run_tests_with_options` (src/test_runner/mod.rs).

## Tests
tests/test_runner_library_detection_test.rs and inline tests.
Run: `cargo test --release --test all <filter>`; architecture rules: `cargo test --release --test all architecture`.

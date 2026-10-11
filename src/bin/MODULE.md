# src/bin/

## Purpose
Binary entry points: `wj` (main CLI), `wj-lint`, and their argument/command modules.

## Inputs/Outputs
In: process args. Out: exit status; delegates to the library crate.

## Invariants
- Contains no compiler logic; parse args and call into the library (`cli`, `compiler`).
- Binaries import via the `windjammer::` crate name, so few `crate::` edges appear here.

## Owns
Argument definitions and command wiring for the binaries.

## Never decides
Anything about compilation semantics.

## Allowed imports
Layer `driver`. May import: every module in a lower layer (support < syntax < resolve < types < ownership < ir < backend < tooling). No known violations.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`src/bin/wj.rs` (main), `wj-lint.rs`, `wj_cli_args.rs`, `wj_cli_cmds.rs`.

## Tests
assert_cmd tests of the `wj` binary; tests/cli_lint_test.rs.
Run: `cargo test --release --test all <filter>`; architecture rules: `cargo test --release --test all architecture`.

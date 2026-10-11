# src/cli/

## Purpose
Subcommand implementations of the `wj` binary: build, run, test, check (in-process parse + lint + transpile, `--json` report; no cargo for `.wj` input), fmt, lint, new, add/remove, update, cache, clean, self-install, JSON diagnostics.

## Inputs/Outputs
In: parsed CLI args. Out: process side effects and exit status.

## Invariants
- Offline subcommands are cwd-independent: no runtime reads of templates/std/runtime relative to the cwd or the build checkout (templates in `new_templates.rs`, assets via `crate::embedded_assets`). Gate: tests/installed_binary_smoke_test.rs.
- Thin: delegate to `compiler`, `test_runner`, `cargo_cache`.
- Driver layer: may read env vars and print (ratcheted).

## Owns
User-facing command behavior.

## Never decides
Compilation semantics.

## Allowed imports
Layer `driver`. May import: every module in a lower layer (support < syntax < resolve < types < ownership < ir < backend < tooling) and peers `build_utils`, `cargo_cache`, `cargo_toml`, `test_runner`. No known violations.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
Each `cli/<cmd>.rs::execute` (src/cli/mod.rs routes).

## Tests
tests/installed_binary_smoke_test.rs (installed-binary matrix), tests/cli_lint_test.rs and assert_cmd tests of the `wj` binary.
Run: `cargo test --release --test all <filter>`; architecture rules: `cargo test --release --test all architecture`.

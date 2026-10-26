# src/cli/

## Purpose
Subcommand implementations of the `wj` binary: build, run, test, check (in-process parse + lint + transpile, `--json` report; no cargo for `.wj` input), fmt, lint, reduce (test-case reducer, `src/reduce/`), errors (non-interactive diagnostics browser over `wj check`), new, add/remove, update, cache, clean, self-install, JSON diagnostics.

## Inputs/Outputs
In: parsed CLI args. Out: process side effects and exit status.

## Invariants
- Offline subcommands are cwd-independent: no runtime reads of templates/std/runtime relative to the cwd or the build checkout (templates in `new_templates.rs`, assets via `crate::embedded_assets`). Gate: tests/installed_binary_smoke_test.rs.
- `wj run --interpret` calls `check::require_clean` first: the interpreter never runs a program `wj check` rejects (gate: tests/interpreter_check_parity_test.rs). Inside a project (`wj.toml`) that gate uses the check caches in `<project>/.wj-cache/`; a lone script is checked uncached and nothing is written next to it. Gate: tests/check_cache_differential_test.rs.
- Thin: delegate to `compiler`, `test_runner`, `cargo_cache`.
- Driver layer: may read env vars and print (ratcheted).

## Owns
User-facing command behavior.

## Never decides
Compilation semantics.

## Allowed imports
Layer `driver`. May import: every module in a lower layer (support < syntax < resolve < types < ownership < ir < backend < tooling) and peers `build_utils`, `cargo_cache`, `cargo_toml`, `test_runner`. No known violations.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

- `path_deps::build_missing` (called by `wj build` and `wj test`): a Windjammer package linked as `path = "../pkg/build"` is built (`wj build src --no-cargo` in the package, depth first) before its dependent when `pkg/build/Cargo.toml` is missing or older than the sources; an in-progress marker makes dependency cycles terminate. Gate: `ecosystem_findings2::path_dependencies_build_before_their_dependents`.

## Entry points
Each `cli/<cmd>.rs::execute` (src/cli/mod.rs routes). `run_dev::execute`: `wj run` policy (interpret by default,
compiled fallback on a preflight hit, `--compiled`/`--release`, ADR-0019); `watch::watch`: `--watch` restart loop;
`host_build::ensure`: cached host bridge library for `extern fn` (ADR-0020); `build_caps::prepare_for`: `wj build` stops on capability findings and returns the grant-install statement a program needs (ADR-0021); `caps::execute`: `wj caps show|approve`; `explain_effects::execute`.

## Tests
tests/installed_binary_smoke_test.rs (installed-binary matrix), tests/cli_lint_test.rs and assert_cmd tests of the `wj` binary.
Run: `cargo test --release --test all <filter>`; architecture rules: `cargo test --release --test all architecture`.

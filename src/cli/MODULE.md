# src/cli/

## Purpose
Subcommand implementations of the `wj` binary: build, run, test, check (in-process parse + lint + transpile, `--json` report; no cargo for `.wj` input), fmt, lint, reduce (test-case reducer, `src/reduce/`), errors (non-interactive diagnostics browser over `wj check`), optimize (`wj optimize`: per-pass report of the post-codegen optimizer with soundness sections, docs/economics/COMMANDS.md; pass table checked by tests/optimize_cli.rs), new, add/remove, update, cache, clean (`tmp_janitor.rs`: `wj clean --tmp` removes stale `wj-*` scratch dirs, dry run unless `--yes`; gate: tests/tmp_hygiene_test.rs), self-install, JSON diagnostics.

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

- `replay.rs` (ADR-0035): `wj replay LOG PROGRAM` / `wj replay --simulate PROGRAM`: reads the command script, `check::require_clean`, runs `interpreter::debugger::session`, prints events (text or `--json`). Gate: tests/time_travel_test.rs.
- `widths_gate.rs` (ADR-0033): WJ0003 predictions for drift families A (widths), B (a reference expected) and C (`String`/`&str`); the in-process generated Rust of each file is a trigger, and only clashes in the Rust the build emits are reported, which the build's own pipeline (`compiler::build_project_with_config`, configured as `wj build` is: `build::with_project_facts`) generates in this process with its narration silenced by `silenced.rs` (a child `wj build --no-cargo` only where the platform cannot silence it or the in-process build fails) (`wj check`: errors; `wj run --interpret`: warnings before the run). Gates: tests/width_prediction_test.rs, tests/prediction_parity_test.rs (in-process Rust equals the child build's, predictions equal, mutation check).
- `silenced.rs` (ADR-0033): runs a closure with stdout and stderr pointed at the null device (unix; `None` elsewhere), so an in-process build does not narrate into a program's output.
- `speculate.rs` (ADR-0033): `wj run --interpret --speculate`: a detached, `nice`d, offline `wj build` of the same content that records the compiled verdict; skipped when the verdict is known, another one runs (pid file), the load average reaches the core count, or the project has Rust crate dependencies. Gate: tests/speculate_test.rs.
- `verify.rs` (ADR-0033): `wj run --verify`: interpreted run (child, stdout captured once), the build `wj run --compiled` does (`run::build_native`, via `run_dev::build_input`), and a second, compiled run only when `main`'s effects are console and `/dev/stdin` and stdin was captured; reports on stderr. Gate: tests/run_verify_test.rs.
- `run.rs`: `wj run --compiled`; build progress only when `run::Output.progress` (the driver sets it: a terminal, or `WJ_PROGRESS`), and the run step never replays rustc's warnings about the generated Rust (ADR-0037). Gate: tests/agent_cost_levers_test.rs.
- `verdict.rs` (ADR-0033): compiled-build verdicts keyed on SHA-256 of the compiler identity and every program input (`check::program_inputs`), one JSON file each under `<cache root>/verdicts/` (newest 512 kept). `wj build` and `wj run --compiled` record success or rustc errors (a failure without a rustc error records nothing); `wj run --interpret` prints a stderr note after the program when the compiled build of the same content failed. Gate: tests/verdict_cache_test.rs.

## Entry points
Each `cli/<cmd>.rs::execute` (src/cli/mod.rs routes). `run_dev::execute`: `wj run` policy (interpret by default,
compiled fallback on a preflight hit, `--compiled`/`--release`, ADR-0019); `watch::watch`: `--watch` restart loop;
`host_build::ensure`: cached host bridge library for `extern fn` (ADR-0020); `crate_host_build::ensure`: cached crate host binary for surfaced Rust crates (ADR-0031, `wj run`); `build_caps::prepare_for`: `wj build` stops on capability findings and returns the grant-install statement a program needs (ADR-0021); `caps::execute`: `wj caps show|approve`; `explain_effects::execute`; `rust_surface::execute`: `wj rust-surface` (ADR-0026; `build` calls `package::rust_surface_gen::cache::ensure` before the crate-call check); `mcp_bind::generate_only`: typed bindings for MCP tools; `code_mode::execute`/`Policy`: run an agent's program in the sandbox (the `windjammer-mcp` `execute` tool).

## Tests
tests/installed_binary_smoke_test.rs (installed-binary matrix), tests/cli_lint_test.rs and assert_cmd tests of the `wj` binary.
Run: `cargo test --release --test all <filter>`; architecture rules: `cargo test --release --test all architecture`.

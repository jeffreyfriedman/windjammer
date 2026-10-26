# src/check/

## Purpose
`wj check`: the compiler front end as a standalone checker. Lex, parse, lints, surface-syntax findings, name resolution, conservative semantic checks, analyzer, numeric inference and IR solvers; no cargo, no rustc, no generated code.

## Inputs/Outputs
In: a `.wj` file or project directory. Out: `Checked { report, sources }` in the shared diagnostics schema.

## Invariants
- Never invokes cargo or rustc.
- Semantic checks are conservative: a program the build accepts must pass (`check_has_no_false_positives_on_the_build_corpus`).
- The result cache is keyed on every input (all paths + contents + compiler build); any change recomputes everything.
- `--fix` applies only machine-applicable fixes and refuses results that no longer parse.

## Owns
Check orchestration, effect findings (`effects.rs`: WJ0040-WJ0048 from `ir::effect_facts`, `durable_sites.rs`: journaled step positions for the interpreter, `wj.toml [capabilities]`/`[policy]`, per-dependency grants via `dep_sources.rs`), file discovery, source locating (`locate.rs`), semantic checks (`semantic/`), result cache, watch loop, fix application.

## Never decides
Diagnostic wording for rustc errors (see `diagnostics/rustc_text.rs`); codegen.

## Allowed imports
Layer `tooling`. Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`check_path`, `check_sources`, `apply_fixes`, `watch::{snapshot, wait_for_change}`; CLI in src/cli/check.rs.

## Tests
Unit tests in `locate.rs`, `files.rs`, `watch.rs`, `semantic/`; tests/diagnostics_suite.rs.

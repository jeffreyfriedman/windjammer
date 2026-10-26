# src/check/

## Purpose
`wj check`: the compiler front end as a standalone checker. Lex, parse, lints, surface-syntax findings, name resolution, conservative semantic checks, analyzer, numeric inference and IR solvers; no cargo, no rustc, no generated code.

## Inputs/Outputs
In: a `.wj` file or project directory. Out: `Checked { report, sources }` in the shared diagnostics schema.

## Invariants
- Never invokes cargo or rustc.
- Semantic checks are conservative: a program the build accepts must pass (`check_has_no_false_positives_on_the_build_corpus`).
- Caches never change diagnostics (gate: tests/check_cache_differential_test.rs, with a recorded mutation check in ADR-0020 round 3 rung 4). Two layers, both under `<project>/.wj-cache/`:
  - `cache.rs`, whole run: SHA-256 over all paths + contents, manifest, lock file, dependency sources, plus the compiler build and the ambient `.wj.meta` sidecars (`ambient.rs`); any change anywhere is a miss.
  - `file_cache.rs`, per file: the analysis phase (analyzer, numeric inference, IR solvers) reads one file and the stdlib only, so it is keyed on path + SHA-256 of the text, under a header of build + ambient digest. Resolution, effects and the semantic index always run. If the analysis phase ever reads another file or project signatures, that input must join the key.
- `ambient.rs`: the numeric inference reads `.wj.meta` sidecars relative to the working directory; their digest is part of both keys, and a walk over budget disables caching for that run.
- `--fix` applies only machine-applicable fixes and refuses results that no longer parse.

## Owns
Check orchestration, effect findings (`effects.rs`: WJ0040-WJ0048 from `ir::effect_facts`, `durable_sites.rs`: journaled step positions for the interpreter, `wj.toml [capabilities]`/`[policy]`, per-dependency grants via `dep_sources.rs`), file discovery, source locating (`locate.rs`), semantic checks (`semantic/`), result caches (`cache.rs`, `file_cache.rs`, `ambient.rs`), watch loop, fix application.

## Never decides
Diagnostic wording for rustc errors (see `diagnostics/rustc_text.rs`); codegen.

## Allowed imports
Layer `tooling`. Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`check_path`, `check_sources`, `apply_fixes`, `watch::{snapshot, wait_for_change}`; CLI in src/cli/check.rs.

## Tests
Unit tests in `locate.rs`, `files.rs`, `watch.rs`, `semantic/`; tests/diagnostics_suite.rs; tests/check_cache_differential_test.rs (caches).

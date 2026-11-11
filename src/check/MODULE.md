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

- `rust_calls.rs` (ADR-0026): calls into a Rust crate with a surface are checked against it (`WJ0050` undeclared or malformed surface, `WJ0013` arity, `WJ0003` literal kind); `rust_crate_calls` is the subset `wj build` stops on. Calls into a crate with no surface: `W0014` with the `wj rust-surface` command; a skipped generated function: `WJ0050` with the generator's reason. `std_calls.rs`: `WJ0002` for a call to a function a std module does not provide (`strings.index_of` after `use std::strings`; locals shadow the module name; gate tests/std_calls_check_test.rs). `rust_surface_health.rs`: `W0015` when a generated surface names another crate version than `Cargo.lock`. Surface files (hand-written and cached) and lock files are inputs of the result cache.

- `moves.rs` (ADR-0030, WJ0007): uses after a move predicted from the Rust generated in the per-file analysis phase. `widths/` (ADR-0033, WJ0003): a walker over generated Rust that finds two integer widths that must be one type, `String` against `&str`, or a value where a reference is required (drift families A, B, C), only where both types are certain; pure text in, clashes out. The driver (`cli/widths_gate.rs`) runs it on the build's emitted Rust, because the in-process generator's widths differ from the build's. Gates: tests/move_prediction_test.rs, tests/width_prediction_test.rs (precision scans with rustc as the oracle, pinned lists).
- `local_mut/` (ADR-0002, WJ0051): local bindings changed without `mut`, over the whole program at once (two analyzer passes with all files' signatures, then the IR solver: `facts.rs`); a method changes its receiver when the solver gives it `&mut self` or `std/rust_core.wji` declares `&mut self`; types of locals from declarations only (`types.rs`); the scoped walk (`walk.rs`); the declaration's text position for the fix (`decl.rs`). `local_mutability(path)` is the same report for `wj build`, `wj run` and `wj fix`. Gate: tests/local_mutability_test.rs.
- `program_inputs`/`compiler_identity`: the input list and build identity that also key compiled verdicts (`cli/verdict.rs`).

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

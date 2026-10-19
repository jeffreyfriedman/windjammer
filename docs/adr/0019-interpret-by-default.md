# ADR-0019: `wj run` interprets by default; compiled is the release path

- **Status:** Accepted (2026-10-11)
- **Date:** 2026-10-11
- **Related:** [ADR-0010](0010-package-source-of-truth.md), [ADR-0020](0020-interpreter-rust-crate-bridge.md), [ADR-0021](0021-hot-reload.md), `src/cli/run_dev.rs`, `src/interpreter/{project,preflight,deps}.rs`, `src/module_system/{project,sources}.rs`, `docs/INTERPRETER.md`

## Context
`wj run` used to transpile, `cargo build` and run: seconds to minutes per edit. The interpreter runs the same
source in milliseconds but was opt-in (`--interpret`) and could not load multi-module projects
(`use util::math` then `math::square(7)` failed). The goal is a `go run`-like loop: `wj run` in any directory of
a project runs the whole app instantly, and `wj build` / `wj run --release` produce the compiled binary with
identical behaviour.

## Decision
1. **One module system.** The interpreter loads programs through the compiler's file walker
   (`module_system::sources`, also behind `compiler::find_wj_files`) and binds every `use` with the compiler's resolver
   (`resolve`, `DefId`s). Visibility uses Rust's rule on resolved definitions and reports rustc's code (E0603).
   There is no second resolver to drift.
2. **Project discovery like go.** `wj run` with no path finds `wj.toml` upward and runs the default binary;
   `wj run DIR` runs `DIR/main.wj` or `DIR/src/main.wj`; `--bin NAME` selects `[[bin]]`, `cmd/NAME/main.wj` or
   `src/bin/NAME.wj`. Arguments pass through, the exit status is the program's (101 panic, N for `process::exit(N)`),
   stdin is inherited. Shared by both paths (`module_system::project`).
3. **Interpret by default, fall back loudly.** `wj run` interprets. When the program uses a feature the interpreter
   does not run (an `Unsupported` entry of `src/interpreter/features.rs`), the **preflight** detects it at load time,
   before `main` runs, and `wj run` switches to the compiled path with one line on stderr:
   `wj: running compiled (cargo): not interpreted yet: ... [feature.id] (file:line:col)`.
   `--interpret` disables the fallback (exit 2), `--compiled` / `--release` force cargo. `wj build` is unchanged.
4. **Dependencies are one graph.** `package::graph` resolves `wj.toml` path and git dependencies (git pinned by
   `wj.lock`); Windjammer packages are loaded as source and mounted as crates; Rust crates go through the host bridge
   (ADR-0020) or trigger the fallback.

## Parity gates (must stay green; a red gate blocks the default)
| Gate | Where | What it proves |
|---|---|---|
| Feature registry | `cargo test --release --test interpreter_features` | every `Supported` probe prints its expected output; every `Unsupported` probe is rejected **at load time** by the preflight (so it triggers the fallback), and no `Supported` probe is |
| Project fixtures | `cargo test --release --test interpreter_projects` | each fixture in `tests/interpreter_projects/` matches its hand-written stdout/exit/stderr interpreted, with the default `wj run` (and never falls back), and compiled (`WJ_PROJECT_COMPILED=1`) |
| Module fuzzer | same binary, `fuzz_*` | 40 seeded random module trees (nesting, `mod.wj`, aliases, groups, globs, `crate`/`super` paths, same-name functions) match the generator's independent model interpreted; the first seeds also compiled (`WJ_PROJECT_COMPILED=1`) |
| Differential corpus | `cargo test --release --test all interpreter_differential` | conformance programs: interpreter == Python reference models == compiled Rust |
| No cargo in the loop | `interpreter_cli_test` | the interpreted path never spawns cargo/rustc |

A runtime `Unsupported` that the preflight did not anticipate exits 2 with the feature named and
`wj run --compiled` suggested: never silent divergence, but a registry/preflight gap to close (add the feature to
the registry with a load-time detector).

## Consequences
- Edit-run latency for an interpretable project drops from cargo time to tens of milliseconds (numbers in `docs/INTERPRETER.md`).
- Programs using unsupported features keep working (compiled), with a visible notice instead of an error.
- New unsupported features must come with a preflight detector, or the gate test fails.
- Known compiled-path gaps found while building the gates are tracked in `docs/INTERPRETER.md` ("Projects").

## Kill criteria
Revert the default (keep `--interpret` opt-in) if any gate stays red for more than one release, or if a user-visible
divergence between interpreted and compiled output on a gated program ships.

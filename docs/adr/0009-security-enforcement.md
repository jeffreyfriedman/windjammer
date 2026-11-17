---
status: proposed
last_verified: 3406ef05bb35
---
# ADR-0009: Security enforcement level

- **Status:** Proposed
- **Date:** 2026-10-10
- **Related:** [ADR-0021](0021-effects-and-capabilities.md) (the effects and capabilities mechanism this level applies to); RFCs [WJ-SEC-01..04](../proposals/rfcs/SECURITY-FRAMEWORK-OVERVIEW.md); `src/ir/effects.rs`, `src/ir/taint.rs`, `src/ir/capability_profiles.rs`, `src/output_generation.rs`, `src/diagnostics/`, `src/config.rs`

## Context
Four security RFCs are "Draft" but CHANGELOG 0.50.0 lists them as added. What is actually enforced:
- **Effects (SEC-01):** `EffectSolver` runs in `IrPipeline`. `output_generation.rs` fails the build with
  "capability violation(s)" only when `wj.toml` has an `[app_capabilities]` section. Without it there is no check. Applies to the
  application, not per dependency.
- **Taint (SEC-02):** `TaintSolver` runs by default and produces error-severity diagnostics, but the only consumer of
  `taint_errors` outside the pipeline is `src/diagnostics/`. No build-failing path was found, so the CHANGELOG line
  "compile-time errors for unsanitized data" is not true for the normal `wj build`.
- **Lock file (SEC-03):** The lock-file module was removed with the legacy capability code; the lock file is not yet implemented. No `.wj-capabilities.lock` is read or written by the CLI.
- **Profiles (SEC-04):** `capability_profiles.rs` has five built-in profiles; not wired to anything.
- Unknown external calls (for example Rust crates through `extern`) have no effect classification, so a call can bypass effect checking.

## Options
### A. Fail the build by default, per-dependency
Effects: a project with no `[app_capabilities]` gets a conservative default (pure + stdout). Taint errors fail the build. Lock file records effects per dependency and escalation fails CI until approved. Unknown externals are `unknown` (denied unless declared).
- Pros: matches the RFC pitch. Cons: breaking for existing projects; needs dependency resolution (ADR-0010) first for per-dependency.
### B. Warn by default, fail with `--strict-security` / `[security] enforce = true`
- Pros: gradual adoption. Cons: a flag that nobody sets protects nobody; two modes to test.
### C. Per-crate (app only) enforcement, no dependency tracking yet
Wire effects and taint to fail the build for the application; defer the lock file until path/git dependencies build.

## Recommendation
Option C now, converging on A: (1) make taint errors fail `wj build`; (2) make the absence of `[app_capabilities]` produce a warning that lists the inferred effects; (3) classify unknown external calls as a distinct `unknown` effect; (4) implement the lock file once ADR-0010 lands. Do not ship a warn-only mode (B) as the documented default.

## Consequences
- CHANGELOG/README stop claiming enforcement that is not wired; RFC statuses become Implemented-Partial.
- Stdlib annotations for effects become a compatibility surface.
- `--ir-shadow-validate` and `--json` outputs need the same diagnostics as the console.

## Decision needed from: project owner
Choose the default level (fail vs warn), and whether enforcement is per-crate or per-dependency in the first release.

## Amendment: taint errors fail the build (applied)
Recommendation item (1) is implemented: `WJ0173` ("untrusted data reaches a command, file path or query") is reported by `wj check`, `wj build` and `wj run` (interpreted and compiled), and an error stops the build before code generation. The earlier IR `TaintSolver` never connected a source to a sink (its variables were named per call site, with no flow between them), so no `Taint:` error could be produced; the analysis that reports `WJ0173` is `src/check/taint/` and the IR solver's diagnostics are no longer consumed by `wj check`.
- *What flows.* Every function is walked with symbolic parameters and summarised (what reaches its result, which parameters reach a sink); summaries are iterated to a fixed point, so a flow through helpers, methods and recursion is found and is reported at the call that hands the value over, with the sink named in a note. Struct fields are tracked apart (a page's file name and its body are different values).
- *Sources:* file contents, standard input, an HTTP request, a database row. **The environment and the command line are not sources**: they belong to the operator who starts the program and who already holds all of its authority, so a command or path they name is not an injection. (A first precision run flagged 40+ tools that write to a path given on the command line; none was a vulnerability.) A server that accepts environment-like input from clients, such as CGI variables, reads them through the request type, which is a source.
- *Sinks* come from the effect table: the command of a process (`process::run*`; the arguments of `run_with_args` are not a shell and are not sinks), the path of a file write, the SQL text of `Connection::query`/`execute` (parameters are not).
- *Sanitizers:* std `regex::escape`, `path::file_name`, and a function documented `/// wj-taint: sanitizer` (its result is clean for every sink). `/// wj-taint: trusted(<reason>)` reports nothing inside one function; the reason is required and is the review record. Neither is a blanket allow: both name a function, and `tests/taint/precision_ledger.tsv` lists every remaining finding of the corpus with its reason.
- *Gates:* `tests/taint_flow_test.rs` over the labeled cases in `tests/taint/cases`, and `scripts/wj/taint_precision.wj` (tests, examples, scripts, the pinned ecosystem snapshot and an ecosystem checkout; the ledger only shrinks).
- *Still open:* items (2) to (4) of the recommendation (the `[app_capabilities]` warning is covered by ADR-0021; the lock file per dependency is part of `wj.lock`, ADR-0010).

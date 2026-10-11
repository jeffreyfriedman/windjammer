# ADR-0009: Security enforcement level

- **Status:** Proposed
- **Date:** 2026-10-10
- **Related:** RFCs [WJ-SEC-01..04](../proposals/rfcs/SECURITY-FRAMEWORK-OVERVIEW.md); `src/ir/effects.rs`, `src/ir/taint.rs`, `src/ir/capability_lock.rs`, `src/ir/capability_profiles.rs`, `src/output_generation.rs`, `src/cli/json_diagnostics.rs`, `src/config.rs`

## Context
Four security RFCs are "Draft" but CHANGELOG 0.50.0 lists them as added. What is actually enforced:
- **Effects (SEC-01):** `EffectSolver` runs in `IrPipeline`. `output_generation.rs` fails the build with
  "capability violation(s)" only when `wj.toml` has an `[app_capabilities]` section. Without it there is no check. Applies to the
  application, not per dependency.
- **Taint (SEC-02):** `TaintSolver` runs by default and produces error-severity diagnostics, but the only consumer of
  `taint_errors` outside the pipeline is `cli/json_diagnostics.rs`. No build-failing path was found, so the CHANGELOG line
  "compile-time errors for unsanitized data" is not true for the normal `wj build`.
- **Lock file (SEC-03):** `src/ir/capability_lock.rs` defines the format; nothing outside `src/ir/mod.rs` references it. No `.wj-capabilities.lock` is read or written by the CLI.
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

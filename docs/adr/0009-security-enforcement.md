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

## Amendment: the bounds of the taint walk are sound (applied)
`WJ0173` is a security check, so every bound that keeps the walk linear must over-approximate: a value may only gain origins, never lose one, so a flow that exists is never dropped. The analysis is a monotone walk over sets of origins ordered by inclusion; each bound maps a set to a superset of itself.
- *Wrapping depth* (`MAX_WRAP_DEPTH`): a value wrapped deeper than three levels is flattened to its base origins. `select(field)` keeps a flattened origin for every field, where the wrapped form would have kept it for one field only: a superset.
- *Parameter paths* (`MAX_PATH`): `Param(i, p)` stands for everything reachable from member `p` of parameter `i`; a truncated path stands for the whole sub-structure, which includes every deeper member.
- *Origins per value* (`MAX_ORIGINS`, 48): a larger set collapses to TOP, the function's `top` set: every std source call in its text, every source a callee's summary returns, what an HTTP request parameter yields, and every non-scalar parameter. Any origin a value of that function can hold is in `top` (origins arise only from those four places), so a sink fed by a collapsed value still fires. Tests: `flag_more_than_48_origins_merge`, `flag_one_tainted_origin_among_many`.
- *Loops:* the body is walked until a pass adds nothing, which is the least fixpoint of the join, so a flow carried from one iteration to the next however many assignments long is found (`flag_carry_chain_through_six_variables`). After four passes that still change something, and for every loop nested more than three deep, each variable the body assigns (or stores into) first receives TOP and the body is walked once more: at that point a later iteration cannot hold anything TOP does not (`flag_carried_across_five_nested_loops`, `flag_carry_chain_in_five_nested_loops`, `flag_source_in_innermost_sink_after_outer_loop`). The earlier version walked every loop twice, which missed a carry chain longer than two assignments; that was an unsoundness of the first version, not only of the bound.
- *Sanitizers* are declarations (the std table, `/// wj-taint: sanitizer`): their result is clean for every input, TOP included, exactly as for any value. The bounds neither make a function a sanitizer nor stop one from being one; a sanitizer's input is never "proved" clean, it is trusted by declaration.
- *Gates:* the oracle cases above fail under the mutants "loops walked twice, deep loops once, no havoc" (the two chain cases) and the performance cases fail under "no bounds" (`tests/perf_ceiling`, `scripts/wj/build_ceiling_gate.wj`: t307_mini_scheme and a reduction of its eval/special core, each 25 s and 1.3 to 2.3 GB without the bounds, 0.4 s and 27 MB with them).
- *Not modelled, independent of the bounds* (known false negatives): a user method that stores one of its parameters into `self` is not seen as a store into the receiver; flows through function values held in data; mutating std methods that are not in the storing-method table; the declarations of sanitizers and narrowing methods are trusted.

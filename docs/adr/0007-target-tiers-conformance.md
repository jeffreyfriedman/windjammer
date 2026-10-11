# ADR-0007: Target tiers and conformance contract

- **Status:** Proposed
- **Date:** 2026-10-10
- **Related:** [design/multi-target-codegen.md](../design/multi-target-codegen.md), [EXPORT_TARGETS.md](../EXPORT_TARGETS.md), `tests/conformance/README.md`, `src/codegen/{rust,go,javascript,wasm.rs,wgsl}`, `src/interpreter/`

## Context
`wj build --target` accepts `rust` (default), `javascript`, `wasm` (`src/bin/wj_cli_args.rs`); the legacy `windjammer`
binary also lists `go`, `node`, `python`, `c` (the last three marked "future"). Backends differ widely in maturity:
- Rust: production; consumes the IR solver.
- Go and JavaScript: call `prepare_ir_context` and discard the result (`let _ir = ...`), so they do their own
  ownership-free translation. Go is labelled "experimental" in the CLI.
- WASM: goes through the Rust generator (`src/codegen/wasm.rs`, component generator).
- WGSL: a separate shader path (WJSL), with three overlapping shader stacks noted in the plan.
- Interpreter (`src/interpreter/`, `wj repl`): exists, undocumented as a target.
`tests/conformance/` holds a small set of `.wj` programs with expected stdout and a runner script that today compiles only to Rust;
its README says every backend must give identical output, but nothing enforces that.

## Options
### A. Explicit tiers gated by conformance
- Tier 1 (Rust): full language, every release gate.
- Tier 2 (WASM, JS): conformance suite must pass for the documented subset; failures block promotion, listed in a per-target skip file.
- Tier 3 (Go, WGSL, interpreter): experimental; no stability promise; may be removed.
A target is promoted when it consumes the IR (ADR-0001) and passes the suite.
### B. All backends equal
Pros: simple message. Cons: untrue today; multiplies cost of every language change.
### C. Rust only; remove Go/JS/WASM
Pros: focus. Cons: discards WASM/web value and existing work; contradicts the multi-target pitch.

## Recommendation
Option A. Make `tests/conformance` multi-target (one expected-output file, N runners), add a per-target subset manifest, and
require new language features to ship with conformance cases before any backend claims support. Documentation states the tier of
each target.

## Consequences
- README lists tiers (done in this change, with "unverified" for non-Rust targets).
- CLI help marks Tier 3 targets experimental; unsupported target names (`node`, `python`, `c`) are removed or error clearly.
- Backend trait over `IrModule` (plan WP 11) is the promotion mechanism.

## Decision needed from: project owner
Approve the tier assignment (especially JS and Go) and the rule that conformance gates promotion.

## Amendment: interpreter tier (applied)
*Interpreter tier.* The interpreter (`wj run --interpret`, `wj test --interpret`, `wj repl`) is a Tier-2 development
target with a narrower contract than the compiled targets: it implements the language subset listed in
`docs/INTERPRETER.md` with Rust-identical stdout, panic behaviour and exit codes (debug-build integer semantics), and reports any
other construct as `unsupported in the interpreter` (exit 2). It is gated by `tests/interpreter_differential_test.rs`
(independent Python oracles plus compiled Rust) and `tests/interpreter_semantics_test.rs`; a regression in either blocks a
release. It is not a release path; `wj build` remains the production route. (This supersedes the "Tier 3" placement of the
interpreter in Option A above.)

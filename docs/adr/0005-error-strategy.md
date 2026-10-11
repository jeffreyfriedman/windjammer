# ADR-0005: Error strategy: front-end checking vs rustc translation

- **Status:** Proposed
- **Date:** 2026-10-10
- **Related:** [design/error-mapping.md](../design/error-mapping.md), `src/error_mapper.rs`, `src/source_map.rs`, `src/error_catalog.rs`, `src/cli/check.rs`

## Context
Windjammer diagnoses mistakes in two ways. The parser, the linter (`W0001-W0004` Rust-leakage rules) and some IR
checks (effects, taint) report in `.wj` terms. For type and borrow errors the pipeline emits Rust, runs `cargo`, and
`src/error_mapper.rs` translates rustc JSON back to `.wj` locations using `source_map.rs`. Some failures cannot be
translated and surface as rustc errors on generated code. The IR also emits `compile_error!("missing boundary signature
for ...")` into generated Rust when it cannot resolve a callee (documented in IR_SOLVER_CODEGEN_ARCHITECTURE.md), which
defers a compiler error to rustc. `wj check` (`src/cli/check.rs`) is only `cargo check` in the current directory; it does not check `.wj` files at all. The user promise
("compiler does the hard work", "no Rust leakage") is weakest here.

## Options
### A. Front-end first (own the diagnostics)
A real checker (resolve, types, ownership facts) rejects common errors before codegen. rustc errors become an
internal-error class ("compiler bug, please report") with source-map translation as a fallback.
- Pros: fast `wj check`, stable errors, works for non-Rust targets. Cons: large investment; duplicates parts of rustc; depends on name resolution (plan WP 6, 7).

### B. Translation only (current)
Keep generating Rust and translating. Pros: no duplicate checker. Cons: slow, leaks Rust concepts, unusable for Go/JS targets, error quality capped by source-map precision.

### C. Hybrid with an explicit contract
Front end must prove: unresolved names, arity, obvious type mismatches, mutation of immutable data (ADR-0002), effect/taint violations. Everything else may fall to rustc, but every rustc fallback is counted as a diagnostic-coverage gap.

## Recommendation
Option C, aiming at A over time. Treat each `compile_error!` and each rustc-leaked code as a tracked gap in
`docs/known-gaps.md`. Make `wj check` a real front-end check of `.wj` sources (no cargo) and the primary CI signal.

## Consequences
- Name resolution (`DefId`, plan WP 6-7) becomes a prerequisite for most front-end checks.
- `compile_error!` emission in codegen is reclassified as "internal error" with a stable code.
- Error catalog (`src/error_catalog.rs`, `docs/errors/`) gains a "front-end | rustc-mapped" tag per code.

## Decision needed from: project owner
Confirm C, and the invariant that user programs should never need to read a rustc message.

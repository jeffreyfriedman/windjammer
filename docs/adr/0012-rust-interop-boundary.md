# ADR-0012: Rust interop boundary

- **Status:** Proposed
- **Date:** 2026-10-10
- **Related:** [WJ-IMPL-02](../proposals/rfcs/WJ-IMPL-02-ffi-generation.md), `src/ffi/`, `src/cargo_toml/`, `src/config.rs`, `src/ejector.rs`, ADR-0008, ADR-0009

## Context
README promised "100% Rust compatibility - use any Rust crate". What exists: an `extern` keyword in the lexer and parser (declarations
of external functions), `DependencySpec` entries forwarded into the generated `Cargo.toml` (including a `package` rename), `windjammer-runtime`
as the standard Rust-side support crate, `src/ffi/` for C bindings from an IDL (WJ-IMPL-02), and `windjammer eject` (legacy binary only, not `wj`) that emits a Rust
project. What is undefined: how a `.wj` file names a Rust type or trait from a crate, who writes the signatures the ownership solver needs (the IR
doc shows missing signatures turn into `compile_error!("missing boundary signature ...")`), what effect an external call has (ADR-0009), and how the Rust
boundary works on non-Rust targets (ADR-0007).

## Options
### A. Declared boundary: `extern` signatures plus Cargo passthrough
Rust crates are usable only through `.wj` `extern` declarations (functions, types, traits) that give the solver complete signatures and an effect annotation.
`wj.toml` `[dependencies]` entries with a `rust` marker are forwarded to Cargo. No inline Rust. Non-Rust targets reject `extern` blocks tagged `rust`.
### B. Implicit interop
Any path `crate::item` is assumed to exist; the compiler guesses signatures from usage. Pros: friction-free. Cons: the source of the boundary-signature errors and name-based guessing the plan forbids.
### C. Inline Rust blocks (`rust { ... }`)
Pros: escape hatch. Cons: leaks Rust, breaks all other targets, defeats effect and ownership analysis.

## Recommendation
Option A. Generate the `extern` declarations from `cargo doc`/rustdoc JSON where possible (tool, not language feature); require an effect annotation on every extern function (default `unknown`, ADR-0009). Keep `eject` as the documented exit and port it into `wj` (or remove the claim).

## Consequences
- "Use any Rust crate" becomes "use any Rust crate you wrap" in docs (done).
- Stdlib wrappers (ADR-0008) are the first consumers of the extern mechanism.
- Boundary tests: a generic crate + extern file compiles, ownership of args matches the declared signature.

## Decision needed from: project owner
Choose A, B or C; confirm `eject` is a supported feature and should move into `wj`.

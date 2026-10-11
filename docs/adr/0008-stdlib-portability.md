---
status: proposed
last_verified: 3406ef05bb35
---
# ADR-0008: Stdlib portability

- **Status:** Proposed
- **Date:** 2026-10-10
- **Related:** [STDLIB_ARCHITECTURE.md](../STDLIB_ARCHITECTURE.md), `std/README.md`, `crates/windjammer-runtime`, ADR-0007, ADR-0012

## Context
`std/*.wj` is written as thin wrappers over Rust crates ("wraps serde_json", "wraps reqwest/hyper", "wraps chrono")
backed by `crates/windjammer-runtime`. STDLIB_ARCHITECTURE.md forbids leaking the underlying crate API, which is good for
stability, but the implementation is Rust-only: `fs`, `http`, `net`, `process`, `subprocess`, `db`, `crypto` have no meaning
on WGSL, little on WASM, and no Go/JS runtime counterpart. There are two overlapping `stdlib_method_traits` modules
(`src/analyzer/` and `src/codegen/rust/`) that encode std behaviour as Rust-specific method lists. `tests/conformance/stdlib`
exercises only a few modules.

## Options
### A. Portable core + host modules
Split `std` into `core` (pure `.wj`: collections, strings, math, option/result, iterators, time arithmetic), which every Tier 1/2
target must implement, and host modules (`fs`, `net`, `http`, `process`, `db`) tagged with the effects they require (WJ-SEC-01)
and the targets that provide them. Importing a host module on an unsupported target is a compile error naming the module.
### B. Rust-only stdlib; other targets get their own libraries later
Pros: no work now. Cons: every non-Rust target silently lacks the library; docs overpromise.
### C. Single runtime ABI (extern) all targets link
Pros: uniform. Cons: a large runtime per target; slows WASM/JS; heavy design.

## Recommendation
Option A. Declare capabilities per std module in metadata (`stdlib_meta/` already exists), generate "available on" tables from it,
and gate each module by tier (ADR-0007). Replace the duplicated method-trait tables with signatures declared in `.wj`.

## Consequences
- Std modules need a portability tag and conformance coverage before they can be called "Tier 2".
- The Rust-crate-wrapper policy stays; wrapper crates are internal detail of the Rust target.

## Decision needed from: project owner
Choose A, B or C and the exact contents of the portable core.

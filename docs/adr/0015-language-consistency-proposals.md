# ADR-0015: Language consistency proposals (numerics, casts, paths, returns)

- **Status:** Proposed (2026-10-11) — each numbered decision is accepted or rejected independently by the owner.
- **Date:** 2026-10-11
- **Related:** [ADR-0014](0014-surface-syntax-rust-isms.md), [ADR-0002](0002-local-mutability.md), [ADR-0012](0012-rust-interop-boundary.md), `docs/ARCHITECTURE_IMPROVEMENT_PLAN.md` (WP6-8)

## Context
Most recent compiler fix commits are integer-width and clone corrections; Rust conventions that Windjammer
inherits (width zoo, truncating `as`, `::` paths) push complexity into inference. This ADR lists candidate
simplifications, each independent.

## 1. One default numeric type (recommended)
`int` = 64-bit, overflow-checked; `float` = f64. Indexing and `len()` accept `int` via a checked conversion.
Explicit widths (`u8`, `i32`, `usize`) only at FFI/binary-layout/perf boundaries.
- Pros: removes the usize/i32 cast churn and most of numeric-solver complexity (WP8); same semantics on Rust/Go/JS (JS needs BigInt or checked 53-bit policy — see ADR-0007).
- Cons: 64-bit everywhere costs memory in large arrays (use explicit widths there); behaviour change for existing sources.

## 2. Checked conversions instead of `as` (recommended)
`int(x)`, `float(x)` convert losslessly or fail/panic when out of range; `x.truncate()`/`x.wrapping()` are explicit.
Pros: no silent truncation. Cons: migration via `wj fix`.

## 3. `to_string(self)` auto-provides Display/interpolation (recommended)
Defining `fn to_string(self) -> string` makes the type printable/interpolatable (like derives). Removes `impl Display` boilerplate.

## 4. `::` to `.` (deferred)
Needs authoritative name resolution (a.b.c is ambiguous: module, type, variant, field) and a turbofish replacement
(`parse::<T>()`). Revisit after WP7; then at most accept `.` as an alias with a formatter preference. Cosmetic benefit
does not justify risk before the resolver lands.

## 5. Implicit final expression stays (recommended)
No forced `return`: `if`/`match`/closures/pipes are expressions. Add diagnostics: declared return type with a
non-value last statement or a non-returning path is an error with a span; lint against `return x` as the final
statement. `return` remains for early exit.

## 6. Lower priority
`if x is Some(v)` for `if let`; documented string byte-vs-char indexing rule; no `unsafe` in `.wj` (use `rust { }`, ADR-0012).

## Decision needed from: project owner
Accept/reject 1, 2, 3, 5 (4 is deferred; 6 is advisory).

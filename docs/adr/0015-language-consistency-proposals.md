# ADR-0015: Language consistency proposals (numerics, casts, paths, returns)

- **Status:** Accepted (2026-10-11): 1, 2, 3, 5. Removed: 4 (`::` to `.`; revisit only if needed). 6 advisory.
- **Date:** 2026-10-11
- **Related:** [ADR-0014](0014-surface-syntax-rust-isms.md), [ADR-0002](0002-local-mutability.md), [ADR-0012](0012-rust-interop-boundary.md), `docs/ARCHITECTURE_IMPROVEMENT_PLAN.md` (WP6-8)

## Context
Most recent compiler fix commits are integer-width and clone corrections; Rust conventions that Windjammer
inherits (width zoo, truncating `as`, `::` paths) push complexity into inference. This ADR lists candidate
simplifications, each independent.

## 1. One default numeric type (Accepted)
`int` = 64-bit, overflow-checked; `float` = f64. Indexing and `len()` accept `int` via a checked conversion.
Explicit widths (`u8`, `i32`, `usize`) only at FFI/binary-layout/perf boundaries.
- Pros: removes the usize/i32 cast churn and most of numeric-solver complexity (WP8); same semantics on Rust/Go/JS (JS needs BigInt or checked 53-bit policy — see ADR-0007).
- Cons: 64-bit everywhere costs memory in large arrays; behaviour change for existing sources.
- Control is preserved: explicit widths (`u8`, `i32`, `u32`, `usize`, `f32`) remain first-class types and are never silently widened or narrowed; annotations and literal suffixes win over the default; mixed-width arithmetic requires explicit conversion (#2). Only *unannotated* values default to `int`/`float`. GPU/FFI/binary layouts keep their declared widths.

## 2. Checked conversions instead of `as` (Accepted)
`int(x)`, `float(x)` convert losslessly or fail/panic when out of range; `x.truncate()`/`x.wrapping()` are explicit.
Pros: no silent truncation. Cons: migration via `wj fix`.

## 3. `to_string(self)` auto-provides Display/interpolation (Accepted)
Defining `fn to_string(self) -> string` makes the type printable/interpolatable (like derives). Removes `impl Display` boilerplate.

## 4. Implicit final expression stays (Accepted)
No forced `return`: `if`/`match`/closures/pipes are expressions. Add diagnostics: declared return type with a
non-value last statement or a non-returning path is an error with a span; lint against `return x` as the final
statement. `return` remains for early exit.

## 5. Lower priority (advisory)
`if x is Some(v)` for `if let`; documented string byte-vs-char indexing rule; no `unsafe` in `.wj` (use `rust { }`, ADR-0012).

## Implementation
Sequenced behind the DefId resolver and single numeric solver (WP6-8): #4 and #3 and #2 first (independent), #1 with the solver, each with `wj fix` migrations.

## Implementation status (idiom-gaps)
- #1: unannotated `float` literals now emit `f64` in the Rust backend (was `f32`); a cast operand keeps its own width. The `int` default is still `i32` inside the numeric solver (`type_inference/int_inference`, `Unknown -> I32` plus the "prefer non-default" rule); flipping it needs a weak-default marker and is the largest remaining source of `// rust: gap` programs (E0308 width mismatches).
- #4: W0009 (trailing `return`, fixed by `wj fix`) and W0011 (declared return type, body ends without a value) are lints; `wj check` surfaces them through `RustLeakageLinter`.

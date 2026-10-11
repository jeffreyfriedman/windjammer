# Windjammer optimization architecture (as implemented)

Windjammer emits Rust and relies on rustc/LLVM for machine-level optimization.
Optimization work in this repository therefore happens in three layers, and this
page lists only what exists and is exercised by tests. Evidence and numbers:
[optimization/ASSESSMENT.md](optimization/ASSESSMENT.md),
[optimization/RESULTS.md](optimization/RESULTS.md); soundness arguments:
[optimization/PASSES.md](optimization/PASSES.md).

## 1. Analyzer hints (live)
`analyzer/string_optimization.rs` and `analyzer/optimization_detectors/*` annotate functions
(Cow, SmallVec, defer-drop, loop and struct-mapping hints). Codegen consumes them
(`codegen/rust/function_generation_prepare.rs`, `let_statement_generation.rs`).
`auto_clone.rs` decides where clones must be inserted for by-value semantics.

## 2. Code generation transforms (live)
`defer_drop_generation.rs` (`--defer-drop auto|always|never`), `simd_transform.rs`
(f32 dot-product / array-add loops, Rust target), `constant_folding.rs`, `aosoa_transform.rs`
(advisory SoA sketches).

## 3. Post-codegen Rust optimizer (live, `src/optimizer/`)
`optimizer::run(&OptimizationConfig, rust_source)` parses the emitted Rust with `syn`
and applies span edits: redundant clones (temporaries, Copy primitives, format-macro
arguments), identity casts, `Vec::with_capacity` before push loops, in-place string
append instead of `s = format!("{}..", s, ..)`, `#[inline]` policy, and removal of
private-struct `#[repr(C)]` / unreferenced `to_bytes`. Every pass is a switch in
`OptimizationConfig`; there are no environment toggles. Edits never change line counts.

## Retired
The former 15-phase AST optimizer (`phase11`-`phase15`), its `OptimizerConfig`, and the
`ir/optimizations.rs` pass stubs were never wired into the compiler and were deleted
(2026-10-11; history in git). The earlier "two-tier" description
(`compiler_analysis.rs`, `batching_codegen.rs`, runtime batching optimizers) referred
to files that do not exist in this repository and has been removed. See ADR-0006.

## Not implemented
`wj optimize`, `wj economics`, ownership-solver-driven last-use clone elimination,
literal `to_string` avoidance. Any performance claim must come from
`benchmarks/rust_parity` (see RESULTS.md).

# Windjammer optimization assessment (evidence-based)

Date: 2026-10-11. Method: read the code, compile a corpus with the pre-change
`wj` (debug build of commit 923b4c67), count patterns in the emitted Rust, and
check what the repository benchmarks actually measure. All numbers below were
produced by these commands; nothing is quoted from existing docs.

## 1. Pass inventory: live vs dead

| Component | What it does | Runs where | Tests | Status / why | Risk |
|---|---|---|---|---|---|
| `src/optimizer/` phases 11-15 (string interning, DCE, loop invariant hoisting/unrolling, escape analysis, SIMD) ~5000 lines | AST-to-AST rewrites over arena `Program<'ast>` | Nowhere. Declared as `pub mod optimizer` in `main.rs` only; the one caller was `compiler_database::optimize_program`, which is an identity function ("INTENTIONALLY DISABLED ... 150 lifetime errors") and itself is only reached from `compiler_database::compile_file`, which no CLI path calls | unit tests in phases 11-13 (never run against real programs) | DEAD. Arena owned by `Optimizer` vs Salsa `'db` lifetimes; `alloc_expr` used `unsafe` lifetime laundering. Phases 14/15 default off | High: unsafe lifetime transmute; no differential tests |
| the former `cost_model` module (deleted in convergence C4) | `CompilationCostTracker` timing record | Re-exported, no consumer | small | DEAD (kept, not mine to delete) | None |
| `src/auto_clone.rs` (4055 lines) | Decides where clones must be INSERTED so by-value WJ semantics compile | Live: `analyzer/function_analysis`, `ir/annotations`, codegen | extensive | LIVE; it adds, never removes, clones | Correctness-critical |
| `analyzer/string_optimization.rs` + `optimization_detectors/*` (cow, defer-drop, smallvec/const, loops, struct mapping) | Annotate functions with hints | Live in analyzer; codegen consumes `clone_optimizations`, `smallvec_optimizations`, `cow_optimizations` (`function_generation_prepare.rs`, `let_statement_generation.rs`) | unit tests in analyzer; benches `cow_bench`, `smallvec_bench`, `string_optimization_bench` benchmark hand-written Rust, not generated code | LIVE, effect on the corpus not measured before this work | Hints gated by analyzer; not audited here |
| `codegen/rust/defer_drop_generation.rs` | Moves big drops to a thread (`--defer-drop auto`) | Live (default `auto`, 100 KB threshold) | benches `defer_drop_*` (hand-written) | LIVE, changes drop timing by design | Observable only via Drop side effects |
| `codegen/rust/simd_transform.rs` + `analyzer/simd_loops.rs` | Emits `std::arch` code for `for` loops matching f32 dot-product / array-add patterns | Live, Rust target only (`for_statement_generation.rs`) | unit tests | LIVE, narrow patterns | Medium: arch-specific unsafe emitted |
| `codegen/rust/aosoa_transform.rs` | Emits SoA struct sketches as hints | Live (`program_generation.rs`) | one bench (hand-written) | LIVE, advisory only | None |
| `codegen/rust/constant_folding.rs` | Folds literal arithmetic | Live (`expression_generation.rs:825`) | unit | LIVE | Low |
| `codegen/rust/optimizations.rs` | 19-line trait stub | nothing | none | DEAD | None |
| docs `OPTIMIZATION_ARCHITECTURE.md` | Describes `compiler_analysis.rs`, `batching_codegen.rs` | those files do not exist in the repo | - | FALSE (rewritten) | - |
| former docs `COMPILER_OPTIMIZATIONS.md`, `MULTI_LANGUAGE_OPTIMIZATION.md` (both deleted), WJ-PERF-01 RFC | Targets: ">= 95% of Rust", "99%+", "95% of Rust's runtime speed" | not implemented (`wj optimize`, `wj economics` do not exist) | - | ASPIRATIONAL | - |

Decision (see ADR-0006 option A): the dead AST phases were deleted (history in git) and replaced by
`src/optimizer/` v2, documented in `src/optimizer/MODULE.md` and
`docs/optimization/PASSES.md`.

Deviation from ADR-0006 text: it prefers solver-owned IR passes. Those need
ownership facts the IR does not yet export, so v2 works on the emitted Rust and
uses only facts it can prove locally (see PASSES.md "Considered and rejected").
When the IR exports ownership/last-use facts, the rejected passes (last-use clone
elimination, literal `to_string` avoidance) belong there.

## 2. What the generated Rust looks like

Corpus: every `wj build`-able file in `examples/*.wj`, `tests/windjammer_tests/*.wj`
and `std/*.wj` = 81 files, 4712 lines of Rust (`wj build --no-cargo`), plus 10
compute kernels (`benchmarks/rust_parity/wj`). Counts by `grep -o` on the
emitted text, before and after the v2 optimizer (line count is unchanged by design).

| Category | Before | After | Notes |
|---|---:|---:|---|
| `.clone()` | 18 | 10 | 6 inside `format!/println!` args, 1 temp `vec![..].clone()`, 1 Copy binding; 8 call-argument clones left (need ownership facts) |
| `.to_string()` | 103 | 103 | mostly `"lit".to_string()` into owned `String` formals; replacing needs callee types (not done) |
| `String::from(` | 153 | 153 | `Err(String::from("..."))` etc.; owned value is required |
| `as` casts | 42 | 35 | 7 identity/duplicate casts removed; rest are real (`len() as i64`: 9, widening) |
| `#[inline]` | 509 | 438 | on every fn incl. tests, `main`, large bodies; 71 removed |
| `#[repr(C)]` | 52 | 16 | on every struct; 36 removed from private structs in files without layout-sensitive code |
| `fn to_bytes` helpers | 12 | 2 | emitted for every `repr(C)` struct, 10 unreferenced and private |
| `format!(` | 21 | 20 | 1 self-append rewritten (in kernels: `out = format!("{}{}{}", out, w, ",")`) |
| `Vec::new()` + `with_capacity` | 7 / 18 | - | kernels: every `Vec::new(); for _ in 0..n { push }` lacks capacity (matmul, sieve, vec_fill, grid builders) |
| `Box<` / `Rc<`/`Arc<` | 0 / 0 | - | no smart-pointer overuse |
| `let _ =` | 21 | 21 | `let _ = v.push(..)`: no cost |
| integer literals with suffix (`1_i32`) | 569 | 569 | cosmetic; rustc cost-free |
| `&Vec<` / `&String` | 4 / 1 | - | `&[T]` would be idiomatic; no run-time cost |
| `impl Into<String>` formals | 0 | - | not emitted |
| `unwrap()` | 4 | - | - |
| derive lines | 56 | - | `Default, Hash, Eq, PartialEq, Clone, Copy, Debug` on everything: compile time/code size only |

Patterns after the 81-file corpus rewrite by pass (production config): inline_policy 71,
dead_helpers 46, macro_arg_clone 6, same_type_cast 7, temp_clone 1, copy_clone 1, string_append 1.

### Top root causes (with before code)

1. `#[inline]` stamped on every function, tests and `main` included (509 occurrences):
   `#[test]\n#[inline]\npub fn test_x() {..}`. Effect: code size / compile time; rustc already decides.
2. `#[repr(C)]` + `to_bytes` on every struct (52/12): `#[repr(C)] struct Rng { state: u64 }` plus an
   `impl Rng { pub fn to_bytes(&self) -> Vec<u8> {..} }` nobody calls. Blocks field reordering, adds dead code.
3. Growth reallocations: `let mut c: Vec<f64> = Vec::new(); for _i in 0_usize..n * n { c.push(0.0_f64); }`.
4. Quadratic string building: `out = format!("{}{}{}", out, w, ",");` copies `out` each iteration
   (40000 words in the string_build kernel; see RESULTS.md).
5. Clone inside format macros: `format!("File not found: {}", request.path.clone())` (6 of 18 clones).
6. Clone of a freshly built temporary: `bfs_visit_count(0_i64, vec![1_i64, 2_i64].clone())`.
7. No-op casts: `matmul(n as usize, &a, &b)` with `n: usize`; `(i * i) as i32` patterns that are real are kept.
8. `"lit".to_string()` passed where an owned `String` is required (103): unavoidable without callee typing.
9. `Err(String::from("..."))` allocation on error paths only: fine.
10. Index loops with bounds checks (`a[i * n + k]`): LLVM removes most; matmul/nbody within noise of
    hand-written iterator code (RESULTS.md), so no pass written.

Not found despite looking: `Box`/`Rc` overuse, `impl Into<String>` conversions, `&String` formals
beyond 1, `with_capacity` misuse, `#[inline(always)]`. Compiler inference defects found while writing
kernels are in `docs/known-issues/`.

## 3. Benchmarks: is there evidence for "99% of Rust performance"?

- `benches/*.rs` (12 files): `runtime`, `cow_bench`, `smallvec_bench`, `defer_drop_*`,
  `simd_vectorization_bench`, `string_optimization_bench`, `cache_locality_aosoa_bench` measure
  HAND-WRITTEN Rust (their own header: "Windjammer transpiles to Rust, so runtime performance should be
  identical ... serves as a baseline"). `compilation`, `incremental_compilation`, `arena_performance`,
  `regression_benchmarks` measure compiler speed. None compiles WJ source and times the result against
  Rust.
- `benchmarks/README.md`: Rust-only criterion numbers for a TaskFlow API; "Windjammer: TBD",
  "Build equivalent Windjammer benchmarks" unchecked.
- README "99%+ measured": no benchmark in the repository produces it. `COMPILER_OPTIMIZATIONS.md` (deleted)
  states a target (">= 95%"), not a measurement.
- New: `benchmarks/rust_parity/` (10 kernels, WJ vs idiomatic Rust, release builds); results and their
  limits are in `RESULTS.md`.

## 4. What changed

- Added `src/optimizer/` v2 (7 pass families, 8 switches, one entry `optimizer::run`), hooked after
  `generate_program` in `compiler/mod.rs` (the live path), `file_compiler.rs`, `output_generation.rs`,
  `compiler_database.rs`. Defaults ON only for passes with a soundness argument in PASSES.md.
- Verification: reference-checked fixtures (independent Python model, 5 seeds each), a seeded
  generative differential suite with shrinking (a larger soak run with `--features optimizer-soak`),
  metamorphic checks (idempotence, alpha-renaming, dead-statement insertion), a mutation self-test
  (9 unsound variants, each must be caught by fixtures AND generator), and a ratchet on
  clone/to_string/format/inline/cast counts of a fixed corpus.
- Not done: IR-level last-use clone elimination, `to_string` literal avoidance, iterator-loop
  normalization, `const fn` inference, SIMD or escape analysis (no evidence they matter; see RESULTS.md).

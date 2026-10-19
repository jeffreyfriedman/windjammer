---
status: accepted
last_verified: 0ad66981d7b3
---
# ADR-0020: Interpreter execution engine (tree walker vs resolved/bytecode tier)

- **Status:** Accepted for milestone 1 as an opt-in engine (`--engine=resolved`); the tree walker stays the default (see Outcome)
- **Date:** 2026-10-12
- **Related:** ADR-0007 (interpreter tier), `docs/INTERPRETER.md`, `src/interpreter/MODULE.md`

## Context
`wj run --interpret` is a tree walker over the parser's AST. Variables are looked up by name in a vector of
`(name, Rc<RefCell<Value>>)` per frame, every binding allocates a slot, and every operand is cloned out of its slot.
Measured on the interp-gaps2 branch (release `wj`, same loaded machine, median of 3, user time):

| Program | before | after cheap wins | compiled Rust (order of magnitude) |
|---------|--------|------------------|------------------------------------|
| `fib(30)` (1.3M calls) | 1.03 s | 0.92 s | ~5 ms |
| 5M-iteration `while` with arithmetic | 2.33 s | 2.06 s | ~5 ms |

The cheap wins (call-site cache for plain calls, a frame-vector pool, inline fast paths in `deref_value`, no
type-alias lookup when there are no aliases) gave 10-15%. A pooled-slot experiment was slower than the system
allocator and was removed. Profiles (`sample`, release) now spread time over `exec_stmt`/`eval_raw` (dispatch),
`deref_value`/clone/drop of `Value`, name comparison in `lookup_var`, and `Rc` slot allocation: costs of the design,
not of a hot spot.

## Options
### A. Keep the tree walker; keep picking local wins
Cheap, no risk. Expect another 10-30% at most. The interpreter stays ~200-400x slower than compiled code.

### B. Resolution pass + closure compilation ("resolved tree")
Before running a function, resolve every identifier to a frame slot index (and module item / builtin), every call
site to its target, and every field access to an index when the type is known; then build a tree of Rust closures
(`Box<dyn Fn(&mut Frame) -> R>`) per function. Frames become `Vec<Value>` indexed by slot; closures capture slots
by index (escaping captures boxed). Keeps the current value model, builtins and scheduler.
Expected 3-10x on call- and loop-heavy code (typical for closure compilation of an AST interpreter).

### C. Register bytecode VM
A real compiler to a compact instruction set with an interpreter loop. Highest ceiling (10-30x), largest cost:
a second lowering of the whole language that must match the tree walker bit for bit, plus a debugger/backtrace story.

## Recommendation
Option B, staged per construct with the tree walker as fallback for anything not yet lowered (so coverage never
drops), behind `--engine=resolved` until it passes every interpreter test.

**Kill criteria** (stop and keep A if any holds after the first milestone: locals, arithmetic, `if`/`while`/`for`,
direct calls):
1. Less than 3x on `fib(30)` and the 5M-loop benchmark (release, same machine).
2. Any difference from the tree walker on the interpreter test suites (semantics, differential, concurrency,
   std modules, rustc parity, spec examples) that is not a tree-walker bug.
3. More than ~3k lines for the milestone, or a second implementation of a builtin.

## Consequences
- The value model, `builtins/`, the scheduler and the error/backtrace format are shared by both engines.
- Benchmarks become a tracked test (`scripts/` + a ratchet on the ratio to compiled code).

## Outcome of milestone 1 (Oct 2026)
Implemented in `src/interpreter/resolved/` (about 900 lines): per-function closure compilation over slot-indexed
locals (`Interpreter::rstack`), direct compiled-to-compiled calls with pre-resolved targets, fast paths for integer
arithmetic, comparisons and slot stores, tree-walker fallback for every construct outside the subset (the whole
function falls back, so the engines never disagree about what is supported).

- Criterion 2 (no divergence): held. `tests/interpreter_engine_parity_test.rs` runs the conformance programs,
  `examples/verified`, `tests/spec` and the interpreter corpus (every seed) under both engines and compares stdout,
  stderr and exit status.
- Criterion 3 (size, no second builtin implementation): held. Builtins, the scheduler and error formats are shared.
- Criterion 1 (>= 3x on `fib(30)` and the 5M loop): **not met**. Tracked by `scripts/interp_bench.py`
  (median of 5, load average recorded, `--check` against `bench/interp/baseline.json`).

Measurements (release `wj`, user time, median of 5, machine load average noted; the machine was shared with other
builds, so only ratios are meaningful and no baseline file was written):

| Stage | load | fib(30) walker / resolved | 5M loop walker / resolved |
|-------|------|---------------------------|---------------------------|
| first M1 | ~115 | 0.80 s / 0.48 s = 1.66x | 1.89 s / 0.81 s = 2.33x |
| + direct calls, frame skip, param fast paths | ~115 | 0.884 s / 0.326 s = 2.71x | 2.058 s / 0.841 s = 2.45x |
| + `fast.rs` (unboxed int reads, i64 ops, bool conditions) | ~49 | 0.882 s / 0.331 s = 2.66x | 1.918 s / 0.742 s = 2.58x |
| + table `fits`, boxed `Break`/`Continue`, drop-free slot stores | ~57 | 0.884 s / 0.348 s = 2.54x | 2.76 s / 0.845 s = 3.27x |

Verdict: the loop benchmark reaches about 3.2x, `fib(30)` stays at 2.5-2.7x. Criterion 1 fails on `fib(30)`, so per the
kill criteria the resolved engine stays opt-in and the tree walker stays the default. The engine is kept (it is
parity-tested and gives 2.5-3.3x) as the base for proposals i-iv; the call path (frame, backtrace and depth
bookkeeping per call) and boxed `Value` results are the remaining cost, not name lookup or slot allocation (locals
already live in a flat stack and call targets are resolved once per site).

Profile of the loop benchmark (macOS `sample`, release): the time is in the generated binary/compare closures
(`fast::binary` 43%, `fast::compare` 24%), `Value` drop glue (11%) and `IntTy::fits` (6%, not inlined). The closure
tree itself is cheap; what remains is the cost of boxing every intermediate in a 32-byte `Value` (an `i128` payload)
and returning it through `Result<Value, Ctl>`.

## Proposals beyond milestone 1
i. **Type-specialized unboxed slots.** Consume the solver's semantic facts (the single facts interface that
   `wt/convergence` is building for all backends; do not re-infer): when a local is known `i64`/`f64`/`bool`, compile it to
   an `ICode` closure returning a raw `i64` and keep it in a typed slot, boxing only at calls into untyped code.
   Expected 2-4x on arithmetic loops. Kill: under 1.5x over milestone 1 on the two benchmarks, or any parity divergence.
ii. **Inline caches** for method and field dispatch: a per-call-site `(struct id, field index / method)` cache with
   a shape check, replacing the by-name field lookup in `field_of` and `find_user_method`. Kill: under 10% on the
   struct-heavy corpus programs.
iii. **Cranelift JIT tier (stretch)** for hot, fully typed functions (counter in `CompiledFn`, compile after N calls)
   built on (i). Kill criteria: under 5x over milestone 1 on `fib(30)`, more than 3k lines, or a second source of
   arithmetic semantics (the JIT must call the same overflow-checked helpers or be differentially tested against them
   over generated programs).
iv. **Per-module parse/resolve caching** for fast startup of multi-file projects: cache the parsed AST and resolved
   function tables keyed by file content hash under the project's build cache, invalidated per module. Coordinate with
   `wt/interp-projects` (module loading is theirs). Kill: startup of the 50-module fixture improves under 30%.

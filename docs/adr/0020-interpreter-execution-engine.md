---
status: accepted
last_verified: 9507be50c346
---
# ADR-0020: Interpreter execution engine (tree walker vs resolved/bytecode tier)

- **Status:** Accepted. Milestone 1 shipped opt-in; since the steps 1-7 work (Oct 2026) `--engine=resolved` is the default for
  `wj run`, and the tree walker is the reference semantics (`--engine walker`) and runs what the resolved engine refuses
- **Date:** 2026-10-12 (milestone 1), 2026-10-19 (steps 1-8)
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
- Criterion 1 (>= 3x on `fib(30)` and the 5M loop): **not met**. Tracked by `scripts/wj/interp_bench.wj`
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

## Steps 1-8 (Oct 2026): from opt-in to default

Measured with `scripts/interp_compare.sh` (child CPU seconds, median of 3, release `wj`, at the end of the work; the shared
machine had a load average of about 43, so read ratios, not absolutes). Rust is the program the Windjammer compiler
generates from the same file (`rustc -O`); Python and Node are hand-written twins in `bench/interp/twins/`; every column
prints exactly what the walker prints.

| kernel | walker | resolved | Rust | Python 3.13 | Node 24 | walker / resolved | resolved / Python |
|--------|--------|----------|------|-------------|---------|-------------------|-------------------|
| `fib30` | 0.894 | 0.107 | 0.005 | 0.160 | 0.051 | 8.3x | 0.67 |
| `loop5m` | 2.319 | 0.190 | 0.003 | 1.038 | 0.049 | 12.2x | 0.18 |
| `structs` (method calls) | 0.675 | 0.393 | 0.005 | 0.185 | 0.054 | 1.7x | 2.1 |
| `strings` | 0.201 | 0.114 | 0.011 | 0.075 | 0.040 | 1.8x | 1.5 |
| `sieve` | 1.351 | 0.599 | (1) | 0.405 | 0.057 | 2.3x | 1.5 |
| `hashmap` | 0.308 | 0.218 | (2) | 0.059 | 0.045 | 1.4x | 3.7 |
| `closures` (iterator chains) | 0.195 | 0.173 | (2) | 0.052 | 0.049 | 1.1x | 3.4 |
| `nbody` (floats in structs) | 1.193 | 0.888 | 0.003 | 0.136 | 0.054 | 1.3x | 6.5 |

(1) the generated Rust panics: unannotated integer locals used as indices are emitted as `i32`, so `i * i` overflows past
46340 (compiler bug, numeric defaults; the interpreter computes in `i64` as the language specifies).
(2) the generated Rust does not compile (`E0308`; Rust backend gaps for these iterator/map shapes).

With `--features jit` (step 7) `fib30` takes 0.02 s. The ratchet baseline `bench/interp/baseline.json` (walker / resolved per
kernel, `scripts/wj/interp_bench.wj --check` fails below it minus 15%) was recorded at a load average of 12: `fib30` 9.4x,
`loop5m` 12.2x, `sieve` 2.4x, `structs` 1.8x, `strings` 1.7x, `hashmap` 1.3x, `nbody` 1.3x, `closures` 1.2x.

Milestone 1 measured `fib30` at 0.33 s and `loop5m` at 0.75 s on the resolved engine. The calls/loops goal (at least
Python's speed on both, then within 3-5x of Node) is met on `fib30` and `loop5m`; the other kernels are where the remaining
work is (see "What is left").

### Step 1: values (planned: a 16-byte `Value` with an `i64` immediate)
Done differently. `Value::Int(i128, IntTy)` is matched in about 230 places across the walker and the builtins, which other
streams edit daily, so the shared value model stayed as it is. Instead the resolved engine compiles every expression whose
integer width it knows statically (literals, typed locals, arithmetic, casts, calls to functions declared to return one) to
an `i64`-returning closure (`resolved/typed.rs`), and a local whose every write keeps one width is a typed slot
(`compile.rs`: a write that could change the width poisons the slot and the function is compiled again without it). Only
widths whose values fit an `i64` are unboxed (`u64`, `usize`, `i128`, `u128` stay boxed). Edges go through the walker's
`int_binary` / `coerce`, so overflow panics and literal-range errors are the walker's. `fib30` 0.32 to 0.13 s, `loop5m` 0.75
to 0.22 s. `Value` is still 48 bytes (`Range(i128, i128, _)`, `Ref(Place)`); shrinking it now matters only for untyped code.

### Step 2: call path
Arguments are evaluated straight into the callee's first slots (no argument vector, no `Unit` fill), a call site whose typed
arguments match the declared widths skips parameter binding, the compiled target is borrowed (not cloned) per call, frames
whose slots are all typed pop without drop glue, and a typed body returns a raw `i64`. `fib30` 0.13 to 0.08 s. A typed
parameter that receives a non-integer at run time sends that call to the walker (`call_func_walk`) before anything ran.

### Step 3: superinstructions
`if c { return v }` in a typed body exits without unwinding `Ctl::Return`; comparisons and `&&`/`||`/`!` of comparisons
yield a `bool` directly; checked arithmetic and comparisons are monomorphized for slot/constant operand shapes; `x op= e`
on a typed slot updates it in place; `for i in a..b` over typed bounds counts in place. The `while` condition and the
compare-and-branch are one closure.

### Step 4: inline caches
A field read remembers the field's position (checked by name on every hit); a method call site remembers the user method
for the last receiver type and its compiled form and calls it directly (`call_user_method` / `call_func_run` rules; no
caching under `--hot`). On the walker side, struct and field names are interned in `StructDef`, so building a struct no
longer allocates its names. `structs` 0.64 to 0.34 s. The kill criterion (10% on struct-heavy code) is met.

### Step 5: default engine and coverage
The criteria held, so `--engine=resolved` is the default for `wj run`:
- speed: 8.3x (`fib30`) and 12.2x (`loop5m`) over the walker (criterion: 3x on both);
- no divergence: `tests/interpreter_engine_parity_test.rs` (conformance programs, `examples/verified`, `tests/spec`, every
  seed of the interpreter corpus, and `tests/engine_parity/`, programs aimed at typed paths, islands, method calls and
  deopt) shows no difference in stdout, stderr or exit status; mutants that skip a range check or keep block locals alive
  past their block make it fail. The interpreter suites (semantics, differential, concurrency, std modules, rustc parity,
  hash order, CLI, projects, spec conformance) pass with the resolved engine as default;
- size: `src/interpreter/resolved/` grew by about 2.5k lines to 2.9k (plus the 520-line `jit.rs` behind its feature),
  and the walker by about 90 net lines of refactors that let both engines share one implementation (`PlaceEnv` for place
  resolution, `assign_to_place`, `eval_struct_literal_in`, `call_func_walk`, `poison_if_guard`, interned struct names).
  No builtin, place, pattern or struct rule is implemented twice.

Coverage: a statement or expression the compiler does not lower no longer sends the whole function to the walker. It runs
as a walker island (`resolved/island.rs`) against a walker scope holding the locals it may name; those locals are boxed (their
slot holds a walker slot), so the walker reads, writes, borrows and captures the same storage. Method calls (`method.rs`),
struct literals, assignments through places (`place.rs`: `v[i] = x`, `p.f += y`), `match` on a computed scrutinee
(`matching.rs`), closure bodies (`closure.rs`: environment names are boxed locals holding the environment's own slots) and
parameters the body mutates are compiled; each reuses the walker's routine for the semantics (place resolution, stores,
dispatch, struct construction, pattern matching) and supplies compiled sub-expressions. What still runs entirely in the
walker: functions with `defer`, generic functions and closures, `async` and `extern` functions, a method without `self`
called on an instance, closures with tuple parameters, and `match` on a place. Programs with `impl Drop` are not compiled
(destructor timing needs the walker's scopes).

One documented difference, in programs rustc rejects: a function declared to return an integer that returns something else.
The walker carries the wrong-typed value on; a typed call site reports `mismatched types` (tested in the parity test).

### Step 6: unboxed typed slots from solver facts (spike)
The integer half landed as step 1 by local inference over the function being compiled, not from solver facts: the
interpreter's allowed imports do not include `ir`, and `ir::numeric_defaults` would add nothing here, since its closed
literal classes are exactly the literal-width locals the engine already unboxes. Not done: floats and struct fields. `nbody`
is 7x slower than Python because every `f64` and every field read is a boxed `Value`. Next experiment: an `f64` twin of
`typed.rs` (literals, `f64` locals, arithmetic, `as f64`, calls declared `-> f64`), then field reads typed from the
declared struct field type behind a tag check. Kill: under 2x on `nbody`, or any parity difference.

### Step 7: Cranelift tier (`jit`, a default feature since Oct 19)
`resolved/jit.rs` (520 lines) compiles functions whose every value is `int` and whose body is arithmetic, comparisons, `let`,
assignment, `if`, `while`, `return` and calls to such functions, together with their callees, after 64 calls. A jitted call
has no side effect, so on any edge (overflow, zero divisor, `i64::MIN / -1`, recursion past 2000 jitted frames) it bails
out and the call runs again in the closure engine, which produces the panic, span and backtrace. Kill criteria: at least 5x
over milestone 1 on `fib30` (0.33 s to 0.02 s: about 16x, met), at most 3k lines (met), and either the same arithmetic
helpers or a differential test over generated programs: `tests/interpreter_jit_differential_test.rs` runs programs whose
`b * b` overflows only after the functions are hot under the walker and the jitting engine; a mutant without the multiply
overflow check makes it fail.

Owner decision (Oct 19): on by default. `jit` is a default feature of the CLI build (lib-only builds such as
`--no-default-features --features cli` omit it), `wj run --no-jit` / `WJ_JIT=0` / `--jit off` turn it off at run time, and
a host Cranelift cannot target falls back to the closures once (`Jit::new` fails, `jit_enabled` is cleared). Gates:
- the differential generator also covers unary minus, `&&`/`||`/`!`, compound assignment, near-`i64::MAX` constants, a zero
  divisor planted at a late iteration, 3000-deep recursion and an `i32` function (never jitted) calling jittable ones; every
  program runs under the walker, the jitting engine and `--no-jit`. Over `120` generated ones (`29` clean, `91` panicking): no difference;
  mutants without the overflow or the zero-divisor bail-out make 38 and 3 of them differ;
- `tests/interpreter_jit_caps_test.rs`: a hot function that reaches a sink (`println`, `fs::exists`) only after it is hot
  is never jitted (the jittable subset has no calls outside itself), and under `--caps enforce --sandbox` the late sink is
  denied with exit 126 exactly as with `--no-jit`;
- the engine parity corpus with the jit on: no difference.

Cost, measured (release, `CARGO_BUILD_JOBS=2`, non-incremental, shared machine): the Cranelift crates add about 50 s wall
/ 74 s CPU to a clean build; the `wj` binary grows from 26.1 MB to 28.5 MB (+2.3 MB, 9%); start-up is unchanged (the
module is created on the first hot call: `wj run hello.wj` about 20 ms CPU with and without). `fib30` takes 0.027 s with the
jit and 0.091 s with `--no-jit`; `loop5m` is unchanged (its loop is in `main`, which returns nothing and is not jitted). The
ratchet baseline (load 13.5) now has `fib30` at 37x over the walker.

### Step 8: parse/resolve cache for startup (measured here; built as round 3, rung 4, below)
A generated 200-module project (4k functions) starts in about 0.9 s CPU. Profiling shows that at least 80% of it is the pre-run
check gate (`cli::check::require_clean`: IR lowering, `ir::taint::stdlib_taint_sources`, effect lookups), not the
interpreter's parse/resolve/registration, so a per-module interpreter cache cannot meet its kill criterion (30%). The lever is in
the check pipeline: memoize the stdlib taint sources (a function of the stdlib only, about a quarter of the profile) and cache
check results per file content hash. Those modules belong to the convergence and effects streams.

### Round 3, rung 4: startup
Built step 8's lever, in the check gate rather than the interpreter. Fixture: a generated 200-module project, 20 functions per
module (4k functions: the shape of `scripts/wj/interpreter_project_bench.wj --files 200` plus 17 filler functions per module,
each an `if` and a call). Metric: CPU seconds (user + sys) of `wj run`, median of 5 after one warm-up, release build, with
the 1-minute load average next to every number (the machine was shared with ~10 agents; CPU time is less load-sensitive than
wall time, not immune).

| build | scenario | CPU s (median of 5) | load (1 min) |
|---|---|---|---|
| base (`2088c1e6`) | no cache exists | 0.853 / 0.853 (two batches) | 72.7 / 76.4 |
| 1: stdlib tables once per process (`d5ee5266`, the `OnceLock` cut) | no cache | 0.545 / 0.536 | 71.2 / 79.1 |
| 1 | no cache (re-run beside 2) | 0.558 | 48.7 |
| 2: + check caches | cold (`.wj-cache` deleted before every run) | 0.600 | 48.8 |
| 2 | warm (nothing changed) | **0.065** | 50.1 |
| 2 | one module edited before every run | **0.145** | 50.1 |
| base, re-run for the final comparison | no cache | 0.914 | 37.6 |
| final (`2b9293f7`, session-shared tables + caches) | cold | 0.610 | 37.5 |
| final | warm | **0.070** (-92% vs 0.914) | 37.5 |
| final | one module edited before every run | **0.155** (-83%) | 37.6 |

1. **Stdlib tables.** `ir::taint::stdlib_taint_sources`, `stdlib_sinks` and `ir::effects::stdlib_effect_declarations` are
   functions of the stdlib registry only, and every `IrPipeline` rebuilt them: once per checked file. A first cut memoized
   them in process-global `OnceLock`s; `tests/architecture.rs` (no new static state) rejected that, so they are now
   `ir::stdlib_tables::StdlibTables`, computed once per check run and handed to every pipeline with
   `IrPipeline::with_stdlib_tables`. -37% with no cache at all. Gate: `tests/stdlib_tables_test.rs` (shared tables equal
   the per-pipeline computation, same order).
2. **Check caches** (`src/check/{cache,file_cache,ambient}.rs`), under `<project>/.wj-cache/` (the build's cache directory,
   `metadata::meta_cache_root`). `wj run` now uses them when the program lives in a `wj.toml` project; a lone script is
   still checked uncached and nothing is written next to it. Warm: -89%; one module edited: -75%; cold pays +7% for
   hashing and writing. Kill criterion (warm improvement < 30%) not hit: shipped.
   - What "per file, keyed by everything it depends on" turned out to mean. The whole-program phases (resolution, effects,
     the semantic index) are not per file: WJ0002's "similar name" help scans every definition in the program and the
     semantic index is global, so a file's diagnostics from those phases depend on every file, not just its imports. Their
     honest per-file key is the whole project, which is the whole-run cache (`cache.rs`: SHA-256 over all paths and
     contents, manifest, lock file, dependency sources, the build identity, and the ambient sidecars below). The phase that
     really is per file is the expensive one: `project::analysis_phase` (analyzer, numeric inference, IR solvers) builds a
     fresh stdlib-only `SignatureRegistry` and reads nothing but its file, so `file_cache.rs` keys it on path + SHA-256 of
     the text under a header of build identity + ambient digest. Resolution, effects and the index always run fresh, and
     whether a memoized analysis is reported still depends on their fresh results (the "file already has errors" rule).
   - A hidden input found on the way, and keyed: the numeric inference reads `.wj.meta` sidecars relative to the working
     directory (`load_imported_metadata.rs`, `source_root` is unset in check), so `wj check` results depend on the cwd.
     The pre-existing whole-run cache did not key on this. `ambient.rs` digests the cwd plus every `*.wj.meta` within the
     reach of the program's `use` paths (and under `<cwd>/.wj-cache/`); a walk over 20k entries disables caching for that
     run.
   - Gate, `tests/check_cache_differential_test.rs`: (a) editing a dependency module (never the checked file) changes the
     entry's diagnostics exactly as an uncached run does (arity change -> WJ0013, made private, reverted); (b) 24 generated
     projects x 6 random dependency edits (arity, visibility, rename, syntax error, f32+f64 for analysis-phase WJ0027,
     unresolved call, comment, revert), checking the entry file and the directory: 288 comparisons of diagnostics, success
     and the `wj run` gate decision (`require_clean`, lints on and off) between cached (first and repeat) and uncached runs,
     288 repeat hits, 163 failing states, 71 with analysis-phase findings; (c) 300 real programs (`examples/verified`,
     `tests/check_negative/mutants`) x 2 random line edits: 600 comparisons, 600 repeat hits, 324 failing; (d) the memo is
     live (a planted entry for an unchanged file shows up, and disappears when the file changes); (e) a new `.wj.meta`
     sidecar is a miss.
   - Mutation checks (done, observed, reverted). Whole-run key restricted to the checked file's own content (dependency
     hashes omitted): (a) fails (`left: "[]"`, `right: [WJ0013 dep::f takes 2 arguments but 1 was supplied]`), (b) fails
     (stale `[]` against WJ0002), (d) fails; 3 of 5 red. Per-file key without the content hash: (b) fails once the
     generator includes the f32+f64 edit (stale `[]` against WJ0027; before that edit existed, only (d) caught it, so the
     generator was strengthened), (d) fails.
   - Found, not fixed (resolver, not this rung): `wj check` on a directory where several files define the same top-level
     function names reports some diagnostics at a nondeterministic file (six fresh runs over 12 `check_negative` mutants
     put WJ0013 at six different files). The first draft of (c) grouped 12 programs per project and tripped on it, which is
     why (c) checks one program per project.

### What is left
- Iterator chains (`closures`) and maps (`hashmap`) are bound by the value model, not dispatch: every closure call from a
  builtin allocates its argument vector, `Some(v)` allocates its type and variant names, and maps are `BTreeMap`s ordered by
  `structural_cmp`. Interning the `Option`/`Result` names and a hashed map representation are walker-wide changes.
- Floats and fields unboxed (step 6), `u64`/`usize` typed slots (an `i128` or checked-`u64` variant of `typed.rs`).
- Builtin methods are looked up by name on every call: cache the resolved builtin per call site.
- `match` on a place and closures with tuple parameters are walker islands / walker calls.

## Round 3 (Oct 2026): toward Node

Goal ladder: (1) machine-code loops: `loop5m`, `sieve`, `nbody` within 2x of Node; (2) value model: `hashmap`, `closures`,
`structs`, `strings` within 3x of Node; (3) typed slots from semantic facts; (4) startup; (5) tiering with type feedback,
as a spike only if 1-2 leave kernels more than 3x off Node.

Measured with `scripts/interp_compare.sh --runs 5` (child CPU seconds, median of 5, release `wj`, Node 24, Python 3.13) at a
1-minute load average of 36 (a shared machine; the run is recorded in `bench/interp/compare.json`, which the readiness
scorecard reads as `ratios.resolved_over_node`). Quick checks (best of 3-5) at load 20-30 during the work gave lower ratios
for the memory-bound kernels: `sieve` 0.10 s (1.8x Node), `nbody` 0.10 s (1.9x), `closures` 0.11 s, `hashmap` 0.15 s,
`structs` 0.25 s.

| kernel | walker | resolved (before round 3) | resolved | Node | Python | resolved / Node | goal |
|--------|--------|---------------------------|----------|------|--------|-----------------|------|
| `fib30` | 0.906 | 0.107 | 0.038 | 0.043 | 0.158 | 0.89x | - |
| `loop5m` | 2.369 | 0.190 | 0.042 | 0.048 | 1.102 | 0.86x | 2x: met |
| `sieve` | 1.520 | 0.599 | 0.197 | 0.056 | 0.458 | 3.5x (1.8x at load 25) | 2x: met at low load only |
| `nbody` | 1.352 | 0.888 | 0.133 | 0.050 | 0.152 | 2.6x (1.9x at load 25) | 2x: met at low load only |
| `closures` | 0.218 | 0.173 | 0.148 | 0.052 | 0.056 | 2.8x | 3x: met |
| `strings` | 0.226 | 0.114 | 0.112 | 0.034 | 0.070 | 3.3x | 3x: borderline |
| `hashmap` | 0.311 | 0.218 | 0.206 | 0.048 | 0.063 | 4.3x | 3x: not met |
| `structs` | 0.710 | 0.393 | 0.372 | 0.042 | 0.188 | 8.8x | 3x: not met |

`sieve` is bound by memory: a `Vec<bool>` of a million 48-byte `Value`s (48 MB) is strided through by the inner loop, against
Node's packed array; it is the most load-sensitive number here. A packed representation for `Vec`s of scalars is its lever.
The ratchet (`bench/interp/baseline.json`, walker / resolved) was rewritten from the same run's measurement
(`scripts/wj/interp_bench.wj --write`): `loop5m` 70.7x, `fib30` 36.7x, `sieve` 11.5x, `nbody` 9.8x.

### Rung 1: machine-code loops (met)
A hot `while` or range `for` loop in any compiled function (`main` included) enters Cranelift code at its head: on-stack
replacement at an iteration boundary, where every local is in its slot (`resolved/jit/osr.rs`). Locals the loop names are
classified at entry (`jit/kinds.rs`: a scalar of a given width, a `Vec` with the type its first element had, a struct with its
fields' types) and the loop is compiled for that classification (cached per loop, at most 4). Lowered (`jit/lower*.rs`):
`let`, assignment and compound assignment, `if`/`else`, nested `while` and `for i in a..b`/`a..=b`, `break`/`continue`
(labels included) as jumps, every integer width up to 64 bits with the walker's width tag tracked statically (`u64`/`usize` as
unsigned patterns, so every result the walker can produce is representable), `f64` (literal and `f64` tags), `bool`, casts,
`sqrt`/`abs`/`floor`/`ceil`/`trunc`/`round`, `v[i]`, `v[i].f`, `s.f` loads and stores, `v.push(x)`, `v.len()`; `Vec` and
struct accesses go through helpers (`jit/rt.rs`) that follow the walker's place rules (copy-on-write along the path, `retag`).

How it stays exact: machine code only takes the happy path. Any check that fails (an overflow, a zero divisor, an index out of
bounds, an element of another type than the one speculated, a literal that no longer fits, a width that would change) branches
to the exit of the statement being lowered: the exit stores the variables in scope, `osr.rs` writes them back to their slots and
the closure engine resumes at that statement, which has not done anything yet (a statement's only effect is its last action).
The rest of the iteration, and of any inner loop the statement sits in, runs from the statement codes the compiled function
registered (`resolved/osr.rs`: the very closures the loop body is made of), so panics, locations, backtraces and lock poisoning
are produced by the code that always produced them. `return` and the unit's own `break` are exits too. A simple statement outside the subset (a `let`,
assignment or expression statement with a call, a method call, `println`, a struct literal) does not refuse the loop: machine
code writes the variables in scope to their slots, runs that statement's registered closure code, and reloads them checked
against their types; if the statement raised, or left a variable with another type, machine code leaves after it and the raised
control is handed to the continuation. Machine code itself has no calls and never reaches a sink: every sink runs in closure
code, through the capability gate (`interpreter_jit_caps_test`: the loop with the sinks runs as machine code and the late sink
is denied with exit 126 exactly as with `--no-jit`).

Gates: `tests/interpreter_jit_loops_test.rs` generates programs (hot loops over `i32`/`int`/`usize`/`u8`/`f64`/`bool` locals,
`Vec<int>`/`Vec<f64>`/`Vec<bool>` elements, fields of a `Vec` of structs, NaN/infinity/`-0.0`, nested `while`/`for`,
`break`/`continue`, planted late overflows, out-of-bounds reads and writes, zero divisors, `usize` underflow) and runs each
under the walker, the resolved engine with every loop entering machine code at its first iteration (`WJ_JIT_LOOP_HOT=0`), the
default threshold and `--no-jit`: 150 generated programs, no difference (calls, `println` and a panic inside a callee included since closure-run statements landed). A coverage test requires at least 70% of the generated main loops to run as machine
code. In debug builds `WJ_JIT_MUTANT` leaves one check out (overflow, narrow-width range, zero divisor, bounds, NaN `!=`,
literal retag); the mutation gate requires each to make some generated program differ: all six caught (debug build). The engine parity corpus
(697 programs) shows no difference with the default threshold and with every loop entering machine code at its first
iteration; `tests/engine_parity/jit_loop_*.wj` add hand-written cases (widths, floats, exits, bounds, overflow, division,
speculation, structs, `for`).

Size: `jit/` is 2,797 lines in all (the function tier included), under step 7's 3k cap.

### Rung 2: value model (partly met)
Landed: enum names without allocation (`value::Name`: static for `Option`/`Result`/`Ordering`, interned in `EnumDef` for a
program's enums; a first version interned through a `thread_local!`, which the architecture ratchet forbids, so the names became a
type instead); struct literals resolved once per site (`resolved/literal.rs`); field reads of struct locals in place with a cache
keyed by the interned field name; one receiver read per method call and arguments evaluated straight into a compiled method's
frame; a per-call-site cache of the builtin method implementation (`call_builtin_method` split so the cached path runs the same
dispatch); closure parameters the body only reads are plain slots (no walker slot per call; a reference argument sends that call
to the walker); bare variant patterns (`Some(v)`) match without building a normalized name; integer map keys compare inline.

Not met for `structs` and `hashmap`: `hashmap` 4.3x and `structs` 8.8x Node at load 36 (`closures` and `strings` within or at 3x). Profiles (macOS `sample`, release): `structs` spends its time in the method-call
path (argument vector, frame and scope set-up, parameter binding, result coercion) and in boxed field arithmetic; `hashmap` in
`BTreeMap` search (about 30%) and allocation (about 20%: argument vectors, `Some` payload vectors, the `Rc` of every `Option`).
Both are structural: a hashed map index that every map builtin keeps in step, and a call path that writes arguments into the
callee frame (as direct calls already do) with methods inlined into machine code, are the next experiments.

### Rung 3: typed slots from semantic facts (stopped)
Kill criterion: under 1.5x on the affected kernels, or a parity difference. What the semantic layer would hand over (declared
widths, literal classes, field types) is what the resolved engine and the loop tier already see locally (typed slots from local
inference; field and element types from runtime classification at loop entry). The profiles above show the remaining cost is
not type dispatch but calls, allocation and map search, so a facts handoff cannot move these kernels by 1.5x. Not built; no
`ir` import was added.

### Rung 4: startup
Done on `wt/interp-startup` (separate branch): the stdlib taint/effect tables are computed once per check run and shared by
every pipeline, and check results are cached per project (whole run) and per file (analysis phase), keyed by content hashes of
everything they read (sources, manifest, lock file, dependencies, build identity, `.wj.meta` sidecars). Warm `wj run` of the
200-module project: 0.914 s to 0.070 s CPU; one module edited per run: 0.155 s. Gates: cached vs uncached diagnostics over
generated and real projects after random dependency edits; mutation checks (a key without dependency hashes, a per-file key
without the content hash) fail them. Details in that branch's section of this ADR.

### Rung 5: tiering with type feedback (spike: design only)
The loop tier already specializes on type feedback (classification at entry, guards with exits). For the kernels still more than
3x off Node the next tier is inlining: compile a hot loop together with the small user methods it calls (`acc.add(v)`,
`acc.dot(v)`), with struct values that do not escape the loop kept in registers (scalar replacement) and materialized at exits.
Prerequisites: calls in the loop subset (a call is an exit point today), a struct-construction helper, and an escape check over
the loop body. First experiment: `structs` with `add`/`dot` inlined by hand in Cranelift IR to measure the ceiling. Kill: under
2x on `structs`, or the inliner over 1.5k lines.

### Round 4 (Oct 26): merged-head verification, 32-byte `Value`
- Merged head (`interp-speed3` + `interp-startup`): effect replay/simulation/escape suites, interpreter suites and the engine
  parity corpus pass (the parity test now also runs every corpus program with every loop entering machine code at its first
  iteration, `WJ_JIT_LOOP_HOT=0`). `std_interpreter_agreement_test` fails on `caps::*` names another stream added to the runtime.
- Check caches and effect diagnostics: `tests/check_cache/effects.rs` (part of `check_cache_differential_test`) runs every
  non-xfail WJ004x spec example as a project cached and uncached, and edits the grant and a dependency so WJ0040/WJ0042 change
  for an unchanged file; cached and uncached agree in every state.
- Debug mutation gate re-run on the merged head: all six mutants caught, once NaN and literal-retag edges were planted in fixed
  thirds of the generated programs (the random mix alone had stopped exercising them).
- `Value` is 32 bytes (was 48): ranges and places are boxed, asserted at compile time. First step of the packed-`Vec` lever.

`scripts/interp_compare.sh --runs 5` at load 60 (`bench/interp/compare.json`), resolved / Node: `loop5m` 0.79x, `fib30` 0.76x,
`sieve` 2.31x (3.5x before the smaller `Value`, load 36), `nbody` 2.35x, `strings` 2.28x, `closures` 2.77x, `hashmap` 3.81x,
`structs` 7.12x.

Next, in order: `Vec`s of scalars packed (a `Value::Vec` whose storage is `Vec<i64>`/`Vec<f64>`/`Vec<bool>` with a tag,
unpacked lazily by the accessors the builtins share; every `Vec` builtin goes through `vec_of`/`vec_mut`, which is the seam);
a hashed index for maps kept in step by `KMap`'s mutation methods (today `KMap` derefs to `BTreeMap` mutably, so the index needs
those mutations to go through `KMap` first); method inlining in machine-code loops with non-escaping structs in registers.

## Proposals beyond milestone 1 (as written in milestone 1; see steps 1-8 for where each stands)
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

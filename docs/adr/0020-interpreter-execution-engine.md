---
status: accepted
last_verified: 0ad66981d7b3
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

With `--features jit` (step 7) `fib30` takes 0.02 s.

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

### Step 7: Cranelift tier (spike, `--features jit`)
`resolved/jit.rs` (520 lines) compiles functions whose every value is `int` and whose body is arithmetic, comparisons, `let`,
assignment, `if`, `while`, `return` and calls to such functions, together with their callees, after 64 calls. A jitted call
has no side effect, so on any edge (overflow, zero divisor, `i64::MIN / -1`, recursion past 2000 jitted frames) it bails
out and the call runs again in the closure engine, which produces the panic, span and backtrace. Kill criteria: at least 5x
over milestone 1 on `fib30` (0.33 s to 0.02 s: about 16x, met), at most 3k lines (met), and either the same arithmetic
helpers or a differential test over generated programs: `tests/interpreter_jit_differential_test.rs` runs programs whose
`b * b` overflows only after the functions are hot under the walker and the jitting engine; a mutant without the multiply
overflow check makes it fail. The feature stays off by default (it adds the Cranelift crates); turning it on by default is a
separate decision (binary size and build time against the dev-loop win, which matters only for hot integer kernels).

### Step 8: parse/resolve cache for startup (measured, not built)
A generated 200-module project (4k functions) starts in about 0.9 s CPU. Profiling shows that at least 80% of it is the pre-run
check gate (`cli::check::require_clean`: IR lowering, `ir::taint::stdlib_taint_sources`, effect lookups), not the
interpreter's parse/resolve/registration, so a per-module interpreter cache cannot meet its kill criterion (30%). The lever is in
the check pipeline: memoize the stdlib taint sources (a function of the stdlib only, about a quarter of the profile) and cache
check results per file content hash. Those modules belong to the convergence and effects streams.

### What is left
- Iterator chains (`closures`) and maps (`hashmap`) are bound by the value model, not dispatch: every closure call from a
  builtin allocates its argument vector, `Some(v)` allocates its type and variant names, and maps are `BTreeMap`s ordered by
  `structural_cmp`. Interning the `Option`/`Result` names and a hashed map representation are walker-wide changes.
- Floats and fields unboxed (step 6), `u64`/`usize` typed slots (an `i128` or checked-`u64` variant of `typed.rs`).
- Builtin methods are looked up by name on every call: cache the resolved builtin per call site.
- `match` on a place and closures with tuple parameters are walker islands / walker calls.

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

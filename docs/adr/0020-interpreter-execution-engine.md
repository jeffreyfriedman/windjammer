---
status: proposed
last_verified: d45cde56a728
---
# ADR-0020: Interpreter execution engine (tree walker vs resolved/bytecode tier)

- **Status:** Proposed
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

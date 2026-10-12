---
status: accepted
last_verified: 81b57bf0
---
# ADR-0018: Expressiveness Phase 2

- **Status:** Accepted (2026-10-11)
- **Date:** 2026-10-11
- **Related:** [ADR-0014](0014-surface-syntax-rust-isms.md) (Phase 2 proposals), [ADR-0015](0015-language-consistency-proposals.md), [ADR-0001](0001-ownership-truth.md) (one decision-maker), [ADR-0007](0007-target-tiers-conformance.md), `docs/design/pipe-operator.md`, `docs/spec/`

## Context
Windjammer's surface is still close to Rust for everyday data work: `.iter().map(..).collect()`, explicit closures for
one-field projections, positional-only calls, `Option` spelled out, doc examples that nobody runs, and `if/else if`
ladders where a guard table would read better. Ruby (Enumerable), Elixir (Enum, `cond`, doctests), Kotlin (trailing
lambdas, `?.`/`?:`), Swift and Scala (`_` shorthand) show that these are the places where both humans and agents write
shorter, more obviously-correct code. Every feature below is *sugar over existing core constructs*: the parser (or one
desugaring pass) produces the AST that every backend already handles, and every semantic choice (types, ownership,
integer widths, which lowering applies) is made once by the IR solver (ADR-0001). No backend re-decides.

Cross-cutting rules (all seven features):
- Spec rules get `SPEC-<AREA>-NNN` IDs with runnable examples (`tests/spec/<area>/`), the grammar is in
  `docs/spec/grammar.ebnf`, the formatter round-trips the new syntax, misuse has golden diagnostics.
- Conformance programs with independently derived `// expect:` oracles run on the interpreter, Rust, Go, JS and TS.
- Every feature records an agent-dx measurement (`bench/agent-dx`): reference solutions rewritten idiomatically,
  first-draft pass rate and token deltas, reported honestly including regressions.
- Order: 1, 2, 6 first (independent of parser statement-boundary work), then 3, 4, 5, 7.

## 1. Rich collection library (area `COLL`)
**Surface.** On `List`/`Vec`, `Set`, `Map` (as an iterator of `(k, v)`), ranges and iterators:

| Kind | Methods |
|---|---|
| grouping/aggregation (terminal, eager) | `group_by(f) -> Map<K, List<T>>`, `count_by(f) -> Map<K, int>` (aliases `tally()` = `count_by(identity)`, `frequencies()`), `sum_by(f)`, `min_by_key(f)`, `max_by_key(f)`, `partition_by(f) -> (List<T>, List<T>)`, `index_of(x) -> int?`, `first()`, `last()`, `nth(i)` |
| reshaping (eager on lists, lazy on iterators where Rust is lazy) | `unique()`, `unique_by(f)`, `chunk_by(f) -> List<List<T>>` (runs of equal keys), `chunks(n)` (alias `each_slice`), `windows(n)`, `intersperse(sep)`, `zip_with(other, f)`, `flat_map(f)`, `scan(init, f)`, `take_while(p)`, `skip_while(p)` |
| ordering | `sort_by_key(f)`, `sort_by_key_desc(f)`, `sorted()`, `sorted_by_key(f)` (non-mutating, returns a new list) |

`group_by`/`count_by` return maps with **deterministic iteration order** (insertion order of first key occurrence) on
every backend, so printing them is portable. `min_by_key`/`max_by_key` return the *first* minimal/maximal element (ties
are stable; Rust's `max_by_key` returns the last, so the runtime helper is used rather than std's).

**Definition once.** The typed surface is declared in `std/enumerable.wj` (signatures + doc comments, no bodies) and
in one Rust table, `src/stdlib_surface/enumerable.rs`, which the analyzer, the portable typer (Go/JS/TS) and the Rust
backend all consult for arity, closure parameter types and result types. Implementations: one per backend -
Rust `windjammer_runtime::enumerable` (extension traits `WjIterExt` on `Iterator` and `WjSliceExt` on `[T]`, no
itertools), Go/JS helpers in `std/runtime/{go,js}/wj_core.*`, interpreter `src/interpreter/builtins/enumerable.rs`.

**Types/ownership.** Closure parameters receive the element *by reference* (`&T`) exactly like `filter`; key closures
must return an owned key (the solver inserts the clone for non-Copy keys, as for `map`). Results that contain elements
(`group_by`, `unique`, `chunks`, ...) on a *collection* receiver contain owned copies (clones; free for Copy types);
on an iterator receiver they contain the iterator's items. Counts are `int`. The solver decides the closure parameter
types from the surface table; codegen never matches method names to decide ownership.

**Lowering.** Rust: method call on the receiver with `use windjammer_runtime::enumerable::*;` emitted when used;
lazy adaptors (`scan`, `take_while`, `skip_while`, `flat_map`, `intersperse`, `zip_with`) are `Iterator` adaptors,
zero-cost. Go/JS/TS: helper calls on arrays (`wjGroupBy`, `$groupBy`, ...). Interpreter: native builtins.
Pipes: `xs |> group_by(_.dept)` is `group_by(xs, ..)` only if a free function exists; the method form
`xs |> .group_by(_.dept)` is the pipe spelling (pipe rules unchanged).

**Diagnostics.** Wrong arity / non-closure argument: E-level error naming the surface signature. `chunks(0)`/
`windows(0)` panic with the same message on every backend.

**Kill/rollback.** Any method whose output differs between backends on the conformance corpus is removed from the
surface table (not silently patched per backend). If the runtime helpers cost > 5% versus a hand-written Rust loop in
`benches/`, the helper is rewritten, not the surface.

## 2. Shorthand closures with `_` (area `CLOS`)
**Syntax.** Inside a call or method-call argument, an expression containing `_` (other than the argument being
exactly `_`) is a closure: `users.filter(_.active)`, `xs.map(_ * 2)`, `pairs.sort_by_key(_.1)`.
Each `_` is a new parameter, left to right: `xs.fold(0, _ + _)` is `|a, b| a + b`.

**Scoping rule (normative).** The closure is the *smallest enclosing call argument* that contains the `_`:
`f(g(_ + 1))` is `f(g(|x| x + 1))`. A bare `_` that *is* an argument is never a closure:
- in a `|>` stage, a bare direct argument `_` is the pipe placeholder (unchanged);
- anywhere else it is an error ("`_` alone is not a function; write `|x| x` or name the function").
So `xs |> zip_with(_, ys, _ + _)` puts `xs` at the bare `_` and `_ + _` is a 2-parameter closure; there is no
ambiguity because the two meanings are distinguished syntactically (bare argument vs part of an argument).

**Misuse diagnostics.** `_` outside any call argument (`let f = _ + 1`): error with the `|x| x + 1` suggestion. `_`
inside an explicit closure in the same argument (`map(|x| x + _)`): error "mix of `_` shorthand and explicit closure".
`_` patterns (`let _ =`, `match` arms) are unaffected.

**Lowering.** The parser emits a plain `Closure` with fresh parameter names (`__it0`, `__it1`, ...); all backends
and the solver see an ordinary closure, so ownership and parameter types follow the closure rules (the solver types
`__it0` from the callee signature, e.g. `&T` for `filter`).

**Formatter.** Token-based, prints the source spelling; never expands shorthand. **Kill:** if agent-dx shows more
first-draft failures with `_` than with explicit closures, keep only the single-`_` form.

## 3. Named arguments with defaults (area `ARGS`)
**Syntax.** `fn connect(host: string, timeout: int = 30, retries: int = 3)`; call `connect("db", timeout: 5)`.
Rules: parameters with defaults must follow those without; at a call, positional arguments come first, then named
ones in any order; each parameter is bound at most once; unknown names, duplicates, positional-after-named and missing
required parameters are errors with the parameter list in the message. Defaults are **evaluated at the call site, on
each call, after the explicit arguments, in parameter order** (Python's shared-mutable-default bug cannot happen) and
may refer to earlier parameters (`fn pad(s: string, width: int = s.len())`).

**Lowering.** One desugaring pass (`src/desugar/named_args.rs`) rewrites every resolved call to a positional call
with the defaults inlined (each default expression cloned per call site), before analysis. Backends see positional
calls only. Methods: resolved when the receiver's type is known to the pass or the method name is unique among
user `impl`s; otherwise the call must be positional (error says so). Trait methods: defaults belong to the trait
declaration; an impl may not redeclare them (error). FFI/`extern fn`: named arguments allowed, defaults allowed
(the desugared call is positional, so the foreign ABI is unchanged). Function values (`let f = connect`) have the full
positional signature; no defaults through function pointers.

**Pipes/inference.** `x |> connect(timeout: 5)` inserts `x` first, then desugars. Default expressions are typed against
the parameter type by the solver; an `int` default for an `i32` parameter takes the parameter's width.

**Kill:** if the method resolution heuristic produces a wrong binding in the conformance corpus, restrict named
arguments to free functions and associated functions until the DefId resolver lands.

## 4. Trailing block arguments (area `BLOCK`)
**Syntax.** A call `f(args)` immediately followed, on the same line, by `{ params -> body }` or `{ body }` passes the
block as the **last positional argument**, a closure: `with_file(path) { f -> f.read_all() }`,
`button("Save") { save() }`, `items.each() { x -> println(x) }`. Parameters: `{ a, b -> ... }`, optionally typed
`{ x: int -> ... }`. Exactly one trailing block per call (decided: no multiple trailing blocks; use named arguments
for several callbacks: `request(url, on_ok: |r| .., on_err: |e| ..)`). The parentheses are required (`run() { }`),
so a block after a bare identifier remains a struct literal or a block.

**Disambiguation.** Not recognised in the condition/scrutinee of `if`, `while`, `match`, and `for .. in` (the
same restriction as struct literals), where `{` starts the body. A `{` on the next line starts a new statement.

**Lowering.** Parser appends `Closure { params, body }` to the argument list; everything else is closure semantics.
Combined with named arguments, the block binds the last *positional* parameter not bound by name.
**Formatter:** keeps `) {` on one line, indents the block body; one-line blocks stay one line.
**Kill:** if the parser restriction list grows beyond the struct-literal contexts, drop the zero-parameter form.

## 5. Optional types `T?`, `?.` and `??` (area `OPT`)
**Types.** `T?` is exactly `Option<T>` (same type, interchangeable, printed as `T?` in diagnostics). `T??` is
rejected (write `Option<T?>` if you really mean it).

**`a?.b`, `a?.m(..)`.** When `a: T?`: yields `U?` where `U` is the type of `.b`; if `.b` is itself an option it is
flattened (`and_then`), so `a?.b?.c` is one optional. When `a` is a `Result`, `a?.b` keeps today's meaning (propagate
the error with `?`, then access), so existing Rust-style code is unchanged. The choice is made by the IR solver from
the receiver type (one decision); to propagate a `None` and then access, write `(a?).b`.

**`x ?? d`.** `x.unwrap_or_else(|| d)`: `d` is evaluated only when `x` is `None`; `d` may itself be an option
(`a ?? b ?? 0`). `??` is right-associative and binds looser than arithmetic and tighter than comparison (Swift,
Kotlin): `x ?? 0 + 1` is `x ?? (0 + 1)`, `x ?? 0 > 3` is `(x ?? 0) > 3`.
On a `Result`, `??` uses the `Ok` value or `d` (error discarded).

**Interaction.** `?` (propagate) is unchanged; `|>?` unchanged; `x?.y |> f` pipes the optional.
**Lint.** `.unwrap()` on an option: W-lint suggesting `?` (in an option/result function) or `?? default`;
`wj fix` rewrites `x.unwrap_or(d)` to `x ?? d` (always safe, `??` is lazier) and `Option<T>` to `T?` in signatures.
**Lowering.** Parser: `T?` type -> `Type::Option`; `??` -> `MethodCall unwrap_or_else(Closure)`; `?.` -> an
`OptChain` marker the solver resolves to `map`/`and_then`/try-then-access before codegen.
**Kill:** if `?.` on `Result` vs `Option` confuses agents (agent-dx first-draft failures attributable to it), make
`?.` on `Result` an error with a fix-it instead of the Rust meaning.

## 6. Doctests (area `DOC`)
Fenced ` ```wj ` / ` ```windjammer ` examples in `///` doc comments of `.wj` files are tests. `// expect: line`
lines are the expected stdout (in order); a fence without `expect` must run without panicking; annotations are the
docs-truth ones (`ignore`, `should_fail`, `should_fail=CODE`, `check`, `run`). `wj test` runs them in the interpreter
by default (fast, no cargo) and `wj test --compiled` also builds and runs each through the Rust backend;
`wj test --doc` runs only doctests. A fence that is a statement fragment runs as the body of `fn main`, and the
documented module's items are in scope (the fence is appended to the module source with `main` renamed).
Reuse: the fence extractor, annotation parser and expect-line reader move from `tools/docs_truth` into the library
module `src/doctest/`, used by both docs-truth and `wj test` (one extractor). Reports name `file:line` of the fence.
**Kill:** none expected; if interpreter and compiled results differ, the doctest fails (that is a backend bug).

## 7. Guard-only match (area `MATCH`)
`match { x < 0 => "neg", x == 0 => "zero", _ => "pos" }`: a `match` with no scrutinee; arms are boolean
conditions tried in order (Elixir `cond`). The last arm must be `_` (exhaustiveness: otherwise error "guard-only
match needs a final `_ =>` arm"), and `_` must be last (an arm after `_` is unreachable: error). Lowering: an
`if / else if / else` chain (an expression); typing is that of `if` (all arms same type). Pipes: `match` without a
scrutinee is an expression and can be piped into (`match { .. } |> f`). Formatter: same layout as `match`.
**Kill:** if it causes parse ambiguity with a scrutinee expression starting with `{` (a block scrutinee), require
`match {` to be followed by an arm pattern (`expr =>`) and keep block scrutinees parenthesised.

## Backend lowering summary
| Feature | Parser/desugar | Solver | Rust | Go | JS/TS | Interpreter |
|---|---|---|---|---|---|---|
| 1 collections | method calls | closure param/result types from surface table | runtime ext traits | `wj_core.go` helpers | `wj_core.mjs` helpers | native builtins |
| 2 `_` closures | `Closure` | closure typing | closure | func literal | arrow fn | closure |
| 3 named/defaults | positional call | default typed at param | call | call | call | call |
| 4 trailing block | `Closure` arg | closure typing | closure | func literal | arrow fn | closure |
| 5 `T?`/`?.`/`??` | `Option`, `unwrap_or_else`, `OptChain` | picks map/and_then/try | Option API | wjOption | `$opt` | Option values |
| 6 doctests | n/a | n/a | `--compiled` | n/a | n/a | default runner |
| 7 guard match | `if` chain | if typing | `if` | `if` | `if` | `if` |

## Consequences
- New spec areas COLL, CLOS, ARGS, BLOCK, OPT, DOC, MATCH (chapters in `docs/spec/`, examples in `tests/spec/`).
- The collection surface table becomes the single place a stdlib collection method is declared for analysis.
- `?.` on options changes meaning versus Rust (no in-tree uses relied on the Rust meaning on options; measured
  2026-10-11: `grep -rE ')\?\.' --include=*.wj` finds only comments).
- Implementation status per feature is tracked in the section below and in `docs/spec/BACKEND_STATUS.md`.

## Implementation status
(updated as features land; see the per-feature commits on `wt/expressive`)

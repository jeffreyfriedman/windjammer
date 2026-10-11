# Interpreter (Windjammerscript): status, design, measurements

Goal: once transpilation is correct, Windjammer should offer a Go-like quick loop (edit, run, see the result in
milliseconds) through an interpreted mode, with compiled Rust as the release path. This document records what the
interpreter does, how it relates to the compiler, how it is measured, and what is still missing.
Module map and invariants: `src/interpreter/MODULE.md`.

## How to use it

| Command | Effect |
|---------|--------|
| `wj run --interpret FILE_OR_DIR [args]` | Run a file, or a project directory (`main.wj` / `src/main.wj`), with `use`/`mod` resolved across files. No cargo, no rustc. stdout is program output only. |
| `wj test --interpret [PATH] [--filter S]` | Run `@test`, `@test_cases([...])` and `test_*` functions in `*_test.wj` files. cargo-test-like report, exit 1 on failure. |
| `wj repl` | Persistent session: `let`, `fn`, `struct`, `enum`, `impl`, `use` survive between inputs; the value of an expression is printed. `:reset`, `:help`, `exit`. |

Exit status: 0 ok; 101 panic (Rust's code; message `panicked at file:line:col`, backtrace of Windjammer frames);
N for `std::process::exit(N)`; 2 for "unsupported in the interpreter" (names the feature and points at `wj run`); 1 other.

Latency (same machine, heavily loaded, warm cargo cache): `wj run hello.wj` (transpile, cargo build, run) 13.9 s;
`wj run --interpret hello.wj` 0.016 s. A test asserts the interpreter never spawns `cargo`/`rustc`
(shims on `PATH` that leave a marker file).

## Before / after

Measured with `scripts/interpreter_coverage.py` over every `.wj` file with a `fn main` under `tests/` and `examples/`
(244 files), the 98-program cross-backend conformance corpus (in the go-js-backends worktree: `// expect:` lines and
Python reference vectors, an oracle independent of every backend), and the compiled-Rust output of each file
(`wj build --no-cargo` + `rustc`, 90 of the 244 compile and run).

| Metric | Before (frozen `wj` 923b4c67) | After |
|--------|-------------------------------|-------|
| Files exiting 0 | 231 / 244 (95%), but see below | 136 / 244 |
| Output + exit code equal to compiled Rust, of the 90 that compile | 69 / 90 (77%) | 85 / 90 (94%) |
| Conformance corpus equal to the independent oracle | 27 / 97 (28%) | 97 / 98 |
| Quick feature probes (`scripts/interpreter_probes.py`, 30 programs) | 11 / 30 | 30 / 30 |

"Exit 0" is not a quality signal for the old engine: it evaluated unknown names, failed calls and unsupported
constructs to `Nil` and carried on, so 231 files "ran" while producing wrong output (`2147483648` for an `i32` overflow,
`None` for any iterator chain, unexpanded `{:>5}` format specs, a hang on `while let`). The new engine fails loudly
instead: the 108 files that no longer exit 0 do so with a located diagnostic (unsupported feature, unknown name,
parse error in the shared parser), never with silent wrong output. The meaningful numbers are the output-match and
oracle rows.

Reproduce: `python3 scripts/interpreter_coverage.py <wj> out.json --corpus <conformance_programs> --rust-diff --rust-cache c.json tests examples`
(add `--driver <all-test-binary>` to run in-process through `interpreter_probe_test::probe_one`).

## What works

Structs, impls, associated functions, `Self`; enums with unit/tuple/struct variants, `match` with guards, or-patterns,
tuple/struct/enum patterns, `if let`, `while let`, `let else`; traits with default methods and dynamic dispatch by runtime
type; generics (erased: dispatch is dynamic); closures with by-reference capture and mutation; function values; `?` on
`Result`/`Option` with `From` conversion to a user error type; operator overloading (`add`, `neg`, `eq`, `partial_cmp`...);
`impl Display`/`Debug` (via `write!`); `${}` interpolation, `format!` specs (width, fill, align, sign, `#`, `0`, precision,
`x X b o e`, `?`, `#?`), named and positional arguments; integer widths i8..i128/u8..u128/isize/usize with Rust overflow
panics, `wrapping_*`/`checked_*`/`saturating_*`/`overflowing_*`, shifts, `as` casts (wrap and saturate), f32 rounding;
`Vec`/`VecDeque`, `HashMap`/`BTreeMap` (ordered by key), `HashSet`/`BTreeSet`, `entry` API, `Option`/`Result` combinators,
strings and chars, eager iterator adaptors (`map filter fold sum zip enumerate rev take skip chain flat_map windows chunks
sort_by ...`), `collect` into `Vec/String/HashMap/HashSet/Result/Option`, pipe operator `|>` (desugared by the parser),
`const`/`static mut`, `defer`, multi-file projects, `std::{fs,env,process,io::stdin,time,mem,cmp,math,strings}` subsets,
and the `std::test` assertion helpers. Ternary does not exist in the language (removed in the parser).

## Semantics and design decisions

**Relation to the compiler pipeline.** The interpreter consumes the same lexer and parser output (`Program<'static>`) as
every backend, so syntax cannot fork. It does not consume the analyzer/IR: the IR solver
(ADR-0001) computes ownership and numeric widths for code generation, but it exposes no typed expression tree an evaluator could
walk. Re-using it would need a typed-IR export (the "Backend trait over the IR" work item); until then the interpreter
re-derives the two facts it needs, locally and testably:

1. *Integer width and overflow.* Every integer value carries its width. Unsuffixed literals adopt the other operand or
   the declared type of the binding, parameter, field or return value (`coerce.rs`). Arithmetic panics on overflow (debug-build
   semantics, exit 101), as `rustc` without `-O` does. Known deviation: an expression made only of unsuffixed literals and
   variables bound to them is checked against `i64`, not Rust's default `i32`, so a program that overflows only because
   inference defaults to `i32` will not panic.
2. *Inferred `&mut`.* Windjammer infers `&mut self`/`&mut param` from the body. `analysis.rs` scans the body (assignments,
   mutating method calls, `&mut`, calls into other mutating functions) and passes a `Value::Ref(place)` for exactly those
   parameters. Values are copy-on-write `Rc` trees, so `let b = a` is a cheap, independent copy, `.clone()` is free, and
   mutation through a place copies only when shared.

**Errors map to source spans.** Statements and calls record the parser's source location; a panic prints
`file:line:col`, the message in Rust's wording (`index out of bounds: the len is 3 but the index is 9`,
`called Option::unwrap() on a None value`, `attempt to add with overflow`) and the Windjammer call stack. The parser records a node's
location after consuming it, so the reported line can be the line after the failing expression.

**Unsupported means an error, never a guess.** `async/await`, `spawn`, channels, `thread {}` blocks, `impl Drop`,
extern functions, `T::new()` through type parameters, and std modules that are not implemented (`http`, `json`, `ui`...)
produce `unsupported in the interpreter: ...` and exit 2.

**Modularity.** Evaluator, value model, builtin registry, module loader and CLI port are separate files
(`MODULE.md`). Adding stdlib is registering a function in `builtins/*.rs`; nothing else changes.

## Differential testing

`tests/interpreter_differential_test.rs` runs 11 seeded programs (`tests/interpreter_corpus/*.wj`, `__SEED__` template,
3 to 5 seeds each) and compares the interpreter, through the real `wj run --interpret`, with (a) committed vectors that are
re-derived from independent Python reference models on every run, and (b) the compiled-Rust pipeline. A print-faking
interpreter cannot pass because each seed changes the data. Harness self-tests: a program that ignores its seed must be
rejected; an oracle for seed A must not accept the output of seed B; and source mutants (`+`->`-`, `*`->`+`, a literal bumped)
must be killed (>= 80%) by the oracle comparison. The conventions (`// seeds:`, `// expect:`, `// exit:`, `.vectors/<seed>.out`)
match the cross-backend conformance corpus, so these programs can be added to it unchanged.

Findings from running the same programs through the compiled path (compiler bugs, not interpreter bugs; the corpus avoids them):
- `move` closure capture copies: `let mut c = 0; let mut f = || { c += 1 }; f(); f()` prints 0 when compiled, 2 interpreted (the language intent).
- `map[key]` with a `&str` key transpiles to `map[key as usize]` (rustc error); `map.get(&k).copied().unwrap_or(0)` types the default as `i32`.
- A `u64` literal above `i64::MAX` (for example the FNV offset basis) panics the lexer (`number_lexing.rs:210`).
- `"x" + i.to_string()` is emitted as `&str + &String` (rustc error); `match s.parse::<T>()` parses the scrutinee as field access plus a nameless turbofish call.
- Capturing closures passed where a `fn(int) -> int` parameter is declared fail in rustc.
- Statements starting with `*` after a line ending in `)` are parsed as a multiplication continuation (`*m.entry(k).or_insert(0) += 1`).

## Remaining gaps

- Not interpreted: async/spawn/channels/threads, `impl Drop`, extern/FFI, generic static dispatch (`T::default()`),
  `Rc/RefCell/Mutex` interior mutability, infinite iterators (`cycle`, unbounded ranges: adaptors are eager), UI/http/json/db std modules,
  macros other than the built-in set, `matches!`.
- `u128` values above `i128::MAX` are not representable. `HashMap` iterates in key order (Rust's order is unspecified).
- Performance: a tree walker, roughly two orders of magnitude slower than compiled code; fine for tests and dev, not for benchmarks.
- 108 of 244 repository `.wj` files still fail, dominated by files that need the std modules above, `T::method()` dispatch, UI/engine
  APIs and a few parser limitations; the breakdown is in the JSON the script emits.
- Debug builds of `wj` limit recursion to 10,000 interpreted frames (reported as a stack-overflow panic).

## Proposed amendment to ADR-0007 (one paragraph, not applied)

> *Interpreter tier.* The interpreter (`wj run --interpret`, `wj test --interpret`, `wj repl`) is a Tier-2 development
> target with a narrower contract than the compiled targets: it implements the language subset listed in
> `docs/INTERPRETER.md` with Rust-identical stdout, panic behaviour and exit codes (debug-build integer semantics), and reports any
> other construct as `unsupported in the interpreter` (exit 2). It is gated by `tests/interpreter_differential_test.rs`
> (independent Python oracles plus compiled Rust) and `tests/interpreter_semantics_test.rs`; a regression in either blocks a
> release. It is not a release path; `wj build` remains the production route.

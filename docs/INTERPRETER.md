# Interpreter (Windjammerscript): status, design, measurements

Goal: once transpilation is correct, Windjammer should offer a Go-like quick loop (edit, run, see the result in
milliseconds) through an interpreted mode, with compiled Rust as the release path. This document records what the
interpreter does, how it relates to the compiler, how it is measured, and what is still missing.
Module map and invariants: `src/interpreter/MODULE.md`.

## How to use it

`wj run` interprets by default (ADR-0019): no cargo, no rustc, milliseconds to first output. When a program uses a
feature listed as unsupported below, the load-time preflight detects it before `main` runs and `wj run` switches to
the compiled path with one line on stderr (`wj: running compiled (cargo): ...`).

| Command | Effect |
|---------|--------|
| `wj run` | Run the project containing the working directory: `wj.toml` is found upward, its default binary runs (`src/main.wj`, `main.wj`, the first `[[bin]]`). |
| `wj run PATH [args]` | A `.wj` file, or a directory with `main.wj` / `src/main.wj` (`wj run ./cmd/tool`). Use `wj run . args` to pass arguments, as with `go run . args`. |
| `wj run --bin NAME` | A `[[bin]]` entry of `wj.toml`, `cmd/NAME/main.wj` or `src/bin/NAME.wj`. |
| `wj run --interpret ...` | Interpreter only: never fall back (an unsupported feature exits 2 naming it). |
| `wj run --compiled ...` / `--release` | Transpile to Rust and run with cargo (`--release`: optimized). `wj build` is unchanged. |
| `wj run --watch` | Re-run on every change to `.wj`, `.rs`, `wj.toml`, `wj.lock` under the project (restarts long-running programs). |
| `wj test --interpret [PATH] [--filter S] [--watch]` | Run `@test`, `@test_cases([...])` and `test_*` functions in `*_test.wj` files. cargo-test-like report, exit 1 on failure. |
| `wj repl` | Persistent session: `let`, `fn`, `struct`, `enum`, `impl`, `use` survive between inputs; the value of an expression is printed. `:reset`, `:help`, `exit`. |

Exit status: 0 ok; 101 panic (Rust's code; message `panicked at file:line:col`, backtrace of Windjammer frames);
N for `std::process::exit(N)`; 2 for "unsupported in the interpreter" (names the feature, suggests `wj run --compiled`); 1 other.
Arguments pass through (`std::env::args`), stdin is inherited. A test asserts the interpreter never spawns `cargo`/`rustc`
(shims on `PATH` that leave a marker file); the one exception is the host bridge build below, done once per Rust change.

## Projects

**One module system.** The interpreter loads a program with the compiler's own file walker and module paths
(`module_system::sources`, the same walker behind the compiled build) and binds every `use` with the compiler's resolver
(`resolve`, `DefId`s): nested modules, `mod.wj` with `pub mod`, directories without `mod.wj`, aliases, groups `{a, b}`, globs,
`self`/`super`/`crate` paths, `pub use` re-exports, cyclic imports, and same-name functions in different modules. Visibility follows
Rust's rule on the resolved definitions: calling or importing a private item from outside its module is
``function `helper` is private (E0603)``, as rustc reports it for the compiled program. A `main.wj` owns its directory tree (every
module in it is loaded, like the compiled build); a loose script loads the modules it `use`s.

**Dependencies.** `wj.toml` `[dependencies]` with `path` or `git` (ADR-0010 option A) are resolved by `package::graph`: a
Windjammer package (it has `wj.toml` or `.wj` sources) is loaded as source and mounted as a crate (`use textkit::banner`); git
dependencies are cloned once into the wj cache and pinned in `wj.lock` (written on first resolution; later upstream commits are not
picked up until the lock entry is removed). Rust-crate dependencies are reached through `extern fn` and the host bridge.

**Rust code via the host bridge (ADR-0020).** When a project declares `extern fn`s and has hand-written Rust next to `wj.toml` or in
`src/` (`ffi.rs`, the files the compiled build links), `wj run` compiles that Rust once into a cached dynamic library with one
generated C-ABI shim per `extern fn`, loads it, and calls the same symbols the compiled binary calls. Integers, floats, `bool` and
`string` cross the bridge; any other type is rejected before the program runs, naming the item, and `wj run` falls back to the
compiled path. Measured on the `host_bridge` fixture (loaded machine): first run 214 s (cargo builds `windjammer-runtime` in release
for the bridge crate), every later run 10 to 30 ms.

**Performance** (`scripts/interpreter_project_bench.py`, median wall-clock on a loaded 10-agent machine, release `wj`):

| Measurement | Time |
|---|---|
| `wj run hello.wj` (process start + load + run) | 30 ms |
| `wj run` in a generated 50-file project (3 functions and a struct per module, cross-module calls), first run | 31 ms |
| same, repeated runs | 35 ms |
| `wj run --watch`: source edit to new output | 44 ms (poll interval 20 ms) |

Regenerate with `python3 scripts/interpreter_project_bench.py <wj> --write` (records the keys `interpreter.run_hello_ms`,
`interpreter.project_50_cold_ms`, `interpreter.project_50_warm_ms`, `interpreter.watch_restart_ms` in `docs/generated/measured.json`).

**Parity gates** (ADR-0019; the default stays interpreted only while these are green):
- `cargo test --release --test interpreter_features`: every registry probe behaves as registered, every `Unsupported` probe is caught
  by the preflight at load time (so it triggers the fallback) and no `Supported` probe is.
- `cargo test --release --test interpreter_projects`: the fixtures in `tests/interpreter_projects/` (nested modules, every `use` form,
  same names and cycles, privacy error, binaries/args/exit codes, path dependency, host bridge) match hand-written expectations
  interpreted and through the default `wj run` (which must not fall back); a seeded fuzzer generates 40 random module trees and
  compares the interpreter with the generator's own model (self-test: a mutated program is detected). `WJ_PROJECT_COMPILED=1`
  adds the compiled half for fixtures and the first fuzz seeds.

**Known gaps.** Two types with the same name in different modules are not interpreted yet (types are keyed by name in the
engine; the preflight sends such programs to the compiled path). `crate::` inside a dependency resolves against the program
root. Compiled runs of `src/bin/NAME.wj` are not supported yet (use `cmd/NAME/main.wj`). Hot reload of function bodies for
long-running programs is designed in ADR-0021 (`--watch` restarts today).

## Measurement

Two oracles, neither of them a hand-typed number:

- **Feature registry.** `src/interpreter/features.rs` lists every feature the interpreter is claimed to support or to reject, each with a
  probe program. `tests/interpreter_features_test.rs` runs every probe through the real interpreter. The lists below are generated from it:
  Currently:

  <!-- wj:gen name=metric key=interpreter.features_supported -->
26
  <!-- wj:end -->

  supported entries, and

  <!-- wj:gen name=metric key=interpreter.features_unsupported -->
2
  <!-- wj:end -->

  entries that must fail with "unsupported".
- **Output equality with compiled Rust and independent reference models.** `scripts/interpreter_coverage.py` runs every `.wj` file with a
  `fn main` under `tests/` and `examples/` through the interpreter and, where it compiles, through the Rust pipeline, and compares output
  and exit code; the conformance corpus is compared with its `// expect:` lines and Python reference vectors (see "Differential testing").
  Results are measurements of a given commit, so they are not typed here: CI writes them to `docs/generated/measured.json`.

The old engine evaluated unknown names, failed calls and unsupported constructs to `Nil` and carried on, so programs "ran" while producing
wrong output (`2147483648` for an `i32` overflow, `None` for any iterator chain, unexpanded `{:>5}` format specs, a hang on `while let`).
The current engine fails loudly instead: an unsupported feature, an unknown name or a parse error in the shared parser is a located
diagnostic, never silent wrong output. Exit 0 is therefore a meaningful signal, and the output-equality oracles are the quality measure.

Reproduce: `python3 scripts/interpreter_coverage.py <wj> out.json --corpus <conformance_programs> --rust-diff --rust-cache c.json tests examples`
(add `--driver <all-test-binary>` to run in-process through `interpreter_probe_test::probe_one`).

## What works

Generated from the registry (do not edit; change `src/interpreter/features.rs`):

<!-- wj:gen name=interpreter_supported -->
- **Types**: structs, impls, associated functions, `Self`; enums with unit/tuple/struct variants; `match` with guards and or-patterns; tuples and destructuring
- **Control flow**: `if let`, `while let`, `let else`; `for` over ranges and iterators, `while`, `loop`/`break`/`continue`; `defer`
- **Traits and generics**: traits with default methods; dynamic dispatch by runtime type; generic functions (erased; dispatch is dynamic); operator overloading via `add`, `neg`, `eq`, ...
- **Functions**: closures capturing by reference, mutation of captures, function values; pipe operator `|>`; `?` on `Result`/`Option`
- **Formatting**: `${}` interpolation; format specs: width, fill, align, precision, radix, `?`; `{:?}` and `{:#?}` on collections and structs
- **Numbers**: integer widths with Rust overflow semantics (`wrapping_*`, `checked_*`, `as` casts); arithmetic overflow panics (exit 101)
- **Collections**: `Vec` push/len/index/sort/dedup, `VecDeque`; `HashMap`/`BTreeMap`, `HashSet`/`BTreeSet`, `entry` API; iterator adaptors (`map filter fold sum zip enumerate rev take skip chain flat_map windows chunks sort_by`), lazy `cycle`/`repeat`/`from_fn`; `Option`/`Result` combinators
- **Strings**: string and char methods, `split`, `parse`
- **Memory**: `impl Drop`: destructors at scope exit, in reverse declaration order; `Rc`/`Arc`, `RefCell`/`Cell`, `strong_count` by reachability
- **Concurrency**: `async fn` and `.await` (lazy futures run on `.await`)
- **Intrinsics**: `assert`, `assert_eq`, `panic`, `todo`, `debug`
<!-- wj:end -->

Also covered by the engine but not itemised in the registry yet: `std::{fs,env,process,io::stdin,time,mem,cmp,math,strings}` subsets and
the `std::test` assertion helpers. Ternary does not exist in the language (removed in the parser).

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

**Unsupported means an error, never a guess.** Registered unsupported features (generated from the registry) produce
`unsupported in the interpreter: ...` and exit 2:

<!-- wj:gen name=interpreter_unsupported -->
- **Concurrency**: `thread { }` and `async { }` blocks (ADR-0004: surface not settled; use `async fn` and `.await`)
- **FFI**: calls to `extern fn`
<!-- wj:end -->

Other unimplemented built-ins (for example std modules such as `http`, `json`, `ui`) fail the same way, with the name of the missing function.

**Modularity.** Evaluator, value model, builtin registry, module loader and CLI port are separate files
(`MODULE.md`). Adding stdlib is registering a function in `builtins/*.rs`; nothing else changes.

**Intrinsics (ADR-0014 Phase 1).** `crate::intrinsics::Intrinsic` is the only name registry the interpreter consults;
`interpreter/intrinsics.rs` evaluates `print/println/eprint/eprintln` (and `stdout.*`/`stderr.*` sugar, lowered by the parser),
`panic/todo/unreachable`, `assert/assert_eq/assert_ne`, `debug(x)`, `format` and the list literal, for both the plain call and the
deprecated `name!(..)` macro spelling (same code path; `dbg!` is `debug`, `unimplemented!`/`debug_assert*!` map onto it). A user
function, import or local variable with an intrinsic's name shadows it. Behaviour mirrors compiled Rust: stdout and stderr stay
separate; panics exit 101 with Rust's text (`assertion failed: <cond>`, ``assertion `left == right` failed: <msg>\n  left: ..\n right: ..``,
`not yet implemented: <msg>`, `internal error: entered unreachable code: <msg>`); `debug(x)` writes `[file:line:col] <expr> = <{:#?} value>`
to stderr and returns `x`; a single message argument is printed verbatim (`println("{x}")` prints braces), several arguments are the
legacy `{}` template and an argument the template never uses is an error, as in rustc. `[a, b]` is a growable `Vec`; a closure literal in call
position (`(|v| v + 1)(x)`, which is what `x |> |v| v + 1` desugars to) is applied directly. Shapes the interpreter does not model
(`debug(a, b)`, `assert()`, `assert_eq(a)`, named arguments to an intrinsic) are `unsupported` errors (exit 2).
Known deviations: a `[T; N]` annotated array is not distinguished from a `Vec`, so `.push` on it is accepted (rustc rejects it);
the `<cond>`/`<expr>` text is re-rendered from the AST (identifiers, literals, operators, calls, fields, indexing) and falls back to
a bare `assertion failed` for other shapes, and rustc echoes the generated source, so an integer literal there carries the suffix
the compiler inferred (`x == 6_i64`).

## Differential testing

`tests/interpreter_differential_test.rs` runs the seeded programs (`tests/interpreter_corpus/*.wj`, `__SEED__` template,
3 to 5 seeds each) and compares the interpreter, through the real `wj run --interpret`, with (a) committed vectors that are
re-derived from independent Python reference models on every run, and (b) the compiled-Rust pipeline. A print-faking
interpreter cannot pass because each seed changes the data. Harness self-tests: a program that ignores its seed must be
rejected; an oracle for seed A must not accept the output of seed B; and source mutants (`+`->`-`, `*`->`+`, a literal bumped)
must be killed (>= 80%) by the oracle comparison. The conventions (`// seeds:`, `// expect:`, `// exit:`, `.vectors/<seed>.out`)
match the cross-backend conformance corpus, so these programs can be added to it unchanged. The `intrinsic_*` programs cover
interpolation specs, list literals, pipe/closure-in-call-position, stream separation with `debug`, and a failing `assert_eq`
(exit 101). `intrinsic_stderr_and_exit_match_compiled_rust` additionally compares stderr (locations and the backtrace footer normalised)
for every panic/assert/todo/unreachable/debug/eprintln form against compiled Rust; its self-test checks that a message sent to the wrong
stream or with wrong text is detected.

Findings from running the same programs through the compiled path (compiler bugs, not interpreter bugs; the corpus avoids them):
- `move` closure capture copies: `let mut c = 0; let mut f = || { c += 1 }; f(); f()` prints 0 when compiled, 2 interpreted (the language intent).
- `map[key]` with a `&str` key transpiles to `map[key as usize]` (rustc error); `map.get(&k).copied().unwrap_or(0)` types the default as `i32`.
- A `u64` literal above `i64::MAX` (for example the FNV offset basis) panics the lexer (`number_lexing.rs:210`).
- `"x" + i.to_string()` is emitted as `&str + &String` (rustc error); `match s.parse::<T>()` parses the scrutinee as field access plus a nameless turbofish call.
- Capturing closures passed where a `fn(int) -> int` parameter is declared fail in rustc.
- Statements starting with `*` after a line ending in `)` are parsed as a multiplication continuation (`*m.entry(k).or_insert(0) += 1`).

## Added in the interp-gaps work

- **Generic static dispatch** (`generics.rs`): `T::new()`, `T::default()`, `T::CONST` resolve through a per-call table
  `T -> concrete type`, bound from a turbofish, then argument runtime types, then the expected type (`let x: Foo = make()`,
  a function's declared return type). An uninferable parameter is a located `unsupported` error naming the fix.
  Impl-level parameters are bound the same way; they are not recovered from `&self` field types.
- **`matches!(e, pat [if guard])`** is lowered by the parser to a `match` (all backends). Open ranges `(a..)` parse as
  `std::ops::RangeFrom { start }` (the Rust backend emits `(a..)`).
- **Lazy iterators** (`iter.rs`, `builtins/iter.rs`): adaptors are pull-based nodes, so `cycle`, unbounded ranges,
  `repeat`, `from_fn`, `successors` work under `take`/`take_while`/`find`/`break`, and closure side effects interleave exactly as in
  Rust. Materialising more than 50M items from one iterator is a loud error. `rev` is lazy for double-ended sources
  and materialises otherwise.
- **`impl Drop`** (`heap.rs`): destructors run at block/function exit in reverse declaration order, on reassignment, `drop(x)`,
  discarded temporaries and `let _ =`; owner before fields, `Vec` elements in order. Ownership is answered by reachability from
  the live frames (the value model has no move tracking), so a value passed into a callee is dropped when its last holder dies
  (the caller's scope exit), not at the callee's end. Panics unwind through destructors; `process::exit` does not.
- **`Rc`/`Arc`/`Weak`, `RefCell`/`Cell`/`Mutex`/`RwLock`** (`builtins/shared.rs`): `Rc` is a shared slot; `strong_count`,
  `weak_count` and `Weak::upgrade` are computed by reachability, so they follow scope exit and `drop`. Reference cycles
  are not leaked (a cycle with no outside holder is considered dead). `borrow_mut()` never panics with "already borrowed"
  and `lock()` never blocks or poisons.
- **`async fn` / `.await`**: calling an async function returns a lazy future; `.await` (and an `async fn main`) runs it
  to completion, so order matches Rust for single-task programs. There is no executor: `spawn`, `async expr`, channels,
  `thread {}` stay unsupported.
- Recursion is bounded by stack bytes used on the 4 GiB interpreter thread (about 120k interpreted frames in a debug
  build), not a frame count; exceeding it is a panic, never a native stack overflow.
- Smaller: `None` patterns no longer match everything; `module.func()` and `Enum.Variant()` legacy spellings; bare names
  from `use std::math`; `math::*` float functions; `fs::is_dir/is_file`; an implicit `self` in `fn f()` impl methods.

## Remaining gaps

- Not interpreted: `spawn`/channels/threads (needs a settled ADR-0004 surface and a scheduler), extern/FFI, UI/http/json/db/regex/
  subprocess std modules, wall-clock `time::now()` objects, `Rc::make_mut`/`get_mut`, RefCell borrow-state panics,
  macros other than the built-in set plus `matches!`.
- `u128` values above `i128::MAX` are not representable. `HashMap`/`HashSet` iterate in key order (Rust's order is
  unspecified, so a correct program cannot depend on it; programs that print an unordered map differ from any single Rust run).
- Drop/`Rc` reachability limits: values held only by a pending Rust-side temporary are invisible to the scan; a moved-in value
  outlives the callee; iterator adaptors other than `Vec`-backed ones do not hold values alive for the scan.
- Performance: a tree walker, roughly two orders of magnitude slower than compiled code; the optimizer kernel
  benchmarks time out in debug builds.
- Of the 382 repository `.wj` files with a `main`, 160 still do not exit 0: 37 are `__SEED__` corpus templates, about 25 are
  tests that panic on purpose, and the rest need UI/engine/http/json/regex std modules or use syntax the shared parser rejects.
  Re-measure with `scripts/interpreter_coverage.py`.

## ADR-0007

The interpreter tier paragraph is applied as an amendment in `docs/adr/0007-target-tiers-conformance.md`.

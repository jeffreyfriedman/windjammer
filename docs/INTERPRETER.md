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
| `wj run --hot` | Preview: keep the program running and swap in edited function bodies (state survives; ADR-0021). |
| `wj test --interpret [PATH] [--filter S] [--list] [--watch]` | Run exactly the tests `wj test` discovers (shared discovery: `@test`, `@test_cases`, `test_*` at top level, in inline `mod`s and receiver-less `impl` functions, in any `.wj` file). cargo-test-like report, exit 1 on failure. |
| `wj run --sched-seed N` | Run threads/tasks with schedule seed N (0 = FIFO run-to-block; other seeds reproducibly explore interleavings at every synchronisation point). |
| `wj run --deterministic-time` | No real clock: time starts at a fixed instant and only `sleep` moves it; the global RNG is seeded. |
| `wj run --mock-http FILE` / `--allow-network` | Answer `std::http` client calls from canned responses / allow plain `http://` (default: the client refuses). |
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

**Compiled-path bugs found by the project gates** (fixed here unless marked open):
- Fixed: `use greet` in a crate-root `src/main.wj` was emitted as `use super::greet` (E0433); it is now `crate::` (rewritten to the
  package lib in the binary).
- Fixed: a user module named like a runtime std module (`util::math`) also got `use windjammer_runtime::math;` (E0252).
- Fixed: `util::math::cube(3)` or `crate::x::f()` in the binary's `main.rs` did not resolve (the modules live in the package lib);
  the binary now names the lib for `crate::` paths and glob-imports it.
- Fixed: `wj run` (compiled) printed build progress on stdout; it now goes to stderr so both paths print the same bytes.
- Fixed: a path dependency on a Windjammer source package was handed to Cargo as-is; `wj run --compiled` now builds it into the
  package's `gen/` crate first and the generated `Cargo.toml` links that.
- Open: `wj build .` on a project root (rather than `src/`) treats `src/main.wj` as a module and drops `fn main`; `wj run` passes the
  source root, so it is unaffected.
- Open: module aliases and `super::` paths in calls (`use crate::alpha as m3` then `m3::f0()`, `super::beta::f()`) fail with
  `missing boundary signature for m3::f0`: the ownership boundary lookup is keyed by the written path, not the resolved definition.
  The compiled half of the fuzzer therefore uses item aliases, globs, groups and `crate::` paths (`WJ_FUZZ_ALL_FORMS=1` compiles
  every form); the interpreted half covers all forms.
- Open: `env::args().skip(1).collect()` into `Vec<string>` fails in rustc (iterates `&String`).

**Known gaps.** Two types with the same name in different modules are not interpreted yet (types are keyed by name in the
engine; the preflight sends such programs to the compiled path). `crate::` inside a dependency resolves against the program
root. Compiled runs of `src/bin/NAME.wj` are not supported yet (use `cmd/NAME/main.wj`). Hot reload (`wj run --hot`, ADR-0021) is a
preview: edited function and method bodies with unchanged signatures are swapped in at the next call and program state is
kept; other edits print a restart notice (use `--watch` for those).

## Measurement

Two oracles, neither of them a hand-typed number:

- **Feature registry.** `src/interpreter/features.rs` lists every feature the interpreter is claimed to support or to reject, each with a
  probe program. `tests/interpreter_features_test.rs` runs every probe through the real interpreter. The lists below are generated from it:
  Currently:

  <!-- wj:gen name=metric key=interpreter.features_supported -->
35
  <!-- wj:end -->

  supported entries, and

  <!-- wj:gen name=metric key=interpreter.features_unsupported -->
3
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
- **Memory**: `impl Drop`: destructors at scope exit, in reverse declaration order; `Rc`/`Arc`, `RefCell`/`Cell`, `strong_count` by reachability; `RefCell` borrow tracking with Rust's `BorrowMutError` panic
- **Concurrency**: `async fn` and `.await` (lazy futures run on `.await`); `thread::spawn`/`join`, `thread::scope`, `sleep` on a virtual clock (deterministic cooperative scheduler; `--sched-seed N` explores other interleavings); `mpsc::channel`/`sync_channel`: send, recv, try_recv, recv_timeout, iteration; closes when every sender is gone; `Arc<Mutex>`/`RwLock` guards (released at scope end), `Condvar`, `Barrier`, atomics; deadlocks are reported
- **Intrinsics**: `assert`, `assert_eq`, `panic`, `todo`, `debug`
- **Std modules**: `Duration`, `Instant`, `SystemTime`, `std::time::DateTime` (chrono formats); `std::json` dynamic values and typed `to_string`/`parse_string::<T>`; `std::regex` (same engine as compiled programs); `std::fs` files, directories (`read_dir`, `metadata`), path helpers; `csv`, `url`, `mime`, `crypto` (sha/base64/bcrypt), `yaml`, `compress`, `jwt`, `uuid`, seeded `random`
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
- **Std modules**: `std::http` client without a capability flag (use `--mock-http FILE` or `--allow-network`); `http::serve` servers
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

- **Generic static dispatch** (`generics.rs`): `T::new()`, `T::default()`, `T::CONST` through a per-call table bound from a
  turbofish, argument runtime types, then the expected type. Uninferable parameters are a located `unsupported` error.
- **`matches!`** lowered by the parser; open ranges `(a..)` as `std::ops::RangeFrom`; `-a..b` read as `(-a)..b`.
- **Lazy iterators** (`iter.rs`, `builtins/iter.rs`): pull-based adaptors, infinite sources, Rust's side-effect order.
- **Concurrency** (`sched.rs`, `builtins/{thread,channel,sync,time}.rs`): `std::thread` (spawn, join, scope, sleep,
  Builder), `mpsc` (channel, sync_channel, try/timeout receive, iteration), `Arc<Mutex/RwLock>` guards released at scope
  end, `Condvar`, `Barrier`, atomics, and the ADR-0004 surface (`spawn f(x)`/`spawn { }`, `async f(x)`, `scope { }`,
  `Task.wait()`/`.await`, `std::sync::channel()` with `send`/`recv` methods). One OS thread per task, but a baton runs
  exactly one at a time: a run is a pure function of the program and `--sched-seed`. `sleep`/timeouts use a virtual clock
  (no real waiting); a state where every task is blocked and none waits on time is reported as a deadlock. Properly
  synchronised programs print the same output under every seed (tested over 10 seeds); a check-then-act race across two lock
  acquisitions shows both outcomes across seeds (tested).
- **Ownership and memory** (`heap.rs`, `locks.rs`, `builtins/shared.rs`): `impl Drop` at scope exit, on reassignment,
  `drop(x)` (forced: the object dies even if an auto-borrowed alias remains), discarded temporaries, `let _ =`, and for elements a
  mutating builtin discards (`clear`, `truncate`, `remove`, overwritten entries); `Rc`/`Arc` counts by reachability,
  `Weak` is a real weak pointer so `Rc` cycles leak exactly as in Rust; `Rc::make_mut`/`get_mut`; `RefCell` borrow tracking
  with Rust's `BorrowMutError`/`BorrowError` panics (guards are tokens that live as long as some copy of the guard value).
  `mut` parameters are owned copies (Q-015).
- **Std modules**: `fs` (all of `std/fs.wj`, `read_dir`, metadata, path helpers), `path`, `env`, `process`, `time`
  (`Duration`, `Instant`, `SystemTime`, `DateTime`), `json` (dynamic and typed `to_string`/`parse_string::<T>`), `regex`,
  `csv`, `url`, `mime`, `crypto` (sha1/256/512, base64, bcrypt), `yaml`, `compress`, `jwt`, `uuid`, seeded `random`, `log`,
  `strings`, `testing`; `http` client behind a port (`--mock-http`, `--allow-network`). Same crates as `windjammer-runtime`,
  so results match compiled programs; tests use Python oracles (`interpreter_std_modules_test`).
- **Language**: Rust's slicing panic messages (`v[a..b]`, `s[a..b]`, char boundaries), `char::to_lowercase()` displays,
  top-level macro invocations are loud, `Type::default()` for `@derive(Default)` (Q-016), recursion bounded by stack bytes on
  the interpreter thread (Q-023), `wj test --interpret` shares `wj test` discovery.
- **Performance**: call-site cache, frame-vector pool, fast paths (10-15% in release). ADR-0020 proposes a resolved/closure-compiled
  tier with kill criteria.

Measured (release `wj`, Oct 2026): repository `.wj` files with `fn main` under `tests/` and `examples/`: 585 of 723 exit 0
(first interp-gaps measurement: 222 of 382 on the smaller tree then). Of the 138 that do not: 40 are `__SEED__` corpus templates,
35 panic on purpose (spec/error examples), 18 are parse errors in negative spec examples or syntax the shared parser rejects,
9 time out (optimizer benchmark kernels), and about 30 need unimplemented libraries (UI/game engine, http servers,
`std::cli`, subprocess pipes). `bench/agent-dx` reference solutions: 60/60 (was 54/60).

## Remaining gaps

- Not interpreted: extern/FFI; `http::serve` servers; `std::cli`; `std::subprocess` pipes; `db`; `config::*`; UI/game engine
  APIs; `thread {}`/`async {}` statement blocks (deprecated by ADR-0004); `Rc` cycles are leaked but their destructors
  are not distinguished from reachable values in drop order; mutex poisoning (a panicking holder does not poison).
- Moves are not tracked: a value moved into a callee by an owned parameter (other than via `drop(x)`) stays reachable from the
  caller's variable until that dies, so its destructor runs at the caller's scope exit rather than the callee's. Values held only
  by pending Rust-side temporaries are invisible to the reachability scan.
- `u128` above `i128::MAX` is not representable. `HashMap`/`HashSet` iterate in key order (Rust's order is unspecified).
- `"${f(\"x\")}"` (escaped quotes inside an interpolation) is a lexer error.
- Performance: release `wj` is roughly 200-400x slower than compiled Rust on call/loop-heavy code (ADR-0020).

## ADR-0007

The interpreter tier paragraph is applied as an amendment in `docs/adr/0007-target-tiers-conformance.md`.

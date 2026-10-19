# Interpreter (Windjammerscript): status, design, measurements

Goal: once transpilation is correct, Windjammer should offer a Go-like quick loop (edit, run, see the result in
milliseconds) through an interpreted mode, with compiled Rust as the release path. This document records what the
interpreter does, how it relates to the compiler, how it is measured, and what is still missing.
Module map and invariants: `src/interpreter/MODULE.md`.

## How to use it

| Command | Effect |
|---------|--------|
| `wj run --interpret FILE_OR_DIR [args]` | Run a file, or a project directory (`main.wj`, or `main.wj` under a `src` directory), with `use`/`mod` resolved across files. No cargo, no rustc. stdout is program output only. |
| `wj test --interpret [PATH] [--filter S] [--list]` | Run exactly the tests `wj test` discovers (shared discovery: `@test`, `@test_cases`, `test_*` at top level, in inline `mod`s and receiver-less `impl` functions, in any `.wj` file). cargo-test-like report, exit 1 on failure. |
| `wj run --interpret --sched-seed N` | Run threads/tasks with schedule seed N (0 = FIFO run-to-block; other seeds reproducibly explore interleavings at every synchronisation point). |
| `wj run --interpret --deterministic-time` | No real clock: time starts at a fixed instant and only `sleep` moves it; the global RNG is seeded. |
| `wj run --interpret --mock-http FILE` / `--allow-network` | Answer `std::http` client calls from canned responses / allow plain `http://` (default: the client refuses). |
| `wj repl` | Persistent session: `let`, `fn`, `struct`, `enum`, `impl`, `use` survive between inputs; the value of an expression is printed. `:reset`, `:help`, `exit`. |

Exit status: 0 ok; 101 panic (Rust's code; message `panicked at file:line:col`, backtrace of Windjammer frames);
N for `std::process::exit(N)`; 2 for "unsupported in the interpreter" (names the feature and points at `wj run`); 1 other.

Latency (same machine, heavily loaded, warm cargo cache): `wj run hello.wj` (transpile, cargo build, run) 13.9 s;
`wj run --interpret hello.wj` 0.016 s. A test asserts the interpreter never spawns `cargo`/`rustc`
(shims on `PATH` that leave a marker file).

## Measurement

Two oracles, neither of them a hand-typed number:

- **Feature registry.** `src/interpreter/features.rs` lists every feature the interpreter is claimed to support or to reject, each with a
  probe program. `tests/interpreter_features_test.rs` runs every probe through the real interpreter. The lists below are generated from it:
  Currently:

  <!-- wj:gen name=metric key=interpreter.features_supported -->
40
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
- **Numbers**: integer widths with Rust overflow semantics (`wrapping_*`, `checked_*`, `as` casts); arithmetic overflow panics (exit 101); `u128` over its full range
- **Collections**: `Vec` push/len/index/sort/dedup, `VecDeque`; `HashMap`/`BTreeMap`, `HashSet`/`BTreeSet`, `entry` API; iterator adaptors (`map filter fold sum zip enumerate rev take skip chain flat_map windows chunks sort_by`), lazy `cycle`/`repeat`/`from_fn`; `Option`/`Result` combinators; `OrderedMap`/`OrderedSet`: insertion order, `remove` keeps the order of the rest; `HashMap`/`HashSet` order is unspecified (`--hash-seed N` permutes it)
- **Strings**: string and char methods, `split`, `parse`
- **Memory**: `impl Drop`: destructors at scope exit, in reverse declaration order; `Rc`/`Arc`, `RefCell`/`Cell`, `strong_count` by reachability; `RefCell` borrow tracking with Rust's `BorrowMutError` panic; moves decide where destructors run: into collections, aggregates, `let`, partial moves, callee parameters
- **Concurrency**: `async fn` and `.await` (lazy futures run on `.await`); `thread::spawn`/`join`, `thread::scope`, `sleep` on a virtual clock (deterministic cooperative scheduler; `--sched-seed N` explores other interleavings); `mpsc::channel`/`sync_channel`: send, recv, try_recv, recv_timeout, iteration; closes when every sender is gone; `Arc<Mutex>`/`RwLock` guards (released at scope end), `Condvar`, `Barrier`, atomics; deadlocks are reported
- **Intrinsics**: `assert`, `assert_eq`, `panic`, `todo`, `debug`
- **Std modules**: `Duration`, `Instant`, `SystemTime`, `std::time::DateTime` (chrono formats); `std::json` dynamic values and typed `to_string`/`parse_string::<T>`; `std::regex` (same engine as compiled programs); `std::fs` files, directories (`read_dir`, `metadata`), path helpers; `csv`, `url`, `mime`, `crypto` (sha/base64/bcrypt), `yaml`, `compress`, `jwt`, `uuid`, seeded `random`; `std::http` client and servers (`serve`, `serve_fn`, `Server::serve`) behind a capability port: refused by default, `--mock-http FILE` replays responses/requests, `--allow-network` uses real sockets; `std::config` (toml/yaml, same code as the runtime), `std::db` on SQLite, `std::subprocess` pipes, `std::cli` (clap)
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
- **Ownership and memory** (`heap.rs`, `moves.rs`, `locks.rs`, `builtins/shared.rs`): `impl Drop` at scope exit, on reassignment,
  `drop(x)` (forced: the object dies even if an auto-borrowed alias remains), discarded temporaries, `let _ =`, and for elements a
  mutating builtin discards (`clear`, `truncate`, `remove`, overwritten entries); `Rc`/`Arc` counts by reachability,
  `Weak` is a real weak pointer so `Rc` cycles leak exactly as in Rust; `Rc::make_mut`/`get_mut`; `RefCell` borrow tracking
  with Rust's `BorrowMutError`/`BorrowError` panics (guards are tokens that live as long as some copy of the guard value).
  `mut` parameters are owned copies (Q-015). Moves decide where an object dies: `let y = x`, partial moves
  (`let l = p.left`), aggregates (`S { f: x }`, tuples, arrays, variants) and consuming builtins (`push`, `insert`, `send`)
  move a variable's ownership; a callee that moves its parameter (or owns it by rule: `owned`, `mut`, generic `T`) takes
  the caller's object, so it dies at the callee's end; a parameter the callee only reads stays a borrow. Mutex poisoning:
  a panic unwinding past a held guard poisons the lock (`PoisonError`, `is_poisoned`, `clear_poison`).
- **Std modules**: `fs` (all of `std/fs.wj`, `read_dir`, metadata, path helpers), `path`, `env`, `process`, `time`
  (`Duration`, `Instant`, `SystemTime`, `DateTime`), `json` (dynamic and typed `to_string`/`parse_string::<T>`), `regex`,
  `csv`, `url`, `mime`, `crypto` (sha1/256/512, base64, bcrypt), `yaml`, `compress`, `jwt`, `uuid`, seeded `random`, `log`,
  `strings`, `testing`; `http` client and servers behind a port (`--mock-http`, `--allow-network`; axum routing rules,
  mock files replay recorded requests), `config` (the runtime's own code path), `db` on SQLite (Postgres is refused with an
  error), `subprocess` line pipes, `cli` (clap), `u128` over its full range. Same crates as `windjammer-runtime`, so results
  match compiled programs; tests use Python, rustc and runtime-code oracles (`interpreter_std_modules_test`,
  `interpreter_http_server_test`, `interpreter_config_db_test`). Every top-level `std/*.wj` function has a builtin
  (`std_interpreter_agreement_test`, ratchet at 0); the four legacy `config::get*` hooks and the two `csv::*_with_delimiter`
  builders (no implementation in any backend) are loud errors.
- **Language**: Rust's slicing panic messages (`v[a..b]`, `s[a..b]`, char boundaries), `char::to_lowercase()` displays,
  top-level macro invocations are loud, `Type::default()` for `@derive(Default)` (Q-016), recursion bounded by stack bytes on
  the interpreter thread (Q-023), `wj test --interpret` shares `wj test` discovery.
- **Performance**: call-site cache, frame-vector pool, fast paths (10-15% in release). `--engine=resolved` (ADR-0020 option B,
  milestone 1): functions compiled to closures over slot-indexed locals, walker fallback per function; results in ADR-0020.
  `scripts/interp_bench.py` is the benchmark ratchet (median of 5, load recorded).

Measured (release `wj`, Oct 2026): repository `.wj` files with `fn main` under `tests/` and `examples/`: 585 of 723 exit 0
(first interp-gaps measurement: 222 of 382 on the smaller tree then). Of the 138 that do not: 40 are `__SEED__` corpus templates,
35 panic on purpose (spec/error examples), 18 are parse errors in negative spec examples or syntax the shared parser rejects,
9 time out (optimizer benchmark kernels), and about 30 need unimplemented libraries (UI/game engine, http servers,
`std::cli`, subprocess pipes). `bench/agent-dx` reference solutions: 60/60 (was 54/60).

## Remaining gaps

- Not interpreted: extern/FFI; UI/game engine APIs; Postgres (`db::connect("postgres://..")`); `thread {}`/`async {}`
  statement blocks (deprecated by ADR-0004); `Rc` cycles are leaked but their destructors are not distinguished from
  reachable values in drop order.
- Moves: a moved variable stays readable (a valid program never reads it; where Windjammer would auto-clone it, the
  interpreter shares the object instead of copying it, so that clone's destructor does not run). Returning a parameter
  does not move the caller's variable early (the object is reachable from the result either way). Values held only by
  pending Rust-side temporaries are invisible to the reachability scan.
- `HashMap`/`HashSet` iterate in key order by default (Rust's order is unspecified; ADR-0021 D6). `--hash-seed N` permutes it
  deterministically, and `interpreter_hash_order_test` runs the verified examples, spec examples and conformance programs under
  several seeds to flag order-dependent programs (`tests/data/hash_order_dependent.txt`, shrink-only, currently empty).
  `OrderedMap`/`OrderedSet` iterate in insertion order (`remove` keeps the order of the rest, `swap_remove` does not).
- `"${f(\"x\")}"` (escaped quotes inside an interpolation) is a lexer error (owned by the parser stream; ignored test
  `interpolation_escaped_quotes_test`).
- Performance: release `wj` is roughly 200-400x slower than compiled Rust on call/loop-heavy code; see ADR-0020.

## ADR-0007

The interpreter tier paragraph is applied as an amendment in `docs/adr/0007-target-tiers-conformance.md`.

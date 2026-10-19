# interpreter (Windjammerscript)

## Purpose
Run Windjammer source directly, with no cargo/rustc, for an instant dev loop
(`wj run` interprets by default, ADR-0019; `wj test --interpret`, `wj repl`). Rust remains the release path;
the interpreter must never print something the compiled program would not print.
Status and measurements: `docs/INTERPRETER.md`.

## Inputs / outputs
- In: `.wj` source parsed by the compiler's own lexer/parser (`Program<'static>`); a file or a project dir.
- Out: program stdout/stderr text, an exit status (0 ok, 101 panic, N for `process::exit(N)`,
  2 unsupported-in-interpreter, 1 other), or `RuntimeError { kind, message, span, backtrace }`.

## Layout (one concern per file)
- `value.rs`, `int.rs`   value model; integer widths (`IntTy`) with Rust overflow rules
- `engine.rs`            `Interpreter` state, public entry points (`run`, `run_checked`, `call_main`)
- `register.rs`, `loader.rs`  definitions; `loader::program_files` picks the files of a program (a `main.wj` owns its
  source tree, `module_system::sources`; a loose script loads what it `use`s); legacy lazy `use` for the REPL
- `project.rs`           whole-program load: module tree from module paths, every `use` bound by the compiler's resolver
  (`crate::resolve`), Rust privacy (E0603) on resolved defs; dependencies mounted as crates
- `preflight.rs`         load-time detection of registry-`Unsupported` features (the `wj run` compiled-fallback trigger)
- `deps.rs`              Windjammer dependencies as source (`package::graph`: path/git + `wj.lock`)
- `host_bridge.rs`       `extern fn` through a cached bridge library (ADR-0020): signatures, shim source, libloading calls
- `eval_expr.rs`, `eval_stmt.rs`, `pattern.rs`, `place.rs`  evaluation, patterns, `&mut` places
- `generics.rs`, `fasthash.rs`  type-parameter bindings (`T::new()`); address hasher for call-site caches
- `iter.rs`, `builtins/{iter,shared}.rs`  lazy iterators; Rc/Weak/cells. `heap.rs` reachability: Rc counts, `impl Drop`
- `call.rs`, `analysis.rs`  name resolution, calls, closures, inferred-`&mut` parameter analysis
- `ops.rs`, `coerce.rs`, `format.rs`, `defaults.rs`  operators, `as`/binding coercions, `format!`, `Default`
- `intrinsics.rs`        ADR-0014 intrinsics (print/panic/assert/debug/list) driven by `crate::intrinsics::Intrinsic`
- `builtins/`            native stdlib registry (`Registry`; `native(type, method)` for host objects): core types, `std_*` modules, `test_assert` (std::test Option/Result/string assertions), `thread`/`channel`/`sync`/`time`
- `sched.rs`, `locks.rs`, `native.rs`  deterministic task scheduler (virtual clock), guard tokens, host-object trait; `slicing.rs` panic texts
- `runner.rs`, `repl.rs` CLI port: run/test/REPL, exit codes, big-stack thread; only place with process I/O policy
- `error.rs`, `features.rs`  `RuntimeError`, `Ctl` (break/continue/return/error), spans, backtraces; `features.rs` = executable feature registry that generates the INTERPRETER.md lists

## Invariants
- Values are immutable trees behind `Rc` with copy-on-write; `&mut` is a `Place` (slot + projections).
- Integers carry their width; unsuffixed literals (`IntTy::Lit`) adopt the other operand/declared type.
  Overflow panics (debug-build semantics); `wrapping_*`/`checked_*`/`as` follow Rust.
- A parameter/receiver the body mutates is passed as `Value::Ref(place)` (mirrors inferred `&mut`).
- Never wrong-but-quiet: unknown name/method => `Semantic` error; unimplemented feature => `Unsupported`
  error naming it. No silent `Nil` fallbacks.
- No `cargo`/`rustc` spawn anywhere in this module (asserted by `tests/interpreter_cli_test.rs`); the host bridge library
  is built by `cli::host_build`, the interpreter only loads it.
- Names are bound by `crate::resolve`, never by a private resolver: a `use` means the same thing interpreted and compiled.
- Every `Unsupported` registry feature has a preflight detector (gate: `interpreter_features`).

## Owns / never decides
Owns evaluation semantics, value model, native stdlib registry, CLI port. Never decides compiled-path behaviour (mirrors Rust).

## Allowed imports
`crate::parser` (AST), `crate::lexer`, `crate::source_map::Location`, `crate::resolve` + `crate::module_system`
(the compiler's module layout and resolver: one source of truth), `crate::package` (dependency graph),
`crate::type_display`, `crate::intrinsics` (name registry only; a ratcheted known violation like codegen/parser/linter), std, libloading. Nothing from `codegen`, `analyzer`, `cli`, `optimizer`. CLI crates call only `runner::*`, `repl::*` and `Interpreter`.

## Entry points
`runner::run_entry` (fallback-aware, used by `cli::run_dev`), `runner::{run_path, run_path_with, run_specs}` (test discovery is `test_runner::interpreted`), `Interpreter::load_program_at`, `repl::run_repl_stdio`, `Interpreter::{new, new_capturing, run, run_checked}`.

## Intrinsics
Names come only from `crate::intrinsics::Intrinsic` (plain call and `name!` spelling alike); do not register them in
`builtins/`. Panic text mirrors rustc; unmodelled shapes return `unsupported` (exit 2). A user item or local of the
same name shadows the intrinsic. Add a new intrinsic: extend the registry, then one arm in `eval_intrinsic`.

## Extending the stdlib
Add `reg.pure/mutating/place(Kind::X, "name", f)` or `reg.func("path::fn", f)` in the matching `builtins/*.rs`.

## Test
- `interpreter_semantics_test` hand-derived Rust semantics; `interpreter_differential_test` vs Python reference models and compiled Rust (+ harness self-tests; stdout, exit status and normalised stderr)
- `interpreter_cli_test`, `interpreter_concurrency_test`, `interpreter_std_modules_test` (Python oracles), `interpreter_rust_parity_test` (rustc oracle)
- `python3 scripts/interpreter_coverage.py <wj> out.json --corpus <dir> --rust-diff tests examples`
- `cargo test --test interpreter_features` every registry entry's probe program behaves as registered (replaces the old probes script);
  `Unsupported` ones are caught by the preflight at load, `Supported` ones are not
- `cargo test --release --test interpreter_projects` multi-file fixtures (`tests/interpreter_projects/`) and the seeded module
  fuzzer; `WJ_PROJECT_COMPILED=1` adds the compiled half (cargo)

# interpreter (Windjammerscript)

## Purpose
Run Windjammer source directly, with no cargo/rustc, for an instant dev loop
(`wj run` interprets by default, ADR-0022; `wj test --interpret`, `wj repl`). Rust remains the release path;
the interpreter must never print something the compiled program would not print. Status: `docs/INTERPRETER.md`.

## Inputs / outputs
- In: `.wj` source parsed by the compiler's own lexer/parser (`Program<'static>`); a file or a project dir.
- Out: program stdout/stderr text, an exit status (0 ok, 101 panic, N for `process::exit(N)`,
  2 unsupported-in-interpreter, 1 other), or `RuntimeError { kind, message, span, backtrace }`.

## Layout (one concern per file)
- `value.rs`, `int.rs`, `u128ops.rs`  value model; integer widths (`IntTy`) with Rust overflow rules; u128 bit patterns
- `engine.rs`            `Interpreter` state, public entry points (`run`, `run_checked`, `call_main`)
- `register.rs`, `loader.rs`, `project.rs`  definitions; program file selection; whole-program module tree bound by `crate::resolve` (E0603 privacy)
- `preflight.rs`, `deps.rs`, `host_bridge.rs`  `Unsupported`-feature detection (compiled-fallback trigger); dependencies as source (transitive, each crate once; `path = "../pkg/build"` is the package `../pkg`); `extern fn` bridge (ADR-0023)
- `eval_expr.rs`, `eval_stmt.rs`, `pattern.rs`, `place.rs`  evaluation, patterns, `&mut` places
- `generics.rs`, `fasthash.rs`  type-parameter bindings (`T::new()`); address hasher for call-site caches
- `iter.rs`, `builtins/{iter,shared}.rs`  lazy iterators; Rc/Weak/cells. `heap.rs` reachability: Rc counts, `impl Drop`; `moves.rs` ownership moves
- `call.rs`, `analysis.rs`  name resolution, calls, closures, inferred-`&mut` parameter analysis
- `from_impls.rs`, `type_keys.rs`, `c_library.rs`, `task_blocks.rs`  `From` impl choice by argument type; same-name struct/enum keys (`b::P`); scalar `extern fn` in the C library; `thread {}`/`async {}` tasks
- `ops.rs`, `coerce.rs`, `format.rs`, `defaults.rs`  operators, `as`/binding coercions, `format!`, `Default`
- `intrinsics.rs`        ADR-0014 intrinsics (print/panic/assert/debug/list) driven by `crate::intrinsics::Intrinsic`
- `builtins/`            native stdlib registry (`std_runtime_gaps.rs`: runtime functions called module-qualified) (`Registry`; `native(type, method)` for host objects): core types, `std_*` modules (`std_http{,_server}` port, `std_proc` subprocess/cli, `std_config`, `std_db`, `std_ordered`), `test_assert`, `thread`/`channel`/`sync`/`time`, `arena` (`std::arena`: Arena/Handle/Scratch, debug-build handle checks)
- `resolved/`            default engine of `wj run` (ADR-0020): closures over `rstack` slots; `typed.rs` unboxed ints, `island.rs` walker islands, `method.rs`/`place.rs`/`literal.rs` via the walker's own routines, `forloop.rs`; `osr.rs` statement registry for machine-code loops; `jit/` (default feature `jit`; `--no-jit`): `mod.rs` int functions, `osr.rs`+`lower*.rs`+`rt.rs` hot `while`/range-`for` loops (exits resume the closures)
- `sched.rs`, `locks.rs`, `native.rs`, `slicing.rs`  task scheduler (virtual clock), guard tokens + poisoning, host objects, panic texts `arena` (`std::arena`: Arena/Handle/Scratch with debug-build handle checks).
- `runner.rs`, `repl.rs` CLI port: run/test/REPL, exit codes, big-stack thread; only place with process I/O policy
- `error.rs`, `features.rs`  `RuntimeError`, `Ctl` (break/continue/return/error), spans, backtraces; `features.rs` = executable feature registry that generates the INTERPRETER.md lists; `caps.rs` = capability gate (ADR-0021): every sink builtin is checked against the run's grant (`ir::effect_facts`) before it runs, denial = exit 126, uncatchable; `durable_rt.rs` = `durable fn` step journal (ADR-0021 E4); `limits.rs` = `--limits` (fuel, wall, output, calls, mem; exit 124); `mcp_host.rs` = MCP servers behind `mcp::call` (`--bind`); `effect_port/` = effect port (ADR-0021 E3): `--record`/`--replay` logs (`log.rs`, `codec.rs`), `sim.rs` seeded world; one choke point per effect; `HANDLE_SINKS` refused; a test audits every host read

## Invariants
- Values are immutable trees behind `Rc` with copy-on-write; `&mut` is a `Place` (slot + projections).
- Integers carry their width; unsuffixed literals (`IntTy::Lit`) adopt the other operand/declared type.
  Overflow panics (debug-build semantics); `wrapping_*`/`checked_*`/`as` follow Rust.
- A parameter/receiver the body mutates is passed as `Value::Ref(place)` (mirrors inferred `&mut`).
- Never wrong-but-quiet: unknown name/method => `Semantic` error; unimplemented feature => `Unsupported`
  error naming it. No silent `Nil` fallbacks.
- No `cargo`/`rustc` spawn in this module (`tests/interpreter_cli_test.rs`); `cli::host_build` builds the bridge library, the interpreter loads it.
- Names are bound by `crate::resolve`: a `use` means the same thing interpreted and compiled. Every `Unsupported` registry feature has a preflight detector. Function-body `use`: an overlay module per statement (snapshot of the enclosing module, statics via `fallback`). `use std::m::*` fills `Module::std_globs` (bare names resolve to `m::name`, shadowing intrinsics). `Interpreter::dep_crates` are named from any module; `crate::` inside one is its root. A module re-exports its children's public items (`resolve::lookup_child_reexports`).

## Owns / never decides
Owns evaluation semantics, value model, native stdlib registry, CLI port. Never decides compiled-path behaviour (mirrors Rust).

## Allowed imports
`crate::parser`, `crate::lexer`, `crate::source_map::Location`, `crate::resolve` + `crate::module_system`, `crate::package`,
`crate::type_display`, `crate::intrinsics` (names only; ratcheted), std, libloading. Nothing from `codegen`, `analyzer`, `cli`, `optimizer`. CLI crates call only `runner::*`, `repl::*`, `Interpreter`.

## Entry points
`runner::run_entry` (fallback-aware, used by `cli::run_dev`), `runner::{run_path, run_path_with, run_specs}` (test discovery is `test_runner::interpreted`), `Interpreter::load_program_at`, `repl::run_repl_stdio`, `Interpreter::{new, new_capturing, run, run_checked}`.

## Intrinsics
Names come only from `crate::intrinsics::Intrinsic`; do not register them in `builtins/`. Panic text mirrors rustc; unmodelled
shapes return `unsupported` (exit 2). A user item or local of the same name shadows the intrinsic. New one: registry, then an arm in `eval_intrinsic`.

## Extending the stdlib: `reg.pure/mutating/place(Kind::X, "name", f)` or `reg.func("path::fn", f)` in `builtins/*.rs`.
## Test (`tests/interpreter_engine_parity_test.rs`: both engines on every corpus; `scripts/interp_bench.py`: speed ratchet)
- `interpreter_semantics_test` hand-derived Rust semantics; `interpreter_differential_test` vs Python reference models and compiled Rust (+ harness self-tests; stdout, exit status and normalised stderr)
- `interpreter_cli_test`, `interpreter_concurrency_test`, `interpreter_std_modules_test` (Python oracles), `interpreter_rust_parity_test` (rustc oracle)
- `interpreter_features` every registry entry's probe behaves as registered; `interpreter_projects` multi-file fixtures + module fuzzer (`WJ_PROJECT_COMPILED=1` adds cargo)
- `interpreter_hash_order_test` (`--hash-seed` gate), `interpreter_http_server_test`, `interpreter_config_db_test`; `interpreter_jit_{differential,caps,loops}_test` (machine code vs the walker; `WJ_JIT_LOOP_HOT=0` enters every loop)

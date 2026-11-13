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
- `preflight.rs`, `deps.rs`, `host_bridge.rs`  `Unsupported`-feature detection (compiled-fallback trigger); dependencies as source (transitive, each crate once; `path = "../pkg/build"` is the package `../pkg`); `extern fn` bridge (ADR-0023); `crate_host/` Rust crate calls (ADR-0031): `sig.rs` surface fn -> wire types, `wire.rs` value encoding, `gen.rs` host binary source, `process.rs` the spawned host (pipes on fds 3/4), `stub.rs` `--simulate` stubs, `mod.rs` gate -> effect port -> call -> `&mut` copy-out
- `eval_expr.rs`, `eval_stmt.rs`, `pattern.rs`, `place.rs`  evaluation, patterns, `&mut` places
- `compiled_facts.rs`    the compiler's decisions the interpreter adopts: emitted `&mut` parameters, solver literal widths (lazy, per program; kept by content in the store the driver installs, `install_fact_store`: `cli::check` keys it with `check::run_cache`)
- `generics.rs`, `fasthash.rs`  type-parameter bindings (`T::new()`); address hasher for call-site caches
- `iter.rs`, `builtins/{iter,shared}.rs`  lazy iterators; Rc/Weak/cells. `heap.rs` reachability: Rc counts, `impl Drop`; `moves.rs` ownership moves
- `call.rs`, `analysis.rs`  name resolution, calls, closures, inferred-`&mut` parameter analysis
- `from_impls.rs`, `type_keys.rs`, `c_library.rs`  `From` impl choice by argument type; same-name struct/enum keys (`b::P`); scalar `extern fn` in the C library
- `ops.rs`, `coerce.rs`, `format.rs`, `defaults.rs`  operators, `as`/binding coercions, `format!`, `Default`
- `intrinsics.rs`        ADR-0014 intrinsics (print/panic/assert/debug/list) driven by `crate::intrinsics::Intrinsic`
- `builtins/`            native stdlib registry (`std_runtime_gaps.rs`: runtime functions called module-qualified) (`Registry`; `native(type, method)` for host objects): core types, `std_*` modules (`std_http{,_server}` port, `std_net` raw sockets (ADR-0032: handle ids through `net_core`, the effect port and the virtual network of `effect_port/sim_net.rs`), `std_proc` subprocess/cli, `std_config`, `std_db`, `std_ordered`), `test_assert`, `thread`/`channel`/`sync`/`time`, `arena` (`std::arena`: Arena/Handle/Scratch, debug-build handle checks); `json_codec` (typed JSON, ADR-0027: `serde::Serialize`/`DeserializeSeed` over declared types, so both engines share serde_json's bytes and error texts)
- `resolved/`            default engine of `wj run` (ADR-0020): closures over `rstack` slots; `typed.rs` unboxed ints, `island.rs` walker islands, `method.rs`/`place.rs`/`literal.rs` via the walker's own routines, `forloop.rs`, `mapfast.rs`/`strfast.rs` map get/insert, `push_str` in place; `osr.rs` statement registry for machine-code loops; `jit/` (default feature `jit`; `--no-jit`): `mod.rs` int functions, `osr.rs`+`lower*.rs`+`rt.rs` hot `while`/range-`for` loops (exits resume the closures; `lower_struct.rs` struct locals, inlined methods; `rows.rs`+`lower_rows.rs` struct-vector fields in place)
- `sched.rs`, `locks.rs`, `native.rs`, `slicing.rs`  task scheduler (virtual clock; each task has its own stack of open `scope { }` blocks, inherited by the tasks it starts: `builtins/thread.rs` `run_scope`, ADR-0004), guard tokens + poisoning, host objects, panic texts, `virtual_cwd.rs` the program's own working directory (`fs::set_current_dir` never moves the host process; relative paths and spawned processes follow it) `arena` (`std::arena`: Arena/Handle/Scratch with debug-build handle checks).
- `runner.rs`, `repl.rs` CLI port: run/test/REPL, exit codes, big-stack thread; only place with process I/O policy; `source_scan.rs` = the one lexical scan of REPL and `wj eval` input (strings, `${..}`, char literals, comments): statement splitting and "is the input still open"; `debugger/` time-travel debugger of `wj replay` (ADR-0035): `script.rs` commands, `mod.rs` the stop hook in `exec_stmt` (walker only; one `Option` test when off) and the stop decision (breakpoints and conditions, watches, `next`, `finish`, the `--at-effect` probe; stops before a failure unwinds), `commands.rs` what each command does at a stop, `readonly.rs` the whitelist guard and evaluator behind `print EXPR` and conditions, `session.rs` re-executes a replayed or simulated run from its start for `back` (and applies the grants in the log header), `render.rs` text events, `lines.rs` the line a statement starts on
- `error.rs`, `failure_locals.rs`, `error_report.rs`, `features.rs`  `RuntimeError`; the failing frame's bindings (bounded, redacted, ADR-0036); the `--error-format json` report, `Ctl` (break/continue/return/error), spans, backtraces; `features.rs` = executable feature registry that generates the INTERPRETER.md lists; `caps.rs` = capability gate (ADR-0021): every sink builtin is checked against the run's grant (`ir::effect_facts`) before it runs, denial = exit 126, uncatchable; `durable_rt.rs` = `durable fn` step journal (ADR-0021 E4); `limits.rs` = `--limits` (fuel, wall, output, calls, mem; exit 124); `mcp_host.rs` = MCP servers behind `mcp::call` (`--bind`); `effect_port/` = effect port (ADR-0021 E3): `--record`/`--replay` logs (`log.rs`, `codec.rs`), `sim.rs` seeded world, `sim_net.rs` its virtual loopback network (ADR-0032); one choke point per effect; `HANDLE_SINKS` refused, `PORTED_HANDLE_SINKS` (sockets) log handle ids; a test audits every host read

## Invariants
- Values are immutable trees behind `Rc` with copy-on-write; `&mut` is a `Place` (slot + projections).
- Integers carry their width; unsuffixed literals (`IntTy::Lit`) adopt the other operand/declared type.
  Overflow panics (debug-build semantics); `wrapping_*`/`checked_*`/`as` follow Rust.
- A parameter/receiver the body mutates is passed as `Value::Ref(place)` (mirrors inferred `&mut`).
- Never wrong-but-quiet: unknown name/method => `Semantic` error; unimplemented feature => `Unsupported`
  error naming it. No silent `Nil` fallbacks.
- No `cargo`/`rustc` spawn in this module (`tests/interpreter_cli_test.rs`); `cli::host_build` builds the bridge library, the interpreter loads it; `cli::crate_host_build` builds the crate host, the interpreter spawns that binary (never cargo).
- Names are bound by `crate::resolve`: a `use` means the same thing interpreted and compiled. Every `Unsupported` registry feature has a preflight detector. Function-body `use`: an overlay module per statement (snapshot of the enclosing module, statics via `fallback`). `use std::m::*` fills `Module::std_globs` (bare names resolve to `m::name`, shadowing intrinsics). `Interpreter::dep_crates` are named from any module; `crate::` inside one is its root. A module re-exports its children's public items (`resolve::lookup_child_reexports`).

## Owns / never decides
Owns evaluation semantics, value model, native stdlib registry, CLI port. Browser host: `crates/windjammer-playground` (ADR-0034) builds this module for wasm32-unknown-unknown: host-only crates (SQLite, ring, libloading) are `cfg(not(target_arch = "wasm32"))`, the `mem` limit reads linear memory there, and `set_capture_stderr`/`guard_stack_bytes` serve hosts without a process stderr or big-stack thread (`guard_stack_bytes` lives in `host_api.rs`, the entry points for embedding hosts). Under `--simulate`/`--replay` the gate decides fd-verified file calls by name (nothing is opened). Never decides compiled-path behaviour (mirrors Rust).

## Allowed imports
`crate::parser`, `crate::lexer`, `crate::source_map::Location`, `crate::resolve` + `crate::module_system`, `crate::package`,
`crate::type_display`, `crate::intrinsics` (names only; ratcheted), std, libloading. `compiled_facts.rs` alone reads the compiler's own
decisions where the interpreter would otherwise guess differently (ADR-0038 C6b): `crate::codegen::generate_rust_source` (emitted `&mut`
parameters), `crate::analyzer` + `crate::ir` + `crate::type_inference` (the numeric solver's `literal_widths`: an unsuffixed literal gets the
width the compiled program gives it, so a `usize` countdown underflows where the compiled one panics; computed once per program, only for
programs whose AST can give a literal another width than `int`). Nothing else from `codegen`, `analyzer`, `cli`, `optimizer`. CLI crates call only `runner::*`, `repl::*`, `Interpreter`.

## Entry points
`runner::run_entry` (fallback-aware, used by `cli::run_dev`), `runner::{run_path, run_path_with, run_specs}` (test discovery is `test_runner::interpreted`), `Interpreter::load_program_at`, `Interpreter::{new, new_capturing, run, run_checked}`.

## Intrinsics and stdlib
Intrinsic names come only from `crate::intrinsics::Intrinsic` (never register them in `builtins/`); panic text mirrors rustc; unmodelled shapes return `unsupported` (exit 2); a user item or local of the same name shadows one. New one: registry, then an arm in `eval_intrinsic`. Stdlib: `reg.pure/mutating/place(Kind::X, "name", f)` or `reg.func("path::fn", f)` in `builtins/*.rs`.
## Test (`tests/interpreter_engine_parity_test.rs`: both engines on every corpus; `scripts/wj/interp_bench.wj`: speed ratchet; `scripts/wj/interp_instr.wj`: instruction-count ratchet; `tests/interpreter_fallbacks_test.rs`: the resolved engine's slower paths (`Fallbacks`: islands, walker calls, deopts, refused/abandoned/exited machine-code loops) held to `bench/interp/fallbacks.tsv`; `tests/interpreter_compiled_agreement_test.rs`: reduced repros the compiled build rejects or fails must not pass interpreted)
- Oracles: `interpreter_semantics_test` (hand-derived Rust), `interpreter_differential_test` (Python models and compiled Rust; stdout, exit status, normalised stderr), `interpreter_rust_parity_test` (rustc), `interpreter_std_modules_test` (Python)
- Surfaces: `interpreter_cli_test`, `interpreter_concurrency_test`, `interpreter_features` (every registry probe), `interpreter_projects` (multi-file fixtures + module fuzzer; `WJ_PROJECT_COMPILED=1` adds cargo), `interpreter_hash_order_test`, `interpreter_http_server_test`, `interpreter_config_db_test`, `interpreter_jit_{differential,caps,loops}_test` (`WJ_JIT_LOOP_HOT=0` enters every loop)

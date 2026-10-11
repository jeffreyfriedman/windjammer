# interpreter (Windjammerscript)

Purpose: run Windjammer source directly, with no cargo/rustc, for an instant dev loop
(`wj run --interpret`, `wj test --interpret`, `wj repl`). Rust remains the release path;
the interpreter must never print something the compiled program would not print.
Status and measurements: `docs/INTERPRETER.md`.

## Inputs / outputs
- In: `.wj` source parsed by the compiler's own lexer/parser (`Program<'static>`); a file or a project dir.
- Out: program stdout/stderr text, an exit status (0 ok, 101 panic, N for `process::exit(N)`,
  2 unsupported-in-interpreter, 1 other), or `RuntimeError { kind, message, span, backtrace }`.

## Layout (one concern per file)
- `value.rs`, `int.rs`   value model; integer widths (`IntTy`) with Rust overflow rules
- `engine.rs`            `Interpreter` state, public entry points (`run`, `run_checked`, `call_main`)
- `register.rs`, `loader.rs`  definitions, `use`/`mod` resolution, multi-file loading
- `eval_expr.rs`, `eval_stmt.rs`, `pattern.rs`, `place.rs`  evaluation, patterns, `&mut` places
- `call.rs`, `analysis.rs`  name resolution, calls, closures, inferred-`&mut` parameter analysis
- `ops.rs`, `coerce.rs`, `format.rs`, `defaults.rs`  operators, `as`/binding coercions, `format!`, `Default`
- `builtins/`            native stdlib registry (`Registry`): `num`, `text`, `seq`, `map`, `option`, `std_fns`
- `runner.rs`, `repl.rs` CLI port: run/test/REPL, exit codes, big-stack thread. Only place with process I/O policy.
- `error.rs`             `RuntimeError`, `Ctl` (break/continue/return/error), spans, backtraces

## Invariants
- Values are immutable trees behind `Rc` with copy-on-write; `&mut` is a `Place` (slot + projections).
- Integers carry their width; unsuffixed literals (`IntTy::Lit`) adopt the other operand/declared type.
  Overflow panics (debug-build semantics); `wrapping_*`/`checked_*`/`as` follow Rust.
- A parameter/receiver the body mutates is passed as `Value::Ref(place)` (mirrors inferred `&mut`).
- Never wrong-but-quiet: unknown name/method => `Semantic` error; unimplemented feature => `Unsupported`
  error naming it. No silent `Nil` fallbacks.
- No `cargo`/`rustc` spawn anywhere in this module (asserted by `tests/interpreter_cli_test.rs`).

## Allowed imports
`crate::parser` (AST), `crate::lexer`, `crate::source_map::Location`, std. Nothing from `codegen`,
`analyzer`, `cli`, `optimizer`. CLI crates call only `runner::*`, `repl::*` and `Interpreter`.

## Entry points
`runner::run_path`, `runner::run_tests`, `repl::run_repl_stdio`, `Interpreter::{new, new_capturing, run, run_checked}`.

## Extending the stdlib
Add `reg.pure/mutating/place(Kind::X, "name", f)` or `reg.func("path::fn", f)` in the matching
`builtins/*.rs`; no evaluator change needed. Mutating methods receive `&mut Value` (taken out of its place).

## Testing
- `cargo test --test interpreter_semantics_test` hand-derived Rust semantics
- `cargo test --test interpreter_differential_test` vs Python reference models and compiled Rust (+ harness self-tests)
- `cargo test --test interpreter_cli_test` real `wj` binary: no-cargo, multi-file, `wj test`, REPL, exit codes
- `python3 scripts/interpreter_coverage.py <wj> out.json --corpus <dir> --rust-diff tests examples`
- `python3 scripts/interpreter_probes.py <wj>` quick feature matrix

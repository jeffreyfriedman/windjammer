---
status: proposed
last_verified: a9bcd09a3
---
# ADR-0036: Runtime errors that explain themselves

- **Status:** Proposed (implemented for the interpreter; the compiled runtime keeps Rust's panic message)
- **Date:** 2026-11-11
- **Related:** [ADR-0022](0022-interpret-by-default.md) (interpreted `wj run`), [ADR-0021](0021-effects-and-capabilities.md)
  (capabilities, `--sandbox`), [ADR-0033](0033-interpreted-first-cost.md) (interpreted-first, drift gate),
  `docs/INTERPRETER.md`, `docs/testing/INTERPRETER_CONTRACT.md`, `docs/DIAGNOSTICS.md`,
  `docs/diagnostics-schema.json`, `src/interpreter/error.rs`, `failure_locals.rs`, `error_report.rs`

## Context

A compile failure is data: `wj check --json` gives a code, a span and a fix. A runtime failure used to be prose:
`panicked at file:line:col` and a one-line chain of calls. To fix `index out of bounds` an agent had to guess which
value was out of range and add `println` calls, run again, and read the output. Every extra round trip costs a
build-and-run and the context of the previous attempt.

The interpreter already knows everything a debugger would show at the moment of the failure: the error span, the
frame stack with call sites, and the live bindings. The question is how much to print by default, in what shape, and
how to do it without touching the contract that interpreted and compiled runs agree on stdout and the exit status.

## Decision

### 1. Default (human) output

On a failed interpreted run, stderr keeps its Rust-shaped head and gains, after `stack backtrace:`, exactly this:

```
thread 'main' panicked at p.wj:3:5:
index out of bounds: the len is 3 but the index is 5
stack backtrace:
 --> p.wj:3:5
 3 |     scores[which]
   |     ^
  0: `pick` at p.wj:3:5
  1: `main` at p.wj:9:21
locals in `pick`:
  scores = [90, 85, 70]
  which = 5
  total = 3
```

- the primary span with the source line and a caret;
- the Windjammer frames as `index: `function` at file:line:col`, innermost first; frame 0 is at the error span, every
  other frame at the call it is executing;
- up to 8 bindings of the innermost frame (the newest eight; a shadowed name shows its newest value; `.. and N more`
  when cut), each value in Debug style, structurally rendered, at most 80 characters, 4 items per collection, 3 levels.
  Rendering never runs user code, so it cannot have effects or fail a second time.

Nothing changes before `stack backtrace:`. Drift tooling compares the head (the existing normaliser cuts at that
line), so the new detail is invisible to it.

### 2. `--error-format json`

`wj run --error-format json` prints one line on stderr, the last one, in the diagnostics schema family
(`docs/diagnostics-schema.json`, `schema_version` 2) with `command: "run"`:

```json
{"schema_version":2,"command":"run","success":false,"exit_code":101,"summary":{"errors":1,...},
 "diagnostics":[{"code":"WJ0170","severity":"error","category":"runtime","origin":"runtime",
   "message":"index out of bounds: ...","primary_span":{"file":"p.wj","line_start":3,"col_start":5,...},
   "stack":[{"function":"pick","span":{...}},{"function":"main","span":{...}}],
   "locals":[{"name":"which","value":"5","redacted":false}],"docs":{...}}]}
```

`stack` and `locals` are the new members of a diagnostic; `exit_code` the new member of a report. Codes:
`WJ0170` panic, `WJ0171` `main` returned `Err` (the value's Debug form is in the message), `WJ0172` any other runtime
failure (refusal, limit, capability). The program's own stdout is untouched, and a program's own stderr output comes
before the report line. Default stays `human`; there is no environment switch, so a run never changes shape
behind a script's back.

### 3. What the compiled runtime can and cannot match

Compiled programs print Rust's panic message and a Rust backtrace on `RUST_BACKTRACE`; the exit status is 101 (panic)
and 1 (`main` returned `Err`), identical to the interpreter. We do not give the compiled runtime locals: a release
binary has no names for its registers, and a debug-info based dump would be a debugger, not a language feature.
What a compiled failure can get cheaply is the span: `wj run --verify` replays a failing compiled run in the
interpreter (ADR-0033), and an agent that has a compiled failure runs `wj run --interpret` to get the detail.

The rule that keeps the two honest: the detail is interpreter-only and stderr-only. It must never change stdout or the
exit status. Gates: `tests/runtime_error_report_test.rs` runs every failure kind with and without the flags and
compares stdout, exit status and the head of stderr; the drift gate (`scripts/wj/drift_gate.wj`) is unchanged and
stays green.

### 4. Secret safety

A value is redacted (`<redacted>`, `redacted: true`) when

1. it is a string equal to the value of a process environment variable of 8 or more characters (decided by source:
   the value came from, or equals, the environment), or
2. it is a struct or enum whose type name starts with `Secret` (decided by declaration; the standard library has no
   `Secret` type today, so this is the hook for one), or
3. its binding or field name contains `password`, `passwd`, `secret`, `token`, `api_key`, `apikey`, `credential`,
   `private_key` or `auth` (a conservative convention, applied to fields of structs too).

`--no-locals` drops the bindings entirely, for runs whose output is logged somewhere shared. Redaction applies to the
text form and to JSON. The name list is the only heuristic; it lives in the interpreter, outside the compiler's
heuristic-surface ratchet, and is documented in `docs/INTERPRETER.md`. The path to removing it is a `Secret<T>` type
in `std` with a redacting `Debug`; rule 2 then decides alone.

### 5. Cost

Capture runs only on the failure path: the `Err` arm of a frame's result (`if let Err(..)` on a value the function
already matches). There is no per-call bookkeeping: a closure-compiled function keeps its slot names in its compiled
form, built once. The benchmark guard measured before and after on one kernel (`docs/INTERPRETER.md`, "Runtime
errors"); no measurable change is allowed.

## Limits stated plainly

- A function run as machine code (`jit`) that panics is re-run by the closure engine, which produces the panic and
  the locals. The bindings are the walker's: the names are those of the function's own scope, and the slots of a
  block that ended are released when it ends, so a binding of an ended block is not reported
  (`tests/runtime_error_report_test.rs::locals_match_the_walkers_across_engines_and_the_jit`).
- Panics inside spawned tasks print the same detail for the task's innermost frame.
- `Err` returned from `main` carries the span of the `Err(..)`, `return` or `?` expression that produced it, and the
  bindings of `main`'s frame at that point, when `main` produced the value itself (the `WJ0171` report; the text
  form is unchanged). A value `main` returns after storing it elsewhere is reported at the last located expression
  that ran in `main`.

## Alternatives rejected

- **Print every frame's locals.** Output grows with depth; the innermost frame is where the fix is, and a caller's
  values are one `--no-locals`-free re-run away. Revisit if agents ask for it (a `--locals-depth N`).
- **A debugger protocol (DAP).** Right tool for humans in an editor, wrong tool for an agent that gets one command's
  output.
- **Compiled-runtime locals through `Debug` derives.** Would change generated code and cost on the happy path.

---
status: current
last_verified: 3406ef05bb35
---
# Pipe operator `|>`

Status: implemented in `src/parser/pipe_parser.rs`.

`x |> stage` feeds the value on the left into the call on the right. It is pure
syntax sugar: the parser rewrites every stage into an ordinary `Call` or
`MethodCall` node, so the analyzer, IR, and every backend (Rust, Go, JS, WASM,
interpreter) only ever see plain calls. There is exactly one desugaring site
(`pipe_parser.rs`); no backend has pipe-specific logic.

## Precedence and associativity

`|>` has the lowest precedence of all expression operators (below `||`) and is
left associative. Assignment and `let` are statements, there is no ternary, and
`..` only appears inside index/slice brackets (where its operands are parsed
without pipes), so nothing binds looser.

    a + 1 |> f          // f(a + 1)
    a || b |> f         // f(a || b)
    x |> f |> g         // g(f(x))

Pipes are also accepted as `match` arm values, call arguments, `let`
initializers, and `return`/tail expressions.

## Forms (right-hand side, a "stage")

| Stage                       | Desugars to                  |
|-----------------------------|------------------------------|
| `x \|> f`                   | `f(x)`                       |
| `x \|> f(a, b)`             | `f(x, a, b)`                 |
| `x \|> f::<T>(a)`           | `f::<T>(x, a)`               |
| `x \|> module.f(a)`         | `module.f(x, a)` (x first argument, not receiver) |
| `x \|> Type::assoc(a)`      | `Type::assoc(x, a)`          |
| `x \|> .method(a)`          | `x.method(a)` (method form: x is the receiver) |
| `x \|> .method::<T>()`      | `x.method::<T>()`            |
| `x \|> \|v\| v + 1`         | `(\|v\| v + 1)(x)`           |
| `x \|> f(a, _, b)`          | `f(a, x, b)` (placeholder)   |

Placeholder rules: `_` may appear as a *direct argument* of the stage call.
Exactly one is allowed (zero means "insert first"); two or more is an error
(bind the value with `let` first), and `_` nested inside another expression
(`f(g(_))`) is an error. `_` is not a valid expression anywhere else.

A closure stage extends as far right as a closure body normally does, so a
following stage must be parenthesised: `x |> (|v| v + 1) |> g`.

Anything else on the right (literal, arithmetic, `if`, ...) is an error:
"right-hand side of `|>` must be a function, call, method (`.name(..)`) or
closure".

## Multi-line chains

The lexer discards newlines, so a leading `|>` on a continuation line simply
continues the expression. Comments between stages are skipped by the lexer.

    let total = values
        |> clean()      // drop invalid entries
        // then sum
        |> sum()

## `?` and `.await`

Postfix `?` and `.await` written after a stage apply to *that stage's result*,
before the next stage:

    raw |> parse()? |> validate()      // validate(parse(raw)?)
    id  |> fetch().await |> render()  // render(fetch(id).await)

The `?` is not part of the pipe operator; the stage `parse()?` is `parse` called
with the piped value, then `?`. `|> async f()` / `spawn` stages are not
special: they are rejected as non-callable right-hand sides (write
`x |> f` and put `async`/`spawn` around the whole chain).

## `|>?` (short-circuiting pipe)

`x |>? stage` is `x? |> stage`: the *left-hand side* is unwrapped with `?` and
the stage is then applied to the `Ok`/`Some` payload. On `Err`/`None` the
enclosing function returns early, exactly like `?` (so the same rules apply:
the function must return a compatible `Result`/`Option`, errors convert via
`From`).

    raw |>? parse |>? validate(limit, _)
    // validate(limit, parse(raw?)?)  -- each stage result is unwrapped before the next

Decisions:

* It is a lexer token (`|>?`, no space allowed; `|> ?` stays an error). It
  desugars in `pipe_parser.rs` by wrapping the left-hand side in a `TryOp`
  before the ordinary stage rewrite, so every backend and the interpreter see
  plain `?` and calls. There is no `map`-style lifting: the stage receives the
  unwrapped value and its own result is *not* re-wrapped, so a fallible stage
  is chained with another `|>?`.
* Precedence/associativity are those of `|>`: `a + 1 |>? f` is `f((a + 1)?)`.
* `|>?` and a postfix `?` on a stage are independent: `x |>? f()?` is
  `f(x?)?`.
* All stage forms (`f`, `f(a)`, `f(_, a)`, `Type::assoc`, `.method()`,
  closure) and all diagnostics of `|>` apply; messages name the operator that
  was written (`|>?`).

## Evaluation order and ownership

* Stages are evaluated left to right, each exactly once; the piped value is
  placed into the argument list as an ordinary expression, never duplicated.
* With a placeholder in a non-first position, arguments evaluate in argument
  order (`f(a, _)` evaluates `a` before the piped value). Bind a `let` first if
  the order matters.
* `x |> f` is *identical* to `f(x)` after parsing, so callee ownership
  inference applies unchanged: if `f` borrows its parameter, `x` is borrowed
  (`&x`); if it consumes it, `x` is moved; Copy values are copied. `&self`
  methods piped with `.method()` behave exactly like `x.method()`.

## Diagnostics

All errors carry `file:line:col` (1-based, of the first character of the
offending token):

* `|>` / `|>?` with no left-hand value (statement starts with the operator): at the operator
* trailing `|>` with no stage: at the operator
* right-hand side is not callable (literal, `if`, ...): at the first token of the stage
* a stage followed by a binary operator on the same line (`x |> f + 1`): at the operator;
  parenthesise (`(x |> f) + 1`)
* more than one `_` in a stage: at the second `_`
* `_` nested inside another expression of the stage (`f(g(_))`): at that `_`
* `_` outside a pipe stage: at the `_`

## Other tools

* Formatter (`src/formatter`): token-based; `|>` and `|>?` are binary operators
  and continuation starts (`layout.rs`). A line over the width limit holding two
  or more top-level pipes is broken before every `|>` (`wrap.rs`), stages are
  indented one level, and comments between stages align with the stages. Pipes
  are never desugared by `wj fmt`. Fixture: `tests/fixtures/formatter/pipe_chain.*`.
* Backends: all verified on the cross-backend corpus
  (`tests/conformance_programs/150_pipe_forms_mixed.wj`, `151_pipe_try.wj`; Rust, Go, JS; the
  interpreter and the WASM target in `tests/pipe_backends_test.rs`). The Rust (and WASM)
  backend parenthesises a closure in callee position (`(|v| v + 1)(x)`); the portable
  Go/JS lowering types the closure parameters from the argument (`lower/call.rs`),
  otherwise `5 |> |v| v + 1` picked a different integer width than its argument.
* Interpreter: consumes the parser's AST; closure-literal calls are supported.

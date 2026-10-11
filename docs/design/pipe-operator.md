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

Future work (not implemented): a short-circuiting stage such as `|>?` that maps
over `Result`/`Option` automatically. The explicit `parse()?` form is already
short and unambiguous, so it was not justified for now.

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

All errors carry `file:line:col` of the offending token:

* `|>` with no left-hand value (statement starts with `|>`)
* trailing `|>` with no stage
* right-hand side is not callable
* placeholder misuse: more than one `_`, or `_` nested / outside a stage

## Other tools

* Formatter (`src/formatter`): token-based, already treats `|>` as a binary
  operator and a continuation start (`layout.rs`), so multi-line chains with a
  leading `|>` keep their layout. Because it never builds an AST, pipes are not
  desugared by `wj fmt`. Pipe-specific fixtures were not added there.
* Backends: the Rust backend wraps a closure literal in callee position in
  parentheses (`(|v| v + 1)(x)`). Go/JS/interpreter backends need to do the
  same for `Call { function: Closure, .. }` if they do not already.
* Interpreter (`wt/interpreter`): needs nothing if it consumes the parser's AST,
  since pipes are already plain calls. Closure-literal calls `(|v| ..)(x)` must
  be supported by its call evaluator; `tests/pipe_operator_test.rs` covers the
  parser output the interpreter will see.

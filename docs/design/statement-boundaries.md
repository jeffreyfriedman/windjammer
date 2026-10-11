---
status: current
last_verified: 203e43d114c9
---
# Statement boundaries and newline sensitivity

Windjammer has optional semicolons. A newline ends a statement **unless the parser can
prove the next line continues the expression**. This file is the precise rule; the parser
implements exactly this table and `tests/parser_gaps_test.rs` checks it (including
the property "splitting a program across lines at a boundary token never changes the AST").

## Tokens that begin a new line

| Next line begins with | Rule |
|---|---|
| `(` | Never a call continuation: starts a new statement (so `f(x)` newline `(a, b) = ...` is two statements). |
| `*` | Starts a new statement **iff** the rest of that line contains an assignment operator (`=`, `+=`, ...) at bracket depth 0, i.e. it is a deref-assignment `*p = v` / `*p += 1`. Otherwise it is multiplication continuing the previous expression (`let a = w` newline `* h`). A binary continuation can never contain a depth-0 assignment, so the two readings are never both valid. |
| `.` | Method/field continuation (`xs` newline `.map(f)`). |
| `+ - / % == != < <= > >= && \|\| \|> ^ <<  >>` | Binary or pipe continuation. |
| anything else | New statement. |

## `break` / `continue`

`break` takes an optional label and an optional value: `break`, `break 'outer`, `break v`,
`break 'outer v`. The value is parsed only when an expression starts on the **same line** as
`break` and the next token is not `;`, `,`, `}`, `)`, `]`, `=>`, `else`. `continue` takes an
optional label only. Labels are `'name:` before `for`/`while`/`loop`; `loop` is also an
expression whose value is its `break value`.

## Error recovery boundaries

After a syntax error the parser resynchronises at the next token that starts a new line with all
brackets opened since the failed statement closed, at a `;`, or at the `}` that closes the
enclosing block; at item level at the next `fn`/`struct`/`enum`/`impl`/`trait`/`use`/`mod`/`pub`/
`const`/`static`/`extern`/decorator that starts a line at brace depth 0. Every error carries
`line L, column C`; a file reports up to 25 errors.

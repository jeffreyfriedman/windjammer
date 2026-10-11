# Surface syntax guide (ADR-0014 Phase 1)

Windjammer has no macros. The print/assert/panic family are **built-in functions** (language
intrinsics) and the list literal is `[..]`. The Rust-style `name!(..)` spellings still compile
but are deprecated: `wj lint` and the compiler warn, and `wj fix` rewrites them. After one
release they become errors. Spec: [ADR-0014](../adr/0014-surface-syntax-rust-isms.md).

## The new forms

| Instead of (deprecated)                     | Write                                   |
|---------------------------------------------|-----------------------------------------|
| `println!("a {} b {:?}", x, y)`             | `println("a ${x} b ${y:?}")`            |
| `print!("{}", x)`                           | `print("${x}")`                         |
| `eprintln!("e {}", x)` / `eprint!`          | `stderr.println("e ${x}")` / `stderr.print(..)` |
| (stdout, explicit)                          | `stdout.println(..)` / `stdout.print(..)` (`println` = `stdout.println`) |
| `format!("{}-{}", a, b)`                    | `"${a}-${b}"`                           |
| `assert!(c, "msg {}", y)`                   | `assert(c, "msg ${y}")`                 |
| `assert_eq!(a, b)` / `assert_ne!(a, b)`     | `assert_eq(a, b)` / `assert_ne(a, b)` (optional message as 3rd arg) |
| `panic!("bad {}", n)`                       | `panic("bad ${n}")`                     |
| `todo!()` / `unreachable!()`                | `todo()` / `unreachable()` (optional message) |
| `dbg!(x)`                                   | `debug(x)`                              |
| `vec![1, 2, 3]`, `vec![0; n]`, `vec![]`     | `[1, 2, 3]`, `[0; n]`, `[]`             |

### Interpolation and format specifiers

`"${expr}"` interpolates any expression. A trailing `:spec` after the expression is a format
specifier, using the same spec language as Rust's `{:...}`:

```windjammer
let f = 3.14159
println("pi ~ ${f:.2}")        // pi ~ 3.14
println("name = ${name:?}")    // debug form: name = "bob"
println("[${n:>6}] [${n:<6}] [${n:04}]")
println("{literal braces} need no escaping; write $$ for a literal dollar")
```

Interpolated values are borrowed, never moved: `println("${name} ${name}")` leaves `name` usable.

### Print family

* `println` / `print` write to stdout, `stderr.println` / `stderr.print` to stderr.
* A string literal with braces and no interpolation prints verbatim (`println("{}")` prints `{}`).
* `assert(cond)`, `assert(cond, "message ${x}")`; `assert_eq(a, b)`, `assert_eq(a, b, "message")`.
* `panic("..")`, `todo()`, `unreachable()` diverge; their message is optional.
* `debug(x)` prints `[file:line:col] x = value` to stderr and **returns** `x`; the user's own value
  is never moved (non-`Copy` places are cloned for the return value).

### List literal

`[a, b, c]` is the growable list literal; `[]` infers its element type from context and `[x; n]`
repeats a value. `let x = [1, 2, 3]` and `Struct { items: [1, 2] }` (where the field is a list)
produce a growable `Vec`; in other positions the existing contextual rules still apply (a list
passed where a fixed-size array parameter is expected stays an array). Annotating
`let a: [int; 3] = [1, 2, 3]` keeps a fixed-size array.

## One representation, many backends

Both spellings are lowered by the parser to the same AST node (`Call` of an intrinsic registered
in `src/intrinsics/mod.rs`), so every later phase sees one thing and backends lower it:

| Intrinsic            | Rust                          | Go                              | JavaScript                |
|----------------------|-------------------------------|---------------------------------|---------------------------|
| `println`/`print`    | `println!` / `print!`         | `fmt.Println` / `fmt.Print`     | `console.log` / `process.stdout.write` |
| `stderr.println`     | `eprintln!`                   | builtin `println` (stderr)      | `console.error`           |
| `panic`/`todo`/`unreachable` | `panic!`/`todo!`/`unreachable!` | `panic(..)`             | `throw new Error(..)`     |
| `assert*`            | `assert!`/`assert_eq!`/`assert_ne!` | inline `if .. { panic }`  | inline `throw`            |
| `debug(x)`           | `dbg!`-style (stderr + value) | `wjDebug(x)` helper             | inline IIFE               |

The interpreter implements the same intrinsics natively. Name matching for intrinsics lives only
in `src/intrinsics/` (`Intrinsic::from_call_name`, `from_macro_name`); backends ask the registry.

## Migrating: `wj lint` and `wj fix`

```text
wj lint path/                 # W0007 (macro spelling / vec![]), W0008 ({}-style format string)
wj fix  path/                 # rewrite in place (idempotent)
wj fix  --check path/         # CI gate: exit 1 if anything would change
```

Each diagnostic carries a span and the exact replacement `wj fix` would make:

```text
warning: `println!` with a `{}` format string is deprecated: use `println` with `${..}` interpolation [W0008]
  --> main.wj:7:5
  = help: replace `println!("a {} b", x)` with `println("a ${x} b")` (or run `wj fix`)
```

`wj fix` handles nested, multi-line and escaped-brace (`{{ }}`) macros, positional (`{0}`) and
named (`{name}`, `name = expr`) arguments, and `{:?}` / `{:.2}` / `{:>8}` specs. It refuses (and
reports, never guesses) dynamic specs such as `{:width$}`, and it leaves `vec![..]` alone where the
list type depends on the surrounding context (a call argument, an assignment): migrate those by
hand or give the receiver a typed parameter. It only writes a file if the rewritten text still
parses.

### Migrating a whole repository

```bash
scripts/wj-fix-surface.sh path/to/repo          # rewrite every .wj under the path
scripts/wj-fix-surface.sh --check path/to/repo  # verify (exit 1 if anything is left)
```

The script uses `$WJ`, else `wj` from `PATH`, else builds this checkout. Run your tests before and
after; the rewrite is behaviour preserving (differential tests in
`tests/surface_syntax_phase1_test.rs` compile and run programs before/after and compare output).
Markdown code fences tagged `windjammer`/`wj` are migrated too with `wj fix --docs <dir>`.

## Compatibility

Both spellings are accepted today; `!` forms are warnings, not errors. Compiled Rust for the two
spellings is byte-identical.

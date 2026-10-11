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

| Intrinsic            | Rust                          | Go / JavaScript / TypeScript (portable layer) |
|----------------------|-------------------------------|-----------------------------------------------|
| `println`/`print`    | `println!` / `print!`         | `Print` IR node: `wjPrint` / `$print` (buffered stdout) |
| `stderr.println`/`print` | `eprintln!` / `eprint!`   | `Print { to_stderr }`: `wjEprint` / `$eprint` (flushes stdout first) |
| `panic`/`todo`/`unreachable` | `panic!`/`todo!`/`unreachable!` | `Panic` node; messages `explicit panic`, `not yet implemented[: msg]`, `internal error: entered unreachable code[: msg]`; exit status 101 |
| `assert(c, msg)`     | `assert!`                     | `if !c { panic(msg or "assertion failed") }` (message evaluated only on failure) |
| `assert_eq`/`assert_ne` | `assert_eq!`/`assert_ne!`  | operands bound once; failure prints ``assertion `left == right` failed[: msg]`` plus `left:` / `right:` in `{:?}` form; exit 101 |
| `debug(x)`           | `dbg!`-style (stderr + value) | `let t = x; stderr.println("[file:line:col] <source of x> = ${t:?}"); t` |
| `[a, b]`, `[]`, `[x; n]` | `vec![..]`                | `VecLit` / `VecRepeat`; `let v = []` takes its element type from later uses (`v.push(1)`) |
| `${x:spec}`          | `{:spec}`                     | `FmtSpec`: fill/align/width/`0`/precision, `?`, `x X b o e` |

The interpreter implements the same intrinsics natively. Name matching for intrinsics lives only
in `src/intrinsics/` (`Intrinsic::from_call_name`, `from_macro_name`); the Rust backend, the
interpreter and `portable::lower::intrinsics` all ask the registry. A user function or local with
an intrinsic's name shadows it on every backend.

### Backend notes (Go / JavaScript / TypeScript)

* One lowering serves all three: `src/codegen/portable/lower/intrinsics.rs` desugars every
  intrinsic to existing IR (`Print`, `Format`, `Panic`, `If`, `Block`), so the emitters contain no
  per-intrinsic code and cannot disagree on streams, messages or evaluation order.
* A lone string literal prints verbatim (`println("{}")` prints `{}`); an interpolated string
  carries its specs; a literal followed by arguments is the deprecated `{}` template.
* `debug(x)` prints compact `{:?}` (Rust's `dbg!` uses pretty `{:#?}`), and the label is
  reconstructed from the AST (identifiers, fields, calls, binary operators; other shapes print
  `<expression>`). The column is the argument's column.
* **Explicit errors, never wrong output** (`wj build` fails with `unsupported on this backend`):
  sign `+` and alternate `#` flags (`${x:+}`, `${x:#x}`), `E`, `x?`, and pretty Debug `${v:#?}` of
  an aggregate (scalars are fine). Rust supports all of these; the gaps are recorded per backend in
  `tests/conformance_programs/SUPPORT.toml` (programs 137, 138).
* Conformance: programs `120`-`138` (`*_intr_*`) check stdout, the exact/partial stderr stream
  (`// stderr:`, `// stderr-has:`, `// stderr-empty` headers) and exit status on Rust, Go, JS and
  TypeScript.

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

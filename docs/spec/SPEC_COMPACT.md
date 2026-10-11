# Windjammer in one page (compact spec for LLM context)

Authoritative, checked against the implementation by `scripts/spec_check.sh`. IDs refer to docs/spec/. Marked **[gap]** = specified/accepted but not implemented today (see OPEN_QUESTIONS.md). Files are `.wj`, UTF-8. No macros, no semicolons required, no `null`, no exceptions.

## Program shape
```windjammer
use std::collections::HashMap
struct P { x: int, y: int }
impl P {
    fn new(x: int, y: int) -> P { P { x, y } }      // assoc fn: no self
    fn norm2(self) -> int { self.x * self.x + self.y * self.y }   // &self inferred
    fn shift(self, d: int) { self.x += d }          // &mut self inferred
}
fn main() {
    let mut p = P::new(3, 4)
    p.shift(1)
    println("p=${p.x},${p.y} n2=${p.norm2()}")
}
```
Entry `fn main()`. Items in any order. Newline ends a statement; `;` optional (GRAM-003). A line starting with `.` or a binary operator continues the previous expression; `(` starts a new statement; `[` should too, but today continues the previous line as an index (GRAM-004, Q-019). A bare `return`/`break` swallows the next line: write `return;` (Q-026). Trailing commas OK. Last expression of a block is its value / implicit return; avoid `return` at the end.

## Lexical
`//` and `///` comments only (**block comments [gap]**). Reserved: fn let mut const static struct enum trait impl match if else for in while loop return break continue use mod extern thread async await defer pub self unsafe as where type dyn true false int uint float bool string. Ints: `42 1_000 0xff 0b101 0o17 7u8`; floats `1.5 1e3`; chars `'a'`; strings `"..."` with escapes `\n \t \r \\ \" $$`; unknown `\q` is just `q`; raw `r#"..."#`; strings may span lines. No `~` operator.

## Types
`int` = i64 (checked), `float` = f64 (the Rust backend still defaults literals to i32/f32: Q-027/Q-028), `bool`, `string` (UTF-8; `len()` = bytes), `char`, `()`; explicit `i8..i64 u8..u64 usize f32`; `(A, B)`; `Vec<T>`/`[T]`; `[T; N]`; `Option<T>`; `Result<T, E>`; `HashMap<K,V>`; `HashSet<T>`; `Box<T>`; `fn(A) -> R`. Parameter and return types are mandatory; locals inferred. No implicit numeric conversion: use `x as T` (wraps on narrowing; float->int saturates, NaN->0). `int(x)`/`float(x)` **[gap]**. Generic aliases and struct field defaults **[gap]**.

## Numbers (NUM)
`+ - * unary-` overflow PANIC (exit 101), never wrap, for every width (NUM-002/003). `/` truncates toward zero, `%` takes dividend's sign, `x / 0` panics (NUM-005/006). Floats: IEEE; `1.0/0.0` = `inf`, `0.0/0.0` = `NaN`, no panic (NUM-008). Shifts: `>>` arithmetic. Compound `+= -= *= /= %= &= |= ^= <<= >>=`. Float text: shortest round-trip, `2.0` prints `2`, `${x:.2}` rounds half-even (NUM-013).

## Operators (tight to loose; all left-assoc) (EXPR-001..003)
postfix `. () [] ? as` > prefix `- ! & *` > `* / %` > `<< >>` > `+ -` > `< <= > >=` > `== !=` > `&` > `^` > `|` > `&&` > `||` > `|>`.
Unlike Rust: shifts bind tighter than `+` (`1 << 2 + 1` = 5); `&|^` are looser than `==` (write `(a & m) == 0`); `as` is postfix so `-1 as u8` is `-(1 as u8)` (write `(-1) as u8`). `&&`/`||` short-circuit. Assignment is a statement (`a = b = c` is an error). Evaluation is left to right.

## Strings (STR)
Interpolation `"x=${x} sum=${a + b}"` evaluates any expression once, left to right. Specs: `${x:.2}` `${n:>6}` `${n:<6}` `${n:^6}` `${n:04}` `${n:x}` `${v:?}`. No `{}` templates: `"{}"` prints `{}` literally. `"$$"` is `$`. `${"` starts no interpolation (bind to a variable first). `+` concatenates. Slices `s[a..b]` are byte offsets (panic off char boundary). Methods: len is_empty contains starts_with ends_with trim to_uppercase to_lowercase replace split chars. A type with `to_string(self)` printable **[gap]**; else `@derive(Debug)` and `${x:?}`.

## Statements (STMT)
`let x = e`, `let mut x = e` (assignment needs `mut`; compiler currently still auto-infers, **[gap]** Q-007), shadowing allowed, `let (a, b) = t`, `let P { x, y } = p`, `let Some(v) = o else { return }`. `if c {..} else if c {..} else {..}` (also an expression), `if let P = e`, `while c`, `while let P = e`, `loop` (also an expression: `let r = loop { break v }`), `for p in iterable`, labels `'outer: for ..` with `break 'outer` / `continue 'outer`, `return e`, `defer stmt` (runs at function exit, reverse order; **[gap]** on the Rust backend, Q-029).

## Patterns (PAT)
`_`, name, literal (int, string, bool), `(a, b)`, `E::V`, `E::V(p, q)`, `E::V { f, g: p }`, `S { f, .. }`, `Some(p)`, `None`, `Ok(p)`, `Err(p)`, `p | q`, guards `p if c =>`. Ranges `0..=9`, `'a'..='z'`, `10..100`. Nested freely. Match with no arm panics; enum matches must be exhaustive.

## Items (ITEM, MOD, TRT)
`fn`, `struct S { pub f: T }`, `enum E { A, B(T), C { f: T } }`, `impl S { .. }`, `impl Trait for S { .. }`, `trait T { fn m(self) -> R  fn d(self) { default } }`, `const N: int = 1`, `static`, `type A = T`, `mod m { pub fn f() {} }`, `use a::b`, `use a::{b, c}`, `use a::b as c`. Paths use `::`. `pub` exports; privacy enforced by rustc only. Generics `fn f<T: A + B>(x: T)`, `where T: A`, `impl<T> S<T>`; `T::assoc()`; `Box<dyn Trait>`. Operator overloading by method name (`add sub mul div neg eq partial_cmp`). `@derive(Debug, Clone, PartialEq, Eq, Hash, Default, PartialOrd, Ord, Copy, Serialize, Deserialize)`; `@test` fns run under `wj test`.

## Ownership surface (OWN)
Value semantics: copies are never observable aliases (OWN-001/002). Never write `&`, `&mut` or lifetimes in `.wj`. Functions that mutate a parameter mutate the caller's value; methods that mutate `self` fields need `let mut` receiver. `.clone()` is always allowed. Builder methods may take and return `self`. Inference rules: docs/spec/inference.md (placeholder).

## Errors (ERR)
`Option`/`Result` values; `e?` propagates `Err`/`None` (with `From` conversion); `unwrap_or`, `map`, `and_then`, `ok_or`, `is_some`, `is_none`, `is_ok`, `is_err`, `unwrap`/`expect` (panic). `panic("msg ${x}")` exit 101. Panics are bugs; use `Result` for expected failure.

## Pipe (PIPE)
`x |> f` = `f(x)`; `x |> f(a)` = `f(x, a)`; `x |> f(a, _)` = `f(a, x)` (one `_`); `x |> .m(a)` = `x.m(a)`; `x |> |v| v + 1`; `x |> f?`; lowest precedence; leading `|>` continues a line. RHS must be a call/function/method/closure.

## Intrinsics (INTR)
Functions, not macros: `print println eprint eprintln panic assert(c, msg?) assert_eq(a, b, msg?) assert_ne todo unreachable debug(x)` (returns x, prints to stderr). `stderr.println(..)`, `stdout.println(..)`. List literal `[1, 2]`, `[]`, `[0; n]`. Deprecated and warned (W0007/W0008): `println!`, `vec![]`, `format!`, `{}` templates; `wj fix` rewrites.

## Collections (STD)
`Vec`: push pop len insert remove contains first last get reverse sort sort_by iter; `v[i]` (out of range panics). `HashMap`: insert get contains_key remove len keys values entry; **iteration order unspecified: sort before printing** (EXEC-005). Iterators: `iter map filter enumerate zip rev take skip chain fold sum count any all min max collect::<Vec<T>>()`. Closures `|x| x + 1`, capture by reference, may mutate captured `mut` locals. Turbofish `f::<T>()`, `.collect::<Vec<int>>()`.

## Concurrency (CONC): mostly **[gap]**
Decided (ADR-0004): `spawn f()`, `async f()`, blocks, `Task<T>.wait()`, `scope { }`, channel `send`/`recv`. Today only `async fn`/`.await`/`thread { }` parse (Rust backend); not in the interpreter. Do not use `go`, `chan`, `<-`.

## Execution (EXEC)
Observable = stdout bytes + exit status (0 normal, 101 panic, N via `std::process::exit(N)`). `int` semantics identical on all backends (JS uses BigInt/checked policy). Recursion >= 10,000 frames. Tiers (ADR-0007, proposed): Rust 1, WASM/JS/TS 2, Go/WGSL/interpreter 3. Single-threaded programs are deterministic.

## Idioms
- Return values from last expression; early exit with `return`.
- Model failure with `Result<T, string>` or an enum + `impl From`; use `?`.
- Prefer `match` on enums; keep guards simple.
- Prefer `for x in v` and iterator chains or pipes over index loops.
- Format output only with `${}`; never `{}`.
- Explicit widths only at FFI/binary boundaries; otherwise `int`/`float`.
- `let mut` only where mutated.

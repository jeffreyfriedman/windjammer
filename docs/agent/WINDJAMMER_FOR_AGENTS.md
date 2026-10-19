# Windjammer for agents

Windjammer (`.wj`) is a Rust-family language with inferred ownership: you write `fn f(x: Vec<int>)`, the compiler
decides borrow/move/clone. You have almost certainly seen no Windjammer in training. Do not guess from Rust or
Python: look things up (`wj docs <terms>`), and check every edit. Every code block below is executed by CI.

## The loop

<!-- ignore: command shapes with FILE/DIR placeholders; every subcommand and flag is checked against the CLI by the links check -->
```bash,ignore
wj check --json FILE_OR_DIR     # parse + lint + transpile, no cargo; JSON diagnostics with spans
wj run --interpret FILE         # instant run, no cargo/rustc; stdout is program output only
wj test --interpret DIR         # runs @test and test_* functions in *_test.wj
wj build FILE                   # production path (transpiles to Rust, then cargo)
wj docs hashmap entry           # offline search: stdlib, built-ins, idioms, 150+ verified examples
wj explain WJ0001               # error code explanation
wj fmt FILE                     # formatter; wj lint FILE checks Rust-isms; wj fix PATH migrates them
```

Exit status of a program: 0 ok, 101 panic, N for `process::exit(N)`, 2 means "unsupported in the interpreter".
`wj check` passing does not mean `wj run --interpret` passes, and the interpreter accepts more than `wj build`
(see "Compiled path gaps").

## Core idioms

Types: `int` (64-bit), `float` (f64), `bool`, `string`, `char`, `Vec<T>`, `Option<T>`, `Result<T, E>`,
`HashMap<K, V>`. Locals are immutable unless `let mut`. The last expression of a block is its value; do not write
`return` at the end of a function. Strings interpolate with `${expr}`, with format specs after a colon.

```windjammer
// expect: Ada is 36; pi=3.14; [   42]; [00042]; Some(3)
fn main() {
    let name = "Ada"
    let age = 36
    let mut parts: Vec<string> = []
    parts.push("${name} is ${age}")
    parts.push("pi=${3.14159:.2}")
    parts.push("[${42:>5}]")
    parts.push("[${42:05}]")
    parts.push("${Some(3):?}")
    println(parts.join("; "))
}
```

`println`, `print`, `panic`, `assert`, `assert_eq`, `assert_ne`, `todo`, `debug` are plain functions, not macros.
There are no semicolons. `if`, `match` and blocks are expressions.

```windjammer
// expect: small
// expect: 7
fn size(n: int) -> string {
    if n < 10 { "small" } else if n < 100 { "medium" } else { "large" }
}

fn main() {
    println(size(3))
    let best = { let a = 3
        let b = 7
        if a > b { a } else { b } }
    println("${best}")
}
```

## Structs, enums, traits

Methods take plain `self`; whether it is read-only, mutating or consuming is inferred from the body. Mutating a
receiver still requires the caller to hold it in a `let mut`. Derives (Debug, Clone, PartialEq, Copy) are inferred:
never write `#[derive(...)]`.

```windjammer
// expect: Rect(3x4) area=12
// expect: Rect(6x4) area=24
// expect: Circle r=2
struct Rect { w: int, h: int }

impl Rect {
    fn area(self) -> int { self.w * self.h }
    fn show(self) -> string { "Rect(${self.w}x${self.h}) area=${self.area()}" }
    fn widen(self, by: int) { self.w += by }
}

trait Named {
    fn label(self) -> string
}

struct Circle { r: int }

impl Named for Circle {
    fn label(self) -> string { "Circle r=${self.r}" }
}

fn main() {
    let mut r = Rect { w: 3, h: 4 }
    println(r.show())
    r.widen(3)
    println(r.show())
    println(Circle { r: 2 }.label())
}
```

Enums carry data and are consumed with exhaustive `match`; arms may have guards and or-patterns. Range patterns
(`1..=9`) are not supported yet: use a guard.

```windjammer
// expect: Circle 12
// expect: Rect 6
// expect: big
enum Shape {
    Circle(int),
    Rect(int, int),
}

fn describe(s: Shape) -> string {
    match s {
        Shape::Circle(r) => "Circle ${r * r * 3}",
        Shape::Rect(w, h) => "Rect ${w * h}",
    }
}

fn bucket(n: int) -> string {
    match n {
        0 | 1 => "tiny",
        x if x < 100 => "mid",
        _ => "big",
    }
}

fn main() {
    println(describe(Shape::Circle(2)))
    println(describe(Shape::Rect(2, 3)))
    println(bucket(1000))
}
```

## Error handling

Use `Option`/`Result`, `match`, `?`, `unwrap_or`, `map_err`, `ok_or`. `.unwrap()` and `.expect()` are lint errors
(W0002), so `wj build` rejects them. `?` propagates `Err`/`None` from the enclosing function.

```windjammer
// expect: Ok(10)
// expect: Err("bad: x")
// expect: 0
fn parse(s: string) -> Result<int, string> {
    s.parse::<int>().map_err(|_| "bad: ${s}")
}

fn double(s: string) -> Result<int, string> {
    let n = parse(s)?
    Ok(n * 2)
}

fn main() {
    println("${double("5"):?}")
    println("${double("x"):?}")
    let n: Option<int> = None
    println("${n.unwrap_or(0)}")
}
```

`Result` and `Option` do not implement Display: print with `${x:?}` or `match` first. Custom error types are plain
enums (see `wj docs error enum`).

## Collections and iteration

`let v = [1, 2, 3]` is a growable `Vec`. Iterate directly: `for x in v`, `for (i, x) in v.enumerate()`. Adapter
chains start from the collection itself: write `v.map(..)`, not `v.iter().map(..)` (`.iter()` is lint error W0003).
Maps need `use std::collections::HashMap`; iteration order of a `HashMap` is unspecified, so sort keys before printing.

```windjammer
// expect: [1, 4, 9]
// expect: 6
// expect: a=1 b=2
// expect: 0:x 1:y
use std::collections::HashMap

fn main() {
    let v = [1, 2, 3]
    let squares: Vec<int> = v.map(|x| x * x).collect()
    println("${squares:?}")
    println("${v.sum::<int>()}")
    let mut m: HashMap<string, int> = HashMap::new()
    m.insert("b", 2)
    m.insert("a", 1)
    let mut keys: Vec<string> = m.keys().collect()
    keys.sort()
    let mut out: Vec<string> = []
    for k in keys { out.push("${k}=${m[k]}") }
    println(out.join(" "))
    let mut line: Vec<string> = []
    for (i, s) in ["x", "y"].enumerate() { line.push("${i}:${s}") }
    println(line.join(" "))
}
```

The pipe operator feeds the left value into a call: `x |> f(a)` is `f(x, a)`; `x |> .method(a)` is `x.method(a)`.
It has the lowest precedence, so `a + 1 |> f` is `f(a + 1)`. A trailing `?` applies to that stage.

```windjammer
// expect: 16
// expect: [2, 4, 6]
fn double(x: int) -> int { x * 2 }
fn add(x: int, y: int) -> int { x + y }

fn main() {
    println("${3 |> add(5) |> double}")
    let d: Vec<int> = [1, 2, 3] |> .map(|x| x * 2) |> .collect()
    println("${d:?}")
}
```

## Modules and tests

Inline `mod name { pub fn .. }` and one-file-per-module (`a.wj` or `a/mod.wj`, found next to the importer or in the
project root/`src/`) are both resolved with `use a::f`; paths use `::`. Tests live in `*_test.wj`: `@test`, `test_*`
functions, or `@test_cases([(in, expected), ...])` with one parameter per tuple field.

```windjammer
// expect: 12
mod geo {
    pub fn area(w: int, h: int) -> int { w * h }
}

use geo::area

fn main() {
    println("${area(3, 4)}")
}
```

```windjammer,test
fn square(x: int) -> int { x * x }

@test
fn squares() {
    assert_eq(square(4), 16)
}

@test_cases([(2, 4), (3, 9)])
fn table(x: int, expected: int) {
    assert_eq(square(x), expected)
}
```

## Reading input and arguments

Programs that read standard input should use one spelling that behaves the same under `wj run --interpret` and `wj build`:

```windjammer,parse
use std::fs
use std::env

let text = fs::read_to_string("/dev/stdin").unwrap_or("")
for line in text.lines() {
    println(line)
}
for a in env::args() {
    println(a)
}
```

`io::stdin().read_line(..)` loops forever in compiled programs and `io::read_line()` is not implemented by the interpreter, so
prefer the `/dev/stdin` form. `env::args()` is an iterator: loop over it instead of slicing it.

## Permissions: exit 126 and the fix line

`wj run` is sandboxed. With no `[capabilities]` in `wj.toml` a program may use files inside its project directory (plus time, random,
console) and nothing else. `wj test` and the REPL also get a private scratch directory: write temporary files under `env::temp_dir()`. Anything more (network, env vars, programs, files elsewhere) stops the run with **exit status 126** and a
message that already contains the fix:

```text
error: capability denied: net.connect(api.example.com)
  = to allow just this once: run with `--allow net.connect=api.example.com`
  = to allow it for good, put this in wj.toml:
        [capabilities]
        fs.read = ["./**"]
        fs.write = ["./**"]
        net.connect = ["api.example.com"]
```

Copy that block into `wj.toml` (a `[capabilities]` table is the whole grant, so keep the two `fs` lines), or pass `--allow`. Do not catch it
or work around it; 126 is a policy fault, not an error value. `wj explain-effects FILE` lists what a program will need, `wj check`
reports it before running (WJ0040-WJ0044), `--sandbox` starts from nothing, `WJ_CAPS=warn|off` is the explicit opt-out. Hand untrusted
helpers a `cap::Net`/`cap::Fs` value (`cap::net().only(["host"])`) instead of a grant.

## The 25 things agents get wrong first

1. `println!("{}", x)`: macros do not exist. Write `println("${x}")` (W0007/W0008, ADR-0014).
2. `vec![1, 2]`: write `[1, 2]`; `vec![0; n]` is `[0; n]`.
3. `#[derive(Debug, Clone)]`: the lexer rejects `#`. Derives are inferred; no attribute needed.
4. `&self`, `&mut self`, `&x` arguments: write `self`; ownership and borrows are inferred (W0001, W0004).
5. `.iter()` / `.iter_mut()`: delete them (W0003, an error in `wj build`).
6. `.unwrap()` / `.expect("..")`: use `match`, `?`, `unwrap_or`, `unwrap_or_default` (W0002).
7. `"lit".to_string()`: a literal is already a `string` (W0006). `.clone()` is usually unnecessary (W0005).
8. `i32`/`u64`/`usize` everywhere: use `int` and `float`; widths only at FFI/binary boundaries (ADR-0015).
9. A trailing `return x`: the last expression is the value; `return` is for early exit.
10. Semicolons: not needed.
11. `x as usize` indexing: `v[i]` accepts `int`. `as` still exists for explicit numeric/char casts.
12. `let x = 5; x += 1`: needs `let mut x` (ADR-0002). Parameters and receivers need no `mut`.
13. `{}` placeholders: use `"${x}"`; for literal braces nothing is needed, for a literal dollar write `$$`.
14. A string literal at the start of an interpolation, `"${"abc".len()}"`, fails to parse (open finding): bind it first with `let s = "abc"`.
15. `3.0` prints as `3`, `0.1 + 0.2` as `0.30000000000000004` (Rust's Display): use `${x:.2}` for fixed output.
16. Printing a `Result`/`Option`/`Vec`/struct needs `:?`; there is no auto Display for user types: define `fn to_string(self) -> string` and call it.
17. Range patterns (`1..=9 =>`), labeled `break 'outer`, and `if x is Some(v)` are not parsed yet: use guards, helper functions with `return`, `if let`.
18. `*counter += 1` as the first statement after a line ending in `)` is parsed as multiplication (open finding): use `.entry(k).and_modify(|c| *c += 1).or_insert(1)`.
19. `char::to_uppercase()` returns an iterator: use `to_ascii_uppercase()` for a char.
20. `HashMap` iteration order is unspecified: sort the keys; use `BTreeMap` for ordered output.
21. `mem::swap(&mut a, &mut b)` fails the borrow lint: use `v.swap(i, j)` or a temp.
22. `string.len()` is bytes, `.chars().count()` is characters; slicing is by bytes: `s[1..4]` (no `&`).
23. `x / y` on `int` truncates toward zero and `%` keeps the sign of the dividend; overflow panics (exit 101).
24. No ternary, no `while let` on non-patterns, no `async`/threads/`Rc`/`RefCell`/`impl Drop` in the interpreter (exit 2): use `wj build` for those.
25. Declaring success from `wj check` alone: always `wj run --interpret` (or `wj test --interpret`) to see behaviour.

## What not to write

Each of these is flagged by `wj check` (the block must produce a diagnostic):

```windjammer,bad
fn main() {
    let v = vec![1, 2, 3]
    println!("sum = {}", v.iter().sum::<i32>())
}
```

```windjammer,bad
fn first(v: &Vec<int>) -> int {
    v.get(0).unwrap()
}

fn main() {
    println("${first(&[1])}")
}
```

<!-- should_fail: Rust attribute syntax `#[derive]` is rejected (use `@derive`) -->
```windjammer,should_fail,bad
#[derive(Debug, Clone)]
struct P { x: int }

fn main() {
    println("${P { x: 1 }:?}")
}
```

## Compiled path gaps (be honest with the user)

`wj run --interpret` is the reference for behaviour. `wj build` additionally runs rustc: today about half of
`examples/verified` compiles and matches (`// rust: ok`); the rest carry a recorded `// rust: gap` reason in the
header. Known classes: adapter chains on a collection without `.iter()` (emitted as `v.map(..)` on a `Vec`),
integer-width inference between `i32` and `i64` in mixed expressions, `HashMap` indexing with `&str` keys,
inline `mod` plus `use` duplicates. Annotate numeric accumulators that start as a bare literal and later mix with `int` values (`let mut v: int = 0`, `let mut n: int = 0`): an
unannotated `let mut v = 0` can be emitted as `i32` and fail `wj build` with `mismatched types ... expected i64, found i32` while the
interpreter runs it. If `wj build` fails on other code the interpreter runs, report it with the smallest repro; do not contort the source.

## Where to look next

`wj docs <terms>` searches this guide, the cheat sheets (`cheatsheet_from_rust.md`, `_python`, `_go`, `_typescript`),
built-in methods, stdlib signatures and the verified examples (`examples/verified/*.wj`, indexed by tag in
`manifest.json`). Design decisions: `docs/adr/0014`, `0015`, `docs/design/pipe-operator.md`, `docs/INTERPRETER.md`.

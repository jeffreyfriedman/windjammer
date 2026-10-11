# Errors, Option, Result and `?`

Windjammer has no exceptions and no null. Recoverable failure is a value (`Result`), absence is a value (`Option`), and bugs are panics.

Part of the [Windjammer language specification](README.md). Rule IDs are checked by `scripts/spec_check.sh`; each rule's executable example is the code block below it, stored in `tests/spec/errors/`.

## Option and Result

**SPEC-ERR-001** `Option<T>` is `Some(T)` or `None`; `Result<T, E>` is `Ok(T)` or `Err(E)`. Absence and failure are values, matched with `match`, `if let` or combinators; there are no exceptions and no null.

<!-- example: tests/spec/errors/SPEC-ERR-001.wj -->
```windjammer
fn find(v: Vec<int>, t: int) -> Option<int> {
    let mut i = 0
    while i < v.len() {
        if v[i] == t { return Some(i) }
        i += 1
    }
    None
}
fn main() {
    let v = [5, 6, 7]
    match find(v, 7) { Some(i) => println("at ${i}"), None => println("missing") }
    match find(v, 1) { Some(i) => println("at ${i}"), None => println("missing") }
}
```
Output:
```text
at 2
missing
```

## The ? operator

**SPEC-ERR-002** `e?` on a `Result` yields the `Ok` payload, or returns `Err(e)` from the enclosing function (converted with `From` if the error types differ). On an `Option` it yields the `Some` payload or returns `None`. The enclosing function must return a compatible `Result`/`Option`.

<!-- example: tests/spec/errors/SPEC-ERR-002.wj -->
```windjammer
fn half(n: int) -> Result<int, string> {
    if n % 2 != 0 { return Err("odd ${n}") }
    Ok(n / 2)
}
fn quarter(n: int) -> Result<int, string> {
    let h = half(n)?
    half(h)
}
fn head(v: Vec<int>) -> Option<int> {
    let f = v.first()?
    Some(*f + 1)
}
fn main() {
    println("${quarter(8).unwrap()}")
    match quarter(6) { Ok(v) => println("ok ${v}"), Err(e) => println("err ${e}") }
    match quarter(7) { Ok(v) => println("ok ${v}"), Err(e) => println("err ${e}") }
    println("${head([4]).unwrap()} ${head([]).is_none()}")
}
```
Output:
```text
2
err odd 3
err odd 7
5 true
```

## Combinators

**SPEC-ERR-003** `Option`/`Result` provide `unwrap_or`, `unwrap_or_else`, `map`, `and_then`, `ok_or`, `is_some`, `is_none`, `is_ok`, `is_err`.

<!-- example: tests/spec/errors/SPEC-ERR-003.wj -->
```windjammer
fn main() {
    let a: Option<int> = Some(3)
    let b: Option<int> = None
    println("${a.unwrap_or(0)} ${b.unwrap_or(7)} ${a.map(|x| x * 2).unwrap_or(0)}")
    println("${a.is_some()} ${b.is_none()} ${a.and_then(|x| if x > 5 { Some(x) } else { None }).is_none()}")
    let r: Result<int, string> = b.ok_or("none")
    println("${r.is_err()} ${r.unwrap_or(-1)}")
}
```
Output:
```text
3 7 6
true true true
true -1
```

## Panics

**SPEC-ERR-004** A panic aborts the program: it prints a message and location to standard error, runs no further user code except `defer`red statements of frames being unwound, and exits with status 101. Panics are for bugs (violated invariants), not recoverable errors.

<!-- example: tests/spec/errors/SPEC-ERR-004.wj -->
```windjammer
fn main() {
    println("before")
    panic("boom ${1 + 1}")
}
```
Output:
```text
before
```
Then panics (message contains `boom 2`), exit status 101.

## unwrap and expect

**SPEC-ERR-005** `unwrap()` on `None`/`Err` panics; `expect(msg)` panics with `msg` included in the message. Prefer `?`, `unwrap_or` or `match`.

<!-- example: tests/spec/errors/SPEC-ERR-005.wj -->
```windjammer
fn main() {
    let r: Result<int, string> = Err("bad")
    println("${r.unwrap_or(1)}")
    println("${r.expect("wanted ok")}")
}
```
Output:
```text
1
```
Then panics (message contains `wanted ok`), exit status 101.

## Custom error types and From

**SPEC-ERR-006** A function returning `Result<T, MyError>` may use `?` on a `Result<_, E2>` when `impl From<E2> for MyError` exists; the conversion is applied implicitly.

<!-- example: tests/spec/errors/SPEC-ERR-006.wj -->
```windjammer
enum AppErr { Parse(string), Neg }
impl From<string> for AppErr {
    fn from(s: string) -> AppErr { AppErr::Parse(s) }
}
fn parse(s: string) -> Result<int, string> {
    if s == "x" { Err("not a number") } else { Ok(1) }
}
fn run(s: string) -> Result<int, AppErr> {
    let n = parse(s)?
    if n < 0 { return Err(AppErr::Neg) }
    Ok(n)
}
fn main() {
    match run("x") {
        Ok(_) => println("ok"),
        Err(AppErr::Parse(m)) => println("parse: ${m}"),
        Err(AppErr::Neg) => println("neg"),
    }
}
```
Output:
```text
parse: not a number
```

## Diagnostics are reported in Windjammer terms

**SPEC-ERR-007** [OPEN Q-014] ADR-0005 (proposed): every compile-time error is reported against `.wj` source with a stable error code; the user never needs to read a rustc message. Today type and borrow errors are produced by translating rustc output, and `wj check` does not check `.wj` files without cargo. (untestable: the diagnostic contract (ADR-0005) is a property of the whole toolchain; see OPEN_QUESTIONS Q-014)


# Intrinsics

Built-in functions that are part of the language, not the library (ADR-0014 phase 1). They are registered in one place, `src/intrinsics/mod.rs`, and lowered per backend.

Part of the [Windjammer language specification](README.md). Rule IDs are checked by `scripts/spec_check.sh`; each rule's executable example is the code block below it, stored in `tests/spec/intrinsics/`.

**Registry (checked against `Intrinsic::ALL` by `tests/spec_conformance_test.rs`):** `print`, `println`, `eprint`, `eprintln`, `format`, `panic`, `assert`, `assert_eq`, `assert_ne`, `todo`, `unreachable`, `debug`, `vec`.

| Intrinsic | Stream | Notes |
|---|---|---|
| `print` / `println` | stdout | `println` appends `\n` |
| `eprint` / `eprintln` | stderr | spelled `stderr.print(..)` / `stderr.println(..)` canonically |
| `format` | none | the deprecated `format!`; use an interpolated string |
| `panic` `todo` `unreachable` | stderr | diverge, exit 101 |
| `assert` `assert_eq` `assert_ne` | stderr on failure | exit 101 |
| `debug` | stderr | returns its argument |
| `vec` | none | deprecated spelling of the list literal `[..]` |

## Print family

**SPEC-INTR-001** `print(s)` writes `s` to standard output; `println(s)` writes `s` and a newline; `stderr.print`/`stderr.println` (and `eprint`/`eprintln`) write to standard error, which is not part of conformance output. A string literal with braces and no `${` prints verbatim.

<!-- example: tests/spec/intrinsics/SPEC-INTR-001.wj -->
```windjammer
fn main() {
    print("a")
    print("b")
    println("")
    println("{}")
    eprintln("not on stdout")
    stderr.println("nor this")
    stdout.println("c")
}
```
Output:
```text
ab
{}
c
```

## assert

**SPEC-INTR-002** `assert(cond)` does nothing when `cond` is true and otherwise panics (status 101) with `assertion failed`; `assert(cond, msg)` panics with `msg` (the message expression is evaluated only on failure).

<!-- example: tests/spec/intrinsics/SPEC-INTR-002.wj -->
```windjammer
fn main() {
    assert(1 + 1 == 2)
    assert(true, "never shown")
    println("passed")
    assert(1 > 2, "math ${1 + 1}")
    println("not reached")
}
```
Output:
```text
passed
```
Then panics (message contains `math 2`), exit status 101.

## assert_eq and assert_ne

**SPEC-INTR-003** `assert_eq(a, b)` panics unless `a == b`, printing ``assertion `left == right` failed`` with the `left:` and `right:` debug forms; `assert_ne` is the negation. An optional third argument is an extra message. Operands are evaluated once.

<!-- example: tests/spec/intrinsics/SPEC-INTR-003.wj -->
```windjammer
fn main() {
    assert_eq(2, 1 + 1)
    assert_ne("a", "b")
    println("ok")
    assert_eq(1 + 1, 3)
}
```
Output:
```text
ok
```
Then panics (message contains `left == right`), exit status 101.

## panic

**SPEC-INTR-004** `panic(msg)` aborts with `msg` (an interpolated string) and exit status 101; `panic()` panics with `explicit panic`. It has type `!`-like: it may end any block regardless of the expected type.

<!-- example: tests/spec/intrinsics/SPEC-INTR-004.wj -->
```windjammer
fn pick(n: int) -> int {
    if n > 0 { return n }
    panic("non-positive ${n}")
}
fn main() {
    println("${pick(2)}")
    println("${pick(-1)}")
}
```
Output:
```text
2
```
Then panics (message contains `non-positive -1`), exit status 101.

## todo and unreachable

**SPEC-INTR-005** `todo()` panics with `not yet implemented`; `unreachable()` panics with `internal error: entered unreachable code`; both accept an optional message appended after `: `. Both diverge like `panic`.

<!-- example: tests/spec/intrinsics/SPEC-INTR-005.wj -->
```windjammer
fn f(n: int) -> int {
    match n {
        0 => 10,
        1 => unreachable("n was ${n}"),
        _ => todo(),
    }
}
fn main() {
    println("${f(0)}")
    println("${f(1)}")
}
```
Output:
```text
10
```
Then panics (message contains `entered unreachable code: n was 1`), exit status 101.

## debug

**SPEC-INTR-006** `debug(x)` writes `[file:line:col] x = value` to standard error and returns `x` unchanged; the caller's value is not moved.

<!-- example: tests/spec/intrinsics/SPEC-INTR-006.wj -->
```windjammer
fn main() {
    let a = debug(2 + 3)
    let s = "k"
    let t = debug(s)
    println("${a} ${s} ${t}")
}
```
Output:
```text
5 k k
```

## List literal

**SPEC-INTR-007** `[a, b, c]` is the list literal (growable `Vec` in `let` and field initialisers), `[]` is the empty list (element type from use) and `[x; n]` repeats `x` `n` times.

<!-- example: tests/spec/intrinsics/SPEC-INTR-007.wj -->
```windjammer
fn main() {
    let mut v = [1, 2, 3]
    v.push(4)
    let z = [0; 3]
    let mut e = []
    e.push("a")
    println("${v.len()} ${z.len()} ${z[2]} ${e.len()}")
}
```
Output:
```text
4 3 0 1
```

## Deprecated macro spellings

**SPEC-INTR-008** The Rust spellings `println!(..)`, `print!`, `eprintln!`, `assert!`, `assert_eq!`, `panic!`, `dbg!`, `format!`, `vec![..]` are accepted for now as deprecated aliases of the intrinsics (ADR-0014): the parser lowers them to the same node and emits warning W0007; a `{}` template emits W0008. `wj fix` rewrites them.

<!-- example: tests/spec/intrinsics/SPEC-INTR-008.wj -->
```windjammer
fn main() {
    println!("old {}", 1)
    let v = vec![1, 2]
    println("${v.len()}")
}
```
Output:
```text
old 1
2
```

## Registry

**SPEC-INTR-009** The set of intrinsics is exactly the registry `src/intrinsics` (`Intrinsic::ALL`): print, println, eprint, eprintln, format, panic, assert, assert_eq, assert_ne, todo, unreachable, debug, vec. A user function or local with an intrinsic's name shadows it.

<!-- example: tests/spec/intrinsics/SPEC-INTR-009.wj -->
```windjammer
fn println(s: string) {
    print("shadowed:")
    print(s)
    print("\n")
}
fn main() {
    println("x")
}
```
Output:
```text
shadowed:x
```


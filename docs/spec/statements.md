# Statements

Constructs that sequence effects inside blocks.

Part of the [Windjammer language specification](README.md). Rule IDs are checked by `scripts/spec_check.sh`; each rule's executable example is the code block below it, stored in `tests/spec/statements/`.

## let binding

**SPEC-STMT-001** `let name = e` binds `name` to the value of `e` for the rest of the enclosing block; the type is inferred or given as `let name: T = e`. A later `let` of the same name shadows the earlier one.

<!-- example: tests/spec/statements/SPEC-STMT-001.wj -->
```windjammer
fn main() {
    let x = 1
    let x = x + 1
    let y: float = 2.5
    {
        let x = 100
        println("${x}")
    }
    println("${x} ${y}")
}
```
Output:
```text
100
2 2.5
```

## Explicit mutability

**SPEC-STMT-002** A binding is immutable unless declared `let mut` (ADR-0002, accepted); assigning to an immutable binding is a compile-time error. [OPEN Q-007] The compiler currently infers `mut` silently and the interpreter does not check, so the program below is accepted.

<!-- example: tests/spec/statements/SPEC-STMT-002.wj -->
```windjammer
fn main() {
    let x = 1
    x = 2
    println("${x}")
}
```
Rejected at compile time.
> **Known divergence:** the implementation does not do this yet; see [Q-007](OPEN_QUESTIONS.md). The example is marked `xfail`.

## while

**SPEC-STMT-003** `while c { body }` evaluates `c` before each iteration and stops when it is false; `break` leaves the loop and `continue` starts the next iteration.

<!-- example: tests/spec/statements/SPEC-STMT-003.wj -->
```windjammer
fn main() {
    let mut i = 0
    let mut s = 0
    while i < 10 {
        i += 1
        if i % 2 == 0 { continue }
        if i > 7 { break }
        s += i
    }
    println("${s} ${i}")
}
```
Output:
```text
16 9
```

## loop

**SPEC-STMT-004** `loop { body }` repeats forever until `break` (or `return`).

<!-- example: tests/spec/statements/SPEC-STMT-004.wj -->
```windjammer
fn main() {
    let mut n = 0
    loop {
        n += 3
        if n > 10 { break }
    }
    println("${n}")
}
```
Output:
```text
12
```

## for

**SPEC-STMT-005** `for pat in iterable { body }` evaluates `iterable` once and runs `body` for each element, binding `pat`. Ranges, lists, strings' `chars()` and map/set iterators are iterable.

<!-- example: tests/spec/statements/SPEC-STMT-005.wj -->
```windjammer
fn main() {
    let v = ["a", "b", "c"]
    let mut out = ""
    for s in v { out = out + s }
    for i in 0..3 { out = out + "${i}" }
    for (i, s) in v.iter().enumerate() { out = out + "${i}${s}" }
    println(out)
}
```
Output:
```text
abc0120a1b2c
```

## return

**SPEC-STMT-006** `return e` leaves the enclosing function with value `e` (`return` alone for `()`). The last expression of the body is an implicit return.

<!-- example: tests/spec/statements/SPEC-STMT-006.wj -->
```windjammer
fn sign(n: int) -> string {
    if n < 0 { return "neg" }
    if n == 0 { return "zero" }
    "pos"
}
fn main() {
    println("${sign(-4)} ${sign(0)} ${sign(9)}")
}
```
Output:
```text
neg zero pos
```

## defer

**SPEC-STMT-007** `defer stmt` runs `stmt` when the enclosing function exits, on every path (normal, early `return`); deferred statements run in reverse order of registration.

<!-- example: tests/spec/statements/SPEC-STMT-007.wj -->
```windjammer
fn work(n: int) -> int {
    defer println("cleanup1")
    defer println("cleanup2")
    if n > 0 { return n }
    println("not reached when n > 0")
    0
}
fn main() {
    println("${work(1)}")
}
```
Output:
```text
cleanup2
cleanup1
1
```

## Destructuring let

**SPEC-STMT-008** `let` accepts any irrefutable pattern: tuples, structs and nested combinations.

<!-- example: tests/spec/statements/SPEC-STMT-008.wj -->
```windjammer
struct P { x: int, y: int }
fn main() {
    let (a, (b, c)) = (1, (2, 3))
    let P { x, y } = P { x: 7, y: 8 }
    println("${a}${b}${c} ${x} ${y}")
}
```
Output:
```text
123 7 8
```

## let-else and if-let

**SPEC-STMT-009** `if let P = e { .. } else { .. }` runs the first block when `e` matches `P`; `let P = e else { diverge }` binds on match and otherwise must leave the scope (`return`, `break`, `continue`, `panic`). `while let P = e { .. }` loops while `e` matches.

<!-- example: tests/spec/statements/SPEC-STMT-009.wj -->
```windjammer
fn first(o: Option<int>) -> int {
    let Some(v) = o else { return -1 }
    v
}
fn main() {
    if let Some(a) = Some(5) { println("a=${a}") } else { println("none") }
    println("${first(Some(3))} ${first(None)}")
    let mut cur = Some(2)
    while let Some(n) = cur {
        println("n=${n}")
        cur = if n > 0 { Some(n - 1) } else { None }
    }
}
```
Output:
```text
a=5
3 -1
n=2
n=1
n=0
```

## Shadowing scope

**SPEC-STMT-010** A block introduces a scope; bindings made inside end at its closing brace and never leak out.

<!-- example: tests/spec/statements/SPEC-STMT-010.wj -->
```windjammer
fn main() {
    let a = 1
    if true {
        let a = 2
        let b = 3
        println("${a} ${b}")
    }
    println("${a}")
}
```
Output:
```text
2 3
1
```

## Bare return

**SPEC-STMT-011** [OPEN Q-026] A bare `return` (no value) ends the function even when the next line starts another statement. The parser currently treats a newline after `return` as nothing and parses the next line as the returned expression, so `return` newline `println(..)` runs the `println`.

<!-- example: tests/spec/statements/SPEC-STMT-011.wj -->
```windjammer
fn g() {
    return
    println("g after")
}
fn main() {
    g()
    println("done")
}
```
Output:
```text
done
```
> **Known divergence:** the implementation does not do this yet; see [Q-026](OPEN_QUESTIONS.md). The example is marked `xfail`.


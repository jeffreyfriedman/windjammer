# Pipe operator

`x |> stage` is parser-level sugar for a call (design: docs/design/pipe-operator.md). Because the parser removes it, all backends agree by construction.

Part of the [Windjammer language specification](README.md). Rule IDs are checked by `scripts/wj/spec_check.wj`; each rule's executable example is the code block below it, stored in `tests/spec/pipe/`.

## Basic pipe

**SPEC-PIPE-001** `x |> f` is `f(x)`; `x |> f(a, b)` is `f(x, a, b)`; stages chain left to right: `x |> f |> g` is `g(f(x))`. The parser rewrites pipes into ordinary calls, so every backend behaves identically.

<!-- example: tests/spec/pipe/SPEC-PIPE-001.wj -->
```windjammer
fn double(x: int) -> int { x * 2 }
fn add(x: int, y: int) -> int { x + y }
fn main() {
    println("${5 |> double}")
    println("${5 |> double |> add(1)}")
}
```
Output:
```text
10
11
```

## Placeholder

**SPEC-PIPE-002** In `x |> f(a, _, b)` the single `_` marks where `x` is inserted. Zero placeholders means first argument; two or more is an error.

<!-- example: tests/spec/pipe/SPEC-PIPE-002.wj -->
```windjammer
fn sub(a: int, b: int) -> int { a - b }
fn main() {
    println("${5 |> sub(10, _)} ${5 |> sub(_, 1)}")
}
```
Output:
```text
5 4
```

## Method stage

**SPEC-PIPE-003** `x |> .m(args)` is `x.m(args)`: `x` is the receiver.

<!-- example: tests/spec/pipe/SPEC-PIPE-003.wj -->
```windjammer
fn main() {
    let n = "  hi  " |> .trim() |> .len()
    println("${n}")
}
```
Output:
```text
2
```

## Precedence

**SPEC-PIPE-004** `|>` has the lowest precedence of all expression operators and is left-associative: `a + 1 |> f` is `f(a + 1)`.

<!-- example: tests/spec/pipe/SPEC-PIPE-004.wj -->
```windjammer
fn f(x: int) -> int { x * 10 }
fn main() {
    let a = 1
    println("${a + 1 |> f}")
    println("${false || true |> f2}")
}
fn f2(b: bool) -> string { if b { "T" } else { "F" } }
```
Output:
```text
20
T
```

## Closure stage and multi-line chains

**SPEC-PIPE-005** A closure may be a stage (`x |> |v| v + 1`); a line may begin with `|>` to continue the chain.

<!-- example: tests/spec/pipe/SPEC-PIPE-005.wj -->
```windjammer
fn dbl(x: int) -> int { x * 2 }
fn main() {
    let r = 3
        |> dbl
        |> |v| v + 1
    println("${r}")
}
```
Output:
```text
7
```

## Try stage

**SPEC-PIPE-006** A stage may be followed by `?`: `x |> f? |> g?` propagates errors exactly like `g(f(x)?)?`.

<!-- example: tests/spec/pipe/SPEC-PIPE-006.wj -->
```windjammer
fn parse(s: string) -> Result<int, string> { if s == "1" { Ok(1) } else { Err("bad ${s}") } }
fn inc(x: int) -> Result<int, string> { Ok(x + 1) }
fn run(s: string) -> Result<int, string> {
    let r = s |> parse? |> inc?
    Ok(r)
}
fn main() {
    println("${run("1").unwrap()}")
    match run("2") { Ok(v) => println("ok ${v}"), Err(e) => println(e) }
}
```
Output:
```text
2
bad 2
```

## Invalid right-hand sides

**SPEC-PIPE-007** The right side of `|>` must be a function, call, method stage or closure; anything else (a literal, arithmetic, `if`) is a compile-time error.

<!-- example: tests/spec/pipe/SPEC-PIPE-007.wj -->
<!-- should_fail: the rule says this program is rejected -->
```windjammer,should_fail
fn main() {
    let r = 1 |> 2
}
```
Rejected at compile time.


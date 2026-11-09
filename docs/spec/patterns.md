# Patterns

Patterns appear in `match`, `let`, `if let`, `while let`, `for` and closure parameters.

Part of the [Windjammer language specification](README.md). Rule IDs are checked by `scripts/wj/spec_check.wj`; each rule's executable example is the code block below it, stored in `tests/spec/patterns/`.

## Wildcard, binding and literal

**SPEC-PAT-001** `_` matches anything and binds nothing; an identifier matches anything and binds it; a literal matches an equal value. Arms are tried top to bottom.

<!-- example: tests/spec/patterns/SPEC-PAT-001.wj -->
```windjammer
fn f(n: int) -> string {
    match n {
        0 => "zero",
        7 => "seven",
        k => "other ${k}",
    }
}
fn main() {
    println("${f(0)} ${f(7)} ${f(9)}")
    let s = match 3 { _ => "any" }
    println(s)
}
```
Output:
```text
zero seven other 9
any
```

## Or-patterns

**SPEC-PAT-002** `p1 | p2` matches when either alternative matches.

<!-- example: tests/spec/patterns/SPEC-PAT-002.wj -->
```windjammer
fn f(n: int) -> string {
    match n {
        1 | 2 | 3 => "low",
        4 | 5 => "mid",
        _ => "high",
    }
}
fn main() {
    println("${f(2)} ${f(5)} ${f(6)}")
}
```
Output:
```text
low mid high
```

## Guards

**SPEC-PAT-003** `p if cond => v` matches only when `p` matches and `cond` (which may use the bindings of `p`) is true; otherwise matching continues with the next arm.

<!-- example: tests/spec/patterns/SPEC-PAT-003.wj -->
```windjammer
fn f(n: int) -> string {
    match n {
        x if x > 100 => "big",
        x if x > 0 => "small",
        0 => "zero",
        _ => "negative",
    }
}
fn main() {
    println("${f(500)} ${f(5)} ${f(0)} ${f(-3)}")
}
```
Output:
```text
big small zero negative
```

## Tuple patterns

**SPEC-PAT-004** `(p1, p2, ..)` matches a tuple element-wise.

<!-- example: tests/spec/patterns/SPEC-PAT-004.wj -->
```windjammer
fn f(t: (int, int)) -> string {
    match t {
        (0, 0) => "origin",
        (0, y) => "y-axis ${y}",
        (x, 0) => "x-axis ${x}",
        (x, y) => "pt ${x},${y}",
    }
}
fn main() {
    println("${f((0, 0))} | ${f((0, 4))} | ${f((3, 0))} | ${f((1, 2))}")
}
```
Output:
```text
origin | y-axis 4 | x-axis 3 | pt 1,2
```

## Enum patterns

**SPEC-PAT-005** `E::V`, `E::V(p, ..)` and `E::V { f, g: p }` match enum variants by name and destructure their payload. `Some(p)`, `None`, `Ok(p)`, `Err(p)` are the built-in Option/Result patterns.

<!-- example: tests/spec/patterns/SPEC-PAT-005.wj -->
```windjammer
enum Shape { Circle(float), Rect { w: float, h: float }, Dot }
fn area(s: Shape) -> float {
    match s {
        Shape::Circle(r) => 3.0 * r * r,
        Shape::Rect { w, h } => w * h,
        Shape::Dot => 0.0,
    }
}
fn main() {
    println("${area(Shape::Circle(2.0))} ${area(Shape::Rect { w: 2.0, h: 5.0 })} ${area(Shape::Dot)}")
    let o: Option<int> = Some(4)
    match o { Some(v) => println("some ${v}"), None => println("none") }
}
```
Output:
```text
12 10 0
some 4
```

## Struct patterns

**SPEC-PAT-006** `S { f, g: p }` matches a struct, binding or matching named fields; `..` ignores the rest.

<!-- example: tests/spec/patterns/SPEC-PAT-006.wj -->
```windjammer
struct P { x: int, y: int, z: int }
fn main() {
    let p = P { x: 1, y: 2, z: 3 }
    match p {
        P { x: 0, .. } => println("x is zero"),
        P { x, y: 2, .. } => println("x=${x} y is two"),
        _ => println("other"),
    }
}
```
Output:
```text
x=1 y is two
```

## Nested patterns

**SPEC-PAT-007** Patterns nest to any depth.

<!-- example: tests/spec/patterns/SPEC-PAT-007.wj -->
```windjammer
enum Expr { Num(int), Add(Box<Expr>, Box<Expr>) }
fn eval(e: Expr) -> int {
    match e {
        Expr::Num(n) => n,
        Expr::Add(a, b) => eval(*a) + eval(*b),
    }
}
fn main() {
    let e = Expr::Add(Box::new(Expr::Num(2)), Box::new(Expr::Add(Box::new(Expr::Num(3)), Box::new(Expr::Num(4)))))
    println("${eval(e)}")
    let o: Option<(int, int)> = Some((1, 2))
    if let Some((a, b)) = o { println("${a + b}") }
}
```
Output:
```text
9
3
```

## Range patterns

**SPEC-PAT-008** `lo..=hi` (inclusive) and `lo..hi` (exclusive) match integers and chars in the range.

<!-- example: tests/spec/patterns/SPEC-PAT-008.wj -->
```windjammer
fn f(n: int) -> string {
    match n {
        0..=9 => "digit",
        10..100 => "two",
        _ => "more",
    }
}
fn main() {
    println("${f(5)} ${f(10)} ${f(100)}")
}
```
Output:
```text
digit two more
```

## Match with no matching arm

**SPEC-PAT-009** A `match` that matches no arm at run time panics; for enum scrutinees the compiler requires the arms to be exhaustive, and for `int` a `_` or binding arm.

<!-- example: tests/spec/patterns/SPEC-PAT-009.wj -->
```windjammer
fn f(n: int) -> string {
    match n {
        1 => "one",
        2 => "two",
    }
}
fn main() {
    println("${f(1)}")
    println("${f(3)}")
}
```
Output:
```text
one
```
Then panics, exit status 101.

## String and bool patterns

**SPEC-PAT-010** String literals and `true`/`false` are valid patterns.

<!-- example: tests/spec/patterns/SPEC-PAT-010.wj -->
```windjammer
fn f(s: string) -> int {
    match s {
        "a" => 1,
        "bc" => 2,
        _ => 0,
    }
}
fn main() {
    println("${f("a")} ${f("bc")} ${f("z")}")
    match true { true => println("T"), false => println("F") }
}
```
Output:
```text
1 2 0
T
```


## Guards, ranges, or-patterns and struct variants

**SPEC-PAT-011** `match` tries arms in order; a guard (`if`) decides after the pattern binds; integer ranges (`1..=9`) and or-patterns (`10 | 20 | 30`) match literals; struct and tuple variants bind their fields, `..` ignores the rest.

<!-- example: tests/spec/patterns/SPEC-PAT-011.wj -->
```windjammer
enum Shape {
    Circle(int),
    Rect(int, int),
    Tri { a: int, b: int, c: int },
    Empty,
}

fn classify(s: Shape) -> string {
    match s {
        Shape::Circle(r) if r > 10 => "big circle",
        Shape::Circle(_) => "circle",
        Shape::Rect(w, h) if w == h => "square",
        Shape::Rect(_, _) => "rect",
        Shape::Tri { a, b, c } if a == b && b == c => "equilateral",
        Shape::Tri { a, .. } => if a > 3 { "tall tri" } else { "tri" },
        Shape::Empty => "empty",
    }
}

fn bucket(n: int) -> string {
    match n {
        n if n < 0 => "negative",
        0 => "zero",
        1..=9 => "digit",
        10 | 20 | 30 => "round",
        _ => "large",
    }
}

fn main() {
    println(classify(Shape::Circle(11)))
    println(classify(Shape::Circle(2)))
    println(classify(Shape::Rect(3, 3)))
    println(classify(Shape::Rect(3, 4)))
    println(classify(Shape::Tri { a: 2, b: 2, c: 2 }))
    println(classify(Shape::Tri { a: 5, b: 1, c: 1 }))
    println(classify(Shape::Empty))
    println("${bucket(-5)} ${bucket(0)} ${bucket(7)} ${bucket(20)} ${bucket(99)}")
}
```
Output:
```text
big circle
circle
square
rect
equilateral
tall tri
empty
negative zero digit round large
```

## Destructuring

**SPEC-PAT-012** `let` destructures tuples, nested tuples and structs; `for` destructures tuple items; nested `Option` patterns with guards select the innermost case.

<!-- example: tests/spec/patterns/SPEC-PAT-012.wj -->
```windjammer
struct P {
    x: int,
    y: int,
}

fn main() {
    let pair = (1, ("two", 3.5))
    let (a, (b, c)) = pair
    println("${a} ${b} ${c}")
    let p = P { x: 4, y: 5 }
    let P { x, y } = p
    println("${x + y}")
    let v = vec![10, 20, 30, 40]
    println("${v[0]} ${v[v.len() - 1]} ${v.len() - 2}")
    let opt = Some(Some(3))
    match opt {
        Some(Some(n)) if n > 2 => println("deep ${n}"),
        Some(Some(_)) => println("small"),
        Some(None) => println("inner none"),
        None => println("none"),
    }
    let n = 5
    let msg = match n {
        1..=5 => "low",
        _ => "high",
    }
    println("${msg} ${n}")
    let items = vec![(1, "a"), (2, "b")]
    for (i, s) in items {
        println("${i}${s}")
    }
}
```
Output:
```text
1 two 3.5
9
10 40 2
deep 3
low 5
1a
2b
```

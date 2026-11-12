# Control flow, patterns and match
<!-- feature: statements -->

Programs branch and repeat; Windjammer writes both as expressions where it can, so a branch can give a value straight to a
`let`. This chapter covers `if`, the three loops, and then `match` and the patterns that let a program take a value apart.

## Branches and loops

`if` is an expression: both branches give a value of the same type. `while` repeats while a condition holds, `loop`
repeats until it hits `break`, and `break` can carry a value out of a `loop`. `for` walks a range or a collection, and
`continue` skips to the next round:

```wj
// expect: big
// expect: 1 2 3 
// expect: found 6
// expect: 3
fn main() {
    let n = 42
    let size = if n > 10 { "big" } else { "small" }
    println("${size}")

    let mut line = ""
    let mut i = 1
    while i <= 3 {
        line = line + "${i} "
        i = i + 1
    }
    println("${line}")

    let mut k = 0
    let found = loop {
        k = k + 1
        if k * k > 30 {
            break k
        }
    }
    println("found ${found}")

    let mut count = 0
    for j in 0..10 {
        if j % 2 == 0 {
            continue
        }
        if j > 6 {
            break
        }
        count = count + 1
    }
    println("${count}")
}
```

## Match

`match` compares a value with patterns, top to bottom, and runs the first arm that fits. A pattern can be a literal, a
name that binds the value, a range (`0..=9` includes the end, `0..9` does not), several alternatives joined by `|`, and any
of these followed by an `if` guard that decides after the pattern has bound its names:

```wj
// expect: negative
// expect: digit
// expect: teen
// expect: small even
// expect: big
fn describe(n: int) -> string {
    match n {
        x if x < 0 => "negative",
        0..=9 => "digit",
        10 | 11 | 12 | 13 => "teen",
        x if x < 100 && x % 2 == 0 => "small even",
        _ => "big",
    }
}

fn main() {
    println("${describe(-5)}")
    println("${describe(7)}")
    println("${describe(12)}")
    println("${describe(42)}")
    println("${describe(501)}")
}
```

For an enum the compiler requires every variant to be covered, so adding a variant to an enum shows you each `match` that
needs an arm for it. For an `int` or a `string`, end with `_` (anything) or a name.

## Taking values apart

A pattern can also open a value. A tuple pattern matches element by element, an enum pattern names the variant and binds
its payload, and a struct pattern binds fields by name; `..` ignores the rest. The same patterns work in a plain `let`:

```wj
// expect: area 12
// expect: w=2 h=5
// expect: on the y axis at 4
// expect: id 7 is Ada
enum Shape {
    Circle(float),
    Rect { w: float, h: float },
    Dot,
}

struct User {
    id: int,
    name: string,
    admin: bool,
}

fn area(s: Shape) -> float {
    match s {
        Shape::Circle(r) => 3.0 * r * r,
        Shape::Rect { w, h } => w * h,
        Shape::Dot => 0.0,
    }
}

fn where_is(p: (int, int)) -> string {
    match p {
        (0, 0) => "at the origin",
        (0, y) => "on the y axis at ${y}",
        (x, 0) => "on the x axis at ${x}",
        (x, y) => "at ${x},${y}",
    }
}

fn main() {
    println("area ${area(Shape::Circle(2.0))}")
    let r = Shape::Rect { w: 2.0, h: 5.0 }
    if let Shape::Rect { w, h } = r {
        println("w=${w} h=${h}")
    }
    println("${where_is((0, 4))}")
    let u = User { id: 7, name: "Ada", admin: false }
    let User { id, name, .. } = u
    println("id ${id} is ${name}")
}
```

## One case: `if let` and `while let`

When only one shape matters, `if let` runs a block when the pattern fits and does nothing otherwise. `while let` repeats
for as long as it fits, which is how you drain a list with `pop`. `match` on an `Option` is the full form of the same idea:

```wj
// expect: 3
// expect: 2
// expect: 1
// expect: user: ada
// expect: no port
fn main() {
    let mut stack = [1, 2, 3]
    while let Some(top) = stack.pop() {
        println("${top}")
    }

    let name: Option<string> = Some("ada")
    if let Some(n) = name {
        println("user: ${n}")
    }

    let port: Option<int> = None
    match port {
        Some(p) => println("port ${p}"),
        None => println("no port"),
    }
}
```

## Guard-only match

A `match` with no scrutinee is a table of conditions, and it is an expression. [Pipe notation and expression-oriented
style](pipe-and-expressions.md) shows it. [Errors and Result](errors.md) uses these same patterns for `Ok`, `Err`, `Some`
and `None`, and the compiler's message when the pattern and the value disagree (matching `Ok` against an `Option`) names
the variant you want instead.

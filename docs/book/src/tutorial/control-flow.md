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

[Run this in the playground](../playground/index.html#lang=wj&code=Ly8gZXhwZWN0OiBiaWcKLy8gZXhwZWN0OiAxIDIgMyAKLy8gZXhwZWN0OiBmb3VuZCA2Ci8vIGV4cGVjdDogMwpmbiBtYWluKCkgewogICAgbGV0IG4gPSA0MgogICAgbGV0IHNpemUgPSBpZiBuID4gMTAgeyAiYmlnIiB9IGVsc2UgeyAic21hbGwiIH0KICAgIHByaW50bG4oIiR7c2l6ZX0iKQoKICAgIGxldCBtdXQgbGluZSA9ICIiCiAgICBsZXQgbXV0IGkgPSAxCiAgICB3aGlsZSBpIDw9IDMgewogICAgICAgIGxpbmUgPSBsaW5lICsgIiR7aX0gIgogICAgICAgIGkgPSBpICsgMQogICAgfQogICAgcHJpbnRsbigiJHtsaW5lfSIpCgogICAgbGV0IG11dCBrID0gMAogICAgbGV0IGZvdW5kID0gbG9vcCB7CiAgICAgICAgayA9IGsgKyAxCiAgICAgICAgaWYgayAqIGsgPiAzMCB7CiAgICAgICAgICAgIGJyZWFrIGsKICAgICAgICB9CiAgICB9CiAgICBwcmludGxuKCJmb3VuZCAke2ZvdW5kfSIpCgogICAgbGV0IG11dCBjb3VudCA9IDAKICAgIGZvciBqIGluIDAuLjEwIHsKICAgICAgICBpZiBqICUgMiA9PSAwIHsKICAgICAgICAgICAgY29udGludWUKICAgICAgICB9CiAgICAgICAgaWYgaiA-IDYgewogICAgICAgICAgICBicmVhawogICAgICAgIH0KICAgICAgICBjb3VudCA9IGNvdW50ICsgMQogICAgfQogICAgcHJpbnRsbigiJHtjb3VudH0iKQp9Cg)

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

[Run this in the playground](../playground/index.html#lang=wj&code=Ly8gZXhwZWN0OiBuZWdhdGl2ZQovLyBleHBlY3Q6IGRpZ2l0Ci8vIGV4cGVjdDogdGVlbgovLyBleHBlY3Q6IHNtYWxsIGV2ZW4KLy8gZXhwZWN0OiBiaWcKZm4gZGVzY3JpYmUobjogaW50KSAtPiBzdHJpbmcgewogICAgbWF0Y2ggbiB7CiAgICAgICAgeCBpZiB4IDwgMCA9PiAibmVnYXRpdmUiLAogICAgICAgIDAuLj05ID0-ICJkaWdpdCIsCiAgICAgICAgMTAgfCAxMSB8IDEyIHwgMTMgPT4gInRlZW4iLAogICAgICAgIHggaWYgeCA8IDEwMCAmJiB4ICUgMiA9PSAwID0-ICJzbWFsbCBldmVuIiwKICAgICAgICBfID0-ICJiaWciLAogICAgfQp9CgpmbiBtYWluKCkgewogICAgcHJpbnRsbigiJHtkZXNjcmliZSgtNSl9IikKICAgIHByaW50bG4oIiR7ZGVzY3JpYmUoNyl9IikKICAgIHByaW50bG4oIiR7ZGVzY3JpYmUoMTIpfSIpCiAgICBwcmludGxuKCIke2Rlc2NyaWJlKDQyKX0iKQogICAgcHJpbnRsbigiJHtkZXNjcmliZSg1MDEpfSIpCn0K)

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

[Run this in the playground](../playground/index.html#lang=wj&code=Ly8gZXhwZWN0OiBhcmVhIDEyCi8vIGV4cGVjdDogdz0yIGg9NQovLyBleHBlY3Q6IG9uIHRoZSB5IGF4aXMgYXQgNAovLyBleHBlY3Q6IGlkIDcgaXMgQWRhCmVudW0gU2hhcGUgewogICAgQ2lyY2xlKGZsb2F0KSwKICAgIFJlY3QgeyB3OiBmbG9hdCwgaDogZmxvYXQgfSwKICAgIERvdCwKfQoKc3RydWN0IFVzZXIgewogICAgaWQ6IGludCwKICAgIG5hbWU6IHN0cmluZywKICAgIGFkbWluOiBib29sLAp9CgpmbiBhcmVhKHM6IFNoYXBlKSAtPiBmbG9hdCB7CiAgICBtYXRjaCBzIHsKICAgICAgICBTaGFwZTo6Q2lyY2xlKHIpID0-IDMuMCAqIHIgKiByLAogICAgICAgIFNoYXBlOjpSZWN0IHsgdywgaCB9ID0-IHcgKiBoLAogICAgICAgIFNoYXBlOjpEb3QgPT4gMC4wLAogICAgfQp9CgpmbiB3aGVyZV9pcyhwOiAoaW50LCBpbnQpKSAtPiBzdHJpbmcgewogICAgbWF0Y2ggcCB7CiAgICAgICAgKDAsIDApID0-ICJhdCB0aGUgb3JpZ2luIiwKICAgICAgICAoMCwgeSkgPT4gIm9uIHRoZSB5IGF4aXMgYXQgJHt5fSIsCiAgICAgICAgKHgsIDApID0-ICJvbiB0aGUgeCBheGlzIGF0ICR7eH0iLAogICAgICAgICh4LCB5KSA9PiAiYXQgJHt4fSwke3l9IiwKICAgIH0KfQoKZm4gbWFpbigpIHsKICAgIHByaW50bG4oImFyZWEgJHthcmVhKFNoYXBlOjpDaXJjbGUoMi4wKSl9IikKICAgIGxldCByID0gU2hhcGU6OlJlY3QgeyB3OiAyLjAsIGg6IDUuMCB9CiAgICBpZiBsZXQgU2hhcGU6OlJlY3QgeyB3LCBoIH0gPSByIHsKICAgICAgICBwcmludGxuKCJ3PSR7d30gaD0ke2h9IikKICAgIH0KICAgIHByaW50bG4oIiR7d2hlcmVfaXMoKDAsIDQpKX0iKQogICAgbGV0IHUgPSBVc2VyIHsgaWQ6IDcsIG5hbWU6ICJBZGEiLCBhZG1pbjogZmFsc2UgfQogICAgbGV0IFVzZXIgeyBpZCwgbmFtZSwgLi4gfSA9IHUKICAgIHByaW50bG4oImlkICR7aWR9IGlzICR7bmFtZX0iKQp9Cg)

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

[Run this in the playground](../playground/index.html#lang=wj&code=Ly8gZXhwZWN0OiAzCi8vIGV4cGVjdDogMgovLyBleHBlY3Q6IDEKLy8gZXhwZWN0OiB1c2VyOiBhZGEKLy8gZXhwZWN0OiBubyBwb3J0CmZuIG1haW4oKSB7CiAgICBsZXQgbXV0IHN0YWNrID0gWzEsIDIsIDNdCiAgICB3aGlsZSBsZXQgU29tZSh0b3ApID0gc3RhY2sucG9wKCkgewogICAgICAgIHByaW50bG4oIiR7dG9wfSIpCiAgICB9CgogICAgbGV0IG5hbWU6IE9wdGlvbjxzdHJpbmc-ID0gU29tZSgiYWRhIikKICAgIGlmIGxldCBTb21lKG4pID0gbmFtZSB7CiAgICAgICAgcHJpbnRsbigidXNlcjogJHtufSIpCiAgICB9CgogICAgbGV0IHBvcnQ6IE9wdGlvbjxpbnQ-ID0gTm9uZQogICAgbWF0Y2ggcG9ydCB7CiAgICAgICAgU29tZShwKSA9PiBwcmludGxuKCJwb3J0ICR7cH0iKSwKICAgICAgICBOb25lID0-IHByaW50bG4oIm5vIHBvcnQiKSwKICAgIH0KfQo)

## Guard-only match

A `match` with no scrutinee is a table of conditions, and it is an expression. [Pipe notation and expression-oriented
style](pipe-and-expressions.md) shows it. [Errors and Result](errors.md) uses these same patterns for `Ok`, `Err`, `Some`
and `None`, and the compiler's message when the pattern and the value disagree (matching `Ok` against an `Option`) names
the variant you want instead.

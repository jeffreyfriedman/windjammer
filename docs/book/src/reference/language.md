# The language
<!-- feature: grammar -->
<!-- feature: standard-library -->

This page is a map of the language. Each area links to its chapter in the language specification, where every rule has
an identifier and a runnable example that the compiler's own tests check. The [grammar](grammar.md),
[keywords](keywords.md) and [standard library](std.md) pages are generated from the compiler, so they always match the
`wj` you have.

## At a glance

```wj
// expect: Circle area 12
// expect: Rect area 6
// expect: total 18
trait Area {
    fn area(self) -> int
}

struct Circle {
    r: int,
}

struct Rect {
    w: int,
    h: int,
}

impl Area for Circle {
    fn area(self) -> int {
        self.r * self.r * 3
    }
}

impl Area for Rect {
    fn area(self) -> int {
        self.w * self.h
    }
}

fn total(areas: Vec<int>) -> int {
    let mut sum = 0
    for a in areas {
        sum += a
    }
    sum
}

fn main() {
    let c = Circle { r: 2 }
    let r = Rect { w: 2, h: 3 }
    println("Circle area ${c.area()}")
    println("Rect area ${r.area()}")
    println("total ${total([c.area(), r.area()])}")
}
```

## Specification chapters

| Area | Chapter |
|---|---|
| Tokens, comments, literals, escapes | [Lexical structure](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/spec/lexical.md) |
| Syntax conventions and the checked grammar | [Grammar](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/spec/grammar.md) |
| Type forms, inference, conversions | [Types](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/spec/types.md) |
| Numbers: widths, overflow, division, floats | [Numeric semantics](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/spec/numeric.md) |
| Precedence, `if`, `match`, closures, ranges | [Expressions](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/spec/expressions.md) |
| `let`, loops, `defer` | [Statements](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/spec/statements.md) |
| Every pattern form | [Patterns](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/spec/patterns.md) |
| Functions, structs, enums, impls, constants | [Items](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/spec/items.md) |
| Traits and generics | [Traits](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/spec/traits.md) |
| Moves, borrows, copies | [Ownership](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/spec/ownership.md) and [inference](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/spec/inference.md) |
| Strings and formatting | [Strings](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/spec/strings.md) |
| Lists, maps, sets | [Collections](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/spec/collections.md) |
| Closures | [Closures](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/spec/closures.md) |
| `Option` and `Result` | [Optionals](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/spec/optionals.md) and [errors](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/spec/errors.md) |
| The pipe operator | [Pipe](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/spec/pipe.md) |
| Modules and visibility | [Modules](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/spec/modules.md) |
| Attributes such as `@test` | [Attributes](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/spec/attributes.md) |
| Threads, tasks, channels | [Concurrency](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/spec/concurrency.md) |
| Effects, grants, capability values | [Effects](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/spec/effects.md) |
| How a program starts, runs and exits | [Execution](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/spec/execution.md) |

The specification also keeps a list of [open questions](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/spec/OPEN_QUESTIONS.md):
the few places where a decision is recorded and the implementation is still catching up.

## Conventions worth knowing

* No semicolons; a block's last expression is its value.
* `let` is immutable, `let mut` is mutable.
* Strings interpolate with `${expr}` and `${expr:spec}`.
* `println`, `print`, `panic`, `assert`, `assert_eq` and `assert_ne` are functions.
* Ownership is inferred: you write `fn f(v: Vec<int>)`, and the compiler decides between borrow, mutable borrow and move.
* `.iter()` and `.unwrap()` are not needed: iterate with `for x in v` and `v.map(...)`, and handle `Option` and
  `Result` with `match`, `?` and `unwrap_or`.
* Rust attributes such as `#[derive(...)]` are replaced by inference; Windjammer attributes start with `@`.

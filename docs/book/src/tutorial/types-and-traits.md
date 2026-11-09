# Structs, enums, traits and generics
<!-- feature: items-and-traits -->
<!-- feature: patterns -->

You build your own types from three pieces: structs group values, enums choose between alternatives, and traits describe
behaviour that several types share. Generics let one definition work for many types.

## Structs

A struct names its fields and their types. `impl` adds methods and associated functions:

```wj
// expect: 6
// expect: (4, -3)
struct Point {
    x: int,
    y: int,
}

impl Point {
    fn new(x: int, y: int) -> Point {
        Point { x: x, y: y }
    }

    fn manhattan(self) -> int {
        self.x.abs() + self.y.abs()
    }

    fn shift(self, dx: int, dy: int) {
        self.x += dx
        self.y += dy
    }
}

fn main() {
    let mut p = Point::new(3, -4)
    println("${p.manhattan() - 1}")
    p.shift(1, 1)
    println("(${p.x}, ${p.y})")
}
```

Methods take plain `self`. As the [ownership chapter](ownership.md) explains, the compiler works out whether a method
reads, changes or consumes its receiver, and `p.shift(1, 1)` is why `p` is declared with `let mut`.

Derives such as `Debug`, `Clone`, `PartialEq` and `Copy` are inferred from how a type is used, so there is no
`#[derive(...)]` to write. Printing a struct with `${p:?}` works when you want its debug form.

## Enums

An enum is a value that is exactly one of several variants, each with its own data. `match` takes it apart and the
compiler checks that every variant is handled:

```wj
// expect: 12
// expect: 10
// expect: 1
enum Shape {
    Circle(int),
    Rect(int, int),
    Unit,
}

impl Shape {
    fn area(self) -> int {
        match self {
            Shape::Circle(r) => r * r * 3,
            Shape::Rect(w, h) => w * h,
            Shape::Unit => 1,
        }
    }
}

fn main() {
    println("${Shape::Circle(2).area()}")
    println("${Shape::Rect(2, 5).area()}")
    println("${Shape::Unit.area()}")
}
```

Leave out a variant and the program does not compile: `wj explain WJ0010` describes the error for a non-exhaustive
`match`. Arms can carry guards, and one arm can match several patterns with `|`:

```wj
// expect: even small
// expect: odd big
fn describe(n: int) -> string {
    match n {
        0 | 2 | 4 => "even small",
        x if x % 2 == 1 && x > 5 => "odd big",
        _ => "other",
    }
}

fn main() {
    println(describe(2))
    println(describe(7))
}
```

`Option` and `Result` are ordinary enums from the standard library, so everything you learn here applies to them.

## Traits

A trait names behaviour. A type opts in with `impl Trait for Type`. In a trait declaration a method has no body to
read, so you state the receiver you mean: `&self` when implementations only read. A method with a default body can use
the other methods of its trait:

```wj
// expect: point(4, -3)!
// expect: circle r=2!
trait Describe {
    fn describe(&self) -> string

    fn shout(self) -> string {
        "${self.describe()}!"
    }
}

struct Point {
    x: int,
    y: int,
}

struct Circle {
    r: int,
}

impl Describe for Point {
    fn describe(self) -> string {
        "point(${self.x}, ${self.y})"
    }
}

impl Describe for Circle {
    fn describe(self) -> string {
        "circle r=${self.r}"
    }
}

fn main() {
    println(Point { x: 4, y: -3 }.shout())
    println(Circle { r: 2 }.shout())
}
```

## Generics

Generic functions and types take type parameters in angle brackets, optionally bounded by traits. The compiler
generates code for each type you use them with, so there is no run-time cost:

```wj
// expect: 9 pear
fn bigger<T: PartialOrd>(a: T, b: T) -> T {
    if a > b { a } else { b }
}

fn main() {
    println("${bigger(3, 9)} ${bigger("pear", "apple")}")
}
```

A bound can name a trait you defined. This function accepts anything that can describe itself:

```wj
// expect: point(4, -3)!
trait Describe {
    fn describe(&self) -> string

    fn shout(self) -> string {
        "${self.describe()}!"
    }
}

struct Point {
    x: int,
    y: int,
}

impl Describe for Point {
    fn describe(self) -> string {
        "point(${self.x}, ${self.y})"
    }
}

fn announce<T: Describe>(thing: T) -> string {
    thing.shout()
}

fn main() {
    println(announce(Point { x: 4, y: -3 }))
}
```

Types can be generic too. A stack of anything:

```wj
// expect: 2
// expect: b
struct Stack<T> {
    items: Vec<T>,
}

impl<T> Stack<T> {
    fn new() -> Stack<T> {
        Stack { items: [] }
    }

    fn push(self, item: T) {
        self.items.push(item)
    }

    fn size(self) -> int {
        self.items.len() as int
    }

    fn top(self) -> Option<T> {
        self.items.pop()
    }
}

fn main() {
    let mut s: Stack<string> = Stack::new()
    s.push("a")
    s.push("b")
    println("${s.size()}")
    println("${s.top().unwrap_or("none")}")
}
```

A generic function that copies an element out of a list and compares it, such as a `largest` over `Vec<T>`, needs
`T: PartialOrd + Clone` today on the compiled path. [Known gaps](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/known-gaps.md) tracks it.

## The pipe operator

Data flows left to right with `|>`, which feeds the value on its left into the call on its right. `x |> f(a)` means
`f(x, a)`, and `x |> .method(a)` means `x.method(a)`:

```wj
// expect: 16
// expect: [2, 4, 6]
fn double(x: int) -> int {
    x * 2
}

fn add(x: int, y: int) -> int {
    x + y
}

fn main() {
    println("${3 |> add(5) |> double}")
    let doubled: Vec<int> = [1, 2, 3] |> .map(|x| x * 2) |> .collect()
    println("${doubled:?}")
}
```

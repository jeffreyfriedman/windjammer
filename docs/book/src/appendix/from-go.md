# Migrating from Go

Windjammer shares Go's taste: a small language, a fast edit-run loop, tooling that comes with the compiler. `wj run` is
like `go run`, `wj test` like `go test`, and `wj fmt` like `gofmt`. The differences are sum types, generics with traits,
and a native-code path through Rust.

## Mapping

| Go | Windjammer |
|---|---|
| `fmt.Println("a", x)` | `println("a ${x}")` |
| `fmt.Sprintf("%d-%s", n, s)` | `"${n}-${s}"`, with specs like `${x:.2}`, `${n:>5}`, `${n:05}` |
| `x := 1`, `var x int = 1` | `let mut x = 1`, `let x: int = 1` |
| `func f(a int) (int, error)` | `fn f(a: int) -> Result<int, string>` |
| `if err != nil { return err }` | `let v = f()?` |
| `[]int{1, 2}`, `append(xs, 3)` | `[1, 2]`, `xs.push(3)` |
| `map[string]int{}` | `HashMap<string, int>` |
| `for i := 0; i < n; i++` | `for i in 0..n` |
| `for _, x := range xs` | `for x in xs` |
| `for i, x := range xs` | `for (i, x) in xs.enumerate()` |
| `switch x { case 1: ... }` | `match x { 1 => ..., _ => ... }`, exhaustive, no fallthrough |
| `type P struct{}` and methods | `struct P { ... }` and `impl P { fn m(self) }` |
| `interface { M() }` | `trait T { fn m(&self) }` and `impl T for P` |
| `nil` | `Option<T>`: `None` or `Some(v)` |
| `defer f()` | `defer f()` |
| `os.Exit(3)` | `process::exit(3)` |
| goroutines and channels | tasks and channels; see [Concurrency](../tutorial/concurrency.md) |

## Errors as values, with less typing

Go's `if err != nil` ladder becomes `?`:

```wj
// expect: 7
// expect: err: divide by zero
// expect: done
fn div(a: int, b: int) -> Result<int, string> {
    if b == 0 {
        return Err("divide by zero")
    }
    Ok(a / b)
}

fn run() -> Result<int, string> {
    let x = div(21, 3)?
    let y = div(x, 0)?
    Ok(y)
}

fn main() {
    defer println("done")
    match div(21, 3) {
        Ok(v) => println("${v}"),
        Err(e) => println("err: ${e}"),
    }
    match run() {
        Ok(v) => println("${v}"),
        Err(e) => println("err: ${e}"),
    }
}
```

`defer` runs when the function exits, as in Go, which is why `done` prints last.

## Interfaces become traits, and enums carry data

A Go interface with a type switch is an enum with `match` here, and the compiler checks that every case is handled:

```wj
// expect: circle 12
// expect: square 9
enum Shape {
    Circle(int),
    Square(int),
}

fn describe(s: Shape) -> string {
    match s {
        Shape::Circle(r) => "circle ${r * r * 3}",
        Shape::Square(side) => "square ${side * side}",
    }
}

fn main() {
    println(describe(Shape::Circle(2)))
    println(describe(Shape::Square(3)))
}
```

## What is new

* **Ownership is inferred.** Slices and maps are values you pass around as you would in Go, and the compiler decides
  between sharing and copying. See [Ownership and borrowing, inferred](../tutorial/ownership.md).
* **Integers are checked.** `int` overflow stops the program instead of wrapping.
* **Generics** with trait bounds: `fn largest<T: PartialOrd>(a: T, b: T) -> T`.
* **Programs declare what they may do**: files, network, environment. See
  [Effects and capabilities](../tutorial/effects.md).

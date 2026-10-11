# Coming from Go

Same spirit (fast loop, small language, tooling-first) with sum types and generics. `wj run --interpret` is
like `go run`; `wj test --interpret` like `go test`; `wj fmt` like `gofmt`.

## Mapping

| Go | Windjammer |
|----|------------|
| `fmt.Println("a", x)` | `println("a ${x}")` |
| `fmt.Sprintf("%d-%s", n, s)` | `"${n}-${s}"` (`${x:.2}`, `${n:>5}`, `${n:05}`) |
| `x := 1` / `var x int = 1` | `let x = 1` / `let mut x = 1` |
| `func f(a int) (int, error)` | `fn f(a: int) -> Result<int, string>` |
| `if err != nil { return err }` | `let v = f()?` |
| `[]int{1, 2}`, `append(xs, 3)` | `[1, 2]`, `xs.push(3)` |
| `map[string]int{}` | `HashMap<string, int>` (`use std::collections::HashMap`) |
| `for i := 0; i < n; i++` | `for i in 0..n` |
| `for _, x := range xs` | `for x in xs` |
| `for i, x := range xs` | `for (i, x) in xs.enumerate()` |
| `switch x { case 1: .. }` | `match x { 1 => .., _ => .. }` (exhaustive, no fallthrough) |
| `type P struct{}` + methods | `struct P { .. }` + `impl P { fn m(self) }` |
| `interface { M() }` | `trait T { fn m(self) }` and `impl T for P` |
| `nil` | `Option<T>`: `None` / `Some(v)` |
| `defer f()` | `defer f()` |
| `os.Exit(3)` | `process::exit(3)` |
| goroutines, channels | concurrency surface in `docs/adr/0004` (not in the interpreter) |

```windjammer
// expect: 7
// expect: err: divide by zero
// expect: done
fn div(a: int, b: int) -> Result<int, string> {
    if b == 0 { return Err("divide by zero") }
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

## Interfaces become traits, with data-carrying enums

```windjammer
// expect: circle 12
// expect: square 9
trait Shape {
    fn name(self) -> string
    fn area(self) -> int
}

struct Circle { r: int }
struct Square { s: int }

impl Shape for Circle {
    fn name(self) -> string { "circle" }
    fn area(self) -> int { 3 * self.r * self.r }
}

impl Shape for Square {
    fn name(self) -> string { "square" }
    fn area(self) -> int { self.s * self.s }
}

fn show<T: Shape>(s: T) {
    println("${s.name()} ${s.area()}")
}

fn main() {
    show(Circle { r: 2 })
    show(Square { s: 3 })
}
```

## Slices, maps and loops

```windjammer
// expect: [2, 3]
// expect: 3
// expect: 1 2 3
use std::collections::HashMap

fn main() {
    let xs = [1, 2, 3, 4]
    println("${xs[1..3]:?}")
    let mut seen: HashMap<int, bool> = HashMap::new()
    for x in xs { seen.insert(x % 3, true) }
    println("${seen.len()}")
    let mut out: Vec<string> = []
    for i in 1..=3 { out.push("${i}") }
    println(out.join(" "))
}
```

## Tests

```windjammer,test
fn abs_diff(a: int, b: int) -> int {
    if a > b { a - b } else { b - a }
}

@test
fn symmetric() {
    assert_eq(abs_diff(3, 8), abs_diff(8, 3))
}
```

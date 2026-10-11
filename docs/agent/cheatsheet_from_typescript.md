# Coming from TypeScript

Braces, arrow-like closures, template strings and structural data feel familiar; the type system is nominal and
sound: no `any`, no `undefined`/`null` (use `Option`), no exceptions (use `Result`).

## Mapping

| TypeScript | Windjammer |
|------------|------------|
| `console.log(\`x=${x}\`)` | `println("x=${x}")` |
| `const x = 1` / `let x = 1` | `let x = 1` / `let mut x = 1` |
| `number` | `int` or `float` (distinct types) |
| `string`, `boolean` | `string`, `bool` |
| `T[]`, `Array<T>` | `Vec<T>`: `[1, 2, 3]` |
| `Map<string, number>` | `HashMap<string, int>` |
| `x?: T`, `T \| undefined` | `Option<T>` |
| `(x) => x + 1` | `\|x\| x + 1` |
| `xs.map(f).filter(g)` | `xs.map(f).filter(g).collect()` (collect to materialise) |
| `xs.reduce((a, x) => a + x, 0)` | `xs.fold(0, \|a, x\| a + x)` |
| `interface P { x: number }` | `struct P { x: int }` |
| `type T = A \| B` (tagged union) | `enum T { A(..), B(..) }` + `match` |
| `class C implements I` | `impl I for C` |
| `try { } catch (e) { }` | `match f() { Ok(v) => .., Err(e) => .. }` |
| `throw new Error("m")` | `return Err("m")` (or `panic("m")` for bugs) |
| `f(x).then(g)` | `x \|> f \|> g` (synchronous pipe) |
| `xs.forEach(x => ..)` | `for x in xs { .. }` |
| `async`/`await`, Promises | `async` exists in compiled code only (not interpreted) |

```windjammer
// expect: [3, 6, 9]
// expect: 12
// expect: x=1 y=2
fn main() {
    let xs = [1, 2, 3]
    let tripled: Vec<int> = xs.map(|x| x * 3).collect()
    println("${tripled:?}")
    println("${xs.fold(6, |a, x| a + x)}")
    let p = (1, 2)
    println("x=${p.0} y=${p.1}")
}
```

## Discriminated unions are enums

```windjammer
// expect: circle area 12
// expect: rect area 6
enum Shape {
    Circle(int),
    Rect(int, int),
}

fn describe(s: Shape) -> string {
    match s {
        Shape::Circle(r) => "circle area ${3 * r * r}",
        Shape::Rect(w, h) => "rect area ${w * h}",
    }
}

fn main() {
    println(describe(Shape::Circle(2)))
    println(describe(Shape::Rect(2, 3)))
}
```

## No null, no exceptions

```windjammer
// expect: none
// expect: found 2
// expect: bad input
fn find(xs: Vec<int>, target: int) -> Option<int> {
    xs.position(|x| *x == target)
}

fn parse(s: string) -> Result<int, string> {
    match s.parse::<int>() {
        Ok(n) => Ok(n),
        Err(_) => Err("bad input"),
    }
}

fn main() {
    match find([5, 6, 7], 9) {
        Some(i) => println("found ${i}"),
        None => println("none"),
    }
    match find([5, 6, 7], 7) {
        Some(i) => println("found ${i}"),
        None => println("none"),
    }
    match parse("x") {
        Ok(n) => println("${n}"),
        Err(e) => println(e),
    }
}
```

## Pipes replace method-chaining on foreign values

```windjammer
// expect: 14
// expect: HI!
fn double(x: int) -> int { x * 2 }
fn add(x: int, y: int) -> int { x + y }

fn main() {
    println("${4 |> double |> add(6)}")
    let s = "hi" |> .to_uppercase()
    println("${s}!")
}
```

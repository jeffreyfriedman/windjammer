# Migrating from TypeScript

If you write TypeScript, much of Windjammer will read familiarly: template strings, closures, `map` and `filter`,
destructuring, a light syntax without semicolons. What changes is that the types are real all the way down, errors are
values, and a program can compile to a native binary.

## Mapping

| TypeScript | Windjammer |
|---|---|
| `` `Hello ${name}` `` | `"Hello ${name}"` |
| `const x = 1`, `let x = 1` | `let x = 1`, `let mut x = 1` |
| `number` | `int` or `float` |
| `string`, `boolean` | `string`, `bool` |
| `T[]`, `Array<T>` | `Vec<T>`, written `[1, 2, 3]` |
| `{ [key: string]: number }`, `Map<K, V>` | `HashMap<string, int>` |
| `interface P { x: number }` | `struct P { x: int }` |
| a type with methods (`class`) | `struct` and `impl` |
| `type A = B \| C` | `enum A { B(...), C(...) }` |
| `x?: number`, `number \| undefined` | `Option<int>` |
| `throw` and `try/catch` | `Result<T, E>`, `?`, `match` |
| `arr.map(x => x * 2)` | `arr.map(\|x\| x * 2).collect()` |
| `arr.filter(...)` | `arr.filter(...).cloned().collect()` |
| `async/await` | tasks and `wait`; see [Concurrency](../tutorial/concurrency.md) |
| `console.log` | `println` |
| `JSON.stringify`, `JSON.parse` | `json.to_string(v)`, `json.parse_string::<T>(text)` |

## Unions become enums

A TypeScript discriminated union becomes an enum, and `switch` becomes an exhaustive `match`:

```wj
// expect: circle 12
// expect: rect 6
enum Shape {
    Circle(int),
    Rect(int, int),
}

fn describe(s: Shape) -> string {
    match s {
        Shape::Circle(r) => "circle ${r * r * 3}",
        Shape::Rect(w, h) => "rect ${w * h}",
    }
}

fn main() {
    println(describe(Shape::Circle(2)))
    println(describe(Shape::Rect(2, 3)))
}
```

Adding a variant later makes every `match` that forgot it a compile error, which is the check TypeScript's `never`
trick approximates.

## `undefined` becomes `Option`

There is no `null` or `undefined`. A value that may be absent has type `Option<T>`, and you cannot use it without saying
what happens when it is `None`:

```wj
// expect: Ada
// expect: nobody
struct User {
    name: string,
    nickname: Option<string>,
}

fn display(u: User) -> string {
    match u.nickname {
        Some(n) => n,
        None => "nobody",
    }
}

fn main() {
    println(display(User { name: "Grace", nickname: Some("Ada") }))
    println(display(User { name: "Linus", nickname: None }))
}
```

## Errors are values

`try/catch` becomes a `Result` that the caller must look at, and `?` plays the part of rethrowing:

```wj
// expect: Ok(42)
// expect: Err("not a number")
fn parse(text: string) -> Result<int, string> {
    text.parse::<int>().map_err(|_| "not a number")
}

fn main() {
    println("${parse("42"):?}")
    println("${parse("forty"):?}")
}
```

## JSON, without a schema library

Any struct converts to and from JSON, and parse errors name the path, line and column of the problem:

```wj
// expect: {"userName":"ada","tags":["a"],"nick":null}
use std::json

struct User {
    @json(rename = "userName")
    name: string,
    tags: Vec<string>,
    nick: Option<string>,
}

fn main() {
    let text = json.to_string(User { name: "ada", tags: ["a"], nick: None }).unwrap_or("")
    println(text)
}
```

## What is new

* **Numbers have sizes.** `int` is a 64-bit integer that checks overflow and `float` is a 64-bit float; see
  [Numbers and sizes](../tutorial/numbers.md).
* **Ownership is inferred**, so large values are passed without copying where the compiler can prove it is safe.
* **Native output.** `wj build --release` produces a binary; `wj build --target javascript` produces JavaScript and
  `--target wasm` WebAssembly. See [Compile targets](../reference/targets.md).

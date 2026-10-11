# Coming from Python

Windjammer looks like Python with braces and types: no semicolons, `${}` interpolation, closures `|x| expr`.
It is statically typed and compiled; `wj run --interpret` gives the instant loop.

## Mapping

| Python | Windjammer |
|--------|------------|
| `print(f"{x} {y:.2f}")` | `println("${x} ${y:.2f}")` |
| `def f(a: int) -> int:` | `fn f(a: int) -> int {` |
| `x = 1` / `x += 1` | `let mut x = 1` / `x += 1` (`let` is immutable) |
| `[1, 2, 3]`, `[]` | `[1, 2, 3]`, `[]` (a `Vec<T>`, one element type) |
| `{"a": 1}` | `HashMap<string, int>` with `m.insert("a", 1)` |
| `for i in range(5)` | `for i in 0..5` (`0..=5` inclusive) |
| `for i, x in enumerate(xs)` | `for (i, x) in xs.enumerate()` |
| `[x * x for x in xs if x > 0]` | `xs.filter(\|x\| *x > 0).map(\|x\| x * x).collect()` |
| `sum(xs)`, `len(xs)`, `max(xs)` | `xs.sum::<int>()`, `xs.len()`, `xs.max()` (returns Option) |
| `", ".join(parts)` | `parts.join(", ")` |
| `s.split()` | `s.split_whitespace()` |
| `int("42")` | `"42".parse::<int>()` (a Result: handle it) |
| `None`, `x is None` | `None`, `x.is_none()` |
| `try / except` | `Result`, `match`, `?` |
| `class P:` with `__init__` | `struct P { .. }` plus `impl P { fn new(..) -> P }` |
| `assert x == 1` | `assert_eq(x, 1)` |
| `d.get(k, 0)` | `m.get(k).unwrap_or(0)` |

```windjammer
// expect: 0 1 4 9 16
// expect: 3 apples, 2.50 each
// expect: a=1 b=2
use std::collections::HashMap

fn main() {
    let mut sq: Vec<string> = []
    for i in 0..5 { sq.push("${i * i}") }
    println(sq.join(" "))
    let n = 3
    let price = 2.5
    println("${n} apples, ${price:.2} each")
    let mut m: HashMap<string, int> = HashMap::new()
    m.insert("b", 2)
    m.insert("a", 1)
    let mut keys: Vec<string> = m.keys().collect()
    keys.sort()
    let mut out: Vec<string> = []
    for k in keys { out.push("${k}=${m[k]}") }
    println(out.join(" "))
}
```

## Differences that bite

Types are checked: `1 + 1.5` is an error, convert with `x as float`. Integer division truncates (`7 / 2` is `3`);
overflow panics. There is no truthiness: conditions are `bool`. Missing values are `Option`, never `None` by accident.

```windjammer
// expect: 3
// expect: 3.5
// expect: false
fn main() {
    println("${7 / 2}")
    println("${7 as float / 2.0}")
    let items: Vec<int> = []
    println("${!items.is_empty()}")
}
```

## Errors instead of exceptions

```windjammer
// expect: parsed 12
// expect: not a number: twelve
fn parse(s: string) -> Result<int, string> {
    match s.parse::<int>() {
        Ok(n) => Ok(n),
        Err(_) => Err("not a number: ${s}"),
    }
}

fn main() {
    for s in ["12", "twelve"] {
        match parse(s) {
            Ok(n) => println("parsed ${n}"),
            Err(e) => println(e),
        }
    }
}
```

## Classes become structs and impls

```windjammer
// expect: Ann owes 25
// expect: Ann owes 40
struct Account { owner: string, owed: int }

impl Account {
    fn new(owner: string) -> Account { Account { owner: owner, owed: 0 } }
    fn charge(self, n: int) { self.owed += n }
    fn show(self) -> string { "${self.owner} owes ${self.owed}" }
}

fn main() {
    let mut a = Account::new("Ann")
    a.charge(25)
    println(a.show())
    a.charge(15)
    println(a.show())
}
```

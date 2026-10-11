# Coming from Rust

Windjammer transpiles to Rust, so semantics are familiar; the surface drops the ceremony. The compiler infers
ownership, borrows, clones, derives and integer widths. Rust-isms are lint errors or deprecated (ADR-0014/0015).

## Mapping

| Rust | Windjammer |
|------|------------|
| `println!("{} {:?}", a, b)` | `println("${a} ${b:?}")` |
| `format!("{}-{}", a, b)` | `"${a}-${b}"` |
| `vec![1, 2, 3]`, `vec![0; n]` | `[1, 2, 3]`, `[0; n]` |
| `#[derive(Debug, Clone)]` | nothing (inferred) |
| `fn f(&self)`, `&mut self`, `&x` args | `fn f(self)`; mutation inferred; no `&` in calls |
| `v.iter().map(..).collect()` | `v.map(..).collect()` |
| `x.unwrap()`, `x.expect("m")` | `match`, `?`, `unwrap_or(d)` (W0002) |
| `s.to_string()` on a literal, `String::from("a")` | `"a"` |
| `i32`, `u64`, `usize`, `f64` | `int`, `int`, `int`, `float` (explicit widths only at boundaries) |
| `let x: i64 = y as i64` | `let x = y` (checked conversions per ADR-0015) |
| `return x;` at the end | last expression |
| `impl Display for T` | `fn to_string(self) -> string` (ADR-0015 #3; call `.to_string()` explicitly today) |
| `Rc<RefCell<T>>`, threads, async | not in the interpreter; see `docs/adr/0004` for the concurrency surface |

```windjammer
// expect: 6
// expect: Some(4)
// expect: [2, 4]
fn main() {
    let v = [1, 2, 3]
    println("${v.sum::<int>()}")
    let first_even = v.map(|x| x * 2).find(|x| *x > 3)
    println("${first_even:?}")
    let evens: Vec<int> = v.filter(|x| *x < 3).map(|x| x * 2).collect()
    println("${evens:?}")
}
```

## Ownership is inferred

You never write a lifetime. A function that only reads a parameter borrows it; one that mutates it takes `&mut`;
one that stores it takes ownership. Callers keep using their values.

```windjammer
// expect: 3 3
// expect: 9 [1, 2, 3, 9]
fn total(xs: Vec<int>) -> int {
    xs.sum::<int>()
}

fn add_one_more(xs: Vec<int>) {
    xs.push(9)
}

fn main() {
    let mut v = [1, 2]
    v.push(3)
    println("${v.len()} ${total(v) - 3}")
    add_one_more(v)
    println("${v[3]} ${v:?}")
}
```

## Errors, traits, modules

`Result`, `Option`, `?`, `match`, `impl Trait for Type`, `use a::b` and `mod` work as in Rust.

```windjammer
// expect: Err("neg")
// expect: Ok(4)
// expect: Hi Bo
trait Greet {
    fn hi(self) -> string
}

struct P { n: string }

impl Greet for P {
    fn hi(self) -> string { "Hi ${self.n}" }
}

fn root(x: int) -> Result<int, string> {
    if x < 0 { return Err("neg") }
    Ok(x)
}

fn main() {
    println("${root(-1):?}")
    println("${root(4):?}")
    println(P { n: "Bo" }.hi())
}
```

## Escape hatch

`extern fn` and `rust { }` blocks (ADR-0012) are the only places Rust syntax is expected; everything else is linted.

```windjammer,bad
fn main() {
    let v = vec![1, 2, 3]
    let s: i32 = v.iter().sum()
    println!("{}", s)
}
```

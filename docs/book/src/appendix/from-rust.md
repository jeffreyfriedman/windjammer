# Migrating from Rust

Windjammer compiles to Rust, so the semantics you know carry over: ownership, enums, traits, `Result`, `?`, modules,
Cargo. What changes is how much of it you write. The compiler infers ownership, borrows, clones, derives and, for the
ordinary cases, integer widths.

## Mapping

| Rust | Windjammer |
|---|---|
| `println!("{} {:?}", a, b)` | `println("${a} ${b:?}")` |
| `format!("{}-{}", a, b)` | `"${a}-${b}"` |
| `vec![1, 2, 3]`, `vec![0; n]` | `[1, 2, 3]`, `[0; n]` |
| `#[derive(Debug, Clone)]` | nothing; derives are inferred |
| `fn f(&self)`, `fn f(&mut self)` | `fn f(self)`; the compiler infers the borrow |
| `f(&x)`, `f(&mut x)` | `f(x)` |
| `v.iter().map(..).collect()` | `v.map(..).collect()` |
| `x.unwrap()`, `x.expect("m")` | `match`, `?`, `unwrap_or(default)` |
| `"lit".to_string()`, `String::from("lit")` | `"lit"` |
| `String`, `&str` | `string` |
| `i64`, `f64` | `int`, `float` |
| `i32`, `u8`, `usize`, ... | the same names, used where the size matters |
| `x as usize` for indexing | `v[i]` with an `int` |
| a trailing `return x;` | the last expression |
| `Rc<RefCell<T>>` | not needed for most programs; see [Concurrency](../tutorial/concurrency.md) |
| `unsafe`, raw pointers, lifetimes | none; `extern fn` is the boundary |

## The same program, both ways

A function that sums a list and one that pushes onto it:

```wj
// expect: 6
// expect: 4 [1, 2, 3, 9]
fn total(xs: Vec<int>) -> int {
    xs.sum::<int>()
}

fn push_nine(xs: Vec<int>) {
    xs.push(9)
}

fn main() {
    let mut v = [1, 2, 3]
    println("${total(v)}")
    push_nine(v)
    println("${v.len()} ${v:?}")
}
```

In Rust that is `fn total(xs: &[i64]) -> i64`, `fn push_nine(xs: &mut Vec<i64>)`, and `total(&v)`, `push_nine(&mut v)`
at the call sites. In Windjammer the compiler works out `&` for `total` and `&mut` for `push_nine`, and `wj build
--no-cargo` writes out exactly that Rust if you want to compare.

## What carries over unchanged

`Option`, `Result`, `?`, `match`, `impl Trait for Type`, generics with bounds, `use a::b` and `mod`:

```wj
// expect: Err("neg")
// expect: Ok(4)
// expect: Hi Bo
trait Greet {
    fn hi(&self) -> string
}

struct Person {
    name: string,
}

impl Greet for Person {
    fn hi(self) -> string {
        "Hi ${self.name}"
    }
}

fn root(x: int) -> Result<int, string> {
    if x < 0 {
        return Err("neg")
    }
    Ok(x)
}

fn main() {
    println("${root(-1):?}")
    println("${root(4):?}")
    println(Person { name: "Bo" }.hi())
}
```

Note that a trait method declaration states its receiver (`&self`), because it has no body from which to infer it.

## What is different on purpose

* **No `&`, no lifetimes.** If you write a reference in a signature it is accepted as a hint, and the linter nudges you
  to drop it (`W0001`, `W0004`). Read the compiler's decision with `wj explain-ownership`.
* **No macros.** `println`, `assert_eq`, `panic` are functions, so there is no `println!` to forget the bang on.
* **Explicit `.clone()` is rarely needed.** The compiler clones where value semantics require it. `W0005` points out the
  ones you wrote by hand.
* **`.unwrap()` is a lint error** (`W0002`): handle the case or propagate it.
* **Integer literals are `int` by default**, 64-bit and overflow-checked. Use sized types at boundaries, and convert with
  `as`.
* **Attributes start with `@`**, not `#[`: `@test`, `@pure`, `@json(rename = "x")`.

`wj lint` finds the Rust habits in a file, and `wj fix` rewrites the mechanical ones (`println!`, `vec![]`, `format!`).

## Bringing Rust code along

Crates are ordinary dependencies in `wj.toml`, and your own Rust files are reached with `extern fn`. See
[Rust interop](../tutorial/rust-interop.md). Everything compiles to a Cargo project, and `wj eject` hands you that project
as plain Rust whenever you want it.

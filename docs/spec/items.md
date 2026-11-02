# Items

Top-level and module-level declarations.

Part of the [Windjammer language specification](README.md). Rule IDs are checked by `scripts/wj/spec_check.wj`; each rule's executable example is the code block below it, stored in `tests/spec/items/`.

## Functions

**SPEC-ITEM-001** `fn name(p: T, ...) -> R { body }` declares a function; parameter types are mandatory; the return type defaults to `()`. Functions may be declared in any order within a module.

<!-- example: tests/spec/items/SPEC-ITEM-001.wj -->
```windjammer
fn main() {
    println("${later(2)}")
}
fn later(x: int) -> int { helper(x) * 2 }
fn helper(x: int) -> int { x + 1 }
```
Output:
```text
6
```

## Structs

**SPEC-ITEM-002** `struct S { f: T, ... }` declares a record type with named fields; `pub` on the struct or a field exports it. Struct values are built with `S { f: e }` and compared with derived equality only when `@derive(PartialEq)` is present.

<!-- example: tests/spec/items/SPEC-ITEM-002.wj -->
```windjammer
@derive(PartialEq, Clone)
pub struct P { pub x: int, y: int }
fn main() {
    let a = P { x: 1, y: 2 }
    let b = a.clone()
    println("${a == b}")
}
```
Output:
```text
true
```

## Enums

**SPEC-ITEM-003** `enum E { A, B(T, U), C { f: T } }` declares a sum type; values are `E::A`, `E::B(1, 2)`, `E::C { f: 1 }`. Variants are scoped to the enum name.

<!-- example: tests/spec/items/SPEC-ITEM-003.wj -->
```windjammer
enum Op { Add, Neg(int), Pair(int, int), Named { id: int } }
fn show(o: Op) -> string {
    match o {
        Op::Add => "add",
        Op::Neg(n) => "neg ${n}",
        Op::Pair(a, b) => "pair ${a} ${b}",
        Op::Named { id } => "named ${id}",
    }
}
fn main() {
    println("${show(Op::Add)} ${show(Op::Neg(1))} ${show(Op::Pair(2, 3))} ${show(Op::Named { id: 4 })}")
}
```
Output:
```text
add neg 1 pair 2 3 named 4
```

## impl blocks and associated functions

**SPEC-ITEM-004** `impl T { fn f(self, ..) {..} fn g(..) {..} }` adds methods (first parameter `self`) and associated functions (no `self`, called `T::g(..)`). `Self` names the implementing type. Receiver mode (read, mutate, consume) is inferred from the body.

<!-- example: tests/spec/items/SPEC-ITEM-004.wj -->
```windjammer
struct Counter { n: int }
impl Counter {
    fn new() -> Self { Counter { n: 0 } }
    fn bump(self) { self.n += 1 }
    fn get(self) -> int { self.n }
}
fn main() {
    let mut c = Counter::new()
    c.bump()
    c.bump()
    println("${c.get()}")
}
```
Output:
```text
2
```

## Constants and statics

**SPEC-ITEM-005** `const NAME: T = expr` is a compile-time constant (type annotation required); `static NAME: T = expr` is a program-wide value; `static mut` is mutable and its use is outside the safe subset.

<!-- example: tests/spec/items/SPEC-ITEM-005.wj -->
```windjammer
const LIMIT: int = 3 * 4
const GREETING: string = "hi"
fn main() {
    println("${LIMIT} ${GREETING} ${LIMIT + 1}")
}
```
Output:
```text
12 hi 13
```

## Type aliases

**SPEC-ITEM-006** `type Name = T` introduces a synonym for a type.

<!-- example: tests/spec/items/SPEC-ITEM-006.wj -->
```windjammer
type Id = int
type Pair = (int, int)
fn swap(p: Pair) -> Pair { (p.1, p.0) }
fn main() {
    let a: Id = 5
    let s = swap((1, 2))
    println("${a} ${s.0}${s.1}")
}
```
Output:
```text
5 21
```

## Generic type aliases

**SPEC-ITEM-011** [OPEN Q-010] `type Name<T> = ...` introduces a generic synonym. Today the item parser accepts only `type Name = T`.

<!-- example: tests/spec/items/SPEC-ITEM-011.wj -->
<!-- ignore: known divergence Q-010; the example shows the specified behaviour, which the implementation does not have yet -->
```windjammer,ignore
type Pair<T> = (T, T)
fn swap(p: Pair<int>) -> Pair<int> { (p.1, p.0) }
fn main() {
    let s = swap((1, 2))
    println("${s.0}${s.1}")
}
```
Output:
```text
21
```
> **Known divergence:** the implementation does not do this yet; see [Q-010](OPEN_QUESTIONS.md). The example is marked `xfail`.

## Entry point

**SPEC-ITEM-007** Execution starts at `fn main()` in the entry module. Output written by `print`/`println` goes to standard output; the process exits 0 when `main` returns, 101 on panic, or N after `std::process::exit(N)`.

<!-- example: tests/spec/items/SPEC-ITEM-007.wj -->
```windjammer
fn main() {
    println("start")
    println("end")
}
```
Output:
```text
start
end
```

## Default field values

**SPEC-ITEM-008** [OPEN Q-011] `struct S { f: T = expr }` gives a field a default used when a struct literal omits it. The old grammar listed this; the struct parser rejects it.

<!-- example: tests/spec/items/SPEC-ITEM-008.wj -->
<!-- ignore: known divergence Q-011; the example shows the specified behaviour, which the implementation does not have yet -->
```windjammer,ignore
struct Cfg { w: int = 80, h: int = 24, name: string }
fn main() {
    let c = Cfg { name: "x" }
    println("${c.w} ${c.h} ${c.name}")
}
```
Output:
```text
80 24 x
```
> **Known divergence:** the implementation does not do this yet; see [Q-011](OPEN_QUESTIONS.md). The example is marked `xfail`.

## Functions as values

**SPEC-ITEM-009** A function name is a value of function type `fn(A, B) -> R` and may be stored, passed and returned.

<!-- example: tests/spec/items/SPEC-ITEM-009.wj -->
```windjammer
fn inc(x: int) -> int { x + 1 }
fn dbl(x: int) -> int { x * 2 }
fn pick(flag: bool) -> fn(int) -> int { if flag { inc } else { dbl } }
fn main() {
    let f = pick(true)
    let g = pick(false)
    println("${f(10)} ${g(10)}")
}
```
Output:
```text
11 20
```

## Generic functions and structs

**SPEC-ITEM-010** `fn f<T>(x: T) -> T`, `struct S<T> { .. }` and `impl<T> S<T> { .. }` are parameterised over types; type arguments are inferred at use.

<!-- example: tests/spec/items/SPEC-ITEM-010.wj -->
```windjammer
struct Stack<T> { items: Vec<T> }
impl<T> Stack<T> {
    fn new() -> Stack<T> { Stack { items: Vec::new() } }
    fn push(self, x: T) { self.items.push(x) }
    fn len(self) -> int { self.items.len() }
}
fn first<T>(v: Vec<T>) -> T { v[0] }
fn main() {
    let mut s = Stack::new()
    s.push(1)
    s.push(2)
    println("${s.len()} ${first(["x", "y"])}")
}
```
Output:
```text
2 x
```


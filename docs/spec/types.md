# Types

Static types of Windjammer values. Local variables are inferred; item signatures are explicit.

Part of the [Windjammer language specification](README.md). Rule IDs are checked by `scripts/spec_check.sh`; each rule's executable example is the code block below it, stored in `tests/spec/types/`.

## Primitive types

**SPEC-TYP-001** The primitives are `int` (i64), `float` (f64), `bool`, `string`, `char` and the unit type `()`. `uint` is the unsigned counterpart of `int`.

<!-- example: tests/spec/types/SPEC-TYP-001.wj -->
```windjammer
fn main() {
    let i: int = 5
    let f: float = 2.5
    let b: bool = true
    let s: string = "s"
    let c: char = 'c'
    println("${i} ${f} ${b} ${s} ${c}")
}
```
Output:
```text
5 2.5 true s c
```

## Fixed-width types

**SPEC-TYP-002** `i8 i16 i32 i64 u8 u16 u32 u64 usize isize f32 f64` are available and are never implicitly converted to or from `int`/`float` or each other (ADR-0015 section 1).

<!-- example: tests/spec/types/SPEC-TYP-002.wj -->
```windjammer
fn main() {
    let a: u8 = 200
    let b: i16 = -300
    let c: f32 = 1.5
    let d: u64 = 5000000000
    println("${a} ${b} ${c} ${d}")
}
```
Output:
```text
200 -300 1.5 5000000000
```

## No implicit numeric conversion

**SPEC-TYP-003** Combining different numeric types in one operation, or passing one where another is declared, is a compile-time error; use `as` (NUM-010). [OPEN Q-017] The interpreter and the Rust backend's literal inference accept some mixes silently.

<!-- example: tests/spec/types/SPEC-TYP-003.wj -->
<!-- ignore: known divergence Q-017; the example shows the specified behaviour, which the implementation does not have yet -->
```windjammer,ignore
fn main() {
    let a: i32 = 1
    let b: i64 = 2
    let c = a + b
    println("${c}")
}
```
Rejected at compile time.
> **Known divergence:** the implementation does not do this yet; see [Q-017](OPEN_QUESTIONS.md). The example is marked `xfail`.

## Tuples and unit

**SPEC-TYP-004** `(T1, T2, ...)` is a tuple type; `()` is the empty tuple, the type of statements and of functions without a return type.

<!-- example: tests/spec/types/SPEC-TYP-004.wj -->
```windjammer
fn pair() -> (int, string) { (1, "a") }
fn nothing() {}
fn main() {
    let (n, s) = pair()
    nothing()
    println("${n}${s}")
}
```
Output:
```text
1a
```

## Lists, arrays

**SPEC-TYP-005** `Vec<T>` (also written `[T]` in annotations) is the growable list; `[T; N]` is a fixed-size array. Both index with `int`.

<!-- example: tests/spec/types/SPEC-TYP-005.wj -->
```windjammer
fn sum(v: Vec<int>) -> int {
    let mut t = 0
    for x in v { t += x }
    t
}
fn main() {
    let a: [int; 3] = [1, 2, 3]
    let v: Vec<int> = [4, 5]
    println("${a[2]} ${sum(v)} ${a.len()}")
}
```
Output:
```text
3 9 3
```

## Generic types

**SPEC-TYP-006** `Name<T, U>` applies type arguments; `Option<T>`, `Result<T, E>`, `Vec<T>`, `HashMap<K, V>`, `Box<T>` are built in. Type arguments are inferred when omitted at a use site.

<!-- example: tests/spec/types/SPEC-TYP-006.wj -->
```windjammer
fn wrap(x: int) -> Option<Box<int>> { Some(Box::new(x)) }
fn main() {
    let o = wrap(4)
    match o { Some(b) => println("${*b}"), None => println("none") }
    let r: Result<Vec<int>, string> = Ok([1])
    println("${r.is_ok()}")
}
```
Output:
```text
4
true
```

## Function types

**SPEC-TYP-007** `fn(A, B) -> R` is the type of function values and closures that take `A`, `B` and return `R`.

<!-- example: tests/spec/types/SPEC-TYP-007.wj -->
```windjammer
fn twice(f: fn(int) -> int, x: int) -> int { f(f(x)) }
fn main() {
    let inc = |n| n + 1
    println("${twice(inc, 5)}")
}
```
Output:
```text
7
```

## Local inference

**SPEC-TYP-008** Local `let` types are inferred from the initializer and later uses; function parameter and return types must be written. An integer literal takes the type its context demands, otherwise `int`; a float literal otherwise `float`.

<!-- example: tests/spec/types/SPEC-TYP-008.wj -->
```windjammer
fn takes_u8(x: u8) -> u8 { x }
fn main() {
    let a = 200
    let b = takes_u8(100)
    let c = 1.5
    println("${a} ${b} ${c}")
}
```
Output:
```text
200 100 1.5
```

## Missing parameter type

**SPEC-TYP-009** A function parameter without a type annotation is a compile-time error.

<!-- example: tests/spec/types/SPEC-TYP-009.wj -->
<!-- should_fail: the rule says this program is rejected -->
```windjammer,should_fail
fn f(x) -> int { 1 }
fn main() {
    println("${f(1)}")
}
```
Rejected at compile time.

## Recursive types

**SPEC-TYP-010** A type that contains itself must go through `Box<T>` (or a list); `Box::new(v)` allocates and `*b` reads the contents.

<!-- example: tests/spec/types/SPEC-TYP-010.wj -->
```windjammer
enum List { Nil, Cons(int, Box<List>) }
fn sum(l: List) -> int {
    match l {
        List::Nil => 0,
        List::Cons(v, rest) => v + sum(*rest),
    }
}
fn main() {
    let l = List::Cons(1, Box::new(List::Cons(2, Box::new(List::Nil))))
    println("${sum(l)}")
}
```
Output:
```text
3
```


# Ownership and mutability (surface rules)

What a programmer can observe and must write. How the compiler *infers* moves, borrows and clones is out of scope here: see [inference.md](inference.md) (placeholder owned by the inference stream). The binding principle is value semantics: the program behaves as if every assignment, argument pass and return copies.

Part of the [Windjammer language specification](README.md). Rule IDs are checked by `scripts/spec_check.sh`; each rule's executable example is the code block below it, stored in `tests/spec/ownership/`.

## Value semantics

**SPEC-OWN-001** Assigning, passing or returning a value gives the receiver an independent copy as far as the program can observe: mutating one never changes the other. The compiler decides whether that is a move, a borrow or a clone.

<!-- example: tests/spec/ownership/SPEC-OWN-001.wj -->
```windjammer
struct P { x: int }
fn bump(p: P) -> P {
    let mut q = p
    q.x += 1
    q
}
fn main() {
    let a = P { x: 1 }
    let mut b = a
    b.x = 50
    let c = bump(a)
    println("${a.x} ${b.x} ${c.x}")
}
```
Output:
```text
1 50 2
```

## Aliasing through lists

**SPEC-OWN-002** Elements stored in a list or field are independent of the variable they were copied from: later mutation of either side is not seen by the other.

<!-- example: tests/spec/ownership/SPEC-OWN-002.wj -->
```windjammer
struct P { x: int }
fn main() {
    let mut p = P { x: 1 }
    let mut v = [p]
    p.x = 9
    v[0].x = 5
    println("${p.x} ${v[0].x}")
}
```
Output:
```text
9 5
```

## Explicit clone

**SPEC-OWN-003** `.clone()` produces a deep, independent copy of a value; it is always available and never changes behaviour compared with an inferred copy.

<!-- example: tests/spec/ownership/SPEC-OWN-003.wj -->
```windjammer
fn main() {
    let a = [1, 2, 3]
    let mut b = a.clone()
    b.push(4)
    println("${a.len()} ${b.len()}")
}
```
Output:
```text
3 4
```

## Mutation through parameters

**SPEC-OWN-004** A function that mutates a parameter in its body mutates the caller's value (the compiler infers `&mut`); no `&`/`&mut` is written at the call site or in the parameter type.

<!-- example: tests/spec/ownership/SPEC-OWN-004.wj -->
```windjammer
fn fill(v: Vec<int>, n: int) {
    let mut i = 0
    while i < n {
        v.push(i)
        i += 1
    }
}
fn main() {
    let mut v = [100]
    fill(v, 3)
    println("${v.len()} ${v[3]}")
}
```
Output:
```text
4 2
```

## Mutating methods

**SPEC-OWN-005** A method that assigns to `self`'s fields or calls a mutating method on them is treated as `&mut self`; the receiver variable must be `let mut` at the call site.

<!-- example: tests/spec/ownership/SPEC-OWN-005.wj -->
```windjammer
struct Acc { total: int }
impl Acc {
    fn add(self, n: int) { self.total += n }
    fn total(self) -> int { self.total }
}
fn main() {
    let mut a = Acc { total: 0 }
    a.add(2)
    a.add(5)
    println("${a.total()}")
}
```
Output:
```text
7
```

## Builder / consuming methods

**SPEC-OWN-006** A method that returns `self` (or a value built from it) consumes the receiver; chained calls thread ownership through.

<!-- example: tests/spec/ownership/SPEC-OWN-006.wj -->
```windjammer
struct B { n: int, tag: string }
impl B {
    fn new() -> B { B { n: 0, tag: "" } }
    fn n(self, v: int) -> B { B { n: v, tag: self.tag } }
    fn tag(self, t: string) -> B { B { n: self.n, tag: t } }
}
fn main() {
    let b = B::new().n(3).tag("x")
    println("${b.n} ${b.tag}")
}
```
Output:
```text
3 x
```

## No explicit references required

**SPEC-OWN-007** Programs never need to write `&`, `&mut` or lifetimes on `.wj` signatures; where they appear they are accepted and treated as hints. Reading a field of a borrowed value does not move it.

<!-- example: tests/spec/ownership/SPEC-OWN-007.wj -->
```windjammer
struct P { name: string, age: int }
fn show(p: P) -> string { "${p.name} ${p.age}" }
fn main() {
    let p = P { name: "ann", age: 3 }
    println(show(p))
    println(show(p))
}
```
Output:
```text
ann 3
ann 3
```

## Immutable parameters

**SPEC-OWN-008** Parameters are immutable inside the callee unless the body mutates them (which makes the parameter a mutable borrow, OWN-004); `mut` on a parameter makes the callee's own copy mutable without affecting the caller. [OPEN Q-015] the interpreter lets the callee's `mut` parameter write through to the caller.

<!-- example: tests/spec/ownership/SPEC-OWN-008.wj -->
<!-- ignore: known divergence Q-015; the example shows the specified behaviour, which the implementation does not have yet -->
```windjammer,ignore
fn f(mut n: int) -> int {
    n += 10
    n
}
fn main() {
    let a = 1
    println("${f(a)} ${a}")
}
```
Output:
```text
11 1
```
> **Known divergence:** the implementation does not do this yet; see [Q-015](OPEN_QUESTIONS.md). The example is marked `xfail`.

## Inference specification

**SPEC-OWN-009** The rules by which the compiler infers move, borrow, mutable borrow and clone are specified separately in docs/spec/inference.md (placeholder; owned by the ownership-inference stream). This chapter specifies only the observable surface. (untestable: placeholder cross-reference)


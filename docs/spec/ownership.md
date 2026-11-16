# Ownership and mutability (surface rules)

What a programmer can observe and must write. How the compiler *infers* moves, borrows and clones is out of scope here: see [inference.md](inference.md) (placeholder owned by the inference stream). The binding principle is value semantics: the program behaves as if every assignment, argument pass and return copies.

Part of the [Windjammer language specification](README.md). Rule IDs are checked by `scripts/wj/spec_check.wj`; each rule's executable example is the code block below it, stored in `tests/spec/ownership/`.

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

**SPEC-OWN-008** Parameters are immutable inside the callee unless the body mutates them (which makes the parameter a mutable borrow, OWN-004); `mut` on a parameter makes the callee's own copy mutable without affecting the caller.

<!-- example: tests/spec/ownership/SPEC-OWN-008.wj -->
```windjammer
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
The same holds for a struct: `fn bump(mut s: S)` changes its own copy and the caller's `s` is unchanged (compiled and interpreted: tests/drift_corpus/moves/mut_param_is_own_copy.wj).

## Inference specification

**SPEC-OWN-009** The rules by which the compiler infers move, borrow, mutable borrow and clone are specified separately in docs/spec/inference.md (placeholder; owned by the ownership-inference stream). This chapter specifies only the observable surface. (untestable: placeholder cross-reference)


## Drop with moves, loops, Option::take and collections

**SPEC-OWN-012** A value passed through a function is dropped where its final owner ends; each loop iteration drops its locals; `Option::take` moves the value out, `pop` returns it (so it is dropped at the end of the statement when unused) and `clear` drops every element; a value moved into a closure is dropped when the closure is.

<!-- example: tests/spec/ownership/SPEC-OWN-012.wj -->
```windjammer
struct G {
    name: string,
}

impl Drop for G {
    fn drop(self) {
        println("drop ${self.name}")
    }
}

fn pass_through(g: G) -> G {
    println("pass ${g.name}")
    g
}

fn main() {
    let g = pass_through(G { name: "p" })
    println("got ${g.name}")
    for i in 0..2 {
        let it = G { name: "loop${i}" }
        println("body ${it.name}")
    }
    let mut slot: Option<G> = Some(G { name: "slot" })
    let taken = slot.take()
    println("taken ${taken.is_some()} left ${slot.is_none()}")
    let mut v = vec![G { name: "a" }, G { name: "b" }, G { name: "c" }]
    v.pop()
    println("popped")
    v.clear()
    println("cleared")
    let f = move || println("closure ${g.name}")
    f()
    println("end")
}
```
Output:
```text
pass p
got p
body loop0
drop loop0
body loop1
drop loop1
taken true left true
drop c
popped
drop a
drop b
cleared
closure p
end
drop slot
drop p
```

## Drop order

**SPEC-OWN-010** Locals are dropped in reverse declaration order at the end of their block, a shadowed binding is still dropped at the end of its block, a temporary passed to a function is dropped when the callee returns, `let _ = e` drops at once, `let _name = e` at the end of the block, and `drop(x)` drops immediately.

<!-- example: tests/spec/ownership/SPEC-OWN-010.wj -->
```windjammer
struct G {
    name: string,
}

impl Drop for G {
    fn drop(self) {
        println("drop ${self.name}")
    }
}

fn make(name: string) -> G {
    G { name: name }
}

fn consume(g: G) {
    println("consume ${g.name}")
}

fn main() {
    let a = make("a")
    let b = make("b")
    {
        let c = make("c")
        println("inner")
    }
    consume(make("t"))
    let a = make("a2")
    let _ = make("ignored")
    let _keep = make("kept")
    drop(b)
    println("end")
}
```
Output:
```text
inner
drop c
consume t
drop t
drop ignored
drop b
end
drop kept
drop a2
drop a
```

## Drop on assignment, fields and early return

**SPEC-OWN-011** Assigning to a variable drops the old value at once; struct fields drop in declaration order and collection elements in index order after the container; an early `return` drops the locals of the function.

<!-- example: tests/spec/ownership/SPEC-OWN-011.wj -->
```windjammer
struct G {
    name: string,
}

impl Drop for G {
    fn drop(self) {
        println("drop ${self.name}")
    }
}

struct Pair {
    first: G,
    second: G,
}

fn early(flag: bool) -> int {
    let _g = G { name: "early" }
    if flag {
        return 1
    }
    println("not early")
    2
}

fn main() {
    let mut slot = G { name: "old" }
    slot = G { name: "new" }
    println("assigned")
    let p = Pair { first: G { name: "f" }, second: G { name: "s" } }
    println("pair built")
    let v = vec![G { name: "v0" }, G { name: "v1" }]
    println("${v.len()} ${early(true)} ${early(false)}")
    let boxed = Some(G { name: "opt" })
    println("end")
}
```
Output:
```text
drop old
assigned
pair built
drop early
not early
drop early
2 1 2
end
drop opt
drop v0
drop v1
drop f
drop s
drop new
```

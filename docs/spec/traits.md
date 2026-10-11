# Traits and generics

Shared behaviour and parametric polymorphism. Generic code is monomorphised by the compiler backends and dispatched dynamically by the interpreter; the observable behaviour is the same.

Part of the [Windjammer language specification](README.md). Rule IDs are checked by `scripts/spec_check.sh`; each rule's executable example is the code block below it, stored in `tests/spec/traits/`.

## Traits and impls

**SPEC-TRT-001** `trait T { fn m(self) -> R }` declares required methods; `impl T for S { .. }` provides them. A trait method may have a default body, used when the impl omits it.

<!-- example: tests/spec/traits/SPEC-TRT-001.wj -->
```windjammer
trait Shape {
    fn area(self) -> float
    fn name(self) -> string { "shape" }
}
struct Sq { s: float }
struct Ci { r: float }
impl Shape for Sq {
    fn area(self) -> float { self.s * self.s }
}
impl Shape for Ci {
    fn area(self) -> float { 3.0 * self.r * self.r }
    fn name(self) -> string { "circle" }
}
fn main() {
    let a = Sq { s: 2.0 }
    let b = Ci { r: 1.0 }
    println("${a.name()} ${a.area()} ${b.name()} ${b.area()}")
}
```
Output:
```text
shape 4 circle 3
```

## Trait bounds

**SPEC-TRT-002** `fn f<T: Trait>(x: T)` accepts any `T` implementing `Trait`; several bounds combine with `+`, and `where T: A + B` is equivalent.

<!-- example: tests/spec/traits/SPEC-TRT-002.wj -->
```windjammer
trait Named { fn name(self) -> string }
trait Aged { fn age(self) -> int }
struct P { n: string, a: int }
impl Named for P { fn name(self) -> string { self.n } }
impl Aged for P { fn age(self) -> int { self.a } }
fn describe<T: Named + Aged>(x: T) -> string { "${x.name()}:${x.age()}" }
fn describe2<T>(x: T) -> string where T: Named + Aged { "${x.name()}/${x.age()}" }
fn main() {
    println("${describe(P { n: "ann", a: 30 })} ${describe2(P { n: "bo", a: 4 })}")
}
```
Output:
```text
ann:30 bo/4
```

## Dynamic dispatch

**SPEC-TRT-003** A value used through a trait (`Box<dyn Trait>`, `&dyn Trait`) dispatches on its runtime type.

<!-- example: tests/spec/traits/SPEC-TRT-003.wj -->
```windjammer
trait Animal { fn speak(self) -> string }
struct Dog {}
struct Cat {}
impl Animal for Dog { fn speak(self) -> string { "woof" } }
impl Animal for Cat { fn speak(self) -> string { "meow" } }
fn main() {
    let zoo: Vec<Box<dyn Animal>> = [Box::new(Dog {}), Box::new(Cat {})]
    let mut out = ""
    for a in zoo { out = out + a.speak() + " " }
    println(out)
}
```
Output:
```text
woof meow 
```

## Operator traits

**SPEC-TRT-004** Defining a method with an operator trait's name overloads that operator: `add sub mul div neg eq partial_cmp` (`+ - * / unary- == <`).

<!-- example: tests/spec/traits/SPEC-TRT-004.wj -->
```windjammer
struct V { x: int, y: int }
impl V {
    fn add(self, o: V) -> V { V { x: self.x + o.x, y: self.y + o.y } }
    fn eq(self, o: V) -> bool { self.x == o.x && self.y == o.y }
}
fn main() {
    let a = V { x: 1, y: 2 }
    let b = V { x: 10, y: 20 }
    let c = a + b
    println("${c.x} ${c.y} ${c == V { x: 11, y: 22 }}")
}
```
Output:
```text
11 22 true
```

## Derives

**SPEC-TRT-005** `@derive(Debug, Clone, PartialEq, Eq, Hash, Default, PartialOrd, Ord, Copy)` generates the standard implementations. Types whose fields are all `Copy` are `Copy` without a derive.

<!-- example: tests/spec/traits/SPEC-TRT-005.wj -->
```windjammer
@derive(Debug, Clone, PartialEq)
struct P { x: int, s: string }
fn main() {
    let a = P { x: 1, s: "a" }
    let b = a.clone()
    println("${a == b} ${a:?}")
}
```
Output:
```text
true P { x: 1, s: "a" }
```

## Trait static dispatch of associated functions

**SPEC-TRT-006** `T::new()` where `T` is a type parameter calls the associated function of the concrete type chosen at the use site.

<!-- example: tests/spec/traits/SPEC-TRT-006.wj -->
```windjammer
trait Make { fn make() -> Self }
struct A { v: int }
impl Make for A { fn make() -> A { A { v: 7 } } }
fn build<T: Make>() -> T { T::make() }
fn main() {
    let a: A = build()
    println("${a.v}")
}
```
Output:
```text
7
```

## Generic trait methods

**SPEC-TRT-007** [OPEN Q-025] A trait method may declare its own type parameters (`fn map<U>(self, f: fn(T) -> U)`). The trait parser rejects `<` after a method name in a trait body.

<!-- example: tests/spec/traits/SPEC-TRT-007.wj -->
```windjammer
trait Conv { fn conv<T>(self, x: T) -> T }
struct S {}
impl Conv for S { fn conv<T>(self, x: T) -> T { x } }
fn main() {
    let s = S {}
    println("${s.conv(4)}")
}
```
Output:
```text
4
```
> **Known divergence:** the implementation does not do this yet; see [Q-025](OPEN_QUESTIONS.md). The example is marked `xfail`.

## Bounds with type arguments

**SPEC-TRT-008** [OPEN Q-024] A bound may carry type arguments (`T: Into<string>`, `trait A: B<int>`) and a generic trait parameter may be bounded. Today bounds are plain trait names only.

<!-- example: tests/spec/traits/SPEC-TRT-008.wj -->
```windjammer
fn show<T: Into<string>>(x: T) -> string { x.into() }
fn main() {
    println(show("hi"))
}
```
Output:
```text
hi
```
> **Known divergence:** the implementation does not do this yet; see [Q-024](OPEN_QUESTIONS.md). The example is marked `xfail`.

## Supertraits

**SPEC-TRT-009** `trait B: A { .. }` requires every implementor of `B` to implement `A`; `A`'s methods are callable on values bounded by `B`.

<!-- example: tests/spec/traits/SPEC-TRT-009.wj -->
```windjammer
trait Named { fn name(self) -> string }
trait Greeter: Named { fn greet(self) -> string { "hello ${self.name()}" } }
struct P {}
impl Named for P { fn name(self) -> string { "p" } }
impl Greeter for P {}
fn main() {
    println(P {}.greet())
}
```
Output:
```text
hello p
```


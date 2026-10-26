# Modules and imports

Namespaces and name resolution.

Part of the [Windjammer language specification](README.md). Rule IDs are checked by `scripts/wj/spec_check.wj`; each rule's executable example is the code block below it, stored in `tests/spec/modules/`.

## Inline modules

**SPEC-MOD-001** `mod name { items }` declares a namespace; items are private to it unless marked `pub`. Names are reached with `name::item`.

<!-- example: tests/spec/modules/SPEC-MOD-001.wj -->
```windjammer
mod geo {
    pub fn area(w: int, h: int) -> int { w * h }
    pub mod inner {
        pub fn one() -> int { 1 }
    }
}
fn main() {
    println("${geo::area(2, 3)} ${geo::inner::one()}")
}
```
Output:
```text
6 1
```

## use declarations

**SPEC-MOD-002** `use a::b::c` brings `c` into scope under its own name; `use a::b::{c, d}` imports several; `use a::b::c as e` renames.

<!-- example: tests/spec/modules/SPEC-MOD-002.wj -->
```windjammer
mod m {
    pub fn f() -> int { 1 }
    pub fn g() -> int { 2 }
    pub fn h() -> int { 3 }
}
use m::f
use m::{g, h}
fn main() {
    println("${f()} ${g()} ${h()}")
}
```
Output:
```text
1 2 3
```

## Module path separator

**SPEC-MOD-003** Paths use `::` between modules, types and items (`Type::assoc`, `Enum::Variant`, `mod::item`). ADR-0015 considered `.` and rejected it for now.

<!-- example: tests/spec/modules/SPEC-MOD-003.wj -->
```windjammer
mod shapes {
    pub struct Sq { pub s: int }
    impl Sq {
        pub fn area(self) -> int { self.s * self.s }
    }
}
fn main() {
    let q = shapes::Sq { s: 3 }
    println("${q.area()}")
}
```
Output:
```text
9
```

## Private items

**SPEC-MOD-004** An item without `pub` is visible only inside its own module and its children. Using it from outside is a compile-time error. [OPEN Q-009] the interpreter does not enforce privacy.

<!-- example: tests/spec/modules/SPEC-MOD-004.wj -->
<!-- ignore: known divergence Q-009; the example shows the specified behaviour, which the implementation does not have yet -->
```windjammer,ignore
mod m {
    fn secret() -> int { 1 }
    pub fn open() -> int { secret() }
}
fn main() {
    println("${m::secret()}")
}
```
Rejected at compile time.
> **Known divergence:** the implementation does not do this yet; see [Q-009](OPEN_QUESTIONS.md). The example is marked `xfail`.

## Standard library imports

**SPEC-MOD-005** Standard-library modules are imported with `use std::<module>` (for example `std::collections::HashMap`, `std::math`, `std::process`); `std` is always available.

<!-- example: tests/spec/modules/SPEC-MOD-005.wj -->
```windjammer
use std::collections::HashMap
fn main() {
    let mut m = HashMap::new()
    m.insert("a", 1)
    m.insert("b", 2)
    println("${m.len()} ${m.get("a").unwrap()} ${m.contains_key("z")}")
}
```
Output:
```text
2 1 false
```


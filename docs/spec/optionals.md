# Optional types and `??`

`T?` and `x ?? d` ([ADR-0018](../adr/0018-expressiveness-phase-2.md) feature 5), parser sugar over `Option`: `T?` is `Option<T>`, `x ?? d` is `x.unwrap_or_else(|| d)` (`unwrap_or(d)` when `d` is a literal or a variable, which is equivalent). Optional chaining `a?.b` is `a.__opt_chain(|_oc| _oc.b)` (parser sugar; the interpreter, Go, JS and TS implement it, the Rust backend rejects it until the IR solver supplies the receiver type). On a `Result`, write `r.ok() ?? d`.

Part of the [Windjammer language specification](README.md). Rule IDs are checked by `scripts/wj/spec_check.wj`; each rule's executable example is the code block below it, stored in `tests/spec/optionals/`.

## Optional types

**SPEC-OPT-001** `T?` is exactly `Option<T>`: the two spellings are the same type and interchangeable.

<!-- example: tests/spec/optionals/SPEC-OPT-001.wj -->
```windjammer
fn half(x: int) -> int? {
    if x % 2 == 0 {
        Some(x / 2)
    } else {
        None
    }
}

fn main() {
    let a: Option<int> = half(8)
    let b: int? = a
    println("${b.is_some()} ${half(3).is_none()}")
}
```
Output:
```text
true true
```

## Default operator

**SPEC-OPT-002** `x ?? d` is the value inside `x` when it is `Some`, else `d`. `d` is evaluated only when `x` is `None`.

<!-- example: tests/spec/optionals/SPEC-OPT-002.wj -->
```windjammer
fn loud() -> int {
    println("evaluated")
    7
}

fn main() {
    let a: int? = Some(5)
    let b: int? = None
    println("${a ?? loud()}")
    println("${b ?? loud()}")
}
```
Output:
```text
5
evaluated
7
```

**SPEC-OPT-003** `??` is right-associative and binds looser than arithmetic and tighter than comparison: `a ?? b ?? 0` is `a ?? (b ?? 0)`, `x ?? 0 + 1` is `x ?? (0 + 1)`, `x ?? 0 > 3` is `(x ?? 0) > 3`.

<!-- example: tests/spec/optionals/SPEC-OPT-003.wj -->
```windjammer
fn main() {
    let a: int? = None
    let b: int? = Some(4)
    println("${a ?? b ?? 0}")
    println("${a ?? 0 + 1}")
    println("${b ?? 0 > 3}")
}
```
Output:
```text
4
1
true
```

**SPEC-OPT-004** `T??` is not a type (a nested option is written `Option<T?>`).

<!-- example: tests/spec/optionals/SPEC-OPT-004.wj -->
<!-- should_fail: misuse is rejected by the parser -->
```windjammer,should_fail
fn main() {
    let x: int?? = None
}
```
Error contains: `is not a type`

**SPEC-OPT-005** `a?.b` reads field `b` of the value inside the optional `a` and yields an optional: `None` when `a` is `None`, else `Some(a.b)`. `a?.m(args)` calls a method the same way.

<!-- example: tests/spec/optionals/SPEC-OPT-005.wj -->
```windjammer
struct Item {
    qty: int,
}

fn main() {
    let a: Item? = Some(Item { qty: 5 })
    let b: Item? = None
    let qa = a?.qty
    let qb = b?.qty
    println("${qa ?? 0} ${qa.is_some()}")
    println("${qb ?? 0} ${qb.is_none()}")
}
```
Output:
```text
5 true
0 true
```

**SPEC-OPT-006** Each `?.` covers one member and the result is flattened: when the member is itself an option, `a?.b` is that option (not an option of an option), so `a?.b?.c` is a single optional that is `None` as soon as any link is `None`. `a?.m()` for a method returning a plain value is `Some(value)`.

<!-- example: tests/spec/optionals/SPEC-OPT-006.wj -->
```windjammer
struct Profile {
    nick: string?,
}

struct User {
    name: string,
    profile: Profile?,
}

impl User {
    fn name_len(self) -> int {
        self.name.len() as int
    }
}

fn main() {
    let full: User? = Some(User { name: "abc", profile: Some(Profile { nick: Some("ann") }) })
    let no_nick: User? = Some(User { name: "bob", profile: Some(Profile { nick: None }) })
    let no_profile: User? = Some(User { name: "cy", profile: None })
    println(full?.profile?.nick ?? "anon")
    println(no_nick?.profile?.nick ?? "anon")
    println(no_profile?.profile?.nick ?? "anon")
    println("${full?.name_len() ?? 0}")
}
```
Output:
```text
ann
anon
anon
3
```

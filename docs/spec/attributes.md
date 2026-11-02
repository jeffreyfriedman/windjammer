# Attributes

`@name(args)` annotations on items.

Part of the [Windjammer language specification](README.md). Rule IDs are checked by `scripts/wj/spec_check.wj`; each rule's executable example is the code block below it, stored in `tests/spec/attributes/`.

Attributes the compiler interprets today include `derive`, `test`, `property_test`, `test_cases`, `timing`, `route` (web) and several UI/runtime decorators. The authoritative list lives in `src/decorator_registry.rs`; unknown attributes parse and are ignored by the core.

## Attribute syntax

**SPEC-ATTR-001** An attribute `@name` or `@name(args)` precedes an item (function, struct, enum, impl, trait). Several may stack. Unknown attributes are retained for tooling and ignored by the compiler core.

<!-- example: tests/spec/attributes/SPEC-ATTR-001.wj -->
```windjammer
@derive(Debug)
@custom_tag(level: 3)
struct P { x: int }
fn main() {
    let p = P { x: 1 }
    println("${p.x}")
}
```
Output:
```text
1
```

## @derive

**SPEC-ATTR-002** `@derive(T1, T2, ...)` on a struct or enum requests standard trait implementations: Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize. Many are also inferred when every field supports them. [OPEN Q-016] the interpreter does not implement `Default::default` for derived types.

<!-- example: tests/spec/attributes/SPEC-ATTR-002.wj -->
<!-- ignore: known divergence Q-016; the example shows the specified behaviour, which the implementation does not have yet -->
```windjammer,ignore
@derive(Clone, Debug, PartialEq, Default)
struct Cfg { n: int, s: string }
fn main() {
    let d = Cfg::default()
    let e = d.clone()
    println("${d == e} ${d.n} [${d.s}]")
}
```
Output:
```text
true 0 []
```
> **Known divergence:** the implementation does not do this yet; see [Q-016](OPEN_QUESTIONS.md). The example is marked `xfail`.

## @test

**SPEC-ATTR-003** A function marked `@test` (or named `test_*` in a `*_test.wj` file) is collected by `wj test`, which runs it and reports a failure when it panics. `@test` functions are ignored by `wj run`.

<!-- example: tests/spec/attributes/SPEC-ATTR-003.wj -->
```windjammer
@test
fn adds() {
    assert_eq(1 + 1, 2)
}
fn main() {
    println("main only")
}
```
Output:
```text
main only
```

## Attribute arguments

**SPEC-ATTR-004** Attribute arguments are expressions or `name: expr` pairs, kept as written.

<!-- example: tests/spec/attributes/SPEC-ATTR-004.wj -->
```windjammer
@route("/users", method: "GET")
fn list() -> int { 1 }
fn main() {
    println("${list()}")
}
```
Output:
```text
1
```


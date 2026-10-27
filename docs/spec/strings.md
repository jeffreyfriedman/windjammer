# Strings and interpolation

`string` is UTF-8 text. `${...}` interpolation and format specifiers replace Rust's `{}` templates (ADR-0014).

Part of the [Windjammer language specification](README.md). Rule IDs are checked by `scripts/wj/spec_check.wj`; each rule's executable example is the code block below it, stored in `tests/spec/strings/`.

## String type

**SPEC-STR-001** `string` is the text type: an immutable-by-default UTF-8 sequence. `len()` counts bytes, `chars().count()` counts Unicode scalar values.

<!-- example: tests/spec/strings/SPEC-STR-001.wj -->
```windjammer
fn main() {
    let s = "héllo"
    println("${s.len()} ${s.chars().count()}")
}
```
Output:
```text
6 5
```

## Interpolation

**SPEC-STR-002** `${expr}` inside a string literal evaluates `expr` and inserts its Display text. Any expression is allowed, including calls, arithmetic and field access.

<!-- example: tests/spec/strings/SPEC-STR-002.wj -->
```windjammer
struct P { x: int }
fn twice(n: int) -> int { n * 2 }
fn main() {
    let p = P { x: 4 }
    let name = "wj"
    println("hi ${name}! ${1 + 2} ${twice(p.x)} ${p.x > 3}")
}
```
Output:
```text
hi wj! 3 8 true
```

## Interpolation is evaluated once, left to right

**SPEC-STR-003** Each `${...}` in a literal is evaluated exactly once, in textual order.

<!-- example: tests/spec/strings/SPEC-STR-003.wj -->
```windjammer
fn tick(c: int, tag: string) -> int {
    println("eval ${tag}")
    c
}
fn main() {
    println("${tick(1, "a")} ${tick(2, "b")}")
}
```
Output:
```text
eval a
eval b
1 2
```

## Format specifiers

**SPEC-STR-004** `${expr:spec}` applies a format spec: `.N` precision, width with `<` `>` `^` alignment, `0N` zero padding, and the radix/debug letters `x X b o e ?`.

<!-- example: tests/spec/strings/SPEC-STR-004.wj -->
```windjammer
fn main() {
    println("[${42:>6}] [${42:<6}] [${42:^6}] [${7:03}]")
    println("${255:x} ${255:X} ${5:b} ${64:o}")
    println("${3.14159:.2}")
}
```
Output:
```text
[    42] [42    ] [  42  ] [007]
ff FF 101 100
3.14
```

## Braces are literal

**SPEC-STR-005** A `{` or `}` that is not part of `${...}` is an ordinary character; there is no `{}` placeholder syntax (ADR-0014).

<!-- example: tests/spec/strings/SPEC-STR-005.wj -->
```windjammer
fn main() {
    let x = 5
    println("set {a, b} and {}")
    println("x={x}")
}
```
Output:
```text
set {a, b} and {}
x={x}
```

## Concatenation and comparison

**SPEC-STR-006** `+` joins a `string` with a `string` or `&str`; `==` and `<` compare by Unicode scalar value order (byte-lexicographic for UTF-8).

<!-- example: tests/spec/strings/SPEC-STR-006.wj -->
```windjammer
fn main() {
    let a = "foo"
    let b = a + "bar"
    let c = "abc"
    let d = "abd"
    println("${b} ${b == "foobar"} ${c < d} ${c < "b"}")
}
```
Output:
```text
foobar true true true
```

## Core string methods

**SPEC-STR-007** Methods available on every backend: `len is_empty contains starts_with ends_with to_uppercase to_lowercase trim split replace chars`.

<!-- example: tests/spec/strings/SPEC-STR-007.wj -->
```windjammer
fn main() {
    let s = "  Hello, World  "
    let t = s.trim()
    println("[${t}] ${t.len()} ${t.is_empty()}")
    println("${t.contains("World")} ${t.starts_with("Hell")} ${t.ends_with("x")}")
    println("${t.to_uppercase()} ${t.to_lowercase()}")
    println("${t.replace("l", "L")}")
    let parts = "a,b,c".split(",").collect::<Vec<string>>()
    println("${parts.len()} ${parts[1]}")
}
```
Output:
```text
[Hello, World] 12 false
true true false
HELLO, WORLD hello, world
HeLLo, WorLd
3 b
```

## Slicing by byte range

**SPEC-STR-008** `s[a..b]` slices by byte offsets and panics if an offset is not on a character boundary or is out of range.

<!-- example: tests/spec/strings/SPEC-STR-008.wj -->
```windjammer
fn main() {
    let s = "hello"
    println("${s[1..3]} ${s[..2]} ${s[3..]}")
    let u = "é"
    println("${u[0..1]}")
}
```
Output:
```text
el he lo
```
Then panics, exit status 101.

## Display for user types

**SPEC-STR-009** [OPEN Q-006] A type that defines `fn to_string(self) -> string` is printable and interpolatable (ADR-0015 section 3, accepted).

<!-- example: tests/spec/strings/SPEC-STR-009.wj -->
<!-- ignore: known divergence Q-006; the example shows the specified behaviour, which the implementation does not have yet -->
```windjammer,ignore
struct Pt { x: int, y: int }
impl Pt {
    fn to_string(self) -> string { "(${self.x}, ${self.y})" }
}
fn main() {
    let p = Pt { x: 1, y: 2 }
    println("p=${p}")
}
```
Output:
```text
p=(1, 2)
```
> **Known divergence:** the implementation does not do this yet; see [Q-006](OPEN_QUESTIONS.md). The example is marked `xfail`.

## Bool and char display

**SPEC-STR-010** `bool` prints `true`/`false`; `char` prints the character itself.

<!-- example: tests/spec/strings/SPEC-STR-010.wj -->
```windjammer
fn main() {
    println("${true} ${false} ${'x'}")
}
```
Output:
```text
true false x
```

## Dollar-brace-quote

**SPEC-STR-011** `${` immediately followed by `"` is not an interpolation start: the literal `"${"` has the value `${`, the quote closing the string. (An interpolation therefore cannot begin with a string literal; bind it to a variable first.)

<!-- example: tests/spec/strings/SPEC-STR-011.wj -->
```windjammer
fn main() {
    println("a ${")
}
```
Output:
```text
a ${
```


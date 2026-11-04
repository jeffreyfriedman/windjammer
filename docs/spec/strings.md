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


## String operations on non-ASCII text

**SPEC-STR-016** `trim`, case mapping (including `ß` to `SS`), `split` with empty fields, `split_whitespace`, `lines` (which drops `\r\n` and `\n`), `repeat`, `replace`, comparison (byte order, so `"Zebra" < "apple"`) and `chars().rev()` operate on Unicode scalar values and UTF-8 bytes as Rust's `str` does.

<!-- example: tests/spec/strings/SPEC-STR-016.wj -->
```windjammer
fn main() {
    let s = "  Héllo Wörld  "
    println("[${s.trim()}] [${s.trim_start()}] [${s.trim_end()}]")
    println("${s.to_uppercase()} ${s.to_lowercase()}")
    let sharp = "straße"
    println("${sharp.to_uppercase()} ${sharp.len()} ${sharp.chars().count()}")
    let csv = "a,,b,"
    let parts: Vec<string> = csv.split(",").collect()
    println("${parts.len()} [${parts[0]}] [${parts[1]}] [${parts[2]}] [${parts[3]}]")
    let words: Vec<string> = "  one   two  ".split_whitespace().collect()
    println("${words.len()} ${words[0]} ${words[1]}")
    println("${"abc".repeat(3)} ${"a-b-c".replace("-", "+")} ${"aaa".replacen("a", "b", 2)}")
    let apple = "apple"
    let banana = "banana"
    let zebra = "Zebra"
    let acute = "é"
    let zed = "z"
    println("${apple < banana} ${zebra < apple} ${acute > zed} ${apple == "apple"}")
    let rev: string = "héllo".chars().rev().collect()
    println("${rev}")
    println("${"héllo".starts_with("hé")} ${"héllo".ends_with("lo")} ${"héllo".contains("él")}")
    let lines: Vec<string> = "a\nb\r\nc\n".lines().collect()
    println("${lines.len()} [${lines[0]}] [${lines[1]}] [${lines[2]}]")
}
```
Output:
```text
[Héllo Wörld] [Héllo Wörld  ] [  Héllo Wörld]
  HÉLLO WÖRLD     héllo wörld  
STRASSE 7 6
4 [a] [] [b] []
2 one two
abcabcabc a+b+c bba
true true true true
olléh
true true true
3 [a] [b] [c]
```

## split_once

**SPEC-STR-017** `split_once(sep)` splits at the first occurrence of the separator and returns `Some((before, after))`, or `None` when the separator is absent.

<!-- example: tests/spec/strings/SPEC-STR-017.wj -->
```windjammer
fn main() {
    let kv = "key=value"
    match kv.split_once("=") {
        Some((k, v)) => println("${k} -> ${v}"),
        None => println("no separator"),
    }
    println("${"x=1".split_once("=").is_some()} ${"x".split_once("=").is_none()}")
}
```
Output:
```text
key -> value
true true
```
The compiled backend cannot build this program yet (docs/drift/spec-findings.md); the output above is that of the equivalent Rust program.

## Byte offsets

**SPEC-STR-012** `len` counts UTF-8 bytes and `chars().count()` counts scalar values; slicing, `find`, `rfind` and `char_indices` use byte offsets.

<!-- example: tests/spec/strings/SPEC-STR-012.wj -->
```windjammer
fn main() {
    let s = "héllo, wörld"
    println("${s.len()} ${s.chars().count()}")
    println("${s[0..1]}|${s[..3]}|${s[3..]}|${s[7..]}")
    println("${s.find("wö").unwrap_or(99)} ${s.find("z").is_none()} ${s.rfind("l").unwrap_or(99)}")
    let mut idx: Vec<int> = Vec::new()
    for (i, c) in s.char_indices() {
        if !c.is_ascii() {
            idx.push(i as int)
        }
    }
    println("${idx.len()} ${idx[0]} ${idx[1]}")
    println("${s.is_char_boundary(2)} ${s.is_char_boundary(1)}")
}
```
Output:
```text
14 12
h|hé|llo, wörld| wörld
8 true 12
2 1 9
false true
```

## Slicing inside a character

**SPEC-STR-013** Slicing a string at an offset that is not a character boundary panics, with a message containing `boundary`.

<!-- example: tests/spec/strings/SPEC-STR-013.wj -->
```windjammer
fn main() {
    let s = "héllo"
    println("before")
    println("${s[0..2]}")
}
```
Output:
```text
before
```
Then panics (message contains `boundary`), exit status 101.

## Format specifiers

**SPEC-STR-014** `${value:spec}` supports width, alignment (`<`, `>`, `^`), fill, zero padding, sign, precision, and the `x X b o e` radix and exponent types with the `#` prefix flag, exactly as Rust's `format!`.

<!-- example: tests/spec/strings/SPEC-STR-014.wj -->
```windjammer
fn main() {
    let n = 42
    let neg = -42
    println("[${n:5}] [${n:<5}] [${n:^5}] [${n:>5}] [${n:05}] [${neg:05}] [${n:+}]")
    let s = "ab"
    let long = "abcdef"
    println("[${s:6}] [${s:>6}] [${s:^6}] [${s:*^7}] [${long:.3}] [${long:8.3}]")
    let pi = 3.14159
    let half = 2.5
    let odd = 3.5
    let eighth = 0.125
    let neg_f = -1.5
    let one = 1.0
    println("[${pi:.2}] [${pi:8.3}] [${half:.0}] [${odd:.0}] [${eighth:.2}] [${neg_f:08.2}] [${one:.3}]")
    let big = 255
    let small: i8 = -1
    println("[${big:x}] [${big:X}] [${big:#x}] [${big:08b}] [${big:#b}] [${big:o}] [${big:#o}] [${small:x}]")
    let e1 = 1234567.0
    let e2 = 0.00012
    println("[${e1:e}] [${e2:e}] [${big:e}]")
    let t = true
    let ch = 'x'
    let acc = "é"
    println("[${t:>6}] [${ch:^5}] [${acc:>3}]")
}
```
Output:
```text
[   42] [42   ] [ 42  ] [   42] [00042] [-0042] [+42]
[ab    ] [    ab] [  ab  ] [**ab***] [abc] [abc     ]
[3.14] [   3.142] [2] [4] [0.12] [-0001.50] [1.000]
[ff] [FF] [0xff] [11111111] [0b11111111] [377] [0o377] [ff]
[1.234567e6] [1.2e-4] [2.55e2]
[  true] [  x  ] [  é]
```

## Debug formatting

**SPEC-STR-015** `${value:?}` prints the Debug form (strings and chars quoted and escaped, structs, enums, tuples, options and collections with their names) and `${value:#?}` the pretty form with four-space indentation.

<!-- example: tests/spec/strings/SPEC-STR-015.wj -->
```windjammer
struct P {
    x: int,
    name: string,
}

enum E {
    A,
    B(int, string),
    C { v: Vec<int> },
}

fn main() {
    let v: Vec<Option<int>> = vec![Some(1), None]
    println("${v:?}")
    let tup = ("a", 1, 2.0, true, 'c')
    println("${tup:?}")
    let quoted = "q\"uote\n\t\\"
    let nl = '\n'
    let sq = '\''
    println("${quoted:?} ${nl:?} ${sq:?}")
    let p = P { x: 1, name: "n" }
    println("${p:?}")
    let b = E::B(2, "s")
    let c = E::C { v: vec![1, 2] }
    println("${E::A:?} ${b:?} ${c:?}")
    let r: Result<int, string> = Err("bad")
    let some = Some("x")
    let one = 1.0
    let tenth = 0.1
    let huge = 1e21
    let tiny = 1.0e-7
    println("${r:?} ${some:?} ${one:?} ${tenth:?} ${huge:?} ${tiny:?}")
    println("${p:#?}")
    println("${b:#?}")
    let e: Vec<int> = Vec::new()
    println("${e:?} ${e:#?}")
}
```
Output:
```text
[Some(1), None]
("a", 1, 2.0, true, 'c')
"q\"uote\n\t\\" '\n' '\''
P { x: 1, name: "n" }
A B(2, "s") C { v: [1, 2] }
Err("bad") Some("x") 1.0 0.1 1e21 1e-7
P {
    x: 1,
    name: "n",
}
B(
    2,
    "s",
)
[] []
```

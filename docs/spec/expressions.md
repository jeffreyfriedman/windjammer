# Expressions

Everything that has a value. The precedence table below is normative and is *not* Rust's.

Part of the [Windjammer language specification](README.md). Rule IDs are checked by `scripts/wj/spec_check.wj`; each rule's executable example is the code block below it, stored in `tests/spec/expressions/`.

| Level (tight to loose) | Operators | Associativity |
|---|---|---|
| 1 | postfix: `.f` `.m()` `()` `[]` `?` `as T` | left |
| 2 | prefix `-` `!` `&` `*` | right |
| 3 | `*` `/` `%` | left |
| 4 | `<<` `>>` | left |
| 5 | `+` `-` | left |
| 6 | `<` `<=` `>` `>=` | left |
| 7 | `==` `!=` | left |
| 8 | `&` | left |
| 9 | `^` | left |
| 10 | `\|` | left |
| 11 | `&&` | left |
| 12 | `\|\|` | left |
| 13 | `\|>` (pipe) | left |

Source: `get_binary_op` in `src/parser/binary_expression_parser.rs`. Differences from Rust: shifts bind tighter than `+`, and the bitwise operators bind looser than comparisons (like C). Both are surprising to Rust programmers; see Q-004 for `as` and unary minus.

## Operator precedence

**SPEC-EXPR-001** Binary operators bind, from tightest to loosest: `* / %`; `<< >>`; `+ -`; `< <= > >=`; `== !=`; `&`; `^`; `|`; `&&`; `||`. All are left-associative. Unlike Rust and C, shifts bind TIGHTER than `+ -`: `1 << 2 + 1` is `(1 << 2) + 1`.

<!-- example: tests/spec/expressions/SPEC-EXPR-001.wj -->
```windjammer
fn main() {
    println("${2 + 3 * 4} ${(2 + 3) * 4} ${1 << 2 + 1} ${2 * 3 << 1}")
    println("${10 - 3 - 2} ${100 / 10 / 5} ${2 * 3 % 4}")
    println("${1 + 2 == 3 && 4 > 3 || false}")
    println("${1 | 2 ^ 3 & 4}")
}
```
Output:
```text
14 20 5 12
5 2 2
true
3
```

## Relational binds tighter than equality

**SPEC-EXPR-002** `<` `<=` `>` `>=` bind tighter than `==` `!=` (as in C), so `a < b == c < d` compares two booleans.

<!-- example: tests/spec/expressions/SPEC-EXPR-002.wj -->
```windjammer
fn main() {
    println("${1 < 2 == 3 < 4}")
    println("${1 < 2 == false}")
}
```
Output:
```text
true
false
```

## Bitwise vs comparison

**SPEC-EXPR-003** `&`, `^`, `|` bind LOOSER than `==` (as in C, unlike Rust): `a & b == c` is `a & (b == c)` and is a type error for integers; parenthesise.

<!-- example: tests/spec/expressions/SPEC-EXPR-003.wj -->
```windjammer
fn main() {
    let a = 6
    println("${(a & 3) == 2}")
}
```
Output:
```text
true
```

## Unary operators

**SPEC-EXPR-004** Prefix `-` (negate), `!` (boolean not) and `&`/`*` (reference/dereference, mostly inferred) bind tighter than any binary operator but looser than postfix forms (`.`, `()`, `[]`, `?`).

<!-- example: tests/spec/expressions/SPEC-EXPR-004.wj -->
```windjammer
fn main() {
    let v = [3, 4]
    println("${-v[0] + 1} ${!(1 > 2)} ${-2 * 3}")
}
```
Output:
```text
-2 true -6
```

## Unary minus and as

**SPEC-EXPR-005** [OPEN Q-004] `as` binds tighter than prefix `-`?  Recommended: prefix `-` binds tighter, so `-1 as u8` is `(-1) as u8` (Rust, C). Today `as` is a postfix operator, so `-1 as u8` is `-(1 as u8)` and panics.

<!-- example: tests/spec/expressions/SPEC-EXPR-005.wj -->
<!-- ignore: known divergence Q-004; the example shows the specified behaviour, which the implementation does not have yet -->
```windjammer,ignore
fn main() {
    println("${-1 as u8}")
}
```
Output:
```text
255
```
> **Known divergence:** the implementation does not do this yet; see [Q-004](OPEN_QUESTIONS.md). The example is marked `xfail`.

## Short-circuit

**SPEC-EXPR-006** `&&` and `||` evaluate the right operand only when the left does not decide the result.

<!-- example: tests/spec/expressions/SPEC-EXPR-006.wj -->
```windjammer
fn t(tag: string) -> bool {
    println("t ${tag}")
    true
}
fn f(tag: string) -> bool {
    println("f ${tag}")
    false
}
fn main() {
    let a = f("1") && t("2")
    let b = t("3") || f("4")
    let c = t("5") && f("6")
    println("${a} ${b} ${c}")
}
```
Output:
```text
f 1
t 3
t 5
f 6
false true false
```

## Evaluation order

**SPEC-EXPR-007** Operands, call arguments and struct-literal fields are evaluated left to right; a call evaluates its callee and arguments before running the body.

<!-- example: tests/spec/expressions/SPEC-EXPR-007.wj -->
```windjammer
fn n(tag: string, v: int) -> int {
    println(tag)
    v
}
fn sub(a: int, b: int) -> int { a - b }
fn main() {
    let r = n("a", 10) - n("b", 3) * n("c", 2)
    let s = sub(n("x", 9), n("y", 4))
    println("${r} ${s}")
}
```
Output:
```text
a
b
c
x
y
4 5
```

## if expression

**SPEC-EXPR-008** `if c { a } else { b }` is an expression whose value is the value of the taken branch; both branches must have the same type. Without `else` the type is `()`.

<!-- example: tests/spec/expressions/SPEC-EXPR-008.wj -->
```windjammer
fn main() {
    let x = if 3 > 2 { "yes" } else { "no" }
    let y = if 1 > 2 { 10 } else if 2 > 1 { 20 } else { 30 }
    println("${x} ${y}")
}
```
Output:
```text
yes 20
```

## Block value

**SPEC-EXPR-009** A block's value is its final expression when that expression has no trailing `;`; otherwise `()`. The final expression is also the function's implicit return value.

<!-- example: tests/spec/expressions/SPEC-EXPR-009.wj -->
```windjammer
fn sq(n: int) -> int {
    let m = n * n
    m
}
fn main() {
    let v = {
        let a = 4
        a * 2
    }
    println("${v} ${sq(5)}")
}
```
Output:
```text
8 25
```

## match expression

**SPEC-EXPR-010** `match e { p1 => v1, p2 => v2 }` evaluates `e`, selects the first arm whose pattern matches (and whose guard, if any, is true), and yields that arm's value. A match with no matching arm panics; the compiler requires exhaustive matches for enums.

<!-- example: tests/spec/expressions/SPEC-EXPR-010.wj -->
```windjammer
fn name(n: int) -> string {
    match n {
        0 => "zero",
        1 => "one",
        _ => "many",
    }
}
fn main() {
    println("${name(0)} ${name(1)} ${name(9)}")
}
```
Output:
```text
zero one many
```

## Closures

**SPEC-EXPR-011** `|params| expr` or `|params| { block }` creates a closure that captures variables from the enclosing scope; captured variables mutated inside the closure are visible outside.

<!-- example: tests/spec/expressions/SPEC-EXPR-011.wj -->
```windjammer
fn apply(f: fn(int) -> int, x: int) -> int { f(x) }
fn main() {
    let k = 10
    let add = |x| x + k
    println("${apply(add, 5)}")
    let mut count = 0
    let mut bump = || { count += 1 }
    bump()
    bump()
    println("${count}")
}
```
Output:
```text
15
2
```

## Ranges

**SPEC-EXPR-012** `a..b` is the half-open range and `a..=b` the closed range; they are iterable and used for slicing.

<!-- example: tests/spec/expressions/SPEC-EXPR-012.wj -->
```windjammer
fn main() {
    let mut s = 0
    for i in 0..4 { s += i }
    let mut t = 0
    for i in 1..=4 { t += i }
    println("${s} ${t}")
}
```
Output:
```text
6 10
```

## Tuples and fields

**SPEC-EXPR-013** `(a, b)` builds a tuple; elements are read as `t.0`, `t.1`; struct fields as `s.name`; list elements as `v[i]`.

<!-- example: tests/spec/expressions/SPEC-EXPR-013.wj -->
```windjammer
struct P { x: int, y: int }
fn main() {
    let t = (1, "two", 3.5)
    let p = P { x: 7, y: 8 }
    let v = [5, 6, 7]
    println("${t.0} ${t.1} ${t.2} ${p.x} ${p.y} ${v[2]}")
}
```
Output:
```text
1 two 3.5 7 8 7
```

## Struct literal and shorthand

**SPEC-EXPR-014** `T { f: e, ... }` builds a struct; `T { f }` is shorthand for `T { f: f }`. Every field must be given.

<!-- example: tests/spec/expressions/SPEC-EXPR-014.wj -->
```windjammer
struct P { x: int, y: int }
fn main() {
    let x = 1
    let y = 2
    let p = P { x, y }
    let q = P { x: 10, y: p.y + 1 }
    println("${p.x} ${p.y} ${q.x} ${q.y}")
}
```
Output:
```text
1 2 10 3
```

## Method call syntax

**SPEC-EXPR-015** `e.m(args)` calls method `m` with `e` as receiver; the compiler borrows or moves the receiver as the method requires. `T::f(args)` calls an associated function.

<!-- example: tests/spec/expressions/SPEC-EXPR-015.wj -->
```windjammer
struct C { n: int }
impl C {
    fn new(n: int) -> C { C { n: n } }
    fn plus(self, k: int) -> int { self.n + k }
}
fn main() {
    let c = C::new(4)
    println("${c.plus(3)} ${c.plus(4)}")
}
```
Output:
```text
7 8
```

## loop as expression

**SPEC-EXPR-016** `loop { ... break v }` is an expression whose value is the `break` operand; `break v` takes its value only from an expression on the same line (docs/design/statement-boundaries.md).

<!-- example: tests/spec/expressions/SPEC-EXPR-016.wj -->
```windjammer
fn main() {
    let mut k = 0
    let r = loop {
        k += 1
        if k == 4 { break k * 10 }
    }
    println("${r}")
}
```
Output:
```text
40
```

## Assignment is a statement

**SPEC-EXPR-017** Assignment (`=` and `op=`) is a statement, not an expression: it has no value and `a = b = c` is rejected.

<!-- example: tests/spec/expressions/SPEC-EXPR-017.wj -->
<!-- should_fail: the rule says this program is rejected -->
```windjammer,should_fail
fn main() {
    let mut a = 1
    let mut b = 2
    a = b = 3
}
```
Rejected at compile time.

## Index and slice

**SPEC-EXPR-018** `v[i]` indexes a list; `v[a..b]`, `v[..b]`, `v[a..]` slice it, producing a list of the selected elements.

<!-- example: tests/spec/expressions/SPEC-EXPR-018.wj -->
```windjammer
fn main() {
    let v = [1, 2, 3, 4, 5]
    let s = v[1..4]
    println("${s.len()} ${s[0]} ${v[..2].len()} ${v[3..].len()}")
}
```
Output:
```text
3 2 2 2
```


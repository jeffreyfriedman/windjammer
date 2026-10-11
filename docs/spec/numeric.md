# Numeric semantics

ADR-0015 section 1 (accepted): `int` is a 64-bit signed integer with overflow checking, `float` is IEEE-754 binary64, and explicit widths (`i8 ... u64 usize f32`) are first-class and never silently converted. ADR-0014 and 0015 are the authority; where the implementation differs it is listed in OPEN_QUESTIONS.md.

Part of the [Windjammer language specification](README.md). Rule IDs are checked by `scripts/spec_check.sh`; each rule's executable example is the code block below it, stored in `tests/spec/numeric/`.

## Default integer

**SPEC-NUM-001** An integer literal with no suffix, annotation or other typing context has type `int`, a 64-bit signed integer (ADR-0015 section 1).

<!-- example: tests/spec/numeric/SPEC-NUM-001.wj -->
```windjammer
fn main() {
    let big = 9223372036854775807
    println("${big}")
    let small = -9223372036854775807 - 1
    println("${small}")
}
```
Output:
```text
9223372036854775807
-9223372036854775808
```

## Checked integer overflow

**SPEC-NUM-002** Arithmetic (`+ - *`, unary `-`) on integers that overflows the result type panics with a message containing `overflow`; it never wraps silently. Exit status 101.

<!-- example: tests/spec/numeric/SPEC-NUM-002.wj -->
```windjammer
fn main() {
    let a = 9223372036854775807
    println("before")
    let b = a + 1
    println("${b}")
}
```
Output:
```text
before
```
Then panics (message contains `overflow`), exit status 101.

## Narrow widths are checked

**SPEC-NUM-003** Explicit-width integers (`u8`, `i32`, ...) keep their declared width: overflowing the declared range panics just as for `int`.

<!-- example: tests/spec/numeric/SPEC-NUM-003.wj -->
```windjammer
fn main() {
    let x: u8 = 255
    let y = x + 1
    println("${y}")
}
```
Then panics (message contains `overflow`), exit status 101.

## Subtraction below zero

**SPEC-NUM-004** For unsigned types `a - b` with `b > a` panics (overflow); it does not wrap.

<!-- example: tests/spec/numeric/SPEC-NUM-004.wj -->
```windjammer
fn main() {
    let a: u32 = 3
    let b: u32 = 5
    println("${b - a}")
    println("${a - b}")
}
```
Output:
```text
2
```
Then panics (message contains `overflow`), exit status 101.

## Integer division truncates

**SPEC-NUM-005** Integer `/` truncates toward zero and `%` takes the sign of the dividend, so `(a / b) * b + a % b == a`.

<!-- example: tests/spec/numeric/SPEC-NUM-005.wj -->
```windjammer
fn main() {
    println("${7 / 2} ${-7 / 2} ${7 / -2} ${-7 / -2}")
    println("${7 % 3} ${-7 % 3} ${7 % -3} ${-7 % -3}")
}
```
Output:
```text
3 -3 -3 3
1 -1 1 -1
```

## Division by zero

**SPEC-NUM-006** Integer `/` or `%` by zero panics with a message containing `zero`.

<!-- example: tests/spec/numeric/SPEC-NUM-006.wj -->
```windjammer
fn main() {
    let z = 0
    println("${10 % z}")
}
```
Then panics (message contains `zero`), exit status 101.

## Default float

**SPEC-NUM-007** A float literal with no suffix or annotation has type `float`, a 64-bit IEEE-754 double (`f64`); `f32` is available only by explicit annotation or suffix.

<!-- example: tests/spec/numeric/SPEC-NUM-007.wj -->
```windjammer
fn main() {
    let x = 0.1 + 0.2
    println("${x}")
    println("${x == 0.3}")
}
```
Output:
```text
0.30000000000000004
false
```

## Float edge values

**SPEC-NUM-008** Float arithmetic never panics: division by zero yields infinity or NaN, and `NaN` compares unequal to everything including itself. They print as `inf`, `-inf` and `NaN`.

<!-- example: tests/spec/numeric/SPEC-NUM-008.wj -->
```windjammer
fn main() {
    let z = 0.0
    let n = z / z
    println("${1.0 / z} ${-1.0 / z} ${n}")
    println("${n == n} ${n != n}")
}
```
Output:
```text
inf -inf NaN
false true
```

## Bitwise operators

**SPEC-NUM-009** `& | ^` and shifts `<< >>` apply to integers; `>>` on signed integers is arithmetic (sign-extending). Shifts bind tighter than `+` (see SPEC-EXPR-001).

<!-- example: tests/spec/numeric/SPEC-NUM-009.wj -->
```windjammer
fn main() {
    println("${12 & 10} ${12 | 10} ${12 ^ 10}")
    println("${1 << 10} ${1024 >> 3} ${-16 >> 2}")
}
```
Output:
```text
8 14 6
1024 128 -4
```

## Explicit conversion

**SPEC-NUM-010** Values of different integer widths never mix implicitly; a conversion is written `x as T`. Widening is exact; narrowing wraps to the low bits; float to integer truncates toward zero and saturates; NaN becomes 0.

<!-- example: tests/spec/numeric/SPEC-NUM-010.wj -->
```windjammer
fn main() {
    let a: i32 = 5
    let b = a as int
    let c = 300 as u8
    let d = (-1) as u8
    println("${b} ${c} ${d}")
    println("${3.9 as int} ${(-3.9) as int} ${1e30 as int}")
    println("${7 as float / 2.0}")
}
```
Output:
```text
5 44 255
3 -3 9223372036854775807
3.5
```

## Checked conversion functions

**SPEC-NUM-011** [OPEN Q-003] `int(x)` and `float(x)` convert losslessly or panic when out of range (ADR-0015 section 2, accepted). Not implemented: the parser rejects `int(x)`.

<!-- example: tests/spec/numeric/SPEC-NUM-011.wj -->
```windjammer
fn main() {
    let a: i32 = 5
    let b = int(a)
    println("${b + 1}")
}
```
Output:
```text
6
```
> **Known divergence:** the implementation does not do this yet; see [Q-003](OPEN_QUESTIONS.md). The example is marked `xfail`.

## Comparison and equality

**SPEC-NUM-012** `== != < <= > >=` on numbers compare mathematically and yield `bool`; they are non-associative (`a < b < c` is not meaningful).

<!-- example: tests/spec/numeric/SPEC-NUM-012.wj -->
```windjammer
fn main() {
    println("${1 < 2} ${2 <= 2} ${3 > 4} ${2 >= 3} ${1 == 1} ${1 != 1}")
    println("${1.5 < 2.5} ${-1 < 0}")
}
```
Output:
```text
true true false false true false
true true
```

## Float formatting

**SPEC-NUM-013** Interpolating a float prints its shortest decimal representation that round-trips (Rust `Display`): no trailing `.0` for integral values, no exponent. A precision spec `${x:.N}` rounds half to even on the decimal expansion of the stored double.

<!-- example: tests/spec/numeric/SPEC-NUM-013.wj -->
```windjammer
fn main() {
    println("${2.0} ${0.5} ${100.0} ${1e15}")
    println("${3.14159:.2} ${2.5:.0} ${0.125:.2} ${1.0:.3}")
}
```
Output:
```text
2 0.5 100 1000000000000000
3.14 2 0.12 1.000
```

## Integer width of indexing and len

**SPEC-NUM-014** `len()` and indexing accept `int` (ADR-0015 section 1): `v[i]` with `i: int` and `v.len() - 1` need no cast.

<!-- example: tests/spec/numeric/SPEC-NUM-014.wj -->
```windjammer
fn main() {
    let v = [10, 20, 30]
    let last = v.len() - 1
    let i: int = 1
    println("${v[last]} ${v[i]} ${v.len()}")
}
```
Output:
```text
30 20 3
```

## Index out of range

**SPEC-NUM-015** Indexing a list outside `0..len` panics with a message containing `out of bounds`.

<!-- example: tests/spec/numeric/SPEC-NUM-015.wj -->
```windjammer
fn main() {
    let v = [1, 2, 3]
    let i = 3
    println("${v[i]}")
}
```
Then panics (message contains `out of bounds`), exit status 101.

## Compound assignment

**SPEC-NUM-016** `op=` for `+ - * / % & | ^ << >>` is `x = x op rhs` with `x` evaluated once, and requires `x` to be declared `mut`.

<!-- example: tests/spec/numeric/SPEC-NUM-016.wj -->
```windjammer
fn main() {
    let mut x = 10
    x += 5
    x -= 3
    x *= 2
    x /= 4
    x %= 4
    x <<= 3
    x |= 1
    println("${x}")
}
```
Output:
```text
17
```

## Unary minus

**SPEC-NUM-017** Unary `-` negates; `-x` overflows (panics) for the minimum value of a signed type.

<!-- example: tests/spec/numeric/SPEC-NUM-017.wj -->
```windjammer
fn main() {
    let m = -9223372036854775807 - 1
    println("${-5} ${- -5}")
    println("${-m}")
}
```
Output:
```text
-5 5
```
Then panics (message contains `overflow`), exit status 101.


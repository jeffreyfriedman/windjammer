# Numeric semantics

ADR-0015 section 1 (accepted): `int` is a 64-bit signed integer with overflow checking, `float` is IEEE-754 binary64, and explicit widths (`i8 ... u64 usize f32`) are first-class and never silently converted. ADR-0014 and 0015 are the authority; where the implementation differs it is listed in OPEN_QUESTIONS.md.

Part of the [Windjammer language specification](README.md). Rule IDs are checked by `scripts/wj/spec_check.wj`; each rule's executable example is the code block below it, stored in `tests/spec/numeric/`.

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
<!-- ignore: known divergence Q-003; the example shows the specified behaviour, which the implementation does not have yet -->
```windjammer,ignore
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


## Wrapping, saturating and checked operations

**SPEC-NUM-018** `wrapping_*`, `saturating_*`, `checked_*` and `overflowing_*` operate in the receiver's declared width: wrapping wraps modulo 2^N, saturating clamps to the type range, checked returns `None` on overflow.

<!-- example: tests/spec/numeric/SPEC-NUM-018.wj -->
```windjammer
fn main() {
    let a: u8 = 250
    println("${a.wrapping_add(10)} ${a.saturating_add(10)} ${a.checked_add(10).is_none()} ${a.checked_add(5).unwrap_or(0)}")
    let m: i32 = 2147483647
    println("${m.wrapping_add(1)} ${m.saturating_add(1)} ${m.checked_mul(2).is_none()}")
    let lo: i64 = -9223372036854775807 - 1
    println("${lo.wrapping_sub(1)} ${lo.saturating_sub(1)} ${lo.wrapping_neg()} ${lo.checked_neg().is_none()}")
    let z: u32 = 0
    println("${z.wrapping_sub(1)} ${z.saturating_sub(1)} ${z.checked_sub(1).is_none()}")
}
```
Output:
```text
4 255 true 255
-2147483648 2147483647 true
9223372036854775807 -9223372036854775808 -9223372036854775808 true
4294967295 0 true
```

## Casts

**SPEC-NUM-019** `as` between integer widths keeps the low bits (two's complement); `float as int` truncates toward zero, saturates at the target range and maps NaN to 0; `char as int` is the code point and `u8 as char` the character.

<!-- example: tests/spec/numeric/SPEC-NUM-019.wj -->
```windjammer
fn main() {
    println("${300 as u8} ${-1 as u8} ${-1i64 as u32} ${255u8 as i8} ${128u8 as i8} ${70000 as i16}")
    println("${3.99 as int} ${-3.99 as int} ${2.5 as int} ${-0.5 as int}")
    println("${1e20 as int} ${-1e20 as int} ${1e20 as u8} ${-5.0 as u8} ${300.7 as u8}")
    let nan = 0.0 / 0.0
    println("${nan as int} ${nan as u8}")
    println("${'a' as int} ${'z' as u8} ${65u8 as char}")
    println("${4294967296 as i32} ${4294967295 as i32} ${-129 as i8}")
    println("${7 as float / 2.0} ${(7 / 2) as float}")
}
```
Output:
```text
44 255 4294967295 -1 -128 4464
3 -3 2 0
9223372036854775807 -9223372036854775808 255 0 255
0 0
97 122 A
0 -1 127
3.5 3
```

## Shifts and bit operations

**SPEC-NUM-020** Shifts and bitwise operators act on the declared width: `<<` discards high bits, `>>` is arithmetic on signed and logical on unsigned types, `!` flips every bit of the width.

<!-- example: tests/spec/numeric/SPEC-NUM-020.wj -->
```windjammer
fn main() {
    println("${1 << 62} ${-8 >> 1} ${-1 >> 63} ${1u32 << 31} ${255u8 >> 3}")
    let x: i32 = 1
    println("${x << 31} ${(x << 30) << 1}")
    let u: u8 = 0b1011_0001
    println("${u << 1} ${u >> 4} ${u & 0x0F} ${u | 0x0F} ${u ^ 0xFF} ${!u}")
    println("${5 & 3} ${5 | 3} ${5 ^ 3} ${!5} ${-5 & 0xFF}")
    println("${u.count_ones()} ${u.leading_zeros()} ${u.trailing_zeros()}")
}
```
Output:
```text
4611686018427387904 -4 -1 2147483648 31
-2147483648 -2147483648
98 11 1 191 78 78
1 7 6 -6 251
4 0 0
```

## Shift amount out of range

**SPEC-NUM-021** Shifting by an amount greater than or equal to the bit width of the left operand panics with a message containing `overflow`.

<!-- example: tests/spec/numeric/SPEC-NUM-021.wj -->
```windjammer
fn main() {
    let s = "64".parse::<int>().unwrap_or(0)
    println("before")
    println("${1 << s}")
}
```
Output:
```text
before
```
Then panics (message contains `overflow`), exit status 101.

## Integer division, remainder and helpers

**SPEC-NUM-022** `/` truncates and `%` takes the dividend's sign for every sign combination; `div_euclid` and `rem_euclid` round toward negative infinity with a non-negative remainder; `abs`, `signum`, `pow`, `min`, `max`, `clamp` and `abs_diff` follow their mathematical definitions.

<!-- example: tests/spec/numeric/SPEC-NUM-022.wj -->
```windjammer
fn main() {
    let a: int = -7
    let b: int = 2
    let c: int = 7
    let d: int = -2
    println("${a / b} ${a % b} ${c / d} ${c % d} ${a / d} ${a % d}")
    println("${a.div_euclid(b)} ${a.rem_euclid(b)} ${c.div_euclid(d)} ${c.rem_euclid(d)}")
    let z: int = 0
    println("${a.abs()} ${a.signum()} ${z.signum()}")
    let ten: int = 10
    println("${c.abs_diff(ten)} ${ten.abs_diff(c)}")
}
```
Output:
```text
-3 -1 -3 1 3 -1
-4 1 -3 1
7 -1 0
3 3
```

## Division overflow

**SPEC-NUM-023** Dividing the minimum signed value by -1 panics with a message containing `overflow`.

<!-- example: tests/spec/numeric/SPEC-NUM-023.wj -->
```windjammer
fn main() {
    let lo: i64 = -9223372036854775807 - 1
    let d = "-1".parse::<int>().unwrap_or(1)
    println("start")
    println("${lo / d}")
}
```
Output:
```text
start
```
Then panics (message contains `overflow`), exit status 101.

## Float display and rounding

**SPEC-NUM-024** Floats print their shortest round-tripping decimal without exponent notation; `round` rounds halves away from zero, `f32` arithmetic is single precision, and `-0.0` prints as `-0`.

<!-- example: tests/spec/numeric/SPEC-NUM-024.wj -->
```windjammer
fn main() {
    println("${0.1 + 0.2} ${1.0 / 3.0} ${2.0 / 3.0} ${100.0 * 1.1}")
    println("${1e21} ${1e-7} ${123456789012345680.0} ${0.000001}")
    println("${1.5e300 * 1.5e300} ${-1.0 / 0.0} ${0.0 / 0.0}")
    let a: f32 = 0.1
    let b: f32 = 0.2
    println("${a + b} ${a * 3.0} ${16777217.0 as f32} ${(0.1 as f32) as float}")
    println("${(2.5).round()} ${(-2.5).round()} ${(3.5).round()} ${(2.5).floor()} ${(2.5).ceil()} ${(-2.5).trunc()}")
    println("${(9.0).sqrt()} ${(2.0).powi(10)} ${(1.0).exp() > 2.718} ${(-0.0)} ${0.0 == -0.0}")
}
```
Output:
```text
0.30000000000000004 0.3333333333333333 0.6666666666666666 110.00000000000001
1000000000000000000000 0.0000001 123456789012345680 0.000001
inf -inf NaN
0.3 0.3 16777216 0.10000000149011612
3 -3 4 2 3 -2
3 1024 true -0 true
```

## Methods on literal receivers

**SPEC-NUM-025** A numeric method may be called on a literal or a negated literal (`(-7).abs()`, `2.pow(10)`, `7.div_euclid(-2)`); the literal has type `int` (SPEC-NUM-001), so the result is the same as for an `int` variable.

<!-- example: tests/spec/numeric/SPEC-NUM-025.wj -->
```windjammer
fn main() {
    println("${(-7).abs()} ${(-7).signum()} ${2.pow(10)} ${(-2).pow(3)}")
    println("${(-7).div_euclid(2)} ${(-7).rem_euclid(2)} ${7.div_euclid(-2)} ${7.rem_euclid(-2)}")
    println("${7.abs_diff(10)} ${10.abs_diff(7)}")
    let two: int = 2
    let minus_two: int = -2
    println("${two.pow(10)} ${minus_two.pow(3)} ${minus_two.pow(2)}")
}
```
Output:
```text
7 -1 1024 -8
-4 1 -3 1
3 3
1024 -8 4
```

## min, max, clamp and rotate

**SPEC-NUM-026** `min`, `max` and `clamp` take arguments of the receiver's type whether the operands are variables or literals; `rotate_left`/`rotate_right` take a bit count and `swap_bytes` reverses the byte order of the declared width.

<!-- example: tests/spec/numeric/SPEC-NUM-026.wj -->
```windjammer
fn main() {
    let ten: int = 10
    let three: int = 3
    let neg: int = -1
    let zero: int = 0
    println("${ten.min(three)} ${ten.max(three)} ${neg.clamp(zero, 5)} ${ten.clamp(zero, 5)}")
    println("${10.min(3)} ${10.max(3)} ${(-1).clamp(0, 5)} ${9.clamp(0, 5)}")
    let u: u8 = 0b1011_0001
    println("${u.rotate_left(3)} ${u.rotate_right(3)} ${u.swap_bytes()}")
}
```
Output:
```text
3 10 0 5
3 10 0 5
141 54 177
```

## Parsing numbers

**SPEC-NUM-027** `parse` accepts an optional sign and digits for integers (no spaces, no underscores), rejects values outside the target width, accepts decimal and exponent forms for floats, and accepts only `true` and `false` for `bool`; the error texts are Rust's.

<!-- example: tests/spec/numeric/SPEC-NUM-027.wj -->
```windjammer
fn main() {
    println("${"42".parse::<int>().unwrap_or(-1)} ${"-42".parse::<int>().unwrap_or(-1)} ${"+7".parse::<int>().unwrap_or(-1)}")
    println("${" 5".parse::<int>().is_err()} ${"5 ".parse::<int>().is_err()} ${"".parse::<int>().is_err()} ${"1_000".parse::<int>().is_err()}")
    println("${"256".parse::<u8>().is_err()} ${"255".parse::<u8>().unwrap_or(0)} ${"-1".parse::<u8>().is_err()}")
    println("${"9223372036854775807".parse::<int>().unwrap_or(0)} ${"9223372036854775808".parse::<int>().is_err()}")
    println("${"3.5".parse::<float>().unwrap_or(0.0)} ${"1e3".parse::<float>().unwrap_or(0.0)} ${".5".parse::<float>().unwrap_or(0.0)} ${"abc".parse::<float>().is_err()}")
    println("${"true".parse::<bool>().unwrap_or(false)} ${"True".parse::<bool>().is_err()}")
    match "12x".parse::<int>() {
        Ok(v) => println("ok ${v}"),
        Err(e) => println("err ${e}"),
    }
    match "".parse::<int>() {
        Ok(v) => println("ok ${v}"),
        Err(e) => println("err ${e}"),
    }
    match "300".parse::<u8>() {
        Ok(v) => println("ok ${v}"),
        Err(e) => println("err ${e}"),
    }
    match "x".parse::<float>() {
        Ok(v) => println("ok ${v}"),
        Err(e) => println("err ${e}"),
    }
}
```
Output:
```text
42 -42 7
true true true true
true 255 true
9223372036854775807 true
3.5 1000 0.5 true
true true
err invalid digit found in string
err cannot parse integer from empty string
err number too large to fit in target type
err invalid float literal
```

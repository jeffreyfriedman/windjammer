# Types and numerics

This page states the rules. The [numbers tutorial](../tutorial/numbers.md) teaches them with more examples, and the
language specification chapters on types and numeric semantics hold the rule identifiers.

## The type forms

| Form | Meaning |
|---|---|
| `int`, `float`, `bool`, `string`, `char`, `()` | the primitives: 64-bit integer, 64-bit float, truth value, text, one character, the empty tuple |
| `i8 i16 i32 i64 u8 u16 u32 u64 usize isize f32 f64` | sized numbers; `int` is `i64` and `float` is `f64` |
| `(A, B, ...)` | a tuple; `()` is the type of statements and of functions without a return type |
| `Vec<T>` | a growable list, written `[1, 2, 3]` |
| `[T; N]` | a fixed-size array |
| `Option<T>`, written `T?` | a value or nothing |
| `Result<T, E>` | a value or an error |
| `HashMap<K, V>`, `Box<T>` | a map; a heap box, needed for types that contain themselves |
| `fn(A, B) -> R` | the type of functions and closures |
| `Name<T, U>` | a generic type applied to arguments, inferred when omitted at a use site |

Parameter and return types are always written. Local types are inferred from the initializer and later uses. An integer
literal takes the type its context demands, and `int` otherwise; a float literal takes `float` otherwise.

```wj
// expect: 5 7
// expect: 1 two 3
fn find(n: int) -> int? {
    if n > 0 { Some(n) } else { None }
}

fn main() {
    let a = find(5) ?? 0
    let b = find(-1) ?? 7
    println("${a} ${b}")
    let t = (1, "two", 3.0)
    println("${t.0} ${t.1} ${t.2}")
}
```

`T?` is exactly `Option<T>`, and `x ?? d` is the value inside `x`, or `d` when `x` is `None`; `d` is only evaluated in that
case. `??` is right-associative and binds looser than arithmetic and tighter than comparison, so `x ?? 0 + 1` is
`x ?? (0 + 1)`.

## Integers

| Rule | Behaviour |
|---|---|
| default | an unsuffixed integer literal is `int`, a 64-bit signed integer |
| overflow | `+ - *` and unary `-` that leave the range of the type stop the program, exit status 101 |
| division | `/` truncates toward zero; `%` has the sign of the left operand; either by zero stops the program |
| unsigned | `a - b` with `b > a` is an overflow, never a wrap |
| bit operators | the bitwise operators and the shifts apply to integers; `>>` on a signed type is arithmetic |
| conversion | only `as`; narrowing keeps the low bits |
| explicit wrapping | `wrapping_add`, `checked_add` (returns an option), `saturating_add` and their siblings |

```wj
// expect: 3 -3 -3 3
// expect: 1 -1 1 -1
fn main() {
    println("${7 / 2} ${-7 / 2} ${7 / -2} ${-7 / -2}")
    println("${7 % 3} ${-7 % 3} ${7 % -3} ${-7 % -3}")
}
```

The identity `(a / b) * b + a % b == a` holds for every `a` and nonzero `b`.

```wj
// expect: 16 15 5 -4
fn main() {
    println("${1 << 4} ${0xFF & 0x0F} ${6 ^ 3} ${-16 >> 2}")
}
```

## `usize`

`usize` is the unsigned integer wide enough to address memory. Lengths, positions and counts are `usize`. It follows
every integer rule above, so `0 - 1` in `usize` is an overflow. Indexing accepts `int` and `usize`. Convert a length to a
signed number once, with `as int`, when you need to compute with it.

```wj
// expect: 3 2
fn main() {
    let items = ["a", "b", "c"]
    let n: usize = items.len()
    let last: usize = n - 1
    println("${n} ${last}")
}
```

## Floats

`float` is IEEE-754 binary64 and `f32` binary32. Float arithmetic never stops the program: division by zero is infinity or
`NaN`, and `NaN` is unequal to everything, itself included. A float prints as the shortest text that reads back to the same
value; use `${x:.2}` for a fixed number of decimals.

```wj
// expect: 0.30000000000000004 false
fn main() {
    let x = 0.1 + 0.2
    println("${x} ${x == 0.3}")
}
```

## Conversions

Sized types are never converted implicitly. `value as Type` converts between numeric types and from `char` to integer;
float to integer truncates toward zero. A string becomes a number with `parse`, which returns a `Result`.

```wj
// expect: 44 4294967295 3 -3
fn main() {
    let wide = 300
    let minus_one = -1
    println("${wide as u8} ${minus_one as u32} ${3.99 as int} ${(-3.99) as int}")
}
```

Combining different numeric types in one operation is specified as a compile-time error. The enforcement in `wj check` is
being completed, so convert with `as` rather than relying on the compiler to accept a mix; the
[open questions](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/spec/OPEN_QUESTIONS.md) record the state.

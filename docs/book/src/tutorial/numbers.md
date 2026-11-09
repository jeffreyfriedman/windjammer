# Numbers and sizes

Windjammer has two everyday number types and a full family of sized ones. You reach for `int` and `float` almost
always, and for the sized types exactly where the size matters: file formats, network protocols, hardware, and
interop with other code.

| Type | Meaning |
|---|---|
| `int` | 64-bit signed integer, overflow checked |
| `float` | 64-bit IEEE-754 floating point |
| `i8` `i16` `i32` `i64` | signed integers of that many bits |
| `u8` `u16` `u32` `u64` | unsigned integers of that many bits |
| `usize` | the unsigned integer that indexes memory: lengths, positions, counts |
| `f32` | 32-bit IEEE-754 floating point |

Every one of these is a real type with a fixed meaning. None of them is an alias for another, and the compiler never
converts between them behind your back.

## `int` and `float`

An integer literal with no other information is an `int`; a literal with a decimal point is a `float`:

```wj
// expect: 9223372036854775807
// expect: 3 -3 1 -1
// expect: 2.5 2
fn main() {
    let biggest = 9223372036854775807
    println("${biggest}")
    println("${7 / 2} ${-7 / 2} ${7 % 3} ${-7 % 3}")
    println("${10.0 / 4.0} ${10 / 4}")
}
```

Integer division truncates toward zero, and `%` takes the sign of the left operand, so `(a / b) * b + a % b` is always
`a`. Mix an `int` into float arithmetic with `as`:

```wj
// expect: 3.5
// expect: 3 -3
fn main() {
    let total = 7
    println("${(total as float) / 2.0}")
    println("${3.99 as int} ${(-3.99) as int}")
}
```

## Overflow is an error, not a surprise

Integer arithmetic that leaves the range of its type stops the program with a message, instead of wrapping around and
carrying on with a wrong answer. That holds for `int` and for every sized type:

```sh,verify
$ cat > overflow.wj <<'EOF'
fn main() {
    let a: u32 = 3
    let b: u32 = 5
    println("${b - a}")
    println("${a - b}")
}
EOF
$ wj run overflow.wj 2>&1 || true
2
attempt to subtract with overflow
```

The program printed `2`, then stopped at the subtraction that would have gone below zero; the exit status is 101.
When wrapping or saturating is exactly what you want, say so with the method that does it:

```wj
// expect: 4
// expect: None
// expect: Some(251)
// expect: 255
fn main() {
    let a: u8 = 250
    println("${a.wrapping_add(10)}")
    println("${a.checked_add(10):?}")
    println("${a.checked_add(1):?}")
    println("${a.saturating_add(10)}")
}
```

Division or remainder by zero stops the program too. Floats never stop: dividing by zero gives infinity or `NaN`, which
compares unequal to everything, itself included:

```wj
// expect: inf NaN
// expect: false true
fn main() {
    let zero = 0.0
    let nan = zero / zero
    println("${1.0 / zero} ${nan}")
    println("${nan == nan} ${nan != nan}")
}
```

Floats print as the shortest text that reads back to the same value, so `0.1 + 0.2` shows its true nature. Use a
precision in the format spec for fixed output:

```wj
// expect: 0.30000000000000004
// expect: 0.30
fn main() {
    let x = 0.1 + 0.2
    println("${x}")
    println("${x:.2}")
}
```

## Sized types

Annotate a binding, a parameter or a field with a sized type and it keeps that type and that range:

```wj
// expect: 255 18446744073709551615
// expect: 0.1
fn main() {
    let byte: u8 = 255
    let wide: u64 = 18446744073709551615
    let small: f32 = 0.1
    println("${byte} ${wide}")
    println("${small}")
}
```

The associated constants `MAX` and `MIN` are there for each type, and a suffix on a literal picks the type in place:

```wj
// expect: 2147483647 255
// expect: 10
fn main() {
    println("${i32::MAX} ${u8::MAX}")
    let k = 10u8
    println("${k}")
}
```

### Conversions are explicit

Changing size or signedness is a decision, so you write it: `value as Type`. Conversions between integers keep the low
bits, and a conversion from float to integer truncates:

```wj
// expect: 44
// expect: 4294967295
fn main() {
    let wide = 300
    let narrow = wide as u8
    println("${narrow}")
    let minus_one = -1
    println("${minus_one as u32}")
}
```

An `as` in the source marks every place where a value can change meaning, which makes them easy to find and to
review. When an expression combines two different integer types, convert one side with `as` so the result type is the
one you chose. The rule is specified in the language specification, and its enforcement in `wj check` is still being
completed: the [open questions](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/spec/OPEN_QUESTIONS.md) record the current state.

## `usize`: sizes and positions

`usize` is the type of anything that counts or locates things in memory: the length of a list, the position in a
string, the index into an array. It is as wide as a machine address, and it is unsigned, so it never holds a negative
number. That is why `len() - 1` on an empty list is a loud overflow rather than a quiet `-1`:

```wj
// expect: 3
// expect: c
fn main() {
    let items = ["a", "b", "c"]
    let count: usize = items.len()
    println("${count}")
    let last: usize = count - 1
    println("${items[last]}")
}
```

Indexing accepts an `int` as well as a `usize`, so loops over a counter you wrote need no conversion:

```wj
// expect: 20
// expect: 30
fn main() {
    let v = [10, 20, 30, 40]
    let mut i = 1
    while i < 3 {
        println("${v[i]}")
        i += 1
    }
}
```

When you need a length as an ordinary number, for example to compute with signed values, convert it once and name the
result:

```wj
// expect: -2
fn main() {
    let v = [1, 2, 3]
    let n: int = v.len() as int
    println("${n - 5}")
}
```

`usize` follows the same rules as the other sized integers: it is checked, it converts with `as`, and `usize::MAX` is
the largest size a machine can address. Code written against the standard
library meets `usize` in lengths, positions, capacities and counts, and nothing in the language asks you to avoid it.

## Choosing a type

* Use `int` for counts, sums, identifiers and loop counters you compute with. It is wide enough that overflow means a
  bug, and checking catches the bug.
* Use `float` for measurements and anything fractional.
* Use `usize` for lengths and positions, as the standard library does.
* Use `u8`, `u16`, `u32`, `u64`, `i32` and the rest where a format, protocol or API fixes the size.
* Use `f32` where memory or a GPU format calls for it.

The same program means the same thing in the interpreter and in compiled code, including the overflow checks and the
way floats print; the book's test suite runs every number example both ways to keep it that way.

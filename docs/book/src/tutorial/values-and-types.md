# Values and types
<!-- feature: lexical-structure -->
<!-- feature: expressions -->
<!-- feature: statements -->
<!-- feature: intrinsics -->

A Windjammer program is a list of functions, and `main` is where it starts. There are no semicolons: a statement ends
at the end of its line, and the last expression of a block is the block's value.

## Bindings

`let` binds a name to a value. Bindings are immutable until you say otherwise with `let mut`:

```wj
// expect: 5
// expect: 8
fn main() {
    let width = 5
    println("${width}")
    let mut total = width
    total += 3
    println("${total}")
}
```

[Run this in the playground](../playground/index.html#lang=wj&code=Ly8gZXhwZWN0OiA1Ci8vIGV4cGVjdDogOApmbiBtYWluKCkgewogICAgbGV0IHdpZHRoID0gNQogICAgcHJpbnRsbigiJHt3aWR0aH0iKQogICAgbGV0IG11dCB0b3RhbCA9IHdpZHRoCiAgICB0b3RhbCArPSAzCiAgICBwcmludGxuKCIke3RvdGFsfSIpCn0K)

The compiler infers the type of every binding, so annotations are optional. Write one when it helps the reader, or
when a literal could mean several types:

```wj
// expect: Ada, 36, 1.5, true
fn main() {
    let name: string = "Ada"
    let age: int = 36
    let ratio: float = 1.5
    let ready: bool = true
    println("${name}, ${age}, ${ratio}, ${ready}")
}
```

[Run this in the playground](../playground/index.html#lang=wj&code=Ly8gZXhwZWN0OiBBZGEsIDM2LCAxLjUsIHRydWUKZm4gbWFpbigpIHsKICAgIGxldCBuYW1lOiBzdHJpbmcgPSAiQWRhIgogICAgbGV0IGFnZTogaW50ID0gMzYKICAgIGxldCByYXRpbzogZmxvYXQgPSAxLjUKICAgIGxldCByZWFkeTogYm9vbCA9IHRydWUKICAgIHByaW50bG4oIiR7bmFtZX0sICR7YWdlfSwgJHtyYXRpb30sICR7cmVhZHl9IikKfQo)

Write `let mut` for any local you change: assigning to it, assigning to one of its fields, or calling a method that
changes it (`push`, `insert`, or a method of your own that changes `self`). A local changed without `mut` is reported as
`WJ0051` by `wj check`, `wj build` and `wj run` alike, with an automatic fix (`wj fix` adds every missing `mut`;
`wj explain WJ0051` explains it).

## Loop variables, too

The rule is the same for the variable of a `for` loop. A loop over a collection reaches the elements themselves, so
assigning to the variable changes the element, and the declaration says so: write `for mut x in ...`. The loop's borrow
stays inferred; the `mut` is your signal, and the reader's, that the loop writes.

```wj
// expect: [10, 20, 30]
fn main() {
    let mut xs = [1, 2, 3]
    for mut x in xs {
        x = x * 10
    }
    println("${xs:?}")
}
```

Patterns carry the marker where it applies: `for (id, mut qty) in items`. Leave it out and `WJ0051` points at the loop
variable and offers the fix; `wj fix` writes it for you ([Tooling](tooling.md#fix-wj-fix)).

## The everyday types

| Type | What it holds |
|---|---|
| `int` | a 64-bit signed integer |
| `float` | a 64-bit floating-point number |
| `bool` | `true` or `false` |
| `string` | text, as UTF-8 |
| `char` | one Unicode character |
| `Vec<T>` | a growable list |
| `Option<T>` | a value, or nothing |
| `Result<T, E>` | a value, or an error |
| `HashMap<K, V>` | a map from keys to values |

The sized numeric types (`u8`, `i32`, `usize`, `f32` and the rest) have [their own chapter](numbers.md).

## Strings

String literals need no conversion call: a literal is already a `string`. Interpolation uses `${...}`, and a colon
after the expression adds a format spec:

```wj
// expect: pi=3.14 [   42] [00042]
fn main() {
    let pi = 3.14159
    println("pi=${pi:.2} [${42:>5}] [${42:05}]")
}
```

To print a literal dollar sign, write `$$`. Values with no plain text form, such as `Option` and `Vec`, print with the
debug spec `:?`:

```wj
// expect: [1, 2, 3] Some(3)
fn main() {
    let v = [1, 2, 3]
    let last: Option<int> = Some(3)
    println("${v:?} ${last:?}")
}
```

## Tuples

A tuple groups a fixed number of values, and you take it apart by pattern:

```wj
// expect: Ada is 36
fn main() {
    let person = ("Ada", 36)
    let (name, age) = person
    println("${name} is ${age}")
}
```

## Functions

Parameters carry types; the return type follows `->`. The last expression is the result, so a function that ends
in a value has no `return`. Use `return` for an early exit:

```wj
// expect: 12
// expect: -1
fn area(w: int, h: int) -> int {
    w * h
}

fn first_negative(values: Vec<int>) -> int {
    for v in values {
        if v < 0 {
            return v
        }
    }
    0
}

fn main() {
    println("${area(3, 4)}")
    println("${first_negative([4, 7, -1, 9])}")
}
```

## Control flow

`if` and `match` are expressions, so they produce values. `match` checks that you covered every case, and an arm may
carry a guard:

```wj
// expect: small
// expect: mid
// expect: big
fn bucket(n: int) -> string {
    match n {
        0 | 1 => "small",
        x if x < 100 => "mid",
        _ => "big",
    }
}

fn main() {
    println(bucket(1))
    println(bucket(50))
    println(bucket(1000))
}
```

`for` walks anything iterable, `while` loops on a condition, and `loop` runs until `break`:

```wj
// expect: 0 1 2
// expect: 16
fn main() {
    let mut seen: Vec<string> = []
    for i in 0..3 {
        seen.push("${i}")
    }
    println(seen.join(" "))

    let mut n = 1
    while n < 10 {
        n *= 2
    }
    println("${n}")
}
```

## Options

A value that might be absent is an `Option`. You handle both cases, so there is no null to trip over:

```wj
// expect: found 3
// expect: no match
fn find(values: Vec<int>, wanted: int) -> Option<int> {
    for (i, v) in values.enumerate() {
        if v == wanted {
            return Some(i)
        }
    }
    None
}

fn main() {
    match find([5, 6, 7, 8], 8) {
        Some(i) => println("found ${i}"),
        None => println("no match"),
    }
    println(if find([1], 9).is_none() { "no match" } else { "match" })
}
```

## Ranges and the built-in functions

`a..b` runs from `a` up to but not including `b`, and `a..=b` includes `b`. Ranges drive `for` loops and slice strings and
lists by position:

```wj
// expect: no newline, then a line
// expect: 0 1 2 1 2 3
// expect: wind
fn main() {
    print("no newline, ")
    println("then a line")
    assert(1 + 1 == 2)
    assert_eq(2 * 2, 4)
    assert_ne(1, 2)
    let mut seen: Vec<string> = []
    for i in 0..3 {
        seen.push("${i}")
    }
    for i in 1..=3 {
        seen.push("${i}")
    }
    println(seen.join(" "))
    let word = "windjammer"
    println("${word[0..4]}")
}
```

The same example uses the built-in functions. `print` and `println` write to standard output, `eprint` and `eprintln` to
standard error. `assert(cond)` stops the program when `cond` is false, `assert_eq(a, b)` and `assert_ne(a, b)` compare two
values and show both on failure, each with an optional message as the last argument. `panic("...")` stops with a message,
`todo()` marks code you have not written, `unreachable()` marks code that cannot run, and `debug(x)` prints a value with
its source location to standard error and hands it back. They are plain functions, not macros, so there is no `!` to
write.

The next chapter looks closely at numbers; after it, [ownership](ownership.md) is the idea that makes Windjammer
different.

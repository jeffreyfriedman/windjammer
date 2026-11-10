# Closures
<!-- feature: closures -->

A closure is a function you write inline, without a name, that can use the variables around it. They are how you tell
`map`, `sort_by` and friends what to do.

## Writing closures

Parameters go between bars, followed by the body. Types are inferred, and you add them when they help:

```wj
// expect: 8
// expect: 15
fn apply(f: fn(int) -> int, x: int) -> int {
    f(x)
}

fn main() {
    let double = |x| x * 2
    println("${double(4)}")

    let offset: int = 10
    let shift = |x: int| x + offset
    println("${apply(shift, 5)}")
}
```

[Run this in the playground](../playground/index.html#lang=wj&code=Ly8gZXhwZWN0OiA4Ci8vIGV4cGVjdDogMTUKZm4gYXBwbHkoZjogZm4oaW50KSAtPiBpbnQsIHg6IGludCkgLT4gaW50IHsKICAgIGYoeCkKfQoKZm4gbWFpbigpIHsKICAgIGxldCBkb3VibGUgPSB8eHwgeCAqIDIKICAgIHByaW50bG4oIiR7ZG91YmxlKDQpfSIpCgogICAgbGV0IG9mZnNldDogaW50ID0gMTAKICAgIGxldCBzaGlmdCA9IHx4OiBpbnR8IHggKyBvZmZzZXQKICAgIHByaW50bG4oIiR7YXBwbHkoc2hpZnQsIDUpfSIpCn0K)

A parameter of function type is written `fn(int) -> int`. Any closure with that shape fits, whatever it captures.

## Capturing variables

A closure sees the variables in scope where it is written. If it changes one, that change is visible afterwards:

```wj
// expect: 2
fn main() {
    let mut count: int = 0
    let mut bump = || {
        count += 1
    }
    bump()
    bump()
    println("${count}")
}
```

Which captures are reads, which are changes and which are moves is inferred, like every other ownership decision in
Windjammer: see [Ownership and borrowing, inferred](ownership.md).

## Returning closures

A function can build and return a closure. The closure keeps the values it captured:

```wj
// expect: 6
fn make_adder(n: int) -> fn(int) -> int {
    |x| x + n
}

fn main() {
    let amount: int = 5
    let add5 = make_adder(amount)
    println("${add5(1)}")
}
```

## Closures and collections

Most closures you write are arguments to a collection method:

```wj
// expect: [1, 1]
// expect: [3, 2, 1]
fn main() {
    let names = ["b", "a"]
    let lens: Vec<int> = names.map(|s| s.len() as int).collect()
    println("${lens:?}")

    let mut sorted = [3, 1, 2]
    sorted.sort_by(|a, b| b.cmp(a))
    println("${sorted:?}")
}
```

`sort_by` takes a comparison, here "larger first". Chains of `map`, `filter` and `collect` read well with the pipe
operator too; see [the pipe operator](types-and-traits.md#the-pipe-operator).

## A note on number types

When a closure captures a number, give the number a type: `let offset: int = 10` rather than `let offset = 10`. An
unannotated literal takes its type from how it is used, and a literal captured by a closure is used somewhere the
compiler cannot always see yet, so the annotation keeps interpreted and compiled runs in agreement. [Known gaps](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/known-gaps.md) lists the integer-literal typing work.

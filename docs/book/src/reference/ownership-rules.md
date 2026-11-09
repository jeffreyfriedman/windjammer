# Ownership rules

What you can observe and what you must write. The compiler's inference, the choice between move, borrow, mutable borrow
and clone, is explained in the [ownership tutorial](../tutorial/ownership.md) and visible with `wj explain-ownership`.

## The governing principle

Windjammer has value semantics: a program behaves as if every assignment, argument pass and return gave the receiver an
independent copy. The compiler picks the cheapest implementation that preserves that behaviour.

| Rule | Consequence |
|---|---|
| assigning, passing or returning a value gives an independent copy | mutating one name never changes another |
| elements stored in a list or field are independent of the variable they came from | later changes on either side are not seen by the other |
| a function that changes a parameter in its body changes the caller's value | no `&mut` in the signature or at the call |
| a method that assigns to `self`'s fields, or calls a mutating method on them, mutates its receiver | the caller holds the receiver in `let mut` |
| a method that returns `self`, or a value built from it, consumes its receiver | builders chain |
| `.clone()` is always available and never changes behaviour | it makes an inferred copy explicit |
| `&`, `&mut` and lifetimes are never required in signatures | where written, they are accepted as hints |

## Locals and parameters

`let` bindings are immutable and `let mut` allows reassignment and mutation of the binding. Parameters are immutable in
the callee unless its body mutates them, which makes them a mutable borrow. Assigning to a local not declared `mut` is
diagnostic `WJ0004`, a warning with an automatic fix today, and an error once the explicit-`mut` migration lands.

```wj
// expect: 1 50 2
struct P {
    x: int,
}

fn bump(p: P) -> P {
    let mut q = p
    q.x += 1
    q
}

fn main() {
    let a = P { x: 1 }
    let mut b = a
    b.x = 50
    let c = bump(a)
    println("${a.x} ${b.x} ${c.x}")
}
```

## Receivers

A method takes `self`. The body decides whether it reads, mutates or consumes. A trait method declaration has no body, so
it states its receiver: `&self` when no implementation changes it, `&mut self` when one does.

```wj
// expect: 7
struct Total {
    sum: int,
}

impl Total {
    fn add(self, n: int) {
        self.sum += n
    }

    fn value(self) -> int {
        self.sum
    }
}

fn main() {
    let mut t = Total { sum: 0 }
    t.add(2)
    t.add(5)
    println("${t.value()}")
}
```

## Drops

A value is dropped where its last owner ends. Locals drop in reverse declaration order at the end of their block, each
loop iteration drops its own locals, and a temporary passed to a function drops when the callee returns. A type with a
`drop` method in `impl Drop` runs it at those points.

```wj
// expect: body 0
// expect: drop 0
// expect: body 1
// expect: drop 1
struct Guard {
    name: string,
}

impl Drop for Guard {
    fn drop(self) {
        println("drop ${self.name}")
    }
}

fn main() {
    for i in 0..2 {
        let g = Guard { name: "${i}" }
        println("body ${g.name}")
    }
}
```

## Known limits

The inference is owned by one component and is still being tightened; the places it is incomplete are listed in
[Known gaps](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/known-gaps.md). When the compiled path rejects a
program the interpreter accepts, a report with a minimal repro is the most useful thing you can send; `wj reduce`
makes one.

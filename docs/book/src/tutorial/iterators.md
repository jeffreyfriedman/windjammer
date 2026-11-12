# Iterators
<!-- feature: collections -->

An iterator produces items one at a time. Most collection work is a short chain: start from a list, a range or a map,
shape the items with adapters, and finish with something that consumes them. In Windjammer the adapters start at the
collection itself, with no `.iter()` first, and a chain reads top to bottom, much like [pipe notation](pipe-and-expressions.md).

## Adapters change the items

`map` transforms each item, `filter` keeps some of them, `skip` and `take` cut a window, and `rev` reverses. `collect` gathers them into a list of the type you name:

```wj
// expect: [2, 4, 6, 8]
fn main() {
    let v = [1, 2, 3, 4]
    let doubled: Vec<int> = v.map(|x| x * 2).collect()
    println("${doubled:?}")
}
```

[Run this in the playground](../playground/index.html#lang=wj&code=Ly8gZXhwZWN0OiBbMiwgNCwgNiwgOF0KZm4gbWFpbigpIHsKICAgIGxldCB2ID0gWzEsIDIsIDMsIDRdCiAgICBsZXQgZG91YmxlZDogVmVjPGludD4gPSB2Lm1hcCh8eHwgeCAqIDIpLmNvbGxlY3QoKQogICAgcHJpbnRsbigiJHtkb3VibGVkOj99IikKfQo)

`filter` hands your closure a reference to each item, which is why the closures below write `*x`; follow it with
`.cloned()` when you want to collect values (see [Collections and strings](collections-and-strings.md)). Ranges are iterators too, so `skip`, `take` and `rev`
cut and reverse them:

```wj
// expect: [3, 2, 1] [3, 4, 5]
fn main() {
    let down: Vec<int> = (1..=3).rev().collect()
    let window: Vec<int> = (1..=10).skip(2).take(3).collect()
    println("${down:?} ${window:?}")
}
```

[Run this in the playground](../playground/index.html#lang=wj&code=Ly8gZXhwZWN0OiBbMywgMiwgMV0gWzMsIDQsIDVdCmZuIG1haW4oKSB7CiAgICBsZXQgZG93bjogVmVjPGludD4gPSAoMS4uPTMpLnJldigpLmNvbGxlY3QoKQogICAgbGV0IHdpbmRvdzogVmVjPGludD4gPSAoMS4uPTEwKS5za2lwKDIpLnRha2UoMykuY29sbGVjdCgpCiAgICBwcmludGxuKCIke2Rvd246P30gJHt3aW5kb3c6P30iKQp9Cg)

## Consumers produce one answer

`fold` threads a running value through the items: give it a start and a closure from the running value and the next item.
`sum`, `count`, `any`, `all` and `contains` are the common special cases:

```wj
// expect: 10 24
fn main() {
    let v = [1, 2, 3, 4]
    let total = v.fold(0, |acc, x| acc + x)
    let product = v.fold(1, |acc, x| acc * x)
    println("${total} ${product}")
}
```

[Run this in the playground](../playground/index.html#lang=wj&code=Ly8gZXhwZWN0OiAxMCAyNApmbiBtYWluKCkgewogICAgbGV0IHYgPSBbMSwgMiwgMywgNF0KICAgIGxldCB0b3RhbCA9IHYuZm9sZCgwLCB8YWNjLCB4fCBhY2MgKyB4KQogICAgbGV0IHByb2R1Y3QgPSB2LmZvbGQoMSwgfGFjYywgeHwgYWNjICogeCkKICAgIHByaW50bG4oIiR7dG90YWx9ICR7cHJvZHVjdH0iKQp9Cg)

```wj
// expect: 3 of 5 are odd
fn main() {
    let v = [1, 2, 3, 4, 5]
    let odd = v.filter(|x| *x % 2 == 1).count()
    println("${odd} of ${v.len()} are odd")
}
```

[Run this in the playground](../playground/index.html#lang=wj&code=Ly8gZXhwZWN0OiAzIG9mIDUgYXJlIG9kZApmbiBtYWluKCkgewogICAgbGV0IHYgPSBbMSwgMiwgMywgNCwgNV0KICAgIGxldCBvZGQgPSB2LmZpbHRlcih8eHwgKnggJSAyID09IDEpLmNvdW50KCkKICAgIHByaW50bG4oIiR7b2RkfSBvZiAke3YubGVuKCl9IGFyZSBvZGQiKQp9Cg)

## Chains

Adapters compose. Keep the even numbers, square them, add them up. The closures are the ones from
[Closures](closures.md); `cloned` turns the kept references into values, and each step names the type it collects:

```wj
// expect: 20
fn main() {
    let v = [1, 2, 3, 4]
    let evens: Vec<int> = v.filter(|x| *x % 2 == 0).cloned().collect()
    let squares: Vec<int> = evens.map(|x| x * x).collect()
    println("${squares.sum::<int>()}")
}
```

[Run this in the playground](../playground/index.html#lang=wj&code=Ly8gZXhwZWN0OiAyMApmbiBtYWluKCkgewogICAgbGV0IHYgPSBbMSwgMiwgMywgNF0KICAgIGxldCBldmVuczogVmVjPGludD4gPSB2LmZpbHRlcih8eHwgKnggJSAyID09IDApLmNsb25lZCgpLmNvbGxlY3QoKQogICAgbGV0IHNxdWFyZXM6IFZlYzxpbnQ-ID0gZXZlbnMubWFwKHx4fCB4ICogeCkuY29sbGVjdCgpCiAgICBwcmludGxuKCIke3NxdWFyZXMuc3VtOjo8aW50PigpfSIpCn0K)

`enumerate` pairs each item with its position, and a `for` loop consumes any of these directly:
`for (i, item) in names.enumerate()`. Maps iterate as `(key, value)` pairs; their order is not specified, so sort
the keys when the order matters ([Collections and strings](collections-and-strings.md)).

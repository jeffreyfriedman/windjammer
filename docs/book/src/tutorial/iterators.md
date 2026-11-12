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

```wj
// expect: 3 of 5 are odd
fn main() {
    let v = [1, 2, 3, 4, 5]
    let odd = v.filter(|x| *x % 2 == 1).count()
    println("${odd} of ${v.len()} are odd")
}
```

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

`enumerate` pairs each item with its position, and a `for` loop consumes any of these directly:
`for (i, item) in names.enumerate()`. Maps iterate as `(key, value)` pairs; their order is not specified, so sort
the keys when the order matters ([Collections and strings](collections-and-strings.md)).

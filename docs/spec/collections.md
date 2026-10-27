# Collections (enumerable methods)

The enumerable methods of [ADR-0018](../adr/0018-expressiveness-phase-2.md) feature 1, available on lists, sets, maps (as `(key, value)` pairs), ranges and iterators. They are declared once in `src/stdlib_surface/enumerable.rs` (signatures for readers: `std/enumerable.wj`) and implemented once per backend. Closures receive each element by reference, exactly like `filter`. The Rust iterator methods with the same meaning on every backend (`map`, `filter`, `flat_map`, `scan`, `take_while`, `skip_while`, `windows`, `chunks`, `sort_by_key`, `min_by_key`, `max_by_key`, `partition`, `first`, `last`, `nth`) keep their Rust semantics.

Part of the [Windjammer language specification](README.md). Rule IDs are checked by `scripts/wj/spec_check.wj`; each rule's executable example is the code block below it, stored in `tests/spec/collections/`.

## Grouping and counting

**SPEC-COLL-001** `xs.group_by(f)` returns a `Map<K, List<T>>` from each key `f(x)` to the elements with that key, in their original order. Like every `Map`, its iteration order is unspecified; sort the keys to print deterministically.

<!-- example: tests/spec/collections/SPEC-COLL-001.wj -->
```windjammer
fn main() {
    let xs = [1, 2, 3, 4, 5, 6, 7]
    let g = xs.group_by(|x| x % 3)
    let mut keys: Vec<int> = g.keys().cloned().collect()
    keys.sort()
    for k in keys {
        let sum: int = g[&k].sum()
        println("${k}: ${g[&k].len()} ${sum}")
    }
}
```
Output:
```text
0: 2 9
1: 3 12
2: 2 7
```

**SPEC-COLL-002** `xs.count_by(f)` returns a `Map<K, int>` counting the elements per key; `xs.tally()` (alias `xs.frequencies()`) is `count_by` with the element itself as the key.

<!-- example: tests/spec/collections/SPEC-COLL-002.wj -->
```windjammer
fn main() {
    let words = ["a", "bb", "cc", "d", "eee"]
    let by_len = words.count_by(|w| w.len())
    println("${by_len[&1]} ${by_len[&2]} ${by_len[&3]}")
    let t = [3, 1, 3, 3].tally()
    println("${t[&3]} ${t[&1]}")
}
```
Output:
```text
2 2 1
3 1
```

**SPEC-COLL-003** `xs.sum_by(f)` is the sum of `f(x)` over the elements (0 for an empty collection); integer overflow panics as for `+`.

<!-- example: tests/spec/collections/SPEC-COLL-003.wj -->
```windjammer
fn main() {
    let xs = [1, 2, 3, 4]
    println("${xs.sum_by(|x| x * x)}")
}
```
Output:
```text
30
```

## Reshaping

**SPEC-COLL-004** `xs.unique()` keeps the first occurrence of each distinct element, in order; `xs.unique_by(f)` keeps the first element of each distinct key.

<!-- example: tests/spec/collections/SPEC-COLL-004.wj -->
```windjammer
fn main() {
    let u = [3, 1, 3, 2, 1].unique()
    println("${u.len()} ${u[0]} ${u[1]} ${u[2]}")
    let v = [3, 1, 3, 2, 1].unique_by(|x| x % 2)
    println("${v.len()} ${v[0]} ${v[1]}")
}
```
Output:
```text
3 3 1 2
2 3 2
```

**SPEC-COLL-005** `xs.chunk_by(f)` splits the elements into maximal runs of consecutive elements with equal keys; `xs.each_slice(n)` splits them into lists of `n` (the last may be shorter). `each_slice(0)` panics.

<!-- example: tests/spec/collections/SPEC-COLL-005.wj -->
```windjammer
fn main() {
    let runs = [1, 1, 2, 3, 3, 3].chunk_by(|x| x)
    println("${runs.len()} ${runs[0].len()} ${runs[2].len()}")
    let s = [1, 2, 3, 4, 5].each_slice(2)
    println("${s.len()} ${s[2].len()} ${s[2][0]}")
}
```
Output:
```text
3 2 3
3 1 5
```

**SPEC-COLL-006** `xs.partition_by(p)` returns `(yes, no)`: the elements for which `p` holds and the rest, each in order.

<!-- example: tests/spec/collections/SPEC-COLL-006.wj -->
```windjammer
fn main() {
    let (even, odd) = [1, 2, 3, 4, 5].partition_by(|x| x % 2 == 0)
    println("${even.len()} ${odd.len()} ${odd[2]}")
}
```
Output:
```text
2 3 5
```

**SPEC-COLL-007** `xs.intersperse(sep)` puts `sep` between consecutive elements; `xs.zip_with(ys, f)` is `[f(x0, y0), f(x1, y1), ..]` up to the shorter length.

<!-- example: tests/spec/collections/SPEC-COLL-007.wj -->
```windjammer
fn main() {
    let v = [1, 2, 3].intersperse(0)
    println("${v.len()} ${v[1]} ${v[4]}")
    let z = [1, 2, 3].zip_with([10, 20], |a, b| a + b)
    println("${z.len()} ${z[0]} ${z[1]}")
}
```
Output:
```text
5 0 3
2 11 22
```

## Searching and ordering

**SPEC-COLL-008** `xs.index_of(x)` is the index of the first element equal to `x`, or `None`.

<!-- example: tests/spec/collections/SPEC-COLL-008.wj -->
```windjammer
fn main() {
    let xs = [5, 7, 9, 7]
    match xs.index_of(7) {
        Some(i) => println("at ${i}"),
        None => println("absent"),
    }
    println("${xs.index_of(4).is_none()}")
}
```
Output:
```text
at 1
true
```

**SPEC-COLL-009** `xs.sorted()`, `xs.sorted_by_key(f)` and `xs.sorted_by_key_desc(f)` return a new sorted list and leave `xs` unchanged; `xs.sort_by_key_desc(f)` sorts in place. All sorts are stable: equal keys keep their original order, also when descending.

<!-- example: tests/spec/collections/SPEC-COLL-009.wj -->
```windjammer
fn main() {
    let xs = [3, 1, 2]
    let s = xs.sorted()
    println("${s[0]}${s[1]}${s[2]} ${xs[0]}")
    let pairs = [(1, "a"), (2, "b"), (1, "c")]
    let d = pairs.sorted_by_key_desc(|p| p.0)
    println("${d[0].1}${d[1].1}${d[2].1}")
    let mut m = [2, 9, 4]
    m.sort_by_key_desc(|x| x)
    println("${m[0]} ${m[2]}")
}
```
Output:
```text
123 3
bac
9 2
```

**SPEC-COLL-010** `xs.to_list()` collects any sequence (a list, set, range or iterator) into a new list, in iteration order.

<!-- example: tests/spec/collections/SPEC-COLL-010.wj -->
```windjammer
fn main() {
    let r = (1..4).to_list()
    println("${r.len()} ${r[2]}")
}
```
Output:
```text
3 3
```

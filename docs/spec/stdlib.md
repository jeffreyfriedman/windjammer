# Standard library surface index

The language core is small; most behaviour is library. This chapter pins the portable core and indexes the rest. Only the portable core is covered by the conformance contract; the remaining modules are Rust-backend only until ADR-0008 tiers say otherwise.

Part of the [Windjammer language specification](README.md). Rule IDs are checked by `scripts/spec_check.sh`; each rule's executable example is the code block below it, stored in `tests/spec/stdlib/`.

**Modules under `std/` (index checked against the directory):** `async`, `basic_test`, `cap`, `cli`, `collections`, `compress`, `compute`, `config`, `crypto`, `csv`, `db`, `dialog`, `encoding`, `enumerable`, `env`, `fs`, `http`, `json`, `jwt`, `log`, `map`, `math`, `mcp`, `mime`, `net`, `path`, `process`, `random`, `regex`, `runtime`, `storage`, `strings`, `subprocess`, `sync`, `test`, `test_simple`, `testing`, `time`, `url`, `uuid`, `yaml`.

Portable core (conformance-covered): `Vec`, `HashMap`, `HashSet`, `Option`, `Result`, `string` methods, iterator adaptors, `std::math`, `std::process::exit`, `std::time` (monotonic), `std::fs` round trip. Everything else (`http`, `db`, `crypto`, `jwt`, `ui`-adjacent modules, ...) is Rust-backend only.

## Vec

**SPEC-STD-001** `Vec<T>` is the growable list: `push pop len is_empty insert remove clear contains first last get reverse sort extend`; `v[i]` indexes; `pop`/`first`/`last`/`get` return `Option`.

<!-- example: tests/spec/stdlib/SPEC-STD-001.wj -->
```windjammer
fn main() {
    let mut v = [3, 1, 2]
    v.push(0)
    v.sort()
    println("${v[0]}${v[1]}${v[2]}${v[3]} ${v.len()} ${v.contains(2)}")
    let top = v.pop()
    println("${top.unwrap()} ${v.len()} ${v.first().unwrap()} ${v.last().unwrap()}")
    v.insert(1, 9)
    let gone = v.remove(0)
    v.reverse()
    println("${gone} ${v[0]} ${v[1]} ${v.get(7).is_none()} ${v.is_empty()}")
}
```
Output:
```text
0123 4 true
3 3 0 2
0 2 1 true false
```

## HashMap and HashSet

**SPEC-STD-002** `HashMap<K, V>`: `insert get contains_key remove len keys values entry`; `HashSet<T>`: `insert contains remove len`. Iteration order is unspecified (EXEC-005): sort keys before printing.

<!-- example: tests/spec/stdlib/SPEC-STD-002.wj -->
```windjammer
use std::collections::HashMap
use std::collections::HashSet
fn main() {
    let mut m = HashMap::new()
    m.insert("b", 2)
    m.insert("a", 1)
    m.insert("b", 20)
    let mut keys = m.keys().collect::<Vec<&str>>()
    keys.sort()
    println("${keys.len()} ${keys[0]} ${m.get("b").unwrap()} ${m.contains_key("z")}")
    let old = m.remove("a")
    println("${old.unwrap()} ${m.len()}")
    let mut s = HashSet::new()
    s.insert(1)
    s.insert(1)
    s.insert(2)
    println("${s.len()} ${s.contains(2)} ${s.contains(3)}")
}
```
Output:
```text
2 a 20 false
1 1
2 true false
```

## Iterators

**SPEC-STD-003** Lists, ranges and maps provide the adaptors `iter map filter enumerate zip rev take skip chain fold sum count any all min max collect`. Adaptors are lazy in the language semantics but side effects must not be relied on (a pure functional style is portable).

<!-- example: tests/spec/stdlib/SPEC-STD-003.wj -->
```windjammer
fn main() {
    let v = [1, 2, 3, 4, 5, 6]
    let ev = v.iter().filter(|x| *x % 2 == 0).map(|x| x * 10).collect::<Vec<int>>()
    println("${ev.len()} ${ev[0]} ${ev[2]}")
    let total = v.iter().fold(0, |a, x| a + x)
    println("${total} ${v.iter().sum::<int>()} ${v.iter().count()}")
    println("${v.iter().any(|x| *x > 5)} ${v.iter().all(|x| *x > 1)}")
    let pairs = v.iter().zip(ev.iter()).map(|(a, b)| a + b).collect::<Vec<int>>()
    println("${pairs[0]} ${pairs.len()}")
    let r = v.iter().rev().take(2).collect::<Vec<&int>>()
    println("${r[0]} ${r[1]}")
}
```
Output:
```text
3 20 60
21 21 6
true false
21 3
6 5
```

## Sorting

**SPEC-STD-004** `sort()` orders ascending by `<`; `sort_by(|a, b| ...)` takes a comparison returning an ordering; `sort` is stable.

<!-- example: tests/spec/stdlib/SPEC-STD-004.wj -->
```windjammer
fn main() {
    let mut v = [5, 2, 9, 1]
    v.sort()
    println("${v[0]}${v[1]}${v[2]}${v[3]}")
    v.sort_by(|a, b| b.cmp(a))
    println("${v[0]}${v[1]}${v[2]}${v[3]}")
}
```
Output:
```text
1259
9521
```

## Math

**SPEC-STD-005** `std::math` offers `sqrt pow abs floor ceil round min max sin cos` on `float`, and `abs min max pow` on `int`; results follow IEEE-754 double semantics.

<!-- example: tests/spec/stdlib/SPEC-STD-005.wj -->
```windjammer
fn main() {
    let a = 16.0
    println("${a.sqrt()} ${2.0.powf(10.0)} ${(-2.5).abs()} ${2.7.floor()} ${2.1.ceil()} ${2.5.round()}")
    println("${(-7).abs()} ${3.min(8)} ${3.max(8)}")
}
```
Output:
```text
4 1024 2.5 2 3 3
7 3 8
```

## Index of std modules

**SPEC-STD-006** The standard library is the set of modules under `std/` (see stdlib.md for the machine-checked index). (untestable: index completeness is checked by tests/spec_conformance_test.rs (stdlib_index_lists_every_std_module))

